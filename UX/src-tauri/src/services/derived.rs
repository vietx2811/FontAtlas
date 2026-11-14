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
use crate::dto::organized::DerivedCopyDto;
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
    // 5. nạp vào thư viện như font thường: nguồn thư mục chứa đích (Organized Library) hoặc nguồn ẩn `derived`
    let sources = state.with_user(|u| Ok(u.sources()?)).map_err(internal)?;
    let sid = match sources.iter().find(|s| s.kind == SourceKind::Directory && s.path.as_deref().is_some_and(|p| dest.starts_with(p))) {
        Some(s) => s.id,
        None => state
            .with_user(|u| Ok(u.add_source(SourceKind::Derived, DERIVED_SOURCE_NAME, None)?))
            .map_err(internal)?,
    };
    let outcomes = state.with_store(|s| Ok(s.import_files(sid, std::slice::from_ref(&dest))?)).map_err(internal)?;
    if !matches!(outcomes.first().map(|o| o.1), Some(FileImportOutcome::Imported | FileImportOutcome::AlreadyKnown)) {
        let _ = std::fs::remove_file(&dest);
        return Err(DeriveError::Io("không nạp được bản sao vào thư viện".into()));
    }
    let rec = DerivedFont { hash: new_hash, derived_from: *hash, path: dest.to_string_lossy().into_owned(), license_restricted: out.license_restricted };
    state.with_store(|s| Ok(s.derived_record(&rec)?)).map_err(internal)?;
    crate::services::duplicates::invalidate_cache();
    let _ = crate::services::preview::bump_generation(state);
    crate::index_bundle::request_rebuild(state, crate::index_bundle::RebuildWhy::ScanIdle);
    let k = make_key(new_hash);
    Ok((k, DerivedCopyDto { face: FaceKeyDto::from(&k), path: rec.path, created: true, license_restricted: out.license_restricted }))
}

/// Lệnh: nhận face của VF, trả bản sao.
pub fn derive_command(state: &Arc<AppState>, face: &FaceKeyDto) -> Result<DerivedCopyDto, CmdError> {
    let k = FaceKey::try_from(face)?;
    Ok(derive_renamed_copy(state, &k.hash)?.1)
}
