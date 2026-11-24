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

/// Lượt đối soát nào: `Startup` = đúng MỘT lần lúc mở app (chạy trước khi broker/fontreq nhận yêu cầu) — lượt duy nhất được xoá lý do
/// auto-on của phiên trước. `Routine` = mọi lượt khác (sau quét, remove/rebuild nguồn, lượt bù): KHÔNG đụng lý do auto, vì font auto-on
/// của phiên hiện tại đang được dùng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileMode {
    Startup,
    Routine,
}

/// Dữ liệu dùng chung của một lượt đối soát (gom lại để `reconcile_one` không có quá nhiều tham số).
struct Pass<'a> {
    pruned: &'a PrunedReasons,
    existing: &'a HashSet<i64>,
    auto_cleared: &'a HashSet<[u8; 32]>,
    mode: ReconcileMode,
}

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
    /// Font chỉ còn lý do auto-on của phiên trước nên bị tắt lúc mở lại app (auto-on là tạm thời; không hiện như "bị tắt âm thầm").
    #[serde(skip)] // chỉ để test/log; không đổi hợp đồng JSON của báo cáo
    pub session_auto_cleared: usize,
    pub moved: usize,
    pub missing: usize,
    pub not_downloaded: usize,
    pub cleaned: usize,
    pub failed: usize,
    /// Hash bị thao tác khác giữ nên lượt này bỏ qua (lượt sau/bù sẽ làm); không phải lỗi nhưng cũng không phải đã xong.
    pub busy: usize,
    pub results: Vec<FileResult>,
}

/// Kết quả `unregister_orphans`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OrphanReport {
    /// Số font session mồ côi đã gỡ.
    pub unregistered: usize,
    /// Số font hệ điều hành từ chối gỡ.
    pub failed: usize,
    /// Không liệt kê được font hệ thống → không dọn gì.
    pub listing_failed: bool,
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
    /// Lượt đối soát thường (không đụng lý do auto-on của phiên hiện tại).
    pub fn reconcile(
        &self,
        store: &Mutex<Store>,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ReconcileReport, ActivationError> {
        self.reconcile_with(store, ReconcileMode::Routine, progress)
    }

    /// Lượt đối soát lúc mở app: như `reconcile` nhưng trước đó xoá lý do auto-on của phiên trước (xem `ReconcileMode`).
    pub fn reconcile_startup(
        &self,
        store: &Mutex<Store>,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ReconcileReport, ActivationError> {
        self.reconcile_with(store, ReconcileMode::Startup, progress)
    }

    pub fn reconcile_with(
        &self,
        store: &Mutex<Store>,
        mode: ReconcileMode,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ReconcileReport, ActivationError> {
        let (pruned, existing, auto_cleared, rows): (PrunedReasons, HashSet<i64>, HashSet<[u8; 32]>, Vec<ActivationRow>) = {
            let mut s = relock(store);
            let pruned = s.prune_collection_reasons()?;
            // Auto-on chỉ sống trong một phiên của app: bỏ lý do auto của lần chạy trước (kể cả khi app bị kill, không chạy `shutdown`).
            // CHỈ ở lượt Startup: lượt Routine chạy giữa phiên (sau quét, bù…) mà xoá thì gỡ mất font vừa bật cho app thiết kế.
            let auto_cleared = if mode == ReconcileMode::Startup { s.prune_session_auto_reasons()?.into_iter().collect() } else { HashSet::new() };
            let existing = s.collections()?.into_iter().map(|c| c.id).collect();
            (pruned, existing, auto_cleared, s.activation_rows()?)
        };
        let pass = Pass { pruned: &pruned, existing: &existing, auto_cleared: &auto_cleared, mode };
        let mut rep = ReconcileReport::default();
        self.finish_pending_relocations(store);
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
                        self.reconcile_one(store, fresh, &pass, &mut rep)?
                    }
                    None => FileOutcome::AlreadyInactive,
                };
            }
            if matches!(res.outcome, FileOutcome::Failed { .. }) {
                rep.failed += 1;
            }
            if res.outcome == FileOutcome::Busy {
                rep.busy += 1;
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
        pass: &Pass,
        rep: &mut ReconcileReport,
    ) -> Result<FileOutcome, ActivationError> {
        let Pass { pruned, existing, auto_cleared, mode } = *pass;
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
            // Chỉ còn lý do auto-on của phiên trước (và không có collection nào bị xoá): tắt có chủ ý, tạm thời theo thiết kế, không báo như sự cố.
            // Lượt Startup còn coi dòng "Active, không lý do, scope session" là đã dọn: app có thể đã chết giữa lúc xoá lý do auto và lúc tắt
            // dòng này (lần chạy trước), nên lần này không còn dấu vết của auto nữa.
            let session_cleared = auto_cleared.contains(&row.hash) || (mode == ReconcileMode::Startup && row.scope == Scope::Session);
            if session_cleared && !pruned.iter().any(|(h, _)| *h == row.hash) {
                if out == FileOutcome::Deactivated {
                    rep.session_auto_cleared += 1;
                }
                return Ok(out);
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
        // B7: đường đang đăng ký còn là vị trí hợp lệ (đúng hash) thì GIỮ, không nhảy sang vị trí khác chỉ vì nó đứng đầu danh sách
        // (thêm bản chép vào Organized Library không được âm thầm đổi đường của font đang bật; đổi đường là việc của `relocate`).
        let sticky = match (&row.active_path, &target) {
            (Some(cur), Some(t)) => match crate::service::locate_where(t, |l| path_str(&l.path) == *cur || crate::paths::canonical_path(&l.path).is_ok_and(|c| path_str(&c) == *cur)) {
                Located::Ready(p) => Some(Located::Ready(p)),
                _ => None,
            },
            _ => None,
        };
        let located = sticky.unwrap_or_else(|| target.as_ref().map(locate).unwrap_or(Located::Missing(None)));
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

    /// Dọn đăng ký session MỒ CÔI: font còn đăng ký ở scope session (sống qua kill/crash vì gắn với phiên đăng nhập, không với process)
    /// nhưng DB này ghi là KHÔNG bật. Chỉ xét file mà DB này từng bật (dòng `activation_state` còn lại với `state=Inactive` và
    /// `active_path`), không phải mọi file thư viện; bỏ qua file nằm trong thư mục font bảo vệ (hệ thống, ~/Library/Fonts…) và file
    /// mà dòng khác đang bật. Việc phân biệt scope do `unregister(.., Session)` làm: font không đăng ký session → `NotRegistered`, bỏ qua.
    /// Không ghi DB (dòng đã là Inactive). Lỗi từng file được đếm trong báo cáo, không dừng.
    pub fn unregister_orphans(&self, store: &Mutex<Store>) -> Result<OrphanReport, ActivationError> {
        let mut rep = OrphanReport::default();
        let rows = relock(store).activation_rows()?;
        let wanted: HashSet<&str> = rows.iter().filter(|r| r.state == DesiredState::Active).filter_map(|r| r.active_path.as_deref()).collect();
        let candidates: Vec<&ActivationRow> = rows
            .iter()
            .filter(|r| r.state == DesiredState::Inactive)
            .filter(|r| r.active_path.as_deref().is_some_and(|p| !wanted.contains(p)))
            .collect();
        if candidates.is_empty() {
            return Ok(rep);
        }
        let Ok(available) = self.engine().available_font_paths() else {
            rep.listing_failed = true;
            return Ok(rep);
        };
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
        for row in candidates {
            let Some(path) = row.active_path.as_deref().map(PathBuf::from) else { continue };
            if !available.contains(&path) || crate::paths::in_protected_font_dir(&path, &home) {
                continue;
            }
            // Giữ guard để không đè lên một thao tác bật/tắt đang chạy; đọc lại dòng SAU khi giữ guard.
            let Some(_guard) = self.take(row.hash) else { continue };
            // Tính lại SAU khi giữ guard: trong lúc quét, một hash KHÁC có thể vừa được bật trên cùng đường file (file thay tại chỗ).
            let fresh = relock(store).activation_rows()?;
            let still_off = fresh.iter().any(|r| r.hash == row.hash && r.state == DesiredState::Inactive);
            let wanted_now = fresh.iter().any(|r| r.state == DesiredState::Active && r.active_path.as_deref() == row.active_path.as_deref());
            if !still_off || wanted_now {
                continue;
            }
            match self.engine().unregister(&path, Scope::Session) {
                Ok(()) => rep.unregistered += 1,
                Err(EngineError::NotRegistered) => {}
                Err(_) => rep.failed += 1,
            }
        }
        Ok(rep)
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
