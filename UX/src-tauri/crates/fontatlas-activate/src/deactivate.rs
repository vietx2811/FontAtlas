//! Tắt kích hoạt có đếm lý do, và đổi scope.
//!
//! Chỉ gọi `engine.unregister` khi tập lý do của file rỗng. Gỡ luôn theo `active_path` + `scope`
//! đã lưu, không theo đường dẫn hiện tại (file có thể đã bị xoá / đổi tên).
use std::sync::Mutex;

use fontatlas_store::{
    ActivationReason, ActivationRow, ActivationStatus, DesiredState, FaceKey, Store, put_activation_tx,
};

use crate::engine::{ActivationEngine, Scope};
use crate::service::{ActivationService, engine_failure, hex, now_ms, relock};
use crate::types::{ActivationError, ActivationReport, DeactivatePolicy, EngineError, FileOutcome, FileResult, Progress};

/// Lý do cần gỡ khỏi một file.
#[derive(Clone, Copy)]
enum Lift {
    /// `deactivate_faces`: gỡ `Manual`; collection còn giữ → hỏi (Ask) hoặc gỡ hết (Force).
    Manual(DeactivatePolicy),
    Collection(i64),
    All,
}

fn split(reasons: &[ActivationReason]) -> (bool, Vec<i64>) {
    let manual = reasons.contains(&ActivationReason::Manual);
    let colls = reasons
        .iter()
        .filter_map(|r| match r {
            ActivationReason::Collection(id) => Some(*id),
            // Auto không giữ font trước người dùng: không tính vào "collection còn giữ".
            ActivationReason::Manual | ActivationReason::Auto(_) => None,
        })
        .collect();
    (manual, colls)
}

impl<E: ActivationEngine> ActivationService<E> {
    pub fn deactivate_faces(
        &self,
        store: &Mutex<Store>,
        keys: &[FaceKey],
        policy: DeactivatePolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let mut groups: Vec<([u8; 32], Vec<u32>)> = vec![];
        for k in keys {
            match groups.iter_mut().find(|(h, _)| *h == k.hash) {
                Some((_, v)) if !v.contains(&k.face_index) => v.push(k.face_index),
                Some(_) => {}
                None => groups.push((k.hash, vec![k.face_index])),
            }
        }
        self.deactivate_hashes(store, groups, Lift::Manual(policy), progress)
    }

    /// Gỡ lý do `Collection(id)` khỏi mọi file đang mang nó (kể cả face đã rời collection).
    pub fn deactivate_collection(
        &self,
        store: &Mutex<Store>,
        collection_id: i64,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let hashes = relock(store).hashes_with_reason(ActivationReason::Collection(collection_id))?;
        let groups = hashes.into_iter().map(|h| (h, vec![])).collect();
        self.deactivate_hashes(store, groups, Lift::Collection(collection_id), progress)
    }

    /// Gọi khi một collection bị xoá (6a gọi TRƯỚC `Store::delete_collection`; gọi sau cũng đúng).
    /// Gỡ lý do `Collection(id)` khỏi mọi file còn mang nó, rồi gỡ đăng ký của mọi file đang `Active`
    /// mà không còn lý do nào (trường hợp store đã xoá lý do trong cùng transaction với collection).
    pub fn on_collection_deleted(
        &self,
        store: &Mutex<Store>,
        collection_id: i64,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let mut rep = self.deactivate_collection(store, collection_id, progress)?;
        let orphans: Vec<_> = relock(store)
            .activation_rows()?
            .into_iter()
            .filter(|r| r.state == DesiredState::Active && r.reasons.is_empty())
            .map(|r| (r.hash, vec![]))
            .collect();
        let swept = self.deactivate_hashes(store, orphans, Lift::All, progress)?;
        for r in swept.results {
            rep.push(r);
        }
        Ok(rep)
    }

    /// Nút Cài đặt "Tắt mọi font FontAtlas đã kích hoạt" (dọn trước khi gỡ app).
    pub fn deactivate_all(
        &self,
        store: &Mutex<Store>,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let rows = relock(store).activation_rows()?;
        let groups = rows
            .into_iter()
            .filter(|r| {
                r.state == DesiredState::Active
                    || matches!(
                        r.status,
                        ActivationStatus::Registered | ActivationStatus::Unknown | ActivationStatus::Error
                    )
            })
            .map(|r| (r.hash, vec![]))
            .collect();
        self.deactivate_hashes(store, groups, Lift::All, progress)
    }

    fn deactivate_hashes(
        &self,
        store: &Mutex<Store>,
        groups: Vec<([u8; 32], Vec<u32>)>,
        what: Lift,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let mut rep = ActivationReport::default();
        let total = groups.len();
        for (i, (hash, req)) in groups.into_iter().enumerate() {
            let r = self.deactivate_one(store, hash, req, what)?;
            rep.push(r);
            progress(Progress { done: i + 1, total, hash_hex: hex(&hash) });
        }
        Ok(rep)
    }

    fn deactivate_one(
        &self,
        store: &Mutex<Store>,
        hash: [u8; 32],
        requested_faces: Vec<u32>,
        what: Lift,
    ) -> Result<FileResult, ActivationError> {
        let mut res =
            FileResult { hash_hex: hex(&hash), path: None, requested_faces, extra_faces: vec![], outcome: FileOutcome::Busy, replaced: vec![], warning: None };
        let Some(_guard) = self.take(hash) else { return Ok(res) };
        let (row, target) = {
            let s = relock(store);
            (s.activation_row(&hash)?, s.file_target(&hash)?)
        };
        if let Some(t) = &target {
            res.extra_faces =
                t.faces.iter().map(|f| f.face_index).filter(|i| !res.requested_faces.contains(i)).collect();
        }
        let Some(mut row) = row else {
            res.outcome = FileOutcome::AlreadyInactive;
            return Ok(res);
        };
        res.path = row.active_path.clone();

        // Gỡ lý do (và ghi ý định nếu tập lý do rỗng) trong một transaction.
        let (_, colls) = split(&row.reasons);
        if let Lift::Manual(DeactivatePolicy::Ask) = what {
            if !colls.is_empty() {
                res.outcome = FileOutcome::HeldByCollections { collection_ids: colls };
                return Ok(res);
            }
        }
        // Dòng Unknown/Error có thể còn đăng ký ở bất kỳ scope nào (crash / gỡ lỗi lần trước).
        let uncertain = matches!(row.status, ActivationStatus::Unknown | ActivationStatus::Error);
        let needs_unregister =
            row.state == DesiredState::Active || uncertain || row.status == ActivationStatus::Registered;
        let left = relock(store).with_tx(|tx| {
            let left: Vec<ActivationReason> = match what {
                Lift::Manual(DeactivatePolicy::Ask) => {
                    // Người dùng tắt tay thì thắng lý do `auto:*` (app sẽ hỏi lại nếu còn cần).
                    tx.execute("DELETE FROM u.activation_reasons WHERE hash=?1 AND reason GLOB 'auto:*'", [&hash[..]])?;
                    fontatlas_store::remove_activation_reason_tx(tx, &hash, ActivationReason::Manual)?
                }
                Lift::Collection(id) => {
                    fontatlas_store::remove_activation_reason_tx(tx, &hash, ActivationReason::Collection(id))?
                }
                Lift::Manual(DeactivatePolicy::Force) | Lift::All => {
                    tx.execute("DELETE FROM u.activation_reasons WHERE hash=?1", [&hash[..]])?;
                    vec![]
                }
            };
            if left.is_empty() && needs_unregister {
                row.state = DesiredState::Inactive;
                row.status = ActivationStatus::Unknown; // ý định: đang gỡ
                put_activation_tx(tx, &row)?;
            }
            Ok(left)
        })?;
        if !left.is_empty() {
            let (manual, collection_ids) = split(&left);
            res.outcome = FileOutcome::KeptActive { manual, collection_ids };
            return Ok(res);
        }
        if !needs_unregister {
            res.outcome = FileOutcome::AlreadyInactive;
            return Ok(res);
        }
        res.outcome = self.unregister_row(store, &mut row, uncertain)?;
        Ok(res)
    }

    /// Gỡ đăng ký theo `active_path` + `scope` đã lưu; ghi `Unregistered` (hoặc `Error`).
    /// `uncertain` (dòng Unknown/Error): thử cả scope còn lại, bỏ qua lỗi của nó, vì một lần
    /// `set_scope` / kích hoạt bị ngắt có thể để đăng ký ở scope kia. Dùng chung với `reconcile`.
    pub(crate) fn unregister_row(
        &self,
        store: &Mutex<Store>,
        row: &mut ActivationRow,
        uncertain: bool,
    ) -> Result<FileOutcome, ActivationError> {
        let result = match &row.active_path {
            Some(p) => {
                let p = std::path::Path::new(p);
                let first = self.engine().unregister(p, row.scope);
                if uncertain {
                    let other = match row.scope {
                        Scope::Session => Scope::Persistent,
                        Scope::Persistent => Scope::Session,
                    };
                    let _ = self.engine().unregister(p, other);
                }
                first
            }
            None => Ok(()),
        };
        let outcome = match result {
            Ok(()) | Err(EngineError::NotRegistered) => {
                row.status = ActivationStatus::Unregistered;
                row.last_error = None;
                FileOutcome::Deactivated
            }
            Err(e) => {
                row.status = ActivationStatus::Error;
                row.last_error = Some(format!("engine.{}", e.code()));
                engine_failure(&e)
            }
        };
        row.verified_at = Some(now_ms());
        relock(store).put_activation(row)?;
        Ok(outcome)
    }

    /// Đổi scope các file đang bật (`hashes = None` → mọi file). Thứ tự: đăng ký scope mới trước,
    /// rồi gỡ scope cũ, để font không biến mất giữa chừng. CoreText cho phép cùng URL ở hai scope:
    /// test sống `live_same_url_other_scope` trên macOS 27.2 trả `Ok` (không phải 105).
    pub fn set_scope(
        &self,
        store: &Mutex<Store>,
        hashes: Option<&[[u8; 32]]>,
        scope: Scope,
    ) -> Result<ActivationReport, ActivationError> {
        let wanted: Vec<ActivationRow> = relock(store)
            .activation_rows()?
            .into_iter()
            .filter(|r| r.state == DesiredState::Active)
            .filter(|r| hashes.is_none_or(|hs| hs.contains(&r.hash)))
            .collect();
        let mut rep = ActivationReport::default();
        // Dòng Unknown/Error không rõ đang đăng ký ở scope nào: không đổi, nhưng BÁO để UI không im lặng.
        for r in wanted.iter().filter(|r| matches!(r.status, ActivationStatus::Unknown | ActivationStatus::Error)) {
            rep.push(FileResult {
                hash_hex: hex(&r.hash),
                path: r.active_path.clone(),
                requested_faces: vec![],
                extra_faces: vec![],
                outcome: FileOutcome::Failed {
                    code: "scope_unsettled".into(),
                    message: "font chưa đăng ký ổn định nên chưa đổi được phạm vi; hãy tắt rồi bật lại".into(),
                },
                replaced: vec![],
            warning: None,
            });
        }
        let rows: Vec<ActivationRow> =
            wanted.into_iter().filter(|r| r.status == ActivationStatus::Registered && r.scope != scope).collect();
        for mut row in rows {
            let mut res = FileResult {
                hash_hex: hex(&row.hash),
                path: row.active_path.clone(),
                requested_faces: vec![],
                extra_faces: vec![],
                outcome: FileOutcome::Busy,
                replaced: vec![],
            warning: None,
            };
            let Some(_guard) = self.take(row.hash) else {
                rep.push(res);
                continue;
            };
            // Đọc lại SAU khi giữ guard: dòng trong snapshot có thể đã bị thao tác khác đổi (vd bị tắt).
            let fresh = relock(store).activation_row(&row.hash)?;
            let Some(fresh) = fresh.filter(|r| {
                r.state == DesiredState::Active && r.status == ActivationStatus::Registered && r.scope != scope
            }) else {
                continue;
            };
            row = fresh;
            let Some(path) = row.active_path.clone() else { continue };
            let path = std::path::PathBuf::from(path);
            let old = row.scope;
            // Ý định: scope mới, status=unknown. Crash giữa chừng → reconcile đăng ký scope mới, gỡ scope cũ.
            row.scope = scope;
            row.status = ActivationStatus::Unknown;
            relock(store).put_activation(&row)?;
            match self.engine().register(&path, scope) {
                Ok(()) | Err(EngineError::AlreadyRegistered) => {
                    match self.engine().unregister(&path, old) {
                        Ok(()) | Err(EngineError::NotRegistered) => {
                            row.status = ActivationStatus::Registered;
                            row.last_error = None;
                            res.outcome = FileOutcome::Activated;
                        }
                        Err(e) => {
                            // Không ghi đè: giữ bản ghi cũ (scope/path cũ, Error) để reconcile dọn đăng ký thừa.
                            row.scope = old;
                            row.status = ActivationStatus::Error;
                            row.last_error = Some(format!("engine.{}", e.code()));
                            res.outcome = engine_failure(&e);
                        }
                    }
                    row.verified_at = Some(now_ms());
                }
                Err(e) => {
                    row.scope = old; // scope cũ vẫn đang đăng ký
                    row.status = ActivationStatus::Registered;
                    row.last_error = Some(format!("engine.{}", e.code()));
                    res.outcome = engine_failure(&e);
                }
            }
            relock(store).put_activation(&row)?;
            rep.push(res);
        }
        Ok(rep)
    }
}
