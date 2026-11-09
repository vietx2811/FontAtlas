//! Ngôn ngữ (Hyperglot): độ phủ theo cmap thật của từng face, tính bù nền, danh sách ngôn ngữ, ký tự thiếu.
//! Dữ liệu và định nghĩa mức nằm ở `fontatlas_core::langdata`; lưu ở library.db v2 (`face_langs`).
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use fontatlas_core::FontFile;
use fontatlas_core::langdata::{LangCoverage, LangLevel, langdb};
use fontatlas_store::{FaceQuery, LangRow, Store};

use crate::dto::langs::*;
use crate::dto::library::FaceQueryDto;
use crate::error::CmdError;
use crate::state::AppState;

pub const DEFAULT_BADGE_LANG: &str = "vie";
pub const MAX_MISSING_LISTED: usize = 200;

/// Mức đạt ≥ near của một face với mọi ngôn ngữ (một lần tính cmap cho mọi ngôn ngữ).
pub fn evaluate_face(face: &fontatlas_core::Face<'_>) -> Vec<LangRow> {
    fontatlas_preview::supported_langs(face.font())
        .into_iter()
        .map(|(lang, c, l)| LangRow { lang, have: c.base_have, total: c.base_total, level: l.as_str().to_string() })
        .collect()
}

/// Chờ trước khi thử lại các face lượt trước phải bỏ qua (iCloud chưa tải, volume chưa mount, ghi lỗi).
const RETRY_AFTER: Duration = Duration::from_secs(60);

/// Kết quả một lượt tính bù.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pass {
    /// Số face đã ghi.
    pub done: usize,
    /// Số face bỏ qua lượt này (vẫn pending, thử lại sau).
    pub skipped: usize,
}

/// Một luồng nền DUY NHẤT, sống suốt phiên: ngủ trên condvar, chạy lượt khi có tín hiệu
/// (`ensure_started` từ boot / sau quét) hoặc sau `RETRY_AFTER` nếu lượt trước có face bỏ qua.
#[derive(Default)]
pub struct LangSupervisor {
    /// Có lượt đang chạy hoặc đã được yêu cầu nhưng chưa xong.
    running: AtomicBool,
    stop: AtomicBool,
    spawned: AtomicBool,
    /// true = có yêu cầu chạy lượt mới.
    signal: Mutex<bool>,
    wake: Condvar,
    handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    passes: AtomicUsize,
    /// Tăng mỗi khi dữ liệu ngôn ngữ hoặc thư viện đổi: khoá của cache `language_list`.
    epoch: AtomicU64,
    cache: Mutex<Option<ListCache>>,
}

struct ListCache {
    key: String,
    epoch: u64,
    at: Instant,
    value: LangListDto,
}

impl LangSupervisor {
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        let _g = self.signal.lock().unwrap_or_else(|e| e.into_inner());
        self.wake.notify_all();
    }

    pub fn shutdown(&self) {
        self.request_stop();
        let h = self.handle.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(h) = h {
            let t = Instant::now();
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

    /// Số lượt đã chạy từ đầu phiên (test: luồng không quay vòng).
    pub fn passes(&self) -> usize {
        self.passes.load(Ordering::SeqCst)
    }

    /// Báo dữ liệu đổi (làm mất hiệu lực cache danh sách ngôn ngữ).
    pub fn bump(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }
}

/// Yêu cầu tính bù độ phủ cho face chưa có (hoặc lệch `db_version`). Mở luồng nền ở lần gọi đầu;
/// các lần sau chỉ đánh thức luồng đó. Idempotent, không bao giờ sinh thêm luồng.
pub fn ensure_started(state: &Arc<AppState>) {
    let sup = &state.langs;
    if sup.stop.load(Ordering::SeqCst) {
        return;
    }
    {
        let mut g = sup.signal.lock().unwrap_or_else(|e| e.into_inner());
        sup.running.store(true, Ordering::SeqCst);
        *g = true;
        sup.wake.notify_all();
    }
    if sup.spawned.swap(true, Ordering::SeqCst) {
        return;
    }
    let st = Arc::clone(state);
    let h = std::thread::Builder::new().name("fontatlas-langs".into()).spawn(move || worker(&st));
    match h {
        Ok(h) => *sup.handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(h),
        Err(_) => {
            sup.spawned.store(false, Ordering::SeqCst);
            sup.running.store(false, Ordering::SeqCst);
        }
    }
}

fn worker(state: &Arc<AppState>) {
    let sup = &state.langs;
    let mut retry = false;
    loop {
        // ngủ tới khi có tín hiệu (hoặc tới hạn thử lại các face bỏ qua)
        {
            let mut g = sup.signal.lock().unwrap_or_else(|e| e.into_inner());
            let mut waited = Duration::ZERO;
            while !*g && !sup.stop.load(Ordering::SeqCst) {
                if retry && waited >= RETRY_AFTER {
                    break;
                }
                let step = Duration::from_secs(1);
                let (ng, _) = sup.wake.wait_timeout(g, step).unwrap_or_else(|e| e.into_inner());
                g = ng;
                waited += step;
            }
            *g = false;
        }
        if sup.stop.load(Ordering::SeqCst) {
            break;
        }
        retry = false;
        loop {
            let p = run_pass(state);
            sup.passes.fetch_add(1, Ordering::SeqCst);
            sup.bump();
            if sup.stop.load(Ordering::SeqCst) {
                break;
            }
            if p.done > 0 && p.skipped == 0 {
                continue; // còn khả năng có face mới vào giữa lượt: kiểm lại ngay, lượt sau trống thì dừng
            }
            retry = p.skipped > 0;
            break;
        }
        let g = sup.signal.lock().unwrap_or_else(|e| e.into_inner());
        if !*g {
            sup.running.store(false, Ordering::SeqCst);
        }
    }
    sup.running.store(false, Ordering::SeqCst);
}

/// Tính bù đồng bộ một lượt (luồng nền gọi). Trả số face đã ghi.
pub fn run_backfill(state: &Arc<AppState>) -> usize {
    run_pass(state).done
}

/// Một lượt: lấy danh sách pending một lần (một lần sắp xếp), xử lý lần lượt. Face phải bỏ qua
/// (file chưa sẵn sàng, ghi lỗi) KHÔNG được đánh dấu, và không bị thử lại trong lượt này.
pub fn run_pass(state: &Arc<AppState>) -> Pass {
    let version = langdb().version().to_string();
    // kết nối chỉ library.db: không giành khoá `state.store` với giao diện
    let Ok(store) = Store::open_library_only(&state.dirs.data) else { return Pass::default() };
    let Ok(pending) = store.lang_pending(&version, usize::MAX >> 1) else { return Pass::default() };
    let mut pass = Pass::default();
    for p in pending {
        if state.langs.stop.load(Ordering::SeqCst) {
            return pass;
        }
        // nhường quét và embed: ưu tiên thấp
        while state.scans.is_busy() && !state.langs.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(250));
        }
        if fontatlas_activate::probe_file(&p.path) != fontatlas_activate::FileProbe::Ready {
            pass.skipped += 1; // chưa tải về (iCloud) hay mất: giữ pending
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
            pass.done += 1;
            if pass.done % 100 == 0 {
                state.langs.bump(); // tiến độ hiện ra dần trong lúc lượt còn chạy
            }
        } else {
            pass.skipped += 1;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if pass.skipped == 0 {
        let _ = store.prune_langs();
    }
    state.langs.bump();
    pass
}

fn level_dto(l: LangLevel) -> LangLevelDto {
    match l {
        LangLevel::Full => LangLevelDto::Full,
        LangLevel::Near => LangLevelDto::Near,
    }
}

/// Chỉ báo trên thẻ cho một hàng. Một nguồn duy nhất: mức đã lưu (`hit`) luôn thắng.
/// - có `hit` → Full/Near (kèm have/total thật);
/// - `computed` nhưng không có `hit` → dưới near: `vie` có số thiếu chính xác từ `vi_missing` (Low/Partial),
///   ngôn ngữ khác là Partial với have = 0 (không lưu số đếm dưới near; tooltip gọi `lang_coverage`);
/// - chưa tính: `vie` tạm dùng `vi_missing`, ngôn ngữ khác là Unknown.
pub fn badge_of(code: &str, hit: Option<&fontatlas_store::LangHit>, computed: bool, vi_missing: u32) -> Option<LangBadgeDto> {
    let e = langdb().get(code)?;
    let total = e.base.len() as u32;
    let mk = |state, have, total| LangBadgeDto { code: e.id.clone(), bcp47: e.bcp47.clone(), state, have, total };
    if let Some(h) = hit {
        let st = if h.level == "full" { LangBadgeStateDto::Full } else { LangBadgeStateDto::Near };
        return Some(mk(st, h.have, h.total));
    }
    if e.id == "vie" {
        // tổng 186 ký tự; số thiếu từ phân tích Việt (đúng cả khi chưa tính độ phủ, và cho mức dưới near)
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
    if computed {
        return Some(mk(LangBadgeStateDto::Partial, 0, total));
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

/// Cache `language_list` hết hạn sau từng này giây (phòng thay đổi không qua `bump`).
const LIST_TTL: Duration = Duration::from_secs(30);

pub fn language_list(state: &AppState, q: &FaceQueryDto) -> Result<LangListDto, CmdError> {
    // khoá cache: truy vấn đã bỏ phần không ảnh hưởng tới tập (ngôn ngữ, sắp xếp, phân trang)
    let mut norm = q.clone();
    norm.langs.clear();
    norm.lang_mode = Default::default();
    norm.badge_lang = None;
    norm.sort = Default::default();
    norm.descending = false;
    norm.offset = 0;
    norm.limit = 0;
    let key = serde_json::to_string(&norm).unwrap_or_default();
    let epoch = state.langs.epoch.load(Ordering::SeqCst);
    if let Some(c) = state.langs.cache.lock().unwrap_or_else(|e| e.into_inner()).as_ref()
        && c.key == key
        && c.epoch == epoch
        && c.at.elapsed() < LIST_TTL
    {
        return Ok(c.value.clone());
    }
    let db = langdb();
    let scope = crate::services::library::scope_of(state, q)?;
    let sq: FaceQuery = crate::services::library::to_store_query(q);
    let work = |s: &Store| -> Result<_, fontatlas_store::StoreError> {
        let counts = if scope.is_none() && sq.is_unfiltered() {
            s.lang_counts_unfiltered()?
        } else {
            s.lang_counts(&sq, scope.as_deref())?
        };
        Ok((counts, s.lang_progress(db.version())?))
    };
    // kết nối đọc riêng: truy vấn nặng không giữ khoá Store toàn cục (UI vẫn chạy)
    let (counts, prog) = match Store::open_reader(&state.dirs.data).and_then(|r| work(&r)) {
        Ok(v) => v,
        Err(_) => state.with_store(|s| Ok(work(s)?))?,
    };
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
    let value = LangListDto {
        db_version: db.version().to_string(),
        complete: prog.done >= prog.total,
        ready: LangReadyDto { done: prog.done, total: prog.total },
        items,
    };
    *state.langs.cache.lock().unwrap_or_else(|e| e.into_inner()) =
        Some(ListCache { key, epoch, at: Instant::now(), value: value.clone() });
    Ok(value)
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
