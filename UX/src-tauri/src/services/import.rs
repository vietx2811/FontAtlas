//! G8: nhập đường dẫn thả từ Finder vào Collection/Dự án. Thư mục → thêm làm nguồn; file font nằm trong nguồn thư mục có sẵn → quét lại
//! nguồn đó; file font LẺ ngoài mọi nguồn → nguồn `loose` (tham chiếu tại chỗ, KHÔNG thêm thư mục cha, không theo dõi thư mục).
//! Xong thì thêm đúng các face của những đường dẫn đó vào đích. Không chặn theo giấy phép; không đụng file của người dùng.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fontatlas_store::{FONT_EXTENSIONS, FaceKey, FileImportOutcome, SourceKind, is_dataless};

use crate::dto::library::{
    FaceKeyDto, ImportPhaseDto, ImportProgressDto, ImportResultDto, ImportSkipDto, ImportTargetDto, ImportTargetKindDto,
};
use crate::error::CmdError;
use crate::services::{cases, library, sources};
use crate::state::AppState;

pub const LOOSE_NAME: &str = "Font lẻ";

/// Chỉ một lượt nhập tại một thời điểm; giữ cờ huỷ của lượt đang chạy.
#[derive(Default)]
pub struct ImportJob(Mutex<Option<Arc<AtomicBool>>>);

struct Guard<'a>(&'a ImportJob);

impl ImportJob {
    fn begin(&self) -> Option<(Arc<AtomicBool>, Guard<'_>)> {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_some() {
            return None;
        }
        let f = Arc::new(AtomicBool::new(false));
        *g = Some(Arc::clone(&f));
        Some((f, Guard(self)))
    }
    /// Huỷ lượt đang chạy; false nếu không có.
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

fn is_font(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| FONT_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

fn skip(out: &mut Vec<ImportSkipDto>, path: &Path, reason: &str) {
    out.push(ImportSkipDto { path: path.to_string_lossy().into_owned(), reason: reason.into() });
}

pub fn cancel_import(state: &AppState) -> bool {
    state.imports.cancel()
}

pub fn import_paths_into(
    state: &Arc<AppState>,
    paths: Vec<String>,
    target: &ImportTargetDto,
    progress: &mut dyn FnMut(ImportProgressDto),
) -> Result<ImportResultDto, CmdError> {
    let Some((cancel, _guard)) = state.imports.begin() else {
        return Err(CmdError::busy("đang nhập một lượt khác"));
    };
    match target.kind {
        ImportTargetKindDto::Case => {
            cases::get_case(state, target.id)?;
        }
        ImportTargetKindDto::Collection => {
            if !library::list_collections(state)?.iter().any(|c| c.id == target.id) {
                return Err(CmdError::not_found(format!("Collection #{}", target.id)));
            }
        }
    }
    let mut skipped: Vec<ImportSkipDto> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let total_paths = paths.len() as u32;
    let mut emit = |phase, done, total, added, skipped: &Vec<ImportSkipDto>| {
        progress(ImportProgressDto { phase, done, total, added, skipped: skipped.len() as u32 });
    };

    // ---- 1. phân loại và thêm nguồn ----
    let roots: Vec<(i64, PathBuf)> = state
        .with_user(|u| Ok(u.sources()?))?
        .into_iter()
        .filter(|s| s.kind == SourceKind::Directory)
        .filter_map(|s| s.path.map(|p| (s.id, PathBuf::from(p))))
        .collect();
    let mut scan_ids: Vec<i64> = Vec::new();
    let mut dirs: Vec<String> = Vec::new(); // tiền tố để lấy face
    let mut files: Vec<PathBuf> = Vec::new(); // file cần có face
    let mut loose: Vec<PathBuf> = Vec::new();
    let push_scan = |id: i64, v: &mut Vec<i64>| {
        if !v.contains(&id) {
            v.push(id);
        }
    };
    for (i, raw) in paths.iter().enumerate() {
        emit(ImportPhaseDto::AddingSources, i as u32, total_paths, 0, &skipped);
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        let p = PathBuf::from(raw.trim_end_matches('/'));
        if !seen.insert(p.clone()) {
            continue;
        }
        let Ok(md) = std::fs::metadata(&p) else {
            skip(&mut skipped, &p, "missing");
            continue;
        };
        if md.is_dir() {
            match sources::add_source(state, &p.to_string_lossy()) {
                Ok(dto) => push_scan(dto.id, &mut scan_ids),
                Err(e) if e.code == "nested_source" => {
                    // nằm trong / chứa nguồn có sẵn: quét lại nguồn đó, vẫn lấy face theo thư mục đã thả
                    if let Some(id) = e.detail.as_ref().and_then(|d| d["existingId"].as_i64()) {
                        state.scans.enqueue(state, &[id]);
                        push_scan(id, &mut scan_ids);
                    }
                }
                Err(e) => return Err(e),
            }
            dirs.push(p.to_string_lossy().into_owned());
        } else if !is_font(&p) {
            skip(&mut skipped, &p, "not_font");
        } else if is_dataless(&p) {
            skip(&mut skipped, &p, "cloud_not_downloaded");
        } else if let Some((id, _)) = roots.iter().find(|(_, r)| p.starts_with(r)) {
            state.scans.enqueue(state, &[*id]);
            push_scan(*id, &mut scan_ids);
            files.push(p);
        } else {
            loose.push(p);
        }
    }

    // ---- 2. font lẻ ngoài mọi nguồn: nhập tại chỗ vào nguồn `loose` ----
    if !loose.is_empty() && !cancel.load(Ordering::SeqCst) {
        let id = state.with_user(|u| Ok(u.add_source(SourceKind::Loose, LOOSE_NAME, None)?))?;
        let outcomes = state.with_store(|s| Ok(s.import_files(id, &loose)?))?;
        for (p, o) in outcomes {
            match o {
                FileImportOutcome::Imported | FileImportOutcome::AlreadyKnown => files.push(p),
                FileImportOutcome::NotDownloaded => skip(&mut skipped, &p, "cloud_not_downloaded"),
                FileImportOutcome::Missing => skip(&mut skipped, &p, "missing"),
                FileImportOutcome::Unreadable => skip(&mut skipped, &p, "unreadable"),
            }
        }
        let _ = crate::services::preview::bump_generation(state);
    }

    // ---- 3. chờ quét xong (huỷ được) ----
    let total_scans = scan_ids.len() as u32;
    loop {
        let running = scan_ids.iter().filter(|id| state.scans.is_scanning(**id)).count() as u32;
        emit(ImportPhaseDto::Scanning, total_scans - running, total_scans, 0, &skipped);
        if running == 0 || cancel.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if cancel.load(Ordering::SeqCst) {
        return Ok(ImportResultDto { added: vec![], already_there: 0, skipped, cancelled: true });
    }

    // ---- 4. lấy face của đúng những đường dẫn đã thả ----
    let exact: Vec<String> = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let found = state.with_store(|s| Ok(s.faces_at(&exact, &dirs)?))?;
    for f in &files {
        let fs = f.to_string_lossy();
        if !found.iter().any(|(p, _)| *p == *fs) {
            skip(&mut skipped, f, "unreadable");
        }
    }
    for d in &dirs {
        let prefix = format!("{d}/");
        if !found.iter().any(|(p, _)| p.starts_with(&prefix)) {
            skip(&mut skipped, Path::new(d), "not_font");
        }
    }
    let mut keys: Vec<FaceKey> = Vec::new();
    for (_, k) in &found {
        if !keys.contains(k) {
            keys.push(*k);
        }
    }

    // ---- 5. thêm vào đích: chỉ face chưa có ----
    let present = state.with_user(|u| {
        Ok(match target.kind {
            ImportTargetKindDto::Case => u.case_has_faces(target.id, &keys)?,
            ImportTargetKindDto::Collection => u.collection_has_faces(target.id, &keys)?,
        })
    })?;
    let new_keys: Vec<FaceKeyDto> = keys.iter().zip(&present).filter(|(_, has)| !**has).map(|(k, _)| FaceKeyDto::from(k)).collect();
    let already = (keys.len() - new_keys.len()) as u32;
    emit(ImportPhaseDto::Adding, 0, new_keys.len() as u32, 0, &skipped);
    if !new_keys.is_empty() {
        match target.kind {
            ImportTargetKindDto::Case => {
                cases::add_to_case(state, Some(target.id), &new_keys, None)?;
            }
            ImportTargetKindDto::Collection => library::add_to_collection(state, target.id, &new_keys)?,
        }
    }
    emit(ImportPhaseDto::Adding, new_keys.len() as u32, new_keys.len() as u32, new_keys.len() as u32, &skipped);
    Ok(ImportResultDto { added: new_keys, already_there: already, skipped, cancelled: false })
}
