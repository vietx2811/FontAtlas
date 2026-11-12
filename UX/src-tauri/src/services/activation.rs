//! Lớp mỏng trên service kích hoạt của Plan 7 (`ActivationService`): bọc object-safe, gọi trong
//! `spawn_blocking`, dịch lỗi/kết quả sang DTO, nối vòng đời (reconcile lúc mở và sau quét, gỡ session lúc thoát).
//! Mọi quy tắc kích hoạt (đơn vị file, scope, lý do, xung đột, `Busy`) nằm ở Plan 7; 6a không tự đăng ký/gỡ font.
use std::sync::{Arc, Mutex};

use fontatlas_activate::{
    ActivationEngine, ActivationError, ActivationReport, ActivationService, Availability, ActivationStateDto as CoreState, ConflictPolicy,
    DeactivatePolicy, Progress, ReconcileReport, RestoreOutcome, SETTING_ACTIVATION_KEEP, SETTING_ACTIVATION_ONBOARDED, Scope,
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
    fn availability(&self, s: &Mutex<Store>, hashes: &[[u8; 32]]) -> Result<Vec<Availability>, ActivationError>;
    fn reconcile(&self, s: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ReconcileReport, ActivationError>;
    fn shutdown(&self, s: &Mutex<Store>) -> Result<usize, ActivationError>;
    /// "Khôi phục bản cũ" sau khi đã "Tắt bản cũ và bật bản này".
    fn restore_replaced(&self, s: &Mutex<Store>, id: i64) -> Result<RestoreOutcome, ActivationError>;
    /// "Thay bằng bản này" chỉ trên các xung đột đã xác nhận.
    fn replace_faces(
        &self,
        s: &Mutex<Store>,
        keys: &[FaceKey],
        scope: Option<Scope>,
        confirmed: &[fontatlas_activate::ConfirmedConflict],
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError>;
    /// Broker: bật font vì app thiết kế vừa hỏi (lý do `auto:<app>`, scope session).
    fn activate_auto(&self, s: &Mutex<Store>, keys: &[FaceKey], app: fontatlas_store::AutoApp) -> Result<ActivationReport, ActivationError>;
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
    fn availability(&self, s: &Mutex<Store>, hashes: &[[u8; 32]]) -> Result<Vec<Availability>, ActivationError> {
        ActivationService::availability(self, s, hashes)
    }
    fn reconcile(&self, s: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ReconcileReport, ActivationError> {
        ActivationService::reconcile(self, s, progress)
    }
    fn shutdown(&self, s: &Mutex<Store>) -> Result<usize, ActivationError> {
        ActivationService::shutdown(self, s)
    }
    fn restore_replaced(&self, s: &Mutex<Store>, id: i64) -> Result<RestoreOutcome, ActivationError> {
        ActivationService::restore_replaced(self, s, id)
    }
    fn replace_faces(
        &self,
        s: &Mutex<Store>,
        keys: &[FaceKey],
        scope: Option<Scope>,
        confirmed: &[fontatlas_activate::ConfirmedConflict],
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        ActivationService::replace_faces(self, s, keys, scope, confirmed, progress)
    }
    fn activate_auto(&self, s: &Mutex<Store>, keys: &[FaceKey], app: fontatlas_store::AutoApp) -> Result<ActivationReport, ActivationError> {
        ActivationService::activate_auto(self, s, keys, app)
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
    fn availability(&self, _: &Mutex<Store>, _: &[[u8; 32]]) -> Result<Vec<Availability>, ActivationError> {
        Ok(Vec::new())
    }
    fn reconcile(&self, _: &Mutex<Store>, _: &mut dyn FnMut(Progress)) -> Result<ReconcileReport, ActivationError> {
        Ok(ReconcileReport::default())
    }
    fn shutdown(&self, _: &Mutex<Store>) -> Result<usize, ActivationError> {
        Ok(0)
    }
    fn restore_replaced(&self, _: &Mutex<Store>, _: i64) -> Result<RestoreOutcome, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn replace_faces(&self, _: &Mutex<Store>, _: &[FaceKey], _: Option<Scope>, _: &[fontatlas_activate::ConfirmedConflict], _: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
    }
    fn activate_auto(&self, _: &Mutex<Store>, _: &[FaceKey], _: fontatlas_store::AutoApp) -> Result<ActivationReport, ActivationError> {
        Err(ActivationError::Unsupported)
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

/// "Tắt bản cũ và bật bản này": như `activate` nhưng gỡ các bản xung đột theo ma trận (xem fontatlas-activate).
pub fn replace(
    state: &AppState,
    faces: &[FaceKeyDto],
    scope: Option<ScopeDto>,
    confirmed: &[ConfirmedConflictDto],
    progress: &mut dyn FnMut(ProgressDto),
) -> Result<ActivationReportDto, CmdError> {
    writable(state)?;
    let keys = keys_of(faces)?;
    let confirmed = confirmed
        .iter()
        .map(|c| Ok(fontatlas_activate::ConfirmedConflict { hash: FaceKey::from_hex(&c.hash_hex, 0).map_err(|e| CmdError::invalid_arg(e.to_string()))?.hash, existing_path: c.existing_path.clone() }))
        .collect::<Result<Vec<_>, CmdError>>()?;
    let r = state.activation.replace_faces(state.store()?, &keys, scope.map(Into::into), &confirmed, &mut |p| progress((&p).into()))?;
    Ok(finish(state, r))
}

/// Các bản cũ đã gỡ và chưa khôi phục (Cài đặt → Kích hoạt).
pub fn list_replaced(state: &AppState) -> Result<Vec<ReplacedEntryDto>, CmdError> {
    state.with_store(|s| {
        Ok(s.unrestored_replaced()?
            .into_iter()
            .map(|r| ReplacedEntryDto {
                id: r.id,
                hash_hex: r.hash.iter().map(|b| format!("{b:02x}")).collect(),
                path: r.old_path,
                owner: r.owner,
                scopes: r.scopes.into_iter().map(Into::into).collect(),
                postscript_names: r.postscript_names,
                created_at: r.created_at,
            })
            .collect())
    })
}

/// "Khôi phục bản cũ": đăng ký lại bản đã gỡ (cùng URL + scope) và tắt bản của Font Atlas đã thay nó.
pub fn restore_replaced(state: &AppState, id: i64) -> Result<RestoreOutcomeDto, CmdError> {
    writable(state)?;
    // hash của font Atlas đã thay: để giao diện cập nhật chấm bật/tắt của nó
    let hash_hex = state.with_store(|s| Ok(s.get_replaced(id)?.map(|r| r.hash.iter().map(|b| format!("{b:02x}")).collect::<String>())))?;
    let out = state.activation.restore_replaced(state.store()?, id)?;
    if let (RestoreOutcome::Restored, Some(h)) = (out, hash_hex) {
        state.hubs.activation.emit(ActivationEvent::Changed { hashes: vec![h] });
    }
    Ok(out.into())
}

/// "Bật tất cả" theo Dự án: font KHÔNG `missing` của Case đi qua đúng luồng `activate` (xung đột tên family/PostScript,
/// tải trước file cloud, báo cáo theo file); font `missing` không gửi xuống engine mà liệt kê ở `skipped_missing`.
/// Kích hoạt kiểu thủ công (không gắn lý do "dự án": tắt Dự án không tự tắt font).
pub fn activate_case(
    state: &AppState,
    case_id: i64,
    scope: Option<ScopeDto>,
    proceed: bool,
    progress: &mut dyn FnMut(ProgressDto),
) -> Result<CaseActivationDto, CmdError> {
    writable(state)?;
    let case = crate::services::cases::get_case(state, case_id)?;
    let (present, gone): (Vec<_>, Vec<_>) = case.faces.into_iter().partition(|f| !f.missing);
    let keys: Vec<FaceKeyDto> = present.into_iter().map(|f| f.key).collect();
    let report = if keys.is_empty() {
        ActivationReportDto { results: vec![], changed: 0, needs_confirmation: 0, failed: 0 }
    } else {
        activate(state, &keys, scope, proceed, progress)?
    };
    Ok(CaseActivationDto { report, skipped_missing: gone.into_iter().map(|f| f.key).collect() })
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

/// Font nào đang "Có sẵn trên máy" (file giống hệt do bên khác cung cấp)? Tính lại mỗi lần; `faces` là các face
/// cần kiểm thêm ngoài tập đã nhớ. Rỗng khi nền tảng chưa hỗ trợ.
pub fn availability(state: &AppState, faces: &[FaceKeyDto]) -> Result<Vec<AvailabilityDto>, CmdError> {
    if !state.activation.supported() {
        return Ok(vec![]);
    }
    let keys = keys_of(faces)?;
    let mut hashes: Vec<[u8; 32]> = keys.iter().map(|k| k.hash).collect();
    hashes.sort_unstable();
    hashes.dedup();
    let list = state.activation.availability(state.store()?, &hashes)?;
    state.with_store(|s| {
        let mut st = s
            .conn()
            .prepare("SELECT f.face_index FROM faces f JOIN files fi ON fi.file_id = f.file_id WHERE fi.hash = ?1 ORDER BY f.face_index")
            .map_err(fontatlas_store::StoreError::from)?;
        list.into_iter()
            .map(|a| {
                let hash: Vec<u8> = (0..a.hash_hex.len() / 2).filter_map(|i| u8::from_str_radix(&a.hash_hex[2 * i..2 * i + 2], 16).ok()).collect();
                let mut fs: Vec<u32> = st
                    .query_map([&hash[..]], |x| x.get::<_, i64>(0))
                    .map_err(fontatlas_store::StoreError::from)?
                    .filter_map(|x| x.ok().map(|v| v as u32))
                    .collect();
                if fs.is_empty() {
                    fs.push(0);
                }
                Ok(AvailabilityDto { hash_hex: a.hash_hex, faces: fs, owner: a.owner.into(), path: a.path })
            })
            .collect()
    })
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

/// Setting user.db: lựa chọn đã nhớ cho cặp VF↔static (G7). Thiếu / giá trị lạ = hỏi mỗi lần.
pub const SETTING_VF_POLICY: &str = "activation_vf_policy";

pub fn vf_policy(state: &AppState) -> Result<VfPolicyDto, CmdError> {
    state.with_user(|u| Ok(vf_policy_of(u.setting(SETTING_VF_POLICY)?.as_deref())))
}

fn vf_policy_of(raw: Option<&str>) -> VfPolicyDto {
    match raw {
        Some("vf") => VfPolicyDto::Vf,
        Some("static") => VfPolicyDto::Static,
        Some("both") => VfPolicyDto::Both,
        _ => VfPolicyDto::Ask,
    }
}

/// `Ask` xoá lựa chọn đã nhớ (không ghi chuỗi "ask": bản cũ đọc setting này sẽ coi là hỏi mỗi lần).
pub fn set_vf_policy(state: &AppState, policy: VfPolicyDto) -> Result<(), CmdError> {
    let v = match policy {
        VfPolicyDto::Ask => "",
        VfPolicyDto::Vf => "vf",
        VfPolicyDto::Static => "static",
        VfPolicyDto::Both => "both",
    };
    state.with_user(|u| Ok(u.set_setting(SETTING_VF_POLICY, v)?))
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
