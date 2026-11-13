//! Khởi động: mở Store, phát hiện DB cũ, dựng Index, đối soát kích hoạt, rồi `ready`.
//! Quét lại các nguồn chạy nền SAU `ready` (tiến độ qua kênh quét), không chặn giao diện.
use std::sync::{Arc, Mutex};

use fontatlas_store::{LEGACY_DB_FILE, Store, legacy_preview};

use crate::dto::events::BootEvent;
use crate::dto::settings::{BootPhase, BootStatusDto, LegacyImportDto, LegacyResolveDto, LegacyStateDto};
use crate::error::CmdError;
use crate::services::keys;
use crate::state::AppState;

pub fn set_phase(state: &AppState, phase: BootPhase) {
    if let Ok(mut b) = state.boot.write() {
        b.phase = phase;
    }
    state.hubs.boot.emit(BootEvent::Phase { phase });
}

fn fail(state: &AppState, error: CmdError) {
    if let Ok(mut b) = state.boot.write() {
        b.phase = BootPhase::Failed;
        b.store_error = Some(error.clone());
    }
    state.hubs.boot.emit(BootEvent::Failed { error });
}

pub fn snapshot(state: &AppState) -> BootStatusDto {
    state.boot.read().map(|b| b.clone()).unwrap_or_default()
}

/// Luồng nền `fontatlas-boot`.
pub fn start(state: Arc<AppState>) {
    let _ = std::thread::Builder::new().name("fontatlas-boot".into()).spawn(move || {
        run_boot(&state);
    });
}

/// Đồng bộ (để test). Không panic: lỗi mở Store → phase `failed`, không ghi thêm gì.
pub fn run_boot(state: &Arc<AppState>) -> BootStatusDto {
    set_phase(state, BootPhase::Starting);
    let (store, report) = match Store::open(&state.dirs.data) {
        Ok(x) => x,
        Err(e) => {
            fail(state, e.into());
            return snapshot(state);
        }
    };
    let user = match Store::open_user_only(&state.dirs.data) {
        Ok(u) => u,
        Err(e) => {
            fail(state, e.into());
            return snapshot(state);
        }
    };
    let mut store = store;
    // lần gỡ nguồn bị ngắt giữa hai DB (crash/mất điện) để lại vị trí mồ côi: dọn. Bỏ qua khi user.db vừa bị thay
    // (hỏng → bản mới) hoặc library vừa dựng lại, vì lúc đó danh sách nguồn không còn đáng tin để so.
    if report.user_backup.is_none() && !report.library_recreated {
        match store.sweep_orphan_locations() {
            Ok(n) if n > 0 => log::warn!("dọn {n} vị trí file mồ côi của nguồn đã gỡ dở"),
            Ok(_) => {}
            Err(e) => log::warn!("dọn vị trí mồ côi lỗi: {e}"),
        }
    }
    let _ = state.store.set(Mutex::new(store));
    let _ = state.user.set(Mutex::new(user));
    if let Ok(mut b) = state.boot.write() {
        b.library_recreated = report.library_recreated;
        b.user_backup = report.user_backup.map(|p| p.to_string_lossy().into_owned());
    }
    if let Err(e) = crate::services::preview::open_service(state) {
        log::warn!("mở dịch vụ preview lỗi: {e}");
    }
    refresh_legacy(state);
    set_phase(state, BootPhase::Indexing);
    if let Err(e) = crate::index_bundle::rebuild_index(state) {
        log::warn!("dựng Index lúc khởi động lỗi: {e}");
    }
    set_phase(state, BootPhase::Activation);
    crate::services::activation::reconcile_now(state, true);
    crate::services::broker::start(state);
    crate::services::fontreq::start(state);
    set_phase(state, BootPhase::Ready);
    after_ready(state);
    snapshot(state)
}

/// Sau `ready`: quét nền khi người dùng bật `rescanOnLaunch` hoặc `ANALYZER_VERSION` đã đổi (bất kể
/// `rescanOnLaunch`), rồi mở worker embed (luôn sau Store chính).
fn after_ready(state: &Arc<AppState>) {
    let rescan_pref = state
        .with_user(|u| Ok(u.setting(keys::RESCAN_ON_LAUNCH)?))
        .ok()
        .flatten()
        .is_none_or(|v| v != "0" && v != "false");
    let seen = state.with_user(|u| Ok(u.setting(keys::ANALYZER_SEEN)?)).ok().flatten();
    let version_changed = seen.as_deref() != Some(fontatlas_core::ANALYZER_VERSION);
    if rescan_pref || version_changed {
        let _ = state.scans.enqueue_all_directories(state);
    }
    mark_analyzer_seen_if_current(state);
    refresh_loose_fonts(state);
    state.cloud_watch.sync(state);
    let _ = crate::services::embed::ensure_started(state);
    crate::services::langs::ensure_started(state);
    crate::services::cases::purge_trash(state);
}

/// Font lẻ (G8) không có thư mục để quét: đối chiếu từng file (còn/mất, đã tải/chưa) mỗi lần mở app.
fn refresh_loose_fonts(state: &Arc<AppState>) {
    let ids: Vec<i64> = state
        .with_user(|u| Ok(u.sources()?.into_iter().filter(|s| s.kind == fontatlas_store::SourceKind::Loose).map(|s| s.id).collect()))
        .unwrap_or_default();
    for id in ids {
        if let Err(e) = state.with_store(|s| Ok(s.refresh_file_source(id)?)) {
            log::warn!("làm mới font lẻ lỗi: {}", e.message);
        }
    }
}

/// Ghi `analyzer_version_seen` khi mọi face đã có bản phân tích theo phiên bản hiện tại.
pub fn mark_analyzer_seen_if_current(state: &AppState) {
    if matches!(state.with_store(|s| Ok(s.stale_analysis_count()?)), Ok(0)) {
        let _ = state.with_user(|u| Ok(u.set_setting(keys::ANALYZER_SEEN, fontatlas_core::ANALYZER_VERSION)?));
    }
}

fn read_json<T: serde::de::DeserializeOwned>(state: &AppState, key: &str) -> Option<T> {
    let v = state.with_user(|u| Ok(u.setting(key)?)).ok()??;
    serde_json::from_str(&v).ok()
}

/// Trạng thái DB cũ hiện tại (đọc chỉ-đọc `fontatlas.db`; không ghi gì).
pub fn legacy_state(state: &AppState) -> LegacyStateDto {
    let path = state.dirs.data.join(LEGACY_DB_FILE);
    if !path.exists() {
        return LegacyStateDto::default();
    }
    let imported: Option<LegacyImportDto> = read_json(state, keys::LEGACY_IMPORT);
    if imported.is_some() {
        return LegacyStateDto { found: true, imported, resolved: read_json(state, keys::LEGACY_RESOLVE), ..Default::default() };
    }
    match legacy_preview(&path) {
        Ok(p) => LegacyStateDto { found: true, preview: Some(p.into()), ..Default::default() },
        Err(e) => LegacyStateDto { found: true, error: Some(e.into()), ..Default::default() },
    }
}

pub fn refresh_legacy(state: &AppState) -> LegacyStateDto {
    let l = legacy_state(state);
    if let Ok(mut b) = state.boot.write() {
        b.legacy = l.clone();
    }
    l
}

/// Người dùng chọn nhập ở onboarding O2: nhập rồi ghi dấu; lỗi giữ DB cũ nguyên và báo trong `legacy.error`.
pub fn legacy_import(state: &Arc<AppState>) -> Result<LegacyStateDto, CmdError> {
    let path = state.dirs.data.join(LEGACY_DB_FILE);
    if !path.exists() {
        return Err(CmdError::not_found("không có DB cũ để nhập"));
    }
    let rep = state.with_store(|s| s.import_legacy(&path).map_err(CmdError::from));
    match rep {
        Ok(r) => {
            let dto = LegacyImportDto::from(r);
            let json = serde_json::to_string(&dto).map_err(|e| CmdError::internal(e.to_string()))?;
            state.with_user(|u| Ok(u.set_setting(keys::LEGACY_IMPORT, &json)?))?;
            Ok(refresh_legacy(state))
        }
        Err(e) => {
            let mut l = legacy_state(state);
            l.error = Some(e);
            if let Ok(mut b) = state.boot.write() {
                b.legacy = l.clone();
            }
            Ok(l)
        }
    }
}

/// Sau mỗi lần quét về Idle: nối favorite/collection/tag cũ theo đường dẫn (idempotent).
pub fn resolve_legacy_if_needed(state: &Arc<AppState>) {
    let Some(imp) = read_json::<LegacyImportDto>(state, keys::LEGACY_IMPORT) else { return };
    let _ = imp;
    let prev: Option<LegacyResolveDto> = read_json(state, keys::LEGACY_RESOLVE);
    if prev.as_ref().is_some_and(|p| p.unresolved == 0) {
        return;
    }
    if let Ok(r) = state.with_store(|s| s.resolve_legacy().map_err(CmdError::from)) {
        let dto = LegacyResolveDto::from(r);
        if let Ok(json) = serde_json::to_string(&dto) {
            let _ = state.with_user(|u| Ok(u.set_setting(keys::LEGACY_RESOLVE, &json)?));
        }
        refresh_legacy(state);
    }
}
