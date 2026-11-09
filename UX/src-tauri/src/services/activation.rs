//! Lớp mỏng trên service kích hoạt của Plan 7 (`ActivationService`): bọc object-safe, gọi trong
//! `spawn_blocking`, dịch lỗi/kết quả sang DTO, nối vòng đời (reconcile lúc mở và sau quét, gỡ session lúc thoát).
//! Mọi quy tắc kích hoạt (đơn vị file, scope, lý do, xung đột, `Busy`) nằm ở Plan 7; 6a không tự đăng ký/gỡ font.
use std::sync::{Arc, Mutex};

use fontatlas_activate::{
    ActivationEngine, ActivationError, ActivationReport, ActivationService, ActivationStateDto as CoreState, ConflictPolicy,
    DeactivatePolicy, Progress, ReconcileReport, SETTING_ACTIVATION_KEEP, SETTING_ACTIVATION_ONBOARDED, Scope,
};
use fontatlas_store::{FaceKey, Store};

use crate::dto::activation::*;
use crate::dto::events::ActivationEvent;
use crate::dto::library::{FaceKeyDto, keys_of};
use crate::error::CmdError;
use crate::state::AppState;

/// Bọc object-safe để `AppState` giữ engine thật hoặc `FakeEngine` (test) mà không generic hoá `AppState`.
pub trait ActivationApi: Send + Sync {
    /// "coretext" | "gdi" | "fake" | "none"
    fn method(&self) -> &'static str;
    /// Engine có kích hoạt được trên máy này không (Windows stub: không).
    fn supported(&self) -> bool {
        !matches!(self.method(), "gdi" | "none")
    }
    fn preflight(&self, s: &Mutex<Store>, keys: &[FaceKey]) -> Result<ActivationReport, ActivationError>;
    fn activate_faces(
        &self,
        s: &Mutex<Store>,
        keys: &[FaceKey],
        scope: Option<Scope>,
        p: ConflictPolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError>;
    fn activate_collection(
        &self,
        s: &Mutex<Store>,
        id: i64,
        scope: Option<Scope>,
        p: ConflictPolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError>;
    fn deactivate_faces(
        &self,
        s: &Mutex<Store>,
        keys: &[FaceKey],
        p: DeactivatePolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError>;
    fn deactivate_collection(&self, s: &Mutex<Store>, id: i64, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    fn deactivate_all(&self, s: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    fn on_collection_deleted(&self, s: &Mutex<Store>, id: i64, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    fn set_scope(&self, s: &Mutex<Store>, hashes: Option<&[[u8; 32]]>, scope: Scope) -> Result<ActivationReport, ActivationError>;
    fn state(&self, s: &Mutex<Store>) -> Result<Vec<CoreState>, ActivationError>;
    fn reconcile(&self, s: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ReconcileReport, ActivationError>;
    fn shutdown(&self, s: &Mutex<Store>) -> Result<usize, ActivationError>;
}

impl<E: ActivationEngine + 'static> ActivationApi for ActivationService<E> {
    fn method(&self) -> &'static str {
        self.engine().method()
    }
    fn preflight(&self, s: &Mutex<Store>, keys: &[FaceKey]) -> Result<ActivationReport, ActivationError> {
        ActivationService::preflight(self, s, keys)
    }
    fn activate_faces(
        &self,
        s: &Mutex<Store>,
        keys: &[FaceKey],
        scope: Option<Scope>,
        p: ConflictPolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        ActivationService::activate_faces(self, s, keys, scope, p, progress)
    }
    fn activate_collection(
        &self,
        s: &Mutex<Store>,
        id: i64,
        scope: Option<Scope>,
        p: ConflictPolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        ActivationService::activate_collection(self, s, id, scope, p, progress)
    }
    fn deactivate_faces(
        &self,
        s: &Mutex<Store>,
        keys: &[FaceKey],
        p: DeactivatePolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        ActivationService::deactivate_faces(self, s, keys, p, progress)
    }
    fn deactivate_collection(&self, s: &Mutex<Store>, id: i64, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        ActivationService::deactivate_collection(self, s, id, progress)
    }
    fn deactivate_all(&self, s: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        ActivationService::deactivate_all(self, s, progress)
    }
    fn on_collection_deleted(&self, s: &Mutex<Store>, id: i64, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        ActivationService::on_collection_deleted(self, s, id, progress)
    }
    fn set_scope(&self, s: &Mutex<Store>, hashes: Option<&[[u8; 32]]>, scope: Scope) -> Result<ActivationReport, ActivationError> {
        ActivationService::set_scope(self, s, hashes, scope)
    }
    fn state(&self, s: &Mutex<Store>) -> Result<Vec<CoreState>, ActivationError> {
        ActivationService::state(self, s)
    }
    fn reconcile(&self, s: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ReconcileReport, ActivationError> {
        ActivationService::reconcile(self, s, progress)
    }
    fn shutdown(&self, s: &Mutex<Store>) -> Result<usize, ActivationError> {
        ActivationService::shutdown(self, s)
    }
}

/// Trước khi có engine (hoặc ở test không cần kích hoạt): mọi thao tác ghi trả `Unsupported`.
pub struct NoActivation;

impl ActivationApi for NoActivation {
    fn method(&self) -> &'static str {
        "none"
    }
    fn preflight(&self, _: &Mutex<Store>, _: &[FaceKey]) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn activate_faces(&self, _: &Mutex<Store>, _: &[FaceKey], _: Option<Scope>, _: ConflictPolicy, _: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn activate_collection(&self, _: &Mutex<Store>, _: i64, _: Option<Scope>, _: ConflictPolicy, _: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn deactivate_faces(&self, _: &Mutex<Store>, _: &[FaceKey], _: DeactivatePolicy, _: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn deactivate_collection(&self, _: &Mutex<Store>, _: i64, _: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn deactivate_all(&self, _: &Mutex<Store>, _: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn on_collection_deleted(&self, _: &Mutex<Store>, _: i64, _: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn set_scope(&self, _: &Mutex<Store>, _: Option<&[[u8; 32]]>, _: Scope) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn state(&self, _: &Mutex<Store>) -> Result<Vec<CoreState>, ActivationError> {
        Ok(Vec::new())
    }
    fn reconcile(&self, _: &Mutex<Store>, _: &mut dyn FnMut(Progress)) -> Result<ReconcileReport, ActivationError> {
        Ok(ReconcileReport::default())
    }
    fn shutdown(&self, _: &Mutex<Store>) -> Result<usize, ActivationError> {
        Ok(0)
    }
}

/// Engine của nền tảng đang chạy (CoreText trên macOS, stub trên nơi khác).
pub fn platform_api() -> Box<dyn ActivationApi> {
    Box::new(ActivationService::new(fontatlas_activate::platform_engine()))
}

impl From<ActivationError> for CmdError {
    fn from(e: ActivationError) -> Self {
        match e {
            ActivationError::Store(s) => CmdError::from(s),
            other => CmdError::new(&format!("activation_{}", other.code()), other.to_string()),
        }
    }
}

pub fn caps(state: &AppState) -> ActivationCapsDto {
    ActivationCapsDto { method: state.activation.method().into(), supported: state.activation.supported() }
}

fn writable(state: &AppState) -> Result<(), CmdError> {
    if state.activation.supported() {
        Ok(())
    } else {
        Err(CmdError::new("activation_unsupported", "nền tảng này chưa hỗ trợ kích hoạt font"))
    }
}

fn changed_hashes(r: &ActivationReport) -> Vec<String> {
    use fontatlas_activate::FileOutcome as F;
    r.results
        .iter()
        .filter(|x| matches!(x.outcome, F::Activated | F::Deactivated | F::KeptActive { .. }))
        .map(|x| x.hash_hex.clone())
        .collect()
}

fn finish(state: &AppState, r: ActivationReport) -> ActivationReportDto {
    let hashes = changed_hashes(&r);
    if !hashes.is_empty() {
        state.hubs.activation.emit(ActivationEvent::Changed { hashes });
    }
    ActivationReportDto::from(&r)
}

fn policy(proceed: bool) -> ConflictPolicy {
    if proceed { ConflictPolicy::Proceed } else { ConflictPolicy::Ask }
}

/// Mọi dòng trạng thái kèm danh sách face của file (file một face: `[0]`; TTC: nhiều face).
pub fn state_list(state: &AppState) -> Result<Vec<ActivationStateDto>, CmdError> {
    let rows = state.activation.state(state.store()?)?;
    state.with_store(|s| {
        let mut st = s
            .conn()
            .prepare(
                "SELECT f.face_index FROM faces f JOIN files fi ON fi.file_id = f.file_id WHERE fi.hash = ?1 ORDER BY f.face_index",
            )
            .map_err(fontatlas_store::StoreError::from)?;
        rows.into_iter()
            .map(|r| {
                let hash: Vec<u8> = (0..r.hash_hex.len() / 2).filter_map(|i| u8::from_str_radix(&r.hash_hex[2 * i..2 * i + 2], 16).ok()).collect();
                let faces: Vec<u32> = st
                    .query_map([&hash[..]], |x| x.get::<_, i64>(0))
                    .map_err(fontatlas_store::StoreError::from)?
                    .filter_map(|x| x.ok().map(|v| v as u32))
                    .collect();
                Ok(ActivationStateDto::from_core(r, if faces.is_empty() { vec![0] } else { faces }))
            })
            .collect()
    })
}

pub fn preflight(state: &AppState, faces: &[FaceKeyDto]) -> Result<ActivationReportDto, CmdError> {
    writable(state)?;
    let keys = keys_of(faces)?;
    Ok(ActivationReportDto::from(&state.activation.preflight(state.store()?, &keys)?))
}

pub fn activate(
    state: &AppState,
    faces: &[FaceKeyDto],
    scope: Option<ScopeDto>,
    proceed: bool,
    progress: &mut dyn FnMut(ProgressDto),
) -> Result<ActivationReportDto, CmdError> {
    writable(state)?;
    let keys = keys_of(faces)?;
    let r = state.activation.activate_faces(state.store()?, &keys, scope.map(Into::into), policy(proceed), &mut |p| progress((&p).into()))?;
    Ok(finish(state, r))
}

pub fn activate_collection(
    state: &AppState,
    collection_id: i64,
    scope: Option<ScopeDto>,
    proceed: bool,
    progress: &mut dyn FnMut(ProgressDto),
) -> Result<ActivationReportDto, CmdError> {
    writable(state)?;
    let r = state.activation.activate_collection(state.store()?, collection_id, scope.map(Into::into), policy(proceed), &mut |p| progress((&p).into()))?;
    Ok(finish(state, r))
}

pub fn deactivate(state: &AppState, faces: &[FaceKeyDto], force: bool, progress: &mut dyn FnMut(ProgressDto)) -> Result<ActivationReportDto, CmdError> {
    writable(state)?;
    let keys = keys_of(faces)?;
    let pol = if force { DeactivatePolicy::Force } else { DeactivatePolicy::Ask };
    let r = state.activation.deactivate_faces(state.store()?, &keys, pol, &mut |p| progress((&p).into()))?;
    Ok(finish(state, r))
}

pub fn deactivate_collection(state: &AppState, collection_id: i64, progress: &mut dyn FnMut(ProgressDto)) -> Result<ActivationReportDto, CmdError> {
    writable(state)?;
    let r = state.activation.deactivate_collection(state.store()?, collection_id, &mut |p| progress((&p).into()))?;
    Ok(finish(state, r))
}

pub fn deactivate_all(state: &AppState, progress: &mut dyn FnMut(ProgressDto)) -> Result<ActivationReportDto, CmdError> {
    writable(state)?;
    let r = state.activation.deactivate_all(state.store()?, &mut |p| progress((&p).into()))?;
    Ok(finish(state, r))
}

/// Đổi phạm vi (session ↔ persistent) của các font ĐANG bật, không tắt giữa chừng (đăng ký scope mới trước, rồi gỡ scope cũ).
pub fn set_scope(state: &AppState, faces: &[FaceKeyDto], scope: ScopeDto) -> Result<ActivationReportDto, CmdError> {
    writable(state)?;
    let keys = keys_of(faces)?;
    let mut hashes: Vec<[u8; 32]> = keys.iter().map(|k| k.hash).collect();
    hashes.sort_unstable();
    hashes.dedup();
    let r = state.activation.set_scope(state.store()?, Some(&hashes), scope.into())?;
    Ok(finish(state, r))
}

/// Bật/tắt giữ font kích hoạt sau khi thoát: ghi setting, đổi scope các font đang bật, đánh dấu đã onboarding.
pub fn set_keep(state: &AppState, on: bool) -> Result<(), CmdError> {
    state.with_user(|u| {
        u.set_setting(SETTING_ACTIVATION_KEEP, if on { "1" } else { "0" })?;
        u.set_setting(SETTING_ACTIVATION_ONBOARDED, "1")?;
        Ok(())
    })?;
    if state.activation.supported() {
        let scope = if on { Scope::Persistent } else { Scope::Session };
        state.activation.set_scope(state.store()?, None, scope)?;
    }
    Ok(())
}

/// Đối soát DB ↔ hệ điều hành. `always`: gửi `Reconciled` cả khi không có gì mới (lúc mở app).
pub fn reconcile_now(state: &Arc<AppState>, always: bool) {
    if !state.activation.supported() {
        return;
    }
    let Ok(store) = state.store() else { return };
    match state.activation.reconcile(store, &mut |p| state.hubs.activation.emit(ActivationEvent::Progress { progress: (&p).into() })) {
        Ok(r) => {
            let dto = ReconcileReportDto::from(&r);
            if always || dto.is_notable() {
                state.hubs.activation.emit(ActivationEvent::Reconciled { report: dto });
            }
        }
        Err(e) => log::warn!("đối soát kích hoạt lỗi: {e}"),
    }
}

/// Xoá collection: gỡ font collection đang giữ TRƯỚC khi xoá hàng (bảng hợp đồng Plan 7).
pub fn before_delete_collection(state: &AppState, id: i64) -> Result<(), CmdError> {
    if !state.activation.supported() {
        return Ok(());
    }
    state.activation.on_collection_deleted(state.store()?, id, &mut |_| {})?;
    Ok(())
}

/// Thoát app: gỡ đăng ký scope session (persistent giữ nguyên).
pub fn on_exit(state: &AppState) {
    if !state.activation.supported() {
        return;
    }
    if let Ok(store) = state.store()
        && let Err(e) = state.activation.shutdown(store)
    {
        log::warn!("gỡ font session lúc thoát lỗi: {e}");
    }
}
