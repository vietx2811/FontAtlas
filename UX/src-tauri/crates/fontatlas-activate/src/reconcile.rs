//! Đối soát lúc mở app (`reconcile`) và gỡ scope session lúc thoát (`shutdown`).
//!
//! `reconcile` không kiểm tra xung đột lại: người dùng đã quyết lúc kích hoạt. Font bị tắt vì
//! collection đã xoá / face rời collection được trả trong `auto_deactivated`, không tắt âm thầm.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use fontatlas_store::{ActivationReason, ActivationRow, ActivationStatus, DesiredState, PrunedReasons, Store};

use crate::engine::{ActivationEngine, Scope};
use crate::service::{ActivationService, Located, engine_failure, hex, locate, now_ms, path_str, relock};
use crate::types::{ActivationError, EngineError, FileOutcome, FileResult, Progress};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoDeactivated {
    pub hash_hex: String,
    pub faces: Vec<u32>,
    pub removed_reasons: Vec<ActivationReason>,
    /// "collection_deleted" | "left_collection" | "no_reason"
    pub cause: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileReport {
    pub auto_deactivated: Vec<AutoDeactivated>,
    /// Như `auto_deactivated` nhưng hệ điều hành từ chối gỡ: font còn đăng ký, `reconcile` lần sau thử lại.
    pub auto_deactivate_failed: Vec<AutoDeactivated>,
    pub reregistered: usize,
    pub moved: usize,
    pub missing: usize,
    pub not_downloaded: usize,
    pub cleaned: usize,
    pub failed: usize,
    pub results: Vec<FileResult>,
}

fn other(scope: Scope) -> Scope {
    match scope {
        Scope::Session => Scope::Persistent,
        Scope::Persistent => Scope::Session,
    }
}

/// Dòng có thể còn đăng ký trong hệ thống (cần gỡ khi không còn được mong muốn).
fn maybe_registered(r: &ActivationRow) -> bool {
    matches!(r.status, ActivationStatus::Registered | ActivationStatus::Unknown | ActivationStatus::Error)
}

impl<E: ActivationEngine> ActivationService<E> {
    pub fn reconcile(
        &self,
        store: &Mutex<Store>,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ReconcileReport, ActivationError> {
        let (pruned, existing, rows): (PrunedReasons, HashSet<i64>, Vec<ActivationRow>) = {
            let mut s = relock(store);
            let pruned = s.prune_collection_reasons()?;
            let existing = s.collections()?.into_iter().map(|c| c.id).collect();
            (pruned, existing, s.activation_rows()?)
        };
        let mut rep = ReconcileReport::default();
        let total = rows.len();
        for (i, row) in rows.into_iter().enumerate() {
            let hash = row.hash;
            let mut res = FileResult {
                hash_hex: hex(&hash),
                path: row.active_path.clone(),
                requested_faces: vec![],
                extra_faces: vec![],
                outcome: FileOutcome::Busy,
                replaced: vec![],
            warning: None,
            };
            if let Some(_guard) = self.take(hash) {
                // Đọc lại SAU khi giữ guard: snapshot có thể đã cũ (thao tác khác vừa đổi dòng này).
                let fresh = relock(store).activation_row(&hash)?;
                res.outcome = match fresh {
                    Some(fresh) => {
                        res.path = fresh.active_path.clone();
                        self.reconcile_one(store, fresh, &pruned, &existing, &mut rep)?
                    }
                    None => FileOutcome::AlreadyInactive,
                };
            }
            if matches!(res.outcome, FileOutcome::Failed { .. }) {
                rep.failed += 1;
            }
            rep.results.push(res);
            progress(Progress { done: i + 1, total, hash_hex: hex(&hash) });
        }
        Ok(rep)
    }

    fn reconcile_one(
        &self,
        store: &Mutex<Store>,
        mut row: ActivationRow,
        pruned: &PrunedReasons,
        existing: &HashSet<i64>,
        rep: &mut ReconcileReport,
    ) -> Result<FileOutcome, ActivationError> {
        // Không còn được mong muốn: gỡ nếu có thể còn đăng ký.
        if row.state == DesiredState::Inactive {
            if !maybe_registered(&row) {
                return Ok(FileOutcome::AlreadyInactive);
            }
            let uncertain = matches!(row.status, ActivationStatus::Unknown | ActivationStatus::Error);
            let out = self.unregister_row(store, &mut row, uncertain)?;
            if out == FileOutcome::Deactivated {
                rep.cleaned += 1;
            }
            return Ok(out);
        }
        // Active nhưng hết lý do (sau bước 0) → tắt, và báo nếu là do bước 0.
        if row.reasons.is_empty() {
            row.state = DesiredState::Inactive;
            let uncertain = matches!(row.status, ActivationStatus::Unknown | ActivationStatus::Error);
            let out = self.unregister_row(store, &mut row, uncertain)?;
            if out == FileOutcome::Deactivated {
                rep.cleaned += 1;
            }
            // Không tắt âm thầm: mọi file bị tắt vì hết lý do đều được báo cho UI.
            let faces = relock(store)
                .file_target(&row.hash)?
                .map(|t| t.faces.iter().map(|f| f.face_index).collect())
                .unwrap_or_default();
            let (removed, cause) = match pruned.iter().find(|(h, _)| *h == row.hash) {
                Some((_, removed)) => {
                    let deleted = removed
                        .iter()
                        .any(|r| matches!(r, ActivationReason::Collection(id) if !existing.contains(id)));
                    (removed.clone(), if deleted { "collection_deleted" } else { "left_collection" })
                }
                // Lý do đã bị gỡ trước đó (store.delete_collection gỡ trong cùng transaction): không còn biết vì sao.
                None => (vec![], "no_reason"),
            };
            let entry = AutoDeactivated { hash_hex: hex(&row.hash), faces, removed_reasons: removed, cause: cause.into() };
            if out == FileOutcome::Deactivated {
                rep.auto_deactivated.push(entry);
            } else {
                rep.auto_deactivate_failed.push(entry);
            }
            return Ok(out);
        }

        row.verified_at = Some(now_ms());
        let was_unknown = matches!(row.status, ActivationStatus::Unknown | ActivationStatus::Error);
        let target = relock(store).file_target(&row.hash)?;
        let located = target.as_ref().map(locate).unwrap_or(Located::Missing(None));
        let path = match located {
            Located::Ready(p) => p,
            gone => {
                // File không dùng được: gỡ đăng ký cũ để không còn font "ma"; giữ mong muốn.
                if let Some(p) = row.active_path.clone() {
                    for s in [row.scope, other(row.scope)] {
                        let _ = self.engine().unregister(Path::new(&p), s);
                    }
                }
                let (status, outcome) = match gone {
                    Located::NotDownloaded(_) => {
                        rep.not_downloaded += 1;
                        (ActivationStatus::NotDownloaded, FileOutcome::NotDownloaded)
                    }
                    _ => {
                        rep.missing += 1;
                        (ActivationStatus::MissingFile, FileOutcome::MissingFile)
                    }
                };
                row.status = status;
                relock(store).put_activation(&row)?;
                return Ok(outcome);
            }
        };

        let moved = row.active_path.as_deref() != Some(path_str(&path).as_str());
        if moved {
            if let Some(old) = row.active_path.clone() {
                for s in [row.scope, other(row.scope)] {
                    let _ = self.engine().unregister(Path::new(&old), s);
                }
            }
        } else if was_unknown {
            // Crash giữa chừng (activate hoặc set_scope): chỉ giữ đúng scope đã lưu.
            let _ = self.engine().unregister(&path, other(row.scope));
        }
        let outcome = match self.engine().register(&path, row.scope) {
            Ok(()) => {
                if moved {
                    rep.moved += 1;
                } else {
                    rep.reregistered += 1;
                }
                FileOutcome::Activated
            }
            Err(EngineError::AlreadyRegistered) => {
                if moved {
                    rep.moved += 1;
                }
                FileOutcome::AlreadyActive
            }
            Err(e) => {
                row.status = ActivationStatus::Error;
                row.last_error = Some(format!("engine.{}", e.code()));
                relock(store).put_activation(&row)?;
                return Ok(engine_failure(&e));
            }
        };
        row.active_path = Some(path_str(&path));
        row.status = ActivationStatus::Registered;
        row.last_error = None;
        relock(store).put_activation(&row)?;
        Ok(outcome)
    }

    /// Lúc thoát app: gỡ mọi đăng ký scope session, giữ `state=Active` để lần mở sau bật lại.
    /// Persistent không bị đụng. Lỗi từng dòng không dừng vòng lặp. Trả số dòng đã gỡ.
    pub fn shutdown(&self, store: &Mutex<Store>) -> Result<usize, ActivationError> {
        let rows: Vec<ActivationRow> = relock(store)
            .activation_rows()?
            .into_iter()
            .filter(|r| r.scope == Scope::Session && maybe_registered(r))
            .collect();
        let mut n = 0;
        for snapshot in rows {
            let Some(_guard) = self.take(snapshot.hash) else { continue };
            // Đọc lại SAU khi giữ guard: không ghi đè bằng snapshot cũ.
            let fresh = relock(store).activation_row(&snapshot.hash)?;
            let Some(mut row) = fresh.filter(|r| r.scope == Scope::Session && maybe_registered(r)) else { continue };
            let Some(p) = row.active_path.clone().map(PathBuf::from) else { continue };
            match self.engine().unregister(&p, Scope::Session) {
                Ok(()) | Err(EngineError::NotRegistered) => {
                    row.status = ActivationStatus::Unregistered;
                    n += 1;
                }
                Err(e) => row.last_error = Some(format!("engine.{}", e.code())),
            }
            relock(store).put_activation(&row)?;
        }
        Ok(n)
    }
}
