//! B7 Organized Library: thư viện riêng `<gốc>/Library/<Class>/<Họ>/<tên file gốc>`; mặc định CHÉP (file gốc không đụng tới),
//! tên file giữ nguyên, class đổi sau khi xếp thì KHÔNG tự di chuyển — chỉ "Sắp xếp lại" có xem trước và Hoàn tác. Sau mỗi lần chép
//! chụp một gói khôi phục (dữ liệu + embedding + cài đặt, không có model) vào `<gốc>/Snapshots/`.
//! Thư viện nằm ở ổ máy hoặc ổ ngoài; chọn ổ cloud chỉ cảnh báo. Không có thao tác nào ở đây xoá file của người dùng.
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use fontatlas_store::{OrganizeCandidate, OrganizedFile, SourceKind, cloud_provider_of, is_dataless};

use crate::dto::library::FaceKeyDto;
use crate::dto::organized::*;
use crate::error::CmdError;
use crate::services::{cloud_download, keys, library, sources};
use crate::state::AppState;

pub const LIBRARY_DIR: &str = "Library";
pub const SNAPSHOT_DIR: &str = "Snapshots";
pub const MARKER: &str = ".fontatlas-library";
pub const SOURCE_NAME: &str = "Organized Library";
/// Giữ bấy nhiêu gói khôi phục gần nhất.
pub const KEEP_SNAPSHOTS: usize = 5;
pub const SETTING_ROOT: &str = keys::ORGANIZED_ROOT;

/// Một lượt chép tại một thời điểm.
#[derive(Default)]
pub struct OrganizeJob(Mutex<Option<Arc<AtomicBool>>>);

struct Guard<'a>(&'a OrganizeJob);

impl OrganizeJob {
    fn begin(&self) -> Option<(Arc<AtomicBool>, Guard<'_>)> {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_some() {
            return None;
        }
        let f = Arc::new(AtomicBool::new(false));
        *g = Some(Arc::clone(&f));
        Some((f, Guard(self)))
    }
    pub fn cancel(&self) -> bool {
        match self.0.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            Some(f) => {
                f.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        *self.0.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

pub fn cancel(state: &AppState) -> bool {
    state.organize.cancel()
}

// ---------- tên thư mục / file ----------

/// Một thành phần đường dẫn an toàn trên mọi hệ tệp: bỏ `/ \ : * ? " < > |` và ký tự điều khiển, không bắt đầu/kết thúc bằng
/// dấu cách hay chấm, tối đa 80 ký tự; rỗng → `fallback`.
pub fn safe_component(s: &str, fallback: &str) -> String {
    let mut out: String = s.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { '_' } else { c }).collect();
    out = out.trim_matches(|c: char| c == ' ' || c == '.').to_string();
    if out.chars().count() > 80 {
        out = out.chars().take(80).collect::<String>().trim_end().to_string();
    }
    if out.is_empty() { fallback.to_string() } else { out }
}

/// Tên thư mục class hiển thị trong thư viện.
pub fn class_folder(class_key: &str) -> &'static str {
    match class_key {
        "serif" => "Serif",
        "slab" => "Slab",
        "sans" => "Sans",
        "mono" => "Mono",
        "script" => "Script",
        "handwriting" => "Handwriting",
        "display" => "Display",
        "blackletter" => "Blackletter",
        "symbol" => "Symbol",
        _ => "Unclassified",
    }
}

fn hash8(h: &[u8; 32]) -> String {
    h[..4].iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_of(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// `dir/name`; nếu đã bị chiếm (trên đĩa hoặc trong sổ) thì `dir/stem-<hash8>.ext`, rồi thêm số thứ tự.
pub(crate) fn free_dest(dir: &Path, file_name: &str, hash: &[u8; 32], taken: &dyn Fn(&Path) -> bool) -> PathBuf {
    let first = dir.join(file_name);
    if !taken(&first) {
        return first;
    }
    let p = Path::new(file_name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("font");
    let ext = p.extension().and_then(|e| e.to_str());
    let mk = |suffix: String| match ext {
        Some(e) => dir.join(format!("{stem}-{suffix}.{e}")),
        None => dir.join(format!("{stem}-{suffix}")),
    };
    let c = mk(hash8(hash));
    if !taken(&c) {
        return c;
    }
    (2..).map(|n| mk(format!("{}-{n}", hash8(hash)))).find(|c| !taken(c)).unwrap()
}

// ---------- trạng thái ----------

fn root_of(state: &AppState) -> Result<Option<PathBuf>, CmdError> {
    Ok(state.with_user(|u| Ok(u.setting(SETTING_ROOT)?))?.filter(|s| !s.is_empty()).map(PathBuf::from))
}

fn need_root(state: &AppState) -> Result<PathBuf, CmdError> {
    root_of(state)?.ok_or_else(|| CmdError::new("not_configured", "chưa có Organized Library"))
}

/// `<gốc>/Library` dạng chuỗi (không dấu `/` cuối) nếu đã có Organized Library.
pub fn library_prefix(state: &AppState) -> Option<String> {
    root_of(state).ok().flatten().map(|r| lib_dir(&r).to_string_lossy().into_owned())
}

fn lib_dir(root: &Path) -> PathBuf {
    root.join(LIBRARY_DIR)
}

fn free_bytes(p: &Path) -> u64 {
    let mut q = Some(p);
    while let Some(d) = q {
        if d.exists() {
            return fontatlas_embed::disk_free(d).unwrap_or(0);
        }
        q = d.parent();
    }
    0
}

fn snapshots(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(root.join(SNAPSHOT_DIR))
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.join(fontatlas_store::MANIFEST_FILE).exists()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

pub fn check_location(state: &AppState, path: &str) -> Result<OrganizedLocationCheckDto, CmdError> {
    let p = PathBuf::from(path);
    let provider = cloud_provider_of(&p);
    let mut problem = None;
    if p.exists() && !p.is_dir() {
        problem = Some("not_a_directory".to_string());
    }
    // lồng nhau với nguồn thư mục có sẵn
    let lib = lib_dir(&p);
    let nested = state.with_user(|u| Ok(u.sources()?))?.iter().filter(|s| s.kind == SourceKind::Directory).filter_map(|s| s.path.as_deref()).any(|sp| {
        let sp = Path::new(sp);
        sp.starts_with(&lib) || lib.starts_with(sp)
    });
    if nested && problem.is_none() {
        problem = Some("nested_source".into());
    }
    let writable = problem.as_deref() != Some("not_a_directory") && {
        let probe = p.join(format!(".fontatlas-write-test-{}", std::process::id()));
        let created = !p.exists();
        let ok = std::fs::create_dir_all(&p).is_ok() && std::fs::write(&probe, b"x").is_ok();
        let _ = std::fs::remove_file(&probe);
        if created && ok {
            let _ = std::fs::remove_dir(&p);
        }
        ok
    };
    if !writable && problem.is_none() {
        problem = Some("not_writable".into());
    }
    Ok(OrganizedLocationCheckDto {
        path: path.to_string(),
        writable,
        is_cloud: provider.is_some(),
        cloud_provider: provider.map(|c| c.as_str().to_string()),
        free_bytes: free_bytes(&p),
        nested_source: nested,
        problem,
    })
}

pub fn status(state: &AppState) -> Result<OrganizedStatusDto, CmdError> {
    let Some(root) = root_of(state)? else {
        return Ok(OrganizedStatusDto {
            configured: false,
            root: None,
            library_dir: None,
            source_id: None,
            files: 0,
            bytes: 0,
            free_bytes: 0,
            is_cloud: false,
            available: false,
            snapshots: 0,
            last_snapshot: None,
        });
    };
    let lib = lib_dir(&root);
    let (files, bytes) = state.with_store(|s| {
        let f = s.organized_files()?;
        let bytes: u64 = f.iter().map(|x| std::fs::metadata(&x.dest_path).map(|m| m.len()).unwrap_or(0)).sum();
        Ok((f.len() as u32, bytes))
    })?;
    let source_id = state
        .with_user(|u| Ok(u.sources()?.into_iter().find(|s| s.path.as_deref() == Some(&*lib.to_string_lossy())).map(|s| s.id)))?;
    let snaps = snapshots(&root);
    Ok(OrganizedStatusDto {
        configured: true,
        root: Some(root.to_string_lossy().into_owned()),
        library_dir: Some(lib.to_string_lossy().into_owned()),
        source_id,
        files,
        bytes,
        free_bytes: free_bytes(&root),
        is_cloud: cloud_provider_of(&root).is_some(),
        available: lib.is_dir(),
        snapshots: snaps.len() as u32,
        last_snapshot: snaps.last().map(|p| p.to_string_lossy().into_owned()),
    })
}

/// Tạo Organized Library tại `path` (thư mục gốc): `Library/`, `Snapshots/`, file đánh dấu; đăng ký `Library/` làm nguồn thư mục.
pub fn create(state: &Arc<AppState>, path: &str) -> Result<OrganizedStatusDto, CmdError> {
    if root_of(state)?.is_some() {
        return Err(CmdError::new("already_configured", "đã có Organized Library"));
    }
    let chk = check_location(state, path)?;
    if let Some(p) = &chk.problem {
        return Err(CmdError::new(p, format!("không dùng được vị trí này: {p}")).with_detail(serde_json::json!({ "path": path })));
    }
    let root = PathBuf::from(path);
    let lib = lib_dir(&root);
    std::fs::create_dir_all(&lib).map_err(|e| CmdError::app_io("organized_create", &e))?;
    std::fs::create_dir_all(root.join(SNAPSHOT_DIR)).map_err(|e| CmdError::app_io("organized_create", &e))?;
    let marker = serde_json::json!({ "format": 1, "createdAt": crate::services::dup_resolve::now_ms() });
    std::fs::write(lib.join(MARKER), marker.to_string()).map_err(|e| CmdError::app_io("organized_create", &e))?;
    sources::add_source(state, &lib.to_string_lossy())?;
    state.with_user(|u| Ok(u.set_setting(SETTING_ROOT, &root.to_string_lossy())?))?;
    crate::services::duplicates::invalidate_cache();
    status(state)
}

// ---------- kế hoạch chép ----------

struct Item {
    cand: OrganizeCandidate,
    class_key: String,
    dest: PathBuf,
    /// Còn là placeholder trên ổ cloud: phải tải về trước khi chép (chỉ có khi `downloadFirst`).
    cloud: bool,
}

struct Plan {
    items: Vec<Item>,
    already: u32,
    /// Font cloud chưa tải trong phạm vi (đã vào `items` nếu `downloadFirst`, bị bỏ qua nếu không).
    cloud: u32,
    cloud_bytes: u64,
    bytes: u64,
}

fn class_key_of(eff: &library::EffMap, c: &OrganizeCandidate) -> String {
    c.face_ids.first().and_then(|id| eff.get(id)).map(|e| e.class.clone()).unwrap_or_else(|| "uncertain".into())
}

fn plan(state: &AppState, scope: &OrganizeScopeDto) -> Result<(Plan, PathBuf), CmdError> {
    let root = need_root(state)?;
    let lib = lib_dir(&root);
    let eff = library::effective_classes(state)?;
    let in_scope: Option<Vec<PathBuf>> = match &scope.source_ids {
        Some(ids) if !ids.is_empty() => {
            let all = state.with_user(|u| Ok(u.sources()?))?;
            Some(all.into_iter().filter(|s| ids.contains(&s.id)).filter_map(|s| s.path.map(PathBuf::from)).collect())
        }
        _ => None,
    };
    let cands = state.with_store(|s| Ok(s.organize_candidates(&lib.to_string_lossy())?))?;
    let ledger: HashSet<String> = state.with_store(|s| Ok(s.organized_files()?.into_iter().map(|f| f.dest_path).collect()))?;
    let copied: HashSet<[u8; 32]> = state.with_store(|s| Ok(s.organized_files()?.into_iter().map(|f| f.hash).collect()))?;
    let mut planned: HashSet<PathBuf> = HashSet::new();
    let mut p = Plan { items: Vec::new(), already: 0, cloud: 0, cloud_bytes: 0, bytes: 0 };
    for c in cands {
        if c.in_library || copied.contains(&c.hash) {
            p.already += 1;
            continue;
        }
        if c.src_path.is_empty() {
            continue;
        }
        if let Some(roots) = &in_scope
            && !roots.iter().any(|r| Path::new(&c.src_path).starts_with(r))
        {
            continue;
        }
        let cloud = c.cloud_only || is_dataless(Path::new(&c.src_path));
        if cloud {
            p.cloud += 1;
            p.cloud_bytes += c.size.max(0) as u64;
            if !scope.download_first {
                continue;
            }
        }
        let class_key = class_key_of(&eff, &c);
        let dir = lib.join(class_folder(&class_key)).join(safe_component(&c.family, "Unnamed"));
        let name = safe_component(Path::new(&c.src_path).file_name().and_then(|n| n.to_str()).unwrap_or("font"), "font");
        let dest = free_dest(&dir, &name, &c.hash, &|d: &Path| d.exists() || ledger.contains(&*d.to_string_lossy()) || planned.contains(d));
        planned.insert(dest.clone());
        p.bytes += c.size.max(0) as u64;
        p.items.push(Item { cand: c, class_key, dest, cloud });
    }
    Ok((p, lib))
}

pub fn import_plan(state: &AppState, scope: &OrganizeScopeDto) -> Result<OrganizePlanDto, CmdError> {
    let (p, lib) = plan(state, scope)?;
    let free = free_bytes(&lib);
    // ổ chứa file cloud (thường là ổ hệ thống): lấy từ font cloud đầu tiên
    let cloud_free = p.items.iter().find(|i| i.cloud).map(|i| free_bytes(Path::new(&i.cand.src_path))).unwrap_or(u64::MAX);
    let dl_bytes: u64 = p.items.iter().filter(|i| i.cloud).map(|i| i.cand.size.max(0) as u64).sum();
    Ok(OrganizePlanDto {
        to_copy: p.items.len() as u32,
        bytes: p.bytes,
        already_there: p.already,
        cloud_not_downloaded: p.cloud,
        needs_download: p.cloud,
        download_bytes: p.cloud_bytes,
        enough_space_for_download: cloud_free >= dl_bytes + dl_bytes / 20,
        free_bytes: free,
        enough_space: free >= p.bytes + p.bytes / 20,
        sample: p
            .items
            .iter()
            .take(50)
            .map(|i| OrganizePlanItemDto {
                src: i.cand.src_path.clone(),
                dest: i.dest.to_string_lossy().into_owned(),
                class_folder: class_folder(&i.class_key).to_string(),
            })
            .collect(),
    })
}

/// Chép an toàn một file: ghi file tạm cạnh đích, đồng bộ, kiểm BLAKE3 = hash đã biết, rồi đổi tên nguyên tử.
pub fn copy_verified(src: &Path, dest: &Path, expect: &[u8; 32]) -> Result<(), (String, std::io::Error)> {
    let io = |e: std::io::Error| (if e.kind() == std::io::ErrorKind::StorageFull { "disk_full" } else { "io" }.to_string(), e);
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d).map_err(io)?;
    }
    let tmp = dest.with_file_name(format!(".{}.partial", dest.file_name().and_then(|n| n.to_str()).unwrap_or("font")));
    let run = || -> Result<(), (String, std::io::Error)> {
        std::fs::copy(src, &tmp).map_err(io)?;
        std::fs::File::open(&tmp).and_then(|f| f.sync_all()).map_err(io)?;
        let mut h = blake3::Hasher::new();
        let mut f = std::fs::File::open(&tmp).map_err(io)?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf).map_err(io)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        if h.finalize().as_bytes() != expect {
            return Err(("verify_failed".into(), std::io::Error::other("BLAKE3 của bản chép không khớp (file nguồn đổi giữa chừng?)")));
        }
        std::fs::rename(&tmp, dest).map_err(io)
    };
    let r = run();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// Số font cloud tải về mỗi lô trước khi chép (chép ngay khi một lô xong, không đợi tải hết cả thư viện).
pub const DOWNLOAD_BATCH: usize = 16;

/// Chép vào Organized Library. Font cloud chưa tải: bỏ qua (mặc định) hoặc, với `downloadFirst`, tải về qua làn tải của
/// "Tải về máy" (cổng chung, tối đa 4 cùng lúc, huỷ được) rồi chép; file tải về giữ nguyên trạng thái OneDrive/iCloud của nó
/// (không đổi "Always keep on this device"). Lỗi gộp theo mã.
pub fn import(
    state: &Arc<AppState>,
    scope: &OrganizeScopeDto,
    progress: &mut dyn FnMut(OrganizeProgressDto),
) -> Result<OrganizeResultDto, CmdError> {
    import_with(
        state,
        scope,
        &|p, permit, cancel| cloud_download::materialize(p, cloud_download::BULK_FILE_TIMEOUT, permit, cancel),
        progress,
    )
}

/// Như `import` nhưng thay được cách tải (test giả lập file dataless, không cần ổ cloud thật).
pub fn import_with(
    state: &Arc<AppState>,
    scope: &OrganizeScopeDto,
    fetch: &cloud_download::Fetch<'_>,
    progress: &mut dyn FnMut(OrganizeProgressDto),
) -> Result<OrganizeResultDto, CmdError> {
    let Some((cancel, _g)) = state.organize.begin() else {
        return Err(CmdError::busy("đang chép vào Organized Library"));
    };
    let root = need_root(state)?;
    let (p, lib) = plan(state, scope)?;
    if !lib.is_dir() {
        return Err(CmdError::new("source_unavailable", "thư mục Organized Library không truy cập được (ổ chưa cắm?)"));
    }
    let free = free_bytes(&lib);
    if free < p.bytes + p.bytes / 20 {
        return Err(CmdError::new("disk_full", "không đủ chỗ trống để chép").with_detail(serde_json::json!({ "needed": p.bytes, "free": free })));
    }
    let total = p.items.len() as u32;
    let download_total = p.items.iter().filter(|i| i.cloud).count() as u32;
    let st = std::cell::RefCell::new(Counters::default());
    let emit = |phase: OrganizePhaseDto, progress: &mut dyn FnMut(OrganizeProgressDto)| {
        let c = st.borrow();
        progress(OrganizeProgressDto { phase, done: c.done, total, bytes_done: c.bytes, failed: c.failed, downloaded: c.downloaded, download_total });
    };
    emit(OrganizePhaseDto::Copying, progress);
    let mut cancelled = false;

    // font đã có trên máy trước, font cloud theo lô sau (tải rồi chép từng lô)
    let (local, cloud): (Vec<&Item>, Vec<&Item>) = p.items.iter().partition(|i| !i.cloud);
    let mut work: Vec<Vec<&Item>> = vec![local];
    work.extend(cloud.chunks(DOWNLOAD_BATCH).map(|c| c.to_vec()));
    'outer: for (bi, batch) in work.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }
        // lô cloud: tải về trước (bi > 0 là các lô cloud)
        let mut failed_dl: std::collections::HashMap<PathBuf, String> = std::collections::HashMap::new();
        let mut ok_dl: HashSet<PathBuf> = HashSet::new();
        if bi > 0 {
            let paths: Vec<PathBuf> = batch.iter().map(|i| PathBuf::from(&i.cand.src_path)).collect();
            let errs = std::sync::Mutex::new(std::collections::HashMap::<PathBuf, String>::new());
            let oks = std::sync::Mutex::new(HashSet::<PathBuf>::new());
            let wrapped = |path: &Path, permit: crate::services::cloud::Permit, c: &AtomicBool| {
                let r = fetch(path, permit, c);
                if r.is_ok() {
                    oks.lock().unwrap_or_else(|x| x.into_inner()).insert(path.to_path_buf());
                }
                if let Err(e) = &r
                    && !matches!(e, fontatlas_activate::EnsureLocalError::Cancelled)
                {
                    errs.lock().unwrap_or_else(|x| x.into_inner()).insert(path.to_path_buf(), e.code().to_string());
                }
                r
            };
            let base = st.borrow().downloaded;
            emit(OrganizePhaseDto::Downloading, progress);
            let _ = cloud_download::run(
                &paths,
                crate::services::cloud::global_gate(),
                cloud_download::BULK_WORKERS,
                &cancel,
                &wrapped,
                &mut |dp| {
                    st.borrow_mut().downloaded = base + dp.done;
                    emit(OrganizePhaseDto::Downloading, progress);
                },
            );
            failed_dl = errs.into_inner().unwrap_or_else(|x| x.into_inner());
            ok_dl = oks.into_inner().unwrap_or_else(|x| x.into_inner());
            if cancel.load(Ordering::SeqCst) {
                // font đã tải xong trong lô này vẫn được chép bên dưới; font chưa kịp tải thì bỏ
                cancelled = true;
            }
        }
        for it in batch {
            let src = Path::new(&it.cand.src_path);
            let mut c = st.borrow_mut();
            if bi > 0
                && let Some(code) = failed_dl.get(src)
            {
                c.issue(code, &it.cand.src_path);
            } else if bi > 0 && cancelled && !ok_dl.contains(src) {
                // huỷ trước khi tải xong font này: không phải lỗi, chỉ chưa làm
                continue;
            } else if !src.exists() {
                c.issue("source_missing", &it.cand.src_path);
            } else if is_dataless(src) {
                c.issue("cloud_not_downloaded", &it.cand.src_path);
            } else {
                drop(c);
                match copy_verified(src, &it.dest, &it.cand.hash) {
                    Ok(()) => {
                        let rec = OrganizedFile {
                            hash: it.cand.hash,
                            dest_path: it.dest.to_string_lossy().into_owned(),
                            src_path: it.cand.src_path.clone(),
                            class_key: it.class_key.clone(),
                            family: safe_component(&it.cand.family, "Unnamed"),
                            copied_at: crate::services::dup_resolve::now_ms(),
                        };
                        state.with_store(|s| Ok(s.organized_record(&rec)?))?;
                        let mut c = st.borrow_mut();
                        c.copied += 1;
                        c.bytes += it.cand.size.max(0) as u64;
                    }
                    Err((code, e)) => {
                        log::warn!("organized: chép {} lỗi ({code}): {e}", it.cand.src_path);
                        st.borrow_mut().issue(&code, &it.cand.src_path);
                        if code == "disk_full" {
                            st.borrow_mut().done += 1;
                            emit(OrganizePhaseDto::Copying, progress);
                            break 'outer; // hết chỗ: các file sau cũng sẽ hỏng
                        }
                    }
                }
                st.borrow_mut().done += 1;
                emit(OrganizePhaseDto::Copying, progress);
                if cancel.load(Ordering::SeqCst) {
                    cancelled = true;
                }
                continue;
            }
            c.done += 1;
            drop(c);
            emit(OrganizePhaseDto::Copying, progress);
        }
        if cancelled {
            break;
        }
    }
    let c = st.into_inner();
    let mut snapshot = None;
    if c.copied > 0 {
        crate::services::duplicates::invalidate_cache();
        if let Some(src_id) = status(state)?.source_id {
            state.scans.enqueue_follow_up(state, &[src_id]);
        }
        snapshot = snapshot_now(state).ok();
    }
    let _ = root;
    let skipped_cloud = if scope.download_first { 0 } else { p.cloud };
    let (groups, downloaded) = (c.groups(), c.downloaded.saturating_sub(c.download_failed()));
    Ok(OrganizeResultDto {
        copied: c.copied,
        bytes: c.bytes,
        already_there: p.already,
        issue_count: c.failed,
        issue_groups: groups,
        issues: c.issues,
        downloaded,
        skipped_cloud,
        cancelled,
        snapshot,
    })
}

#[derive(Default)]
struct Counters {
    done: u32,
    copied: u32,
    failed: u32,
    bytes: u64,
    downloaded: u32,
    issues: Vec<OrganizeIssueDto>,
    by_code: std::collections::BTreeMap<String, (u32, String)>,
}

impl Counters {
    fn issue(&mut self, code: &str, path: &str) {
        self.failed += 1;
        if self.issues.len() < 200 {
            self.issues.push(OrganizeIssueDto { path: path.into(), code: code.into() });
        }
        let e = self.by_code.entry(code.to_string()).or_insert((0, path.to_string()));
        e.0 += 1;
    }
    /// Số font tải lỗi (mã của lượt tải, không phải của lượt chép).
    fn download_failed(&self) -> u32 {
        self.by_code.iter().filter(|(c, _)| matches!(c.as_str(), "cloud_timeout" | "cloud_download_failed" | "cloud_stub" | "missing_file" | "cloud_busy")).map(|(_, v)| v.0).sum()
    }
    fn groups(&self) -> Vec<OrganizeIssueGroupDto> {
        let mut v: Vec<OrganizeIssueGroupDto> =
            self.by_code.iter().map(|(code, (count, first))| OrganizeIssueGroupDto { code: code.clone(), count: *count, first_path: first.clone() }).collect();
        v.sort_by(|a, b| b.count.cmp(&a.count).then(a.code.cmp(&b.code)));
        v
    }
}

// ---------- gói khôi phục ----------

/// Chụp gói khôi phục vào `<gốc>/Snapshots/<thời điểm>/` và giữ `KEEP_SNAPSHOTS` gói mới nhất. Trả thư mục gói.
pub fn snapshot_now(state: &AppState) -> Result<String, CmdError> {
    let root = need_root(state)?;
    let dir = root.join(SNAPSHOT_DIR).join(format!("{}", crate::services::dup_resolve::now_ms()));
    state.with_store(|s| {
        s.export_pack(&dir)?;
        Ok(())
    })?;
    let all = snapshots(&root);
    if all.len() > KEEP_SNAPSHOTS {
        for old in &all[..all.len() - KEEP_SNAPSHOTS] {
            let _ = std::fs::remove_dir_all(old); // chỉ gói do chính app tạo trong Snapshots/
        }
    }
    Ok(dir.to_string_lossy().into_owned())
}

pub fn pack_inspect(path: &str) -> Result<PackInfoDto, CmdError> {
    let m = fontatlas_store::verify_pack(Path::new(path)).map_err(|e| CmdError::new("invalid_pack", e.to_string()))?;
    Ok(PackInfoDto {
        path: path.to_string(),
        created_at: m.created_at,
        app_version: m.app_version,
        files: m.counts.files,
        faces: m.counts.faces,
        embeddings: m.counts.embeddings,
        sources: m.counts.sources,
        source_roots: m.source_roots,
    })
}

const PENDING_FILE: &str = "pending-restore.json";

/// Xếp lệnh khôi phục chạy ở LẦN KHỞI ĐỘNG SAU (trước khi mở DB): DB đang dùng được dời sang `pre-restore-*`, không xoá.
/// Gói được kiểm toàn vẹn ngay bây giờ để lỗi hiện ra sớm.
pub fn pack_restore_stage(state: &AppState, path: &str, remap: &[PathRemapDto]) -> Result<PackInfoDto, CmdError> {
    let info = pack_inspect(path)?;
    let job = serde_json::json!({ "pack": path, "remap": remap.iter().map(|r| [r.from.clone(), r.to.clone()]).collect::<Vec<_>>() });
    std::fs::write(state.dirs.data.join(PENDING_FILE), job.to_string()).map_err(|e| CmdError::app_io("pack_restore_stage", &e))?;
    Ok(info)
}

pub fn pack_restore_pending(state: &AppState) -> Option<PackInfoDto> {
    let raw = std::fs::read(state.dirs.data.join(PENDING_FILE)).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    pack_inspect(v["pack"].as_str()?).ok()
}

pub fn pack_restore_cancel(state: &AppState) -> bool {
    std::fs::remove_file(state.dirs.data.join(PENDING_FILE)).is_ok()
}

/// Chạy lệnh khôi phục đang chờ (gọi lúc khởi động, TRƯỚC `Store::open`). Trả `Some(thư mục sao lưu DB cũ)` khi đã khôi phục.
pub fn apply_pending_restore(data_dir: &Path) -> Option<Result<Option<PathBuf>, String>> {
    let f = data_dir.join(PENDING_FILE);
    let raw = std::fs::read(&f).ok()?;
    let done = |r: Result<Option<PathBuf>, String>| {
        let _ = std::fs::rename(&f, data_dir.join(if r.is_ok() { "pending-restore.done.json" } else { "pending-restore.failed.json" }));
        Some(r)
    };
    let v: serde_json::Value = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(e) => return done(Err(e.to_string())),
    };
    let Some(pack) = v["pack"].as_str() else { return done(Err("thiếu đường dẫn gói".into())) };
    let remap: Vec<(String, String)> = v["remap"]
        .as_array()
        .map(|a| a.iter().filter_map(|p| Some((p[0].as_str()?.to_string(), p[1].as_str()?.to_string()))).collect())
        .unwrap_or_default();
    done(fontatlas_store::restore_pack(Path::new(pack), data_dir, &remap).map(|(_, b)| b).map_err(|e| e.to_string()))
}

// ---------- sắp xếp lại ----------

struct Move {
    hash: [u8; 32],
    from: PathBuf,
    to: PathBuf,
    class_key: String,
    family: String,
}

fn resort_moves(state: &AppState) -> Result<Vec<Move>, CmdError> {
    let root = need_root(state)?;
    let lib = lib_dir(&root);
    let eff = library::effective_classes(state)?;
    let cands = state.with_store(|s| Ok(s.organize_candidates(&lib.to_string_lossy())?))?;
    let files = state.with_store(|s| Ok(s.organized_files()?))?;
    let taken: HashSet<String> = files.iter().map(|f| f.dest_path.clone()).collect();
    let by_hash: std::collections::HashMap<[u8; 32], &OrganizeCandidate> = cands.iter().map(|c| (c.hash, c)).collect();
    let mut planned: HashSet<PathBuf> = HashSet::new();
    let mut out = Vec::new();
    for f in &files {
        let cur = PathBuf::from(&f.dest_path);
        if !cur.starts_with(&lib) || !cur.exists() {
            continue;
        }
        let Some(c) = by_hash.get(&f.hash) else { continue };
        let class_key = class_key_of(&eff, c);
        let family = safe_component(&c.family, "Unnamed");
        let dir = lib.join(class_folder(&class_key)).join(&family);
        if cur.parent() == Some(dir.as_path()) {
            continue;
        }
        let name = cur.file_name().and_then(|n| n.to_str()).unwrap_or("font").to_string();
        let to = free_dest(&dir, &name, &f.hash, &|d: &Path| d.exists() || taken.contains(&*d.to_string_lossy()) || planned.contains(d));
        planned.insert(to.clone());
        out.push(Move { hash: f.hash, from: cur, to, class_key, family });
    }
    Ok(out)
}

pub fn resort_preview(state: &AppState) -> Result<ResortPlanDto, CmdError> {
    let moves = resort_moves(state)?;
    let lib = need_root(state).map(|r| lib_dir(&r))?;
    let mut dtos = Vec::new();
    for m in moves.iter().take(200) {
        let faces = state.with_store(|s| Ok(s.face_keys_of_hash(&m.hash)?))?;
        dtos.push(ResortMoveDto {
            face: faces.first().map(FaceKeyDto::from).unwrap_or(FaceKeyDto { hash_hex: hex_of(&m.hash), face_index: 0 }),
            from: m.from.strip_prefix(&lib).unwrap_or(&m.from).to_string_lossy().into_owned(),
            to: m.to.strip_prefix(&lib).unwrap_or(&m.to).to_string_lossy().into_owned(),
            class_folder: class_folder(&m.class_key).into(),
            family: m.family.clone(),
        });
    }
    Ok(ResortPlanDto { total: moves.len() as u32, moves: dtos })
}

/// Di chuyển trong CHÍNH thư viện (cùng ổ, đổi tên nguyên tử), ghi nhật ký để hoàn tác. Dọn thư mục trống bị bỏ lại (chỉ thư mục rỗng).
pub fn resort_apply(state: &Arc<AppState>) -> Result<ResortResultDto, CmdError> {
    let Some((_c, _g)) = state.organize.begin() else {
        return Err(CmdError::busy("đang chép vào Organized Library"));
    };
    let lib = need_root(state).map(|r| lib_dir(&r))?;
    let moves = resort_moves(state)?;
    let batch = state.with_store(|s| Ok(s.organized_next_batch()?))?;
    let (mut moved, mut failed) = (0u32, 0u32);
    let mut moved_hashes: Vec<[u8; 32]> = Vec::new();
    for m in &moves {
        let ok = m.to.parent().is_none_or(|d| std::fs::create_dir_all(d).is_ok()) && std::fs::rename(&m.from, &m.to).is_ok();
        if !ok {
            failed += 1;
            continue;
        }
        let (f, t) = (m.from.to_string_lossy().into_owned(), m.to.to_string_lossy().into_owned());
        state.with_store(|s| Ok(s.organized_apply_move(batch, &m.hash, &f, &t, &m.class_key, &m.family)?))?;
        prune_empty_dirs(m.from.parent(), &lib);
        moved_hashes.push(m.hash);
        moved += 1;
    }
    relocate_active(state, &moved_hashes, &lib);
    let _ = crate::services::preview::bump_generation(state);
    Ok(ResortResultDto { batch_id: batch, moved, failed })
}

pub fn resort_undo(state: &Arc<AppState>, batch_id: i64) -> Result<ResortResultDto, CmdError> {
    let Some((_c, _g)) = state.organize.begin() else {
        return Err(CmdError::busy("đang chép vào Organized Library"));
    };
    let lib = need_root(state).map(|r| lib_dir(&r))?;
    let moves = state.with_store(|s| Ok(s.organized_moves(batch_id)?))?;
    let (mut moved, mut failed) = (0u32, 0u32);
    let mut moved_hashes: Vec<[u8; 32]> = Vec::new();
    for m in moves.iter().rev().filter(|m| !m.undone) {
        let (to, from) = (PathBuf::from(&m.to_path), PathBuf::from(&m.from_path));
        let ok = to.exists() && !from.exists() && from.parent().is_none_or(|d| std::fs::create_dir_all(d).is_ok()) && std::fs::rename(&to, &from).is_ok();
        if !ok {
            failed += 1;
            continue;
        }
        let (class_key, family) = state
            .with_store(|s| Ok(s.organized_file(&m.hash)?.map(|f| (f.class_key, f.family))))?
            .unwrap_or(("uncertain".into(), "Unnamed".into()));
        // class/họ "trước khi chuyển" = thư mục cũ; không lưu riêng nên suy từ đường dẫn cũ
        let (ck, fam) = old_class_family(&from, &lib).unwrap_or((class_key, family));
        state.with_store(|s| Ok(s.organized_mark_undone(m, &ck, &fam)?))?;
        prune_empty_dirs(to.parent(), &lib);
        moved_hashes.push(m.hash);
        moved += 1;
    }
    relocate_active(state, &moved_hashes, &lib);
    let _ = crate::services::preview::bump_generation(state);
    Ok(ResortResultDto { batch_id, moved, failed })
}

/// Sau khi file trong thư viện đổi chỗ: font đang bật trỏ đường cũ phải được đăng ký lại ở đường mới (06: `relocate`, đăng ký mới
/// trước, gỡ cũ sau; hash không đang bật thì bỏ qua). Làm một lần theo lô; lỗi chỉ ghi log — `reconcile` lần sau sửa nốt.
fn relocate_active(state: &Arc<AppState>, hashes: &[[u8; 32]], lib: &Path) {
    if hashes.is_empty() {
        return;
    }
    let mut faces: Vec<FaceKeyDto> = Vec::new();
    for h in hashes {
        if let Ok(ks) = state.with_store(|s| Ok(s.face_keys_of_hash(h)?)) {
            faces.extend(ks.iter().take(1).map(FaceKeyDto::from));
        }
    }
    if let Err(e) = crate::services::activation::relocate(state, &faces, Some(&lib.to_string_lossy()), &mut |_| {}) {
        log::debug!("organized: relocate font đang bật bỏ qua: {} {}", e.code, e.message);
    }
}

fn old_class_family(p: &Path, lib: &Path) -> Option<(String, String)> {
    let rel = p.strip_prefix(lib).ok()?;
    let mut it = rel.components();
    let class = it.next()?.as_os_str().to_string_lossy().to_lowercase();
    let fam = it.next()?.as_os_str().to_string_lossy().into_owned();
    let key = match class.as_str() {
        "unclassified" => "uncertain".to_string(),
        k => k.to_string(),
    };
    Some((key, fam))
}

fn prune_empty_dirs(mut d: Option<&Path>, stop: &Path) {
    while let Some(dir) = d {
        if dir == stop || !dir.starts_with(stop) || std::fs::remove_dir(dir).is_err() {
            break; // `remove_dir` chỉ xoá thư mục RỖNG
        }
        d = dir.parent();
    }
}
