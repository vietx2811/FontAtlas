//! Quét lại một nguồn theo diff (path, size, mtime) → BLAKE3 → chỉ phân tích hash chưa có kết quả mới.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use fontatlas_core::{ANALYZER_VERSION, CoreError, analyze_paths};
use rayon::prelude::*;
use rusqlite::params;
use serde::Serialize;

use crate::error::StoreError;
use crate::ingest::{Analysis, FaceKey, IngestItem, ScannedFile, parse_hash};
use crate::open::{Store, now_ms};

/// `pipeline_state.version` đặt khi placeholder đổi size/mtime: kết quả cũ không còn đáng tin, phân tích lại khi tải về.
pub const CLOUD_STALE_MARK: &str = "cloud-stale";
pub const FONT_EXTENSIONS: [&str; 6] = ["ttf", "otf", "ttc", "otc", "woff", "woff2"];
pub const INGEST_BATCH: usize = 256;
/// Vị trí `missing` quá hạn này (kể từ lần cuối thấy file) bị dọn khi quét xong: file đã xoá khỏi đĩa thật sự.
/// Trong thời gian ân hạn vị trí vẫn được giữ để file chuyển chỗ/ổ cắm lại khớp theo hash.
pub const MISSING_RETENTION_MS: i64 = 30 * 24 * 3600 * 1000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RescanStatus {
    #[default]
    Ok,
    /// Gốc nguồn không đọc được (ổ ngoài đã rút…): không đánh missing gì.
    SourceUnavailable,
    /// Người dùng huỷ giữa chừng: không đánh missing gì.
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanPhase {
    Walking,
    Hashing,
    Analyzing,
    MarkingMissing,
}

/// `faces_written`: số face ghi trong lô vừa xong (UI hiện font dần).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub phase: ScanPhase,
    pub done: usize,
    pub total: usize,
    pub faces_written: usize,
    /// Số font chỉ là placeholder (OneDrive/iCloud chưa tải về): không đọc, không đánh missing.
    pub not_downloaded: usize,
}

fn progress(phase: ScanPhase, done: usize, total: usize, faces_written: usize, not_downloaded: usize) -> ScanProgress {
    ScanProgress { phase, done, total, faces_written, not_downloaded }
}

enum Work {
    Known(IngestItem),
    Analyze(ScannedFile, [u8; 32]),
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RescanStats {
    pub status: RescanStatus,
    pub seen: usize,
    pub unchanged: usize,
    /// Font còn là placeholder trên đám mây (chưa tải): bỏ qua đọc, giữ dữ liệu cũ.
    pub not_downloaded: usize,
    pub relinked: usize,
    pub analyzed: usize,
    pub failed: usize,
    pub missing: usize,
    /// Vị trí `missing` quá hạn ân hạn đã bị dọn khỏi thư viện (kèm file/face không còn vị trí nào).
    #[serde(skip)]
    pub purged: usize,
    pub unreadable: Vec<(String, String)>,
}

fn is_font(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| FONT_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Các font còn là placeholder trên ổ cloud (chưa tải về) dưới `root`, sắp theo đường dẫn. Cùng bộ đi cây với lần quét
/// (không theo symlink, bỏ mục ẩn); chỉ `stat`, KHÔNG mở file nên không kích hoạt tải. Stub iCloud kiểu cũ (`.Tên.ttf.icloud`)
/// trả về bằng tên thật `Tên.ttf` (file thật chưa tồn tại).
pub fn cloud_placeholders(root: &Path) -> Vec<PathBuf> {
    walk_fonts_report(root, &os_placeholder).2.into_iter().map(|(p, _)| p).collect()
}

/// Đi cây thư mục: không theo symlink, bỏ mục ẩn, bỏ thư mục không đọc được; sắp theo đường dẫn.
pub fn walk_fonts(root: &Path) -> Vec<ScannedFile> {
    walk_fonts_report(root, &os_placeholder).0
}

/// Hàm kiểm "file giữ chỗ chưa tải về" (thay được trong test).
pub type PlaceholderProbe<'a> = &'a (dyn Fn(&Path, &std::fs::Metadata) -> bool + Sync);

/// macOS: cờ `SF_DATALESS` (OneDrive/iCloud File Provider); Windows: thuộc tính RECALL/OFFLINE.
pub fn os_placeholder(_p: &Path, _m: &std::fs::Metadata) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        const SF_DATALESS: u32 = 0x4000_0000;
        return _m.st_flags() & SF_DATALESS != 0;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const OFFLINE: u32 = 0x1000;
        const RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
        return _m.file_attributes() & (OFFLINE | RECALL_ON_DATA_ACCESS) != 0;
    }
    #[allow(unreachable_code)]
    false
}

/// Đường dẫn này là file giữ chỗ trên ổ cloud (dataless)? Chỉ `stat`, KHÔNG mở file nên không kích hoạt tải.
/// Hàm dùng chung cho mọi đường đọc nội dung ở nền: không stat được (mất, không phải file) → false.
pub fn is_dataless(path: &Path) -> bool {
    std::fs::metadata(path).map(|m| m.is_file() && os_placeholder(path, &m)).unwrap_or(false)
}

/// `".Tên.ttf.icloud"` → `Some("Tên.ttf")` khi tên thật là font (iCloud Drive thay file bằng stub ẩn).
fn icloud_real_name(name: &str) -> Option<&str> {
    let real = name.strip_prefix('.')?.strip_suffix(".icloud")?;
    is_font(Path::new(real)).then_some(real)
}

/// Như `walk_fonts`, kèm các thư mục không đọc được (font bên trong không được coi là đã xoá).
/// Placeholder tìm thấy: đường dẫn + (size, mtime) từ stat nếu có (stub `.icloud` không có file thật nên không có).
/// (size, mtime_ns, created_ns) — chỉ stat, không đọc nội dung.
type Placeholder = (PathBuf, Option<(u64, i64, Option<i64>)>);

fn walk_fonts_report(root: &Path, probe: PlaceholderProbe) -> (Vec<ScannedFile>, Vec<PathBuf>, Vec<Placeholder>) {
    let mut out = Vec::new();
    let mut placeholders = Vec::new();
    let mut failed = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            failed.push(dir);
            continue;
        };
        for e in rd {
            // lỗi ở mức mục (ổ chập chờn) → thư mục cha coi như không đọc được, KHÔNG để font trong đó thành "đã xoá"
            let Ok(e) = e else {
                failed.push(dir.clone());
                continue;
            };
            let fname = e.file_name().to_string_lossy().into_owned();
            if let Some(real) = icloud_real_name(&fname) {
                placeholders.push((dir.join(real), None));
                continue;
            }
            if fname.starts_with('.') {
                continue;
            }
            let Ok(ft) = e.file_type() else {
                failed.push(dir.clone());
                continue;
            };
            let p = e.path();
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() && is_font(&p) {
                let Ok(m) = e.metadata() else {
                    failed.push(dir.clone());
                    continue;
                };
                let mtime_ns = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(0);
                let created_ns = m
                    .created()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos() as i64);
                if probe(&p, &m) {
                    placeholders.push((p, Some((m.len(), mtime_ns, created_ns))));
                    continue;
                }
                out.push(ScannedFile { path: p, size: m.len(), mtime_ns, created_ns });
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    placeholders.sort_by(|a, b| a.0.cmp(&b.0));
    (out, failed, placeholders)
}

fn hash_file(p: &Path) -> std::io::Result<[u8; 32]> {
    let mut h = blake3::Hasher::new();
    h.update_reader(std::fs::File::open(p)?)?;
    Ok(*h.finalize().as_bytes())
}

/// Huỷ hoặc ổ rút → dừng quét, không đánh missing.
fn interrupted(root: &Path, cancel: &AtomicBool) -> Option<RescanStatus> {
    if cancel.load(Ordering::SeqCst) {
        Some(RescanStatus::Cancelled)
    } else if !root_available(root) {
        Some(RescanStatus::SourceUnavailable)
    } else {
        None
    }
}

fn root_available(root: &Path) -> bool {
    root.is_dir() && std::fs::read_dir(root).is_ok()
}

/// Hệ quả của việc gỡ một nguồn (để câu xác nhận nêu số thật).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceRemovalReport {
    /// Số vị trí file của nguồn sẽ bị bỏ khỏi thư viện.
    pub removed_locations: u32,
    /// Số face rời khỏi lưới sau khi gỡ: không còn vị trí nào ở nguồn khác mà hiện diện và chưa bị ẩn bởi Trùng lặp
    /// (face chỉ còn vị trí `missing` cũng tính — chúng không hiện trong lưới).
    pub faces_left_without_location: u32,
    /// Số Dự án (chưa xoá) có chứa face nói trên — các face đó hiện `missing`, KHÔNG bị xoá khỏi Dự án.
    pub cases_affected: u32,
    /// Số Collection có chứa face nói trên (vẫn giữ trong Collection).
    pub collections_affected: u32,
    /// Số Tag đang gán cho face nói trên (vẫn giữ).
    pub tags_affected: u32,
    /// Số font của nguồn đang BẬT; không tự tắt, người dùng tắt ở "Đang bật".
    pub active_fonts: u32,
}

/// Kết quả nhập MỘT file font lẻ ([`Store::import_files`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileImportOutcome {
    Imported,
    /// Đã có vị trí ở một nguồn (không đổi gì).
    AlreadyKnown,
    /// Placeholder trên ổ cloud, chưa tải về.
    NotDownloaded,
    /// Không còn/không phải file font.
    Missing,
    /// Không đọc được (ổ rút, bị khoá…) hoặc lỗi I/O.
    Unreadable,
}

/// Số liệu của [`Store::rebuild_library`]: những gì đã bị xoá khỏi library.db.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RebuildReport {
    pub files: u32,
    pub faces: u32,
    pub locations: u32,
}

fn removal_report(c: &rusqlite::Connection, source_id: i64) -> Result<SourceRemovalReport, StoreError> {
    // face SẼ RỜI LƯỚI: không còn vị trí nào ở nguồn khác mà hiện diện (missing = 0) và chưa bị ẩn bởi Trùng lặp
    const GONE: &str = "WITH gone(hash, face_index) AS (
        SELECT fi.hash, f.face_index FROM faces f JOIN files fi ON fi.file_id = f.file_id
         WHERE EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.source_id = ?1)
           AND NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.source_id <> ?1 AND l.missing = 0
                           AND NOT EXISTS (SELECT 1 FROM u.dup_resolutions dr WHERE dr.loser_path = l.path AND dr.undone_at IS NULL)))";
    let n = |sql: String| -> Result<u32, StoreError> { Ok(c.query_row(&sql, [source_id], |r| r.get::<_, i64>(0))? as u32) };
    Ok(SourceRemovalReport {
        removed_locations: n("SELECT count(*) FROM file_locations WHERE source_id = ?1".into())?,
        faces_left_without_location: n(format!("{GONE} SELECT count(*) FROM gone"))?,
        cases_affected: n(format!(
            "{GONE} SELECT count(*) FROM u.cases c WHERE c.deleted_at IS NULL AND (
               EXISTS (SELECT 1 FROM u.case_faces cf JOIN gone g ON g.hash = cf.hash AND g.face_index = cf.face_index WHERE cf.case_id = c.id)
            OR EXISTS (SELECT 1 FROM u.combos cb JOIN u.combo_slots s ON s.combo_id = cb.id
                       JOIN gone g ON g.hash = s.hash AND g.face_index = s.face_index WHERE cb.case_id = c.id))"
        ))?,
        collections_affected: n(format!(
            "{GONE} SELECT count(DISTINCT ci.collection_id) FROM u.collection_items ci JOIN gone g ON g.hash = ci.hash AND g.face_index = ci.face_index"
        ))?,
        tags_affected: n(format!(
            "{GONE} SELECT count(DISTINCT ft.tag_id) FROM u.face_tags ft JOIN gone g ON g.hash = ft.hash AND g.face_index = ft.face_index"
        ))?,
        active_fonts: n("SELECT count(*) FROM u.activation_state a WHERE a.state = 'active' AND a.hash IN (
               SELECT fi.hash FROM files fi JOIN file_locations l ON l.file_id = fi.file_id WHERE l.source_id = ?1)"
            .into())?,
    })
}

impl Store {
    pub fn rescan_source(&mut self, source_id: i64) -> Result<RescanStats, StoreError> {
        self.rescan_source_with(source_id, &mut |_| {}, &AtomicBool::new(false))
    }

    /// Quét có tiến độ và huỷ. `on_progress` luôn gọi NGOÀI transaction (có thể chặn để tạm dừng);
    /// `cancel` và `root_available` được kiểm sau Walking, Hashing và sau mỗi lô.
    pub fn rescan_source_with(
        &mut self,
        source_id: i64,
        on_progress: &mut dyn FnMut(&ScanProgress),
        cancel: &AtomicBool,
    ) -> Result<RescanStats, StoreError> {
        let src = self.source(source_id)?;
        let root = PathBuf::from(
            src.path
                .ok_or_else(|| StoreError::NotFound(format!("nguồn #{source_id} không có thư mục")))?,
        );
        self.rescan_root_with(source_id, &root, on_progress, cancel)
    }

    /// Như `rescan_source_with` nhưng nhận sẵn thư mục gốc: chạy được trên kết nối chỉ library.db
    /// (`Store::open_library_only`), nên lô ingest không khoá user.db.
    pub fn rescan_root_with(
        &mut self,
        source_id: i64,
        root: &Path,
        on_progress: &mut dyn FnMut(&ScanProgress),
        cancel: &AtomicBool,
    ) -> Result<RescanStats, StoreError> {
        self.rescan_root_with_probe(source_id, root, on_progress, cancel, &os_placeholder)
    }

    /// Như `rescan_root_with`, với hàm nhận diện placeholder thay được (test giả lập cờ dataless).
    /// Placeholder không bị mở/băm, không bị đánh missing, bản ghi cũ giữ nguyên; chỉ được đếm.
    pub fn rescan_root_with_probe(
        &mut self,
        source_id: i64,
        root: &Path,
        on_progress: &mut dyn FnMut(&ScanProgress),
        cancel: &AtomicBool,
        probe: PlaceholderProbe,
    ) -> Result<RescanStats, StoreError> {
        let root = root.to_path_buf();
        let mut stats = RescanStats::default();
        if !root_available(&root) {
            stats.status = RescanStatus::SourceUnavailable;
            return Ok(stats);
        }
        let started = now_ms();
        let generation: i64 = self.conn.query_row(
            "UPDATE meta SET value = value + 1 WHERE key='scan_generation' RETURNING value",
            [],
            |r| r.get(0),
        )?;
        let (files, failed_dirs, placeholders) = walk_fonts_report(&root, probe);
        stats.seen = files.len();
        stats.not_downloaded = placeholders.len();
        on_progress(&progress(ScanPhase::Walking, files.len(), files.len(), 0, stats.not_downloaded));
        if let Some(st) = interrupted(&root, cancel) {
            stats.status = st;
            return Ok(stats);
        }

        // vị trí đã biết của nguồn: path → (size, mtime, có kết quả mới nhất)
        let mut known: HashMap<String, (i64, i64, bool)> = HashMap::new();
        {
            let mut st = self.conn.prepare(
                "SELECT l.path, l.size, l.mtime_ns,
                        COALESCE(p.status IN ('done','failed') AND p.version = ?2, 0)
                 FROM file_locations l JOIN files fi USING(file_id)
                 LEFT JOIN pipeline_state p ON p.hash = fi.hash AND p.face_index = -1 AND p.stage = 'load'
                 WHERE l.source_id = ?1",
            )?;
            let rows = st.query_map(params![source_id, ANALYZER_VERSION], |r| {
                Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?, r.get(3)?)))
            })?;
            for row in rows {
                let (p, v) = row?;
                known.insert(p, v);
            }
        }
        let (unchanged, changed): (Vec<_>, Vec<_>) = files.into_iter().partition(|f| {
            known
                .get(f.path.to_string_lossy().as_ref())
                .is_some_and(|&(size, mtime, fresh)| fresh && size == f.size as i64 && mtime == f.mtime_ns)
        });
        {
            let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            // placeholder đã được phân tích trước đây: vẫn "thấy" ở đó, giữ nguyên dữ liệu
            for (p, meta) in &placeholders {
                let path = p.to_string_lossy();
                let changed = match (known.get(path.as_ref()), meta) {
                    (Some(&(ksize, kmtime, _)), Some((size, mtime, _))) => ksize != *size as i64 || kmtime != *mtime,
                    _ => false,
                };
                // birth time lấy từ stat (an toàn với file dataless); stub .icloud không có stat ⇒ giữ giá trị cũ
                let created = meta.and_then(|m| m.2);
                let csrc = created.map(|_| crate::release::created_src(p));
                if changed {
                    // Nội dung trên cloud đã đổi mà ta không được đọc: kết quả của hash cũ không còn đáng tin. Ghi size/mtime mới
                    // và đánh dấu lần phân tích cũ là "cần làm lại"; khi file được tải về, lần quét sau băm và phân tích lại.
                    let (size, mtime, _) = meta.unwrap_or_default();
                    tx.execute(
                        "UPDATE pipeline_state SET version=?2 WHERE stage='load' AND face_index=-1 AND hash IN
                           (SELECT fi.hash FROM file_locations l JOIN files fi USING(file_id) WHERE l.path=?1)",
                        params![path, CLOUD_STALE_MARK],
                    )?;
                    tx.execute(
                        "UPDATE file_locations SET size=?2, mtime_ns=?3, last_seen_at=?4, scan_gen=?5, missing=0, cloud_only=1,
                                created_ns=COALESCE(?6, created_ns), created_src=COALESCE(?7, created_src) WHERE path=?1",
                        params![path, size as i64, mtime, started, generation, created, csrc],
                    )?;
                } else {
                    tx.execute(
                        "UPDATE file_locations SET last_seen_at=?2, scan_gen=?3, missing=0, cloud_only=1,
                                created_ns=COALESCE(?4, created_ns), created_src=COALESCE(?5, created_src) WHERE path=?1",
                        params![path, started, generation, created, csrc],
                    )?;
                }
            }
            for f in &unchanged {
                tx.execute(
                    "UPDATE file_locations SET last_seen_at=?2, scan_gen=?3, missing=0, cloud_only=0,
                            created_ns=COALESCE(?4, created_ns), created_src=COALESCE(?5, created_src) WHERE path=?1",
                    params![f.path.to_string_lossy(), started, generation, f.created_ns, f.created_ns.map(|_| crate::release::created_src(&f.path))],
                )?;
            }
            tx.commit()?;
        }
        stats.unchanged = unchanged.len();

        // băm song song phần đổi; hash đã có kết quả mới nhất → chỉ nối vị trí
        let hashed: Vec<(ScannedFile, std::io::Result<[u8; 32]>)> =
            changed.into_par_iter().map(|f| {
                // huỷ giữa pha băm: bỏ phần còn lại (kết quả bị bỏ ngay sau đó vì `interrupted`)
                let h = if cancel.load(Ordering::Relaxed) { Err(std::io::ErrorKind::Interrupted.into()) } else { hash_file(&f.path) };
                (f, h)
            }).collect();
        on_progress(&progress(ScanPhase::Hashing, stats.seen, stats.seen, 0, stats.not_downloaded));
        if let Some(st) = interrupted(&root, cancel) {
            stats.status = st;
            return Ok(stats);
        }
        let mut unreadable_known: Vec<String> = Vec::new();
        let mut work: Vec<Work> = Vec::new();
        {
            let mut fresh = self.conn.prepare(
                "SELECT count(*) FROM pipeline_state WHERE hash=?1 AND face_index=-1 AND stage='load'
                   AND status IN ('done','failed') AND version=?2",
            )?;
            for (f, h) in hashed {
                match h {
                    Err(e) => {
                        let p = f.path.to_string_lossy().to_string();
                        if known.contains_key(&p) {
                            unreadable_known.push(p.clone());
                        }
                        stats.unreadable.push((p, format!("io: {}", e.kind())));
                    }
                    Ok(h) => {
                        let n: i64 = fresh.query_row(params![&h[..], ANALYZER_VERSION], |r| r.get(0))?;
                        if n > 0 {
                            work.push(Work::Known(IngestItem { source_id, file: f, hash: h, analysis: Analysis::Known }));
                        } else {
                            work.push(Work::Analyze(f, h));
                        }
                    }
                }
            }
        }
        // phân tích rồi ghi từng lô (font hiện dần); callback và kiểm huỷ nằm ngoài transaction
        let total = work.len();
        let mut done = 0;
        let mut it = work.into_iter();
        loop {
            let chunk: Vec<Work> = it.by_ref().take(INGEST_BATCH).collect();
            if chunk.is_empty() {
                break;
            }
            let n = chunk.len();
            let mut items: Vec<IngestItem> = Vec::with_capacity(n);
            let mut pending: Vec<(ScannedFile, [u8; 32])> = Vec::new();
            for w in chunk {
                match w {
                    Work::Known(i) => items.push(i),
                    Work::Analyze(f, h) => pending.push((f, h)),
                }
            }
            let paths: Vec<PathBuf> = pending.iter().map(|(f, _)| f.path.clone()).collect();
            for ((f, h), (_, res)) in pending.into_iter().zip(analyze_paths(&paths)) {
                match res {
                    // hash của report là chuẩn (file có thể đổi giữa lúc băm và lúc phân tích)
                    Ok(r) => items.push(IngestItem { source_id, file: f, hash: parse_hash(&r.hash_hex)?, analysis: Analysis::Report(r) }),
                    // lỗi I/O (ổ rút, file bị khoá…) không phải lỗi font: để pending, lần quét sau thử lại
                    Err(CoreError::Io(e)) => {
                        let p = f.path.to_string_lossy().to_string();
                        if known.contains_key(&p) {
                            unreadable_known.push(p.clone());
                        }
                        stats.unreadable.push((p, format!("io: {}", e.kind())));
                    }
                    Err(e) => items.push(IngestItem {
                        source_id,
                        file: f,
                        hash: h,
                        analysis: Analysis::Failed { code: e.code().to_string(), detail: e.to_string() },
                    }),
                }
            }
            let faces_written: usize = items
                .iter()
                .map(|i| if let Analysis::Report(r) = &i.analysis { r.faces.len() } else { 0 })
                .sum();
            self.ingest_counting(items, started, &mut stats)?;
            done += n;
            on_progress(&progress(ScanPhase::Analyzing, done, total, faces_written, stats.not_downloaded));
            if let Some(st) = interrupted(&root, cancel) {
                stats.status = st;
                return Ok(stats);
            }
        }

        // file đã biết nhưng không đọc được lần này: vẫn coi là còn
        for p in &unreadable_known {
            self.conn.execute(
                "UPDATE file_locations SET scan_gen=?2 WHERE path=?1",
                params![p, generation],
            )?;
        }
        // ổ rút giữa lúc quét → không đánh missing gì
        if let Some(st) = interrupted(&root, cancel) {
            stats.status = st;
            return Ok(stats);
        }
        on_progress(&progress(ScanPhase::MarkingMissing, 0, 0, 0, stats.not_downloaded));
        if let Some(st) = interrupted(&root, cancel) {
            stats.status = st;
            return Ok(stats);
        }
        let stale: Vec<(i64, String)> = {
            let mut st = self.conn.prepare(
                "SELECT location_id, path FROM file_locations WHERE source_id=?1 AND missing=0 AND scan_gen < ?2",
            )?;
            st.query_map(params![source_id, generation], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for (id, p) in stale {
            if failed_dirs.iter().any(|d| Path::new(&p).starts_with(d)) {
                continue; // nằm trong thư mục không đọc được: không biết đã xoá hay chưa
            }
            stats.missing += tx.execute("UPDATE file_locations SET missing=1 WHERE location_id=?1", [id])?;
        }
        tx.commit()?;
        // quét trọn vẹn → tự dọn, nhưng CHỈ vị trí missing của chính nguồn này và CHỈ khi lần quét thấy ít nhất một font
        // (thư mục gốc còn đó mà trống — điểm gắn kết rỗng của ổ mạng/ổ rút — không chứng minh được font đã mất).
        // Nguồn khác (kể cả ổ đang rút) không bao giờ bị dọn bởi lần quét này.
        stats.purged = if stats.seen + stats.not_downloaded > 0 {
            self.gc_missing(Some(source_id), MISSING_RETENTION_MS)?
        } else {
            self.gc()?;
            0
        };
        Ok(stats)
    }

    fn ingest_counting(&mut self, items: Vec<IngestItem>, at: i64, stats: &mut RescanStats) -> Result<(), StoreError> {
        let analyzed = items.iter().filter(|i| matches!(i.analysis, Analysis::Report(_))).count();
        let sum = self.ingest(items, at)?;
        stats.analyzed += analyzed;
        stats.failed += sum.failed;
        stats.relinked += sum.relinked;
        Ok(())
    }

    /// Số liệu thật cho câu xác nhận "Gỡ nguồn" — chỉ đọc, không đổi gì (xem [`SourceRemovalReport`]).
    pub fn source_removal_report(&self, source_id: i64) -> Result<SourceRemovalReport, StoreError> {
        removal_report(&self.conn, source_id)
    }

    /// Gỡ nguồn (thao tác người dùng): bản ghi `u.sources`, vị trí file của nguồn, rồi dọn rác (`gc`).
    ///
    /// **Dữ liệu người dùng KHÔNG bị đụng**: Dự án (Case/Combo), Collection, Tag, Yêu thích, ghi đè phân loại, cặp
    /// font khoá theo `(hash, face_index)` ở user.db, không phụ thuộc nguồn. Font chỉ có ở nguồn này sẽ `missing`
    /// (Dự án) hoặc ẩn khỏi lưới nhưng còn trong Collection/Tag; thêm lại nguồn ⇒ khớp lại theo hash. Font đang bật
    /// không bị tự tắt. Trả số liệu đã tính TRƯỚC khi xoá.
    ///
    /// Hai DB ở chế độ WAL nên KHÔNG có giao dịch nguyên tử chung: mỗi bước là một giao dịch riêng, theo thứ tự cố
    /// định `u.sources` → `file_locations`. Crash giữa hai bước chỉ để lại vị trí mồ côi (`source_id` không còn trong
    /// `u.sources`), không bao giờ để nguồn "còn tên mà hết font"; [`Store::sweep_orphan_locations`] dọn lại ở lần
    /// khởi động sau (và ở mọi lần gỡ nguồn kế tiếp). Chạy lại `remove_source` cho cùng id cũng an toàn (idempotent).
    pub fn remove_source(&mut self, source_id: i64) -> Result<SourceRemovalReport, StoreError> {
        self.remove_source_with(source_id, &mut || Ok(()))
    }

    /// Như `remove_source`, `between` chạy giữa hai giao dịch (test mô phỏng crash bằng cách trả `Err`).
    pub fn remove_source_with(
        &mut self,
        source_id: i64,
        between: &mut dyn FnMut() -> Result<(), StoreError>,
    ) -> Result<SourceRemovalReport, StoreError> {
        let report = removal_report(&self.conn, source_id)?;
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM u.sources WHERE source_id=?1", [source_id])?;
        tx.commit()?;
        between()?;
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM file_locations WHERE source_id=?1", [source_id])?;
        tx.commit()?;
        self.sweep_orphan_locations()?;
        Ok(report)
    }

    /// Xoá vị trí file của nguồn không còn trong `u.sources` (dấu vết của lần gỡ bị ngắt giữa chừng), rồi `gc`.
    /// Cần kết nối có `u` (không chạy trên `open_library_only`). Trả số vị trí đã xoá.
    pub fn sweep_orphan_locations(&mut self) -> Result<usize, StoreError> {
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let n = tx.execute(
            "DELETE FROM file_locations WHERE source_id NOT IN (SELECT source_id FROM u.sources)",
            [],
        )?;
        tx.commit()?;
        self.gc()?;
        Ok(n)
    }

    /// Dọn vị trí `missing` mà lần cuối thấy file đã quá `retention_ms`, rồi `gc`. `source_id`: chỉ nguồn đó (`None` = mọi
    /// nguồn). `missing` chỉ được đặt khi gốc nguồn đang truy cập được lúc quét, nên file ở ổ chưa gắn không bao giờ vào đây.
    /// Trả số vị trí đã dọn.
    pub fn gc_missing(&mut self, source_id: Option<i64>, retention_ms: i64) -> Result<usize, StoreError> {
        let cutoff = now_ms() - retention_ms;
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let n = tx.execute(
            "DELETE FROM file_locations WHERE missing=1 AND last_seen_at < ?1 AND (?2 IS NULL OR source_id = ?2)",
            params![cutoff, source_id],
        )?;
        tx.commit()?;
        self.gc()?;
        Ok(n)
    }

    /// Dựng lại thư viện từ đầu: xoá file/face/vị trí và mọi dữ liệu dẫn xuất tính cục bộ (phân tích, embedding, ngôn ngữ,
    /// trạng thái pipeline) trong MỘT giao dịch library.db (crash giữa chừng = chưa đổi gì). KHÔNG đụng user.db
    /// (Dự án, Collection, Tag, Yêu thích, nguồn…) và dữ liệu cộng đồng; font trả lại bằng quét lại các nguồn.
    pub fn rebuild_library(&mut self) -> Result<RebuildReport, StoreError> {
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let count = |t: &str| -> Result<u32, StoreError> {
            Ok(tx.query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get::<_, i64>(0))? as u32)
        };
        let report = RebuildReport { files: count("files")?, faces: count("faces")?, locations: count("file_locations")? };
        tx.execute("DELETE FROM file_locations", [])?;
        tx.execute("DELETE FROM files", [])?; // faces theo ON DELETE CASCADE (trigger cập nhật FTS)
        for t in ["analyses", "embeddings"] {
            tx.execute(&format!("DELETE FROM {t} WHERE source <> 'community'"), [])?;
        }
        for t in ["pipeline_state", "face_langs", "face_lang_state"] {
            tx.execute(&format!("DELETE FROM {t}"), [])?;
        }
        tx.commit()?;
        Ok(report)
    }

    /// G8: nhập đúng các file font này (không quét thư mục) vào nguồn `source_id` — dùng cho nguồn `loose`. File đã có vị trí ở
    /// BẤT KỲ nguồn nào thì giữ nguyên (không cướp vị trí của nguồn khác) và báo `AlreadyKnown`.
    pub fn import_files(&mut self, source_id: i64, paths: &[PathBuf]) -> Result<Vec<(PathBuf, FileImportOutcome)>, StoreError> {
        let started = now_ms();
        let mut out: Vec<(PathBuf, FileImportOutcome)> = Vec::new();
        let mut todo: Vec<ScannedFile> = Vec::new();
        for p in paths {
            let known: i64 = self.conn.query_row("SELECT count(*) FROM file_locations WHERE path=?1", [p.to_string_lossy()], |r| r.get(0))?;
            if known > 0 {
                out.push((p.clone(), FileImportOutcome::AlreadyKnown));
                continue;
            }
            let Ok(m) = std::fs::metadata(p) else {
                out.push((p.clone(), FileImportOutcome::Missing));
                continue;
            };
            if !m.is_file() || !is_font(p) {
                out.push((p.clone(), FileImportOutcome::Missing));
                continue;
            }
            if os_placeholder(p, &m) {
                out.push((p.clone(), FileImportOutcome::NotDownloaded));
                continue;
            }
            let ns = |t: std::io::Result<std::time::SystemTime>| t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos() as i64);
            todo.push(ScannedFile { path: p.clone(), size: m.len(), mtime_ns: ns(m.modified()).unwrap_or(0), created_ns: ns(m.created()) });
        }
        let mut items: Vec<IngestItem> = Vec::new();
        let mut to_analyze: Vec<(ScannedFile, [u8; 32])> = Vec::new();
        for f in todo {
            match hash_file(&f.path) {
                Err(_) => out.push((f.path.clone(), FileImportOutcome::Unreadable)),
                Ok(h) => {
                    let fresh: i64 = self.conn.query_row(
                        "SELECT count(*) FROM pipeline_state WHERE hash=?1 AND face_index=-1 AND stage='load' AND status IN ('done','failed') AND version=?2",
                        params![&h[..], ANALYZER_VERSION],
                        |r| r.get(0),
                    )?;
                    if fresh > 0 {
                        items.push(IngestItem { source_id, file: f, hash: h, analysis: Analysis::Known });
                    } else {
                        to_analyze.push((f, h));
                    }
                }
            }
        }
        let paths: Vec<PathBuf> = to_analyze.iter().map(|(f, _)| f.path.clone()).collect();
        for ((f, h), (_, res)) in to_analyze.into_iter().zip(analyze_paths(&paths)) {
            match res {
                Ok(r) => items.push(IngestItem { source_id, file: f, hash: parse_hash(&r.hash_hex)?, analysis: Analysis::Report(r) }),
                Err(CoreError::Io(_)) => out.push((f.path.clone(), FileImportOutcome::Unreadable)),
                Err(e) => items.push(IngestItem {
                    source_id,
                    file: f,
                    hash: h,
                    analysis: Analysis::Failed { code: e.code().to_string(), detail: e.to_string() },
                }),
            }
        }
        for it in &items {
            out.push((it.file.path.clone(), FileImportOutcome::Imported));
        }
        self.ingest(items, started)?;
        Ok(out)
    }

    /// Đồng bộ cờ `missing`/`cloud_only` của các vị trí thuộc nguồn KHÔNG có thư mục (loose): stat từng file, không đọc nội dung.
    /// Trả số vị trí đổi trạng thái.
    pub fn refresh_file_source(&mut self, source_id: i64) -> Result<usize, StoreError> {
        let rows: Vec<(i64, String, i64, i64)> = {
            let mut st = self.conn.prepare("SELECT location_id, path, missing, cloud_only FROM file_locations WHERE source_id=?1")?;
            st.query_map([source_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect::<rusqlite::Result<_>>()?
        };
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut changed = 0;
        for (id, path, missing, cloud) in rows {
            let p = Path::new(&path);
            let (m, c) = match std::fs::metadata(p) {
                Ok(md) if md.is_file() => (0, i64::from(os_placeholder(p, &md))),
                _ => (1, cloud),
            };
            if (m, c) != (missing, cloud) {
                tx.execute("UPDATE file_locations SET missing=?2, cloud_only=?3 WHERE location_id=?1", params![id, m, c])?;
                changed += 1;
            }
        }
        tx.commit()?;
        Ok(changed)
    }

    /// Face (chưa missing) của các vị trí: khớp CHÍNH XÁC `exact` hoặc nằm dưới thư mục `prefixes` (không kèm dấu phân cách cuối).
    /// Trả (đường dẫn vị trí, face), sắp theo đường dẫn rồi `face_index`.
    pub fn faces_at(&self, exact: &[String], prefixes: &[String]) -> Result<Vec<(String, FaceKey)>, StoreError> {
        let mut out: Vec<(String, FaceKey)> = Vec::new();
        let mut st = self.conn.prepare(
            "SELECT l.path, fi.hash, f.face_index FROM file_locations l JOIN files fi ON fi.file_id = l.file_id
               JOIN faces f ON f.file_id = fi.file_id
              WHERE l.missing = 0 AND (l.path = ?1 OR (?2 <> '' AND substr(l.path, 1, length(?2) + 1) = ?2 || '/'))
              ORDER BY l.path, f.face_index",
        )?;
        let mut run = |exact: &str, prefix: &str| -> Result<(), StoreError> {
            let rows = st.query_map(params![exact, prefix], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, u32>(2)?)))?;
            for row in rows {
                let (path, hash, face_index) = row?;
                let hash: [u8; 32] = hash.try_into().map_err(|_| StoreError::BadHash("hash không đủ 32 byte".into()))?;
                out.push((path, FaceKey { hash, face_index }));
            }
            Ok(())
        };
        for e in exact {
            run(e, "")?;
        }
        for p in prefixes {
            run("\u{0}", p)?;
        }
        Ok(out)
    }

    /// Xoá file không còn vị trí nào (vị trí `missing` vẫn giữ file). Không đụng `u`.
    pub fn gc(&mut self) -> Result<usize, StoreError> {
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let n = tx.execute(
            "DELETE FROM files WHERE NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = files.file_id)",
            [],
        )?;
        // dữ liệu dẫn xuất khoá theo hash (không có FK): xoá cái mồ côi; dữ liệu cộng đồng giữ lại
        for t in ["analyses", "embeddings"] {
            tx.execute(
                &format!("DELETE FROM {t} WHERE source <> 'community' AND hash NOT IN (SELECT hash FROM files)"),
                [],
            )?;
        }
        for t in ["pipeline_state", "face_langs", "face_lang_state"] {
            tx.execute(&format!("DELETE FROM {t} WHERE hash NOT IN (SELECT hash FROM files)"), [])?;
        }
        tx.commit()?;
        Ok(n)
    }
}
