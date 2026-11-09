//! Ngôn ngữ (Hyperglot): độ phủ theo cmap thật của từng face, tính bù nền, danh sách ngôn ngữ, ký tự thiếu.
//! Dữ liệu và định nghĩa mức nằm ở `fontatlas_core::langdata`; lưu ở library.db v2 (`face_langs`).
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fontatlas_core::FontFile;
use fontatlas_core::langdata::{LangCoverage, LangLevel, langdb};
use fontatlas_store::{FaceKey, FaceQuery, LangRow, Store};

use crate::dto::langs::*;
use crate::dto::library::FaceQueryDto;
use crate::error::CmdError;
use crate::state::AppState;

pub const DEFAULT_BADGE_LANG: &str = "vie";
pub const MAX_MISSING_LISTED: usize = 200;
const BATCH: usize = 64;

/// Mức đạt ≥ near của một face với mọi ngôn ngữ (một lần tính cmap cho mọi ngôn ngữ).
pub fn evaluate_face(face: &fontatlas_core::Face<'_>) -> Vec<LangRow> {
    fontatlas_preview::supported_langs(face.font())
        .into_iter()
        .map(|(lang, c, l)| LangRow { lang, have: c.base_have, total: c.base_total, level: l.as_str().to_string() })
        .collect()
}

#[derive(Default)]
pub struct LangSupervisor {
    running: AtomicBool,
    stop: AtomicBool,
    handle: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl LangSupervisor {
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    pub fn shutdown(&self) {
        self.request_stop();
        let h = self.handle.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(h) = h {
            let t = std::time::Instant::now();
            while !h.is_finished() && t.elapsed() < Duration::from_secs(5) {
                std::thread::sleep(Duration::from_millis(20));
            }
            if h.is_finished() {
                let _ = h.join();
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Mở luồng nền tính bù độ phủ cho face chưa có (hoặc lệch `db_version`) nếu chưa chạy. Idempotent.
pub fn ensure_started(state: &Arc<AppState>) {
    let sup = &state.langs;
    if sup.stop.load(Ordering::SeqCst) || sup.running.swap(true, Ordering::SeqCst) {
        return;
    }
    let st = Arc::clone(state);
    let h = std::thread::Builder::new().name("fontatlas-langs".into()).spawn(move || {
        run_backfill(&st);
        st.langs.running.store(false, Ordering::SeqCst);
        // face mới vào đúng lúc luồng thoát: kiểm lại một lần
        if matches!(pending_count(&st), Ok(n) if n > 0) {
            ensure_started(&st);
        }
    });
    match h {
        Ok(h) => *sup.handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(h),
        Err(_) => sup.running.store(false, Ordering::SeqCst),
    }
}

fn pending_count(state: &AppState) -> Result<usize, CmdError> {
    state.with_store(|s| Ok(s.lang_pending(langdb().version(), 1)?.len()))
}

/// Tính bù đồng bộ (luồng nền gọi; test gọi trực tiếp). Trả số face đã ghi.
pub fn run_backfill(state: &Arc<AppState>) -> usize {
    let version = langdb().version().to_string();
    // kết nối chỉ library.db: không giành khoá `state.store` với giao diện
    let Ok(store) = Store::open_library_only(&state.dirs.data) else { return 0 };
    let mut skipped: HashSet<FaceKey> = HashSet::new();
    let mut done = 0usize;
    loop {
        if state.langs.stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(batch) = store.lang_pending(&version, BATCH + skipped.len()) else { break };
        let batch: Vec<_> = batch.into_iter().filter(|p| !skipped.contains(&p.key)).take(BATCH).collect();
        if batch.is_empty() {
            let _ = store.prune_langs();
            break;
        }
        for p in batch {
            if state.langs.stop.load(Ordering::SeqCst) {
                return done;
            }
            // nhường quét và embed: ưu tiên thấp
            while state.scans.is_busy() && !state.langs.stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(250));
            }
            if fontatlas_activate::probe_file(&p.path) != fontatlas_activate::FileProbe::Ready {
                skipped.insert(p.key); // chưa tải về (iCloud) hay mất: thử lại lần sau, không đánh dấu
                continue;
            }
            let rows = match FontFile::open(&p.path).and_then(|f| {
                let faces = f.faces()?;
                Ok(faces.iter().find(|x| x.index() == p.key.face_index).map(evaluate_face))
            }) {
                Ok(Some(r)) => r,
                _ => Vec::new(), // font hỏng/không đọc được: đánh dấu đã tính, không nhóm nào đạt
            };
            if store.put_face_langs(&p.key, &version, &rows).is_ok() {
                done += 1;
            } else {
                skipped.insert(p.key);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    done
}

fn level_dto(l: LangLevel) -> LangLevelDto {
    match l {
        LangLevel::Full => LangLevelDto::Full,
        LangLevel::Near => LangLevelDto::Near,
    }
}

/// Chỉ báo trên thẻ cho một hàng.
pub fn badge_of(code: &str, hit: Option<&fontatlas_store::LangHit>, vi_missing: u32) -> Option<LangBadgeDto> {
    let e = langdb().get(code)?;
    let total = e.base.len() as u32;
    let mk = |state, have, total| LangBadgeDto { code: e.id.clone(), bcp47: e.bcp47.clone(), state, have, total };
    if let Some(h) = hit {
        let st = if h.level == "full" { LangBadgeStateDto::Full } else { LangBadgeStateDto::Near };
        return Some(mk(st, h.have, h.total));
    }
    if e.id == "vie" {
        // giữ logic cũ: tổng 186 ký tự, số thiếu từ phân tích Việt
        let have = total.saturating_sub(vi_missing);
        let ratio = have as f32 / total.max(1) as f32;
        let st = if vi_missing == 0 {
            LangBadgeStateDto::Full
        } else if ratio < 0.25 {
            LangBadgeStateDto::Low
        } else {
            LangBadgeStateDto::Partial
        };
        return Some(mk(st, have, total));
    }
    Some(mk(LangBadgeStateDto::Unknown, 0, total))
}

pub(crate) fn lang_filters(q: &FaceQueryDto) -> (Vec<fontatlas_store::LangFilter>, bool) {
    (
        q.langs
            .iter()
            .map(|l| fontatlas_store::LangFilter { code: l.code.clone(), near_ok: l.level == LangLevelDto::Near })
            .collect(),
        q.lang_mode == LangModeDto::Any,
    )
}

pub fn language_list(state: &AppState, q: &FaceQueryDto) -> Result<LangListDto, CmdError> {
    let db = langdb();
    let scope = crate::services::library::scope_of(state, q)?;
    let sq: FaceQuery = crate::services::library::to_store_query(q);
    let (counts, prog) = state.with_store(|s| Ok((s.lang_counts(&sq, scope.as_deref())?, s.lang_progress(db.version())?)))?;
    let items = counts
        .into_iter()
        .filter_map(|c| {
            let e = db.get(&c.lang)?;
            Some(LangItemDto {
                code: e.id.clone(),
                bcp47: e.bcp47.clone(),
                name_en: e.name_en.clone(),
                autonym: e.autonym.clone(),
                script: e.script.clone(),
                full: c.full,
                near: c.near,
            })
        })
        .collect();
    Ok(LangListDto {
        db_version: db.version().to_string(),
        complete: prog.done >= prog.total,
        ready: LangReadyDto { done: prog.done, total: prog.total },
        items,
    })
}

fn coverage_dto(code: &str, c: &LangCoverage) -> LangCoverageDto {
    let ratio = c.base_have as f32 / c.base_total.max(1) as f32;
    let level = match c.level() {
        Some(l) => match level_dto(l) {
            LangLevelDto::Full => LangCoverageLevelDto::Full,
            LangLevelDto::Near => LangCoverageLevelDto::Near,
        },
        None if ratio < 0.25 => LangCoverageLevelDto::Low,
        None => LangCoverageLevelDto::Partial,
    };
    let truncated = c.missing.len() > MAX_MISSING_LISTED || c.missing_marks.len() > MAX_MISSING_LISTED;
    LangCoverageDto {
        code: code.to_string(),
        level,
        base_have: c.base_have,
        base_total: c.base_total,
        marks_have: c.marks_have,
        marks_total: c.marks_total,
        missing: c.missing.iter().take(MAX_MISSING_LISTED).cloned().collect(),
        missing_marks: c.missing_marks.iter().take(MAX_MISSING_LISTED).cloned().collect(),
        truncated,
    }
}

/// Ký tự thiếu của một face với một ngôn ngữ, đọc cmap thật từ file (tooltip trên thẻ).
pub fn lang_coverage(state: &AppState, face_id: i64, code: &str) -> Result<LangCoverageDto, CmdError> {
    let e = langdb().get(code).ok_or_else(|| CmdError::not_found("không có ngôn ngữ này"))?;
    let (key, path) = state
        .with_store(|s| Ok(s.face_file(face_id)?))?
        .ok_or_else(|| CmdError::not_found("không tìm thấy file của face này"))?;
    if fontatlas_activate::probe_file(&path) != fontatlas_activate::FileProbe::Ready {
        return Err(CmdError::new("unavailable", "file chưa sẵn sàng (chưa tải về hoặc đã mất)"));
    }
    let file = FontFile::open(&path).map_err(|e| CmdError::new("invalid_font", e.to_string()))?;
    let faces = file.faces().map_err(|e| CmdError::new("invalid_font", e.to_string()))?;
    let face = faces
        .iter()
        .find(|f| f.index() == key.face_index)
        .ok_or_else(|| CmdError::not_found("không có face này trong file"))?;
    let graphemes: Vec<String> = e.base.iter().chain(&e.marks).cloned().collect();
    let missing = fontatlas_preview::missing_set(face.font(), &graphemes);
    Ok(coverage_dto(code, &e.evaluate(&missing)))
}
