//! G7b: bản sao ĐỔI TÊN của font biến thiên (VF) trùng tên PostScript với bản static ("Giữ cả hai"). Chỉ tạo khi VF nằm trong
//! thư viện và THẬT SỰ trùng tên; file gốc không bị đụng; không bao giờ tạo ngầm (người dùng chọn). Quy tắc tên ở
//! `fontatlas_core::rename`. Bản sao nằm ở `<Organized>/Library/Derived/` nếu có Organized Library, không thì `<data>/derived/`
//! (KHÔNG bao giờ cạnh file gốc hay trong thư mục nguồn khác); sau đó là một font bình thường của thư viện, nối với bản gốc qua sổ
//! `derived_fonts`.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fontatlas_core::rename::{RenameError, rename_vf_copy};
use fontatlas_store::{DerivedFont, FaceKey, FileImportOutcome, SourceKind, is_dataless};

use crate::dto::library::FaceKeyDto;
use crate::dto::organized::{DerivedCopyDto, DerivedCopyInfoDto, RemoveDerivedResultDto, RestoreDerivedResultDto, RestoreViaDto};
use crate::error::CmdError;
use crate::services::organized::{free_dest, safe_component};
use crate::state::AppState;

pub const DERIVED_DIR: &str = "Derived";
pub const DERIVED_SOURCE_NAME: &str = "Derived fonts";

#[derive(Debug, thiserror::Error)]
pub enum DeriveError {
    #[error("font không nằm trong thư viện (hoặc không đọc được)")]
    NotInLibrary,
    #[error("font không phải Variable")]
    NotVariable,
    #[error("font Variable này không trùng tên PostScript với bản static nào")]
    NoClash,
    #[error("định dạng chưa hỗ trợ: {0}")]
    UnsupportedContainer(&'static str),
    #[error("file còn là placeholder trên ổ cloud, tải về máy trước")]
    NotDownloaded,
    #[error("hết chỗ trống")]
    DiskFull,
    #[error("lỗi đọc/ghi: {0}")]
    Io(String),
}

impl From<DeriveError> for CmdError {
    fn from(e: DeriveError) -> Self {
        let code = match &e {
            DeriveError::NotInLibrary => "not_in_library",
            DeriveError::NotVariable => "not_variable",
            DeriveError::NoClash => "no_clash",
            DeriveError::UnsupportedContainer(_) => "unsupported_container",
            DeriveError::NotDownloaded => "not_downloaded",
            DeriveError::DiskFull => "disk_full",
            DeriveError::Io(_) => "io",
        };
        let c = CmdError::new(code, e.to_string());
        match e {
            DeriveError::UnsupportedContainer(k) => c.with_detail(serde_json::json!({ "container": k })),
            _ => c,
        }
    }
}

fn io_err(e: std::io::Error) -> DeriveError {
    if e.kind() == std::io::ErrorKind::StorageFull { DeriveError::DiskFull } else { DeriveError::Io(e.to_string()) }
}

fn derived_dir(state: &AppState) -> PathBuf {
    match crate::services::organized::library_prefix(state) {
        Some(lib) => Path::new(&lib).join(DERIVED_DIR),
        None => state.dirs.data.join("derived"),
    }
}

/// Ghi `bytes` vào `dest` an toàn: file tạm cùng thư mục, đồng bộ, đổi tên nguyên tử (chỉ trả khi đã xong).
fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<(), DeriveError> {
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d).map_err(io_err)?;
    }
    let tmp = dest.with_file_name(format!(".{}.partial", dest.file_name().and_then(|n| n.to_str()).unwrap_or("font")));
    let run = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, dest)
    };
    let r = run().map_err(io_err);
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

pub fn is_derived(state: &AppState, hash: &[u8; 32]) -> bool {
    state.with_store(|s| Ok(s.is_derived(hash)?)).unwrap_or(false)
}

pub fn derived_from(state: &AppState, hash: &[u8; 32]) -> Option<[u8; 32]> {
    state.with_store(|s| Ok(s.derived_from(hash)?)).ok().flatten()
}

/// Tạo (hoặc trả lại) bản sao đổi tên của VF `hash`. Idempotent: đã có và file còn thì `created = false`.
pub fn derive_renamed_copy(state: &Arc<AppState>, hash: &[u8; 32]) -> Result<(FaceKey, DerivedCopyDto), DeriveError> {
    let internal = |e: CmdError| DeriveError::Io(e.message);
    // 1. phải là VF trong thư viện và thật sự trùng tên
    let (face_id, is_vf) = state
        .with_store(|s| Ok(s.face_is_variable(hash, 0)?))
        .map_err(internal)?
        .ok_or(DeriveError::NotInLibrary)?;
    if !is_vf {
        return Err(DeriveError::NotVariable);
    }
    let make_key = |h: [u8; 32]| FaceKey { hash: h, face_index: 0 };
    // 2. đã có bản sao còn trên đĩa → trả lại
    if let Some(d) = state.with_store(|s| Ok(s.derived_for(hash)?)).map_err(internal)?
        && Path::new(&d.path).is_file()
    {
        let k = make_key(d.hash);
        return Ok((k, DerivedCopyDto { face: FaceKeyDto::from(&k), path: d.path, created: false, license_restricted: d.license_restricted }));
    }
    let clashes = state.with_store(|s| Ok(s.name_clashes()?)).map_err(internal)?;
    if !clashes.iter().any(|c| c.vf_face_id == face_id) {
        return Err(DeriveError::NoClash);
    }
    // 3. đọc file gốc (chỉ đọc)
    let (src, cloud_only) = state
        .with_store(|s| Ok(s.best_location(hash)?))
        .map_err(internal)?
        .ok_or(DeriveError::NotInLibrary)?;
    let src = PathBuf::from(src);
    if cloud_only || is_dataless(&src) {
        return Err(DeriveError::NotDownloaded);
    }
    let bytes = std::fs::read(&src).map_err(io_err)?;
    let out = rename_vf_copy(&bytes).map_err(|e| match e {
        RenameError::Unsupported(k) => DeriveError::UnsupportedContainer(k),
        RenameError::Malformed(m) => DeriveError::Io(m),
    })?;
    // 4. đích: Library/Derived hoặc <data>/derived; tên `<gốc>-VF.<ext>`, không đè file khác
    let dir = derived_dir(state);
    let stem = safe_component(src.file_stem().and_then(|s| s.to_str()).unwrap_or("font"), "font");
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("ttf");
    let new_hash: [u8; 32] = *blake3::hash(&out.bytes).as_bytes();
    let dest = free_dest(&dir, &format!("{stem}-VF.{ext}"), &new_hash, &|p: &Path| p.exists());
    write_atomic(&dest, &out.bytes)?;
    // 5. nạp vào thư viện như font thường và ghi sổ
    let rec = DerivedFont { hash: new_hash, derived_from: *hash, path: dest.to_string_lossy().into_owned(), license_restricted: out.license_restricted };
    if let Err(e) = register_copy(state, &dest, &rec) {
        let _ = std::fs::remove_file(&dest);
        return Err(e);
    }
    let k = make_key(new_hash);
    Ok((k, DerivedCopyDto { face: FaceKeyDto::from(&k), path: rec.path, created: true, license_restricted: out.license_restricted }))
}

/// Nạp file bản sao `dest` vào thư viện (nguồn thư mục chứa nó — Organized Library — hoặc nguồn ẩn `derived`) và ghi sổ `derived_fonts`.
fn register_copy(state: &Arc<AppState>, dest: &Path, rec: &DerivedFont) -> Result<(), DeriveError> {
    let internal = |e: CmdError| DeriveError::Io(e.message);
    let sources = state.with_user(|u| Ok(u.sources()?)).map_err(internal)?;
    let sid = match sources.iter().find(|s| s.kind == SourceKind::Directory && s.path.as_deref().is_some_and(|p| dest.starts_with(p))) {
        Some(s) => s.id,
        None => state.with_user(|u| Ok(u.add_source(SourceKind::Derived, DERIVED_SOURCE_NAME, None)?)).map_err(internal)?,
    };
    let outcomes = state.with_store(|s| Ok(s.import_files(sid, std::slice::from_ref(&dest.to_path_buf()))?)).map_err(internal)?;
    if !matches!(outcomes.first().map(|o| o.1), Some(FileImportOutcome::Imported | FileImportOutcome::AlreadyKnown)) {
        return Err(DeriveError::Io("không nạp được bản sao vào thư viện".into()));
    }
    state.with_store(|s| Ok(s.derived_record(rec)?)).map_err(internal)?;
    crate::services::duplicates::invalidate_cache();
    let _ = crate::services::preview::bump_generation(state);
    crate::index_bundle::request_rebuild(state, crate::index_bundle::RebuildWhy::ScanIdle);
    Ok(())
}

/// Lệnh: nhận face của VF, trả bản sao.
pub fn derive_command(state: &Arc<AppState>, face: &FaceKeyDto) -> Result<DerivedCopyDto, CmdError> {
    let k = FaceKey::try_from(face)?;
    Ok(derive_renamed_copy(state, &k.hash)?.1)
}

// ---------- gỡ bản sao + hoàn tác ----------

/// Gỡ MỘT bản sao đổi tên: tắt nếu đang bật, chuyển file vào Thùng rác (KHÔNG BAO GIỜ xoá vĩnh viễn; chỉ file do chính app tạo trong
/// thư mục bản sao), rồi dọn sổ + vị trí trong thư viện. `trasher` tiêm được để test.
pub fn remove_derived_copy_with(
    state: &Arc<AppState>,
    face: &FaceKeyDto,
    trasher: &dyn Fn(&Path) -> Result<PathBuf, String>,
) -> Result<RemoveDerivedResultDto, CmdError> {
    let key = FaceKey::try_from(face)?;
    let d = state
        .with_store(|s| Ok(s.derived_for_copy(&key.hash)?))?
        .ok_or_else(|| CmdError::new("not_derived", "font này không phải bản sao đổi tên"))?;
    let path = PathBuf::from(&d.path);
    // chỉ file nằm trong thư mục bản sao của app (Library/Derived hoặc <data>/derived) mới được chuyển vào Thùng rác
    let derived_roots = [derived_dir(state), state.dirs.data.join("derived")];
    let in_derived = |p: &Path| derived_roots.iter().any(|r| p.starts_with(r));
    if path.exists() && !in_derived(&path) {
        return Err(CmdError::new("not_allowed", "bản sao nằm ngoài thư mục bản sao của app; không tự chuyển vào Thùng rác"));
    }
    // 1. tắt trước (font đang bật trỏ vào file sắp rời đi)
    let mut was_active = false;
    if state.activation.supported() {
        let rep = crate::services::activation::deactivate(state, std::slice::from_ref(face), true, &mut |_| {})
            .map_err(|e| CmdError::new("deactivate_failed", e.message))?;
        was_active = rep.changed > 0;
    }
    // 2. Thùng rác
    let trash_path = if path.exists() {
        Some(trasher(&path).map_err(|e| CmdError::new("trash_failed", e))?.to_string_lossy().into_owned())
    } else {
        None
    };
    // 3. sổ + vị trí thư viện
    let id = state.with_store(|s| Ok(s.derived_remove(&d, trash_path.as_deref())?))?;
    crate::services::duplicates::invalidate_cache();
    let _ = crate::services::preview::bump_generation(state);
    crate::index_bundle::request_rebuild(state, crate::index_bundle::RebuildWhy::ScanIdle);
    Ok(RemoveDerivedResultDto { removal_id: id, trashed: trash_path.is_some(), was_active })
}

pub fn remove_derived_copy(state: &Arc<AppState>, face: &FaceKeyDto) -> Result<RemoveDerivedResultDto, CmdError> {
    remove_derived_copy_with(state, face, &crate::trash::trash_item)
}

/// Hoàn tác một lần gỡ: đưa file từ Thùng rác về chỗ cũ (nếu còn và chỗ cũ trống), không thì TẠO LẠI từ font gốc.
pub fn restore_derived_copy(state: &Arc<AppState>, removal_id: i64) -> Result<RestoreDerivedResultDto, CmdError> {
    let r = state
        .with_store(|s| Ok(s.derived_removal(removal_id)?))?
        .ok_or_else(|| CmdError::not_found(format!("lần gỡ #{removal_id}")))?;
    if r.undone {
        return Err(CmdError::new("already_undone", "lần gỡ này đã được hoàn tác"));
    }
    let orig = PathBuf::from(&r.original_path);
    let from_trash = r.trash_path.as_deref().map(PathBuf::from).filter(|t| t.is_file());
    if let Some(t) = from_trash
        && !orig.exists()
        && orig.parent().is_none_or(|d| std::fs::create_dir_all(d).is_ok())
        && std::fs::rename(&t, &orig).is_ok()
    {
        let rec = DerivedFont { hash: r.hash, derived_from: r.derived_from, path: r.original_path.clone(), license_restricted: r.license_restricted };
        register_copy(state, &orig, &rec).map_err(CmdError::from)?;
        state.with_store(|s| Ok(s.derived_removal_mark_undone(removal_id)?))?;
        let k = FaceKey { hash: r.hash, face_index: 0 };
        return Ok(RestoreDerivedResultDto { via: RestoreViaDto::Trash, face: FaceKeyDto::from(&k), path: r.original_path });
    }
    // Thùng rác đã dọn / chỗ cũ bị chiếm: tạo lại từ font gốc (cần gốc còn trong thư viện và vẫn trùng tên)
    let (k, c) = derive_renamed_copy(state, &r.derived_from).map_err(CmdError::from)?;
    state.with_store(|s| Ok(s.derived_removal_mark_undone(removal_id)?))?;
    Ok(RestoreDerivedResultDto { via: RestoreViaDto::ReDerived, face: FaceKeyDto::from(&k), path: c.path })
}

/// Các bản sao của font gốc `face` (để hỏi "gỡ luôn các bản sao?" trước khi xoá font gốc).
pub fn derived_copies_of(state: &AppState, face: &FaceKeyDto) -> Result<Vec<DerivedCopyInfoDto>, CmdError> {
    let key = FaceKey::try_from(face)?;
    let list = state.with_store(|s| Ok(s.derived_copies_of(&key.hash)?))?;
    let mut out = Vec::new();
    for d in list {
        let name = state.with_store(|s| Ok(s.face_label(&d.hash, 0)?))?.unwrap_or_default();
        out.push(DerivedCopyInfoDto { face: FaceKeyDto::from(&FaceKey { hash: d.hash, face_index: 0 }), path: d.path, name, license_restricted: d.license_restricted });
    }
    Ok(out)
}
