//! B5 App Font Sets: bật font của một Case khi app gắn với Case chạy, gỡ sau thời gian ân hạn khi app cuối cùng thoát.
//!
//! `sync_app_sets` là hàm ĐỒNG BỘ THEO TRẠNG THÁI, gọi lặp lại được (mỗi khi tập app đang chạy đổi, và theo nhịp để hết ân hạn):
//! nó so tập bundle id đang chạy với bảng `case_apps` rồi bật / gỡ phần chênh. Không có sự kiện nào bị mất vì không có sự kiện
//! (app đã chạy sẵn lúc Font Atlas mở, app chết khi Font Atlas tắt) — mọi thứ suy ra từ trạng thái hiện tại.
//!
//! Quy tắc:
//! - Một Case bật khi BẤT KỲ app gắn với nó đang chạy; gỡ khi KHÔNG còn app nào, sau `grace` (mở lại trong ân hạn thì giữ nguyên).
//! - Lý do `caseapp:<id>` cộng dồn với bật tay / Case khác / app khác: gỡ lý do này không tắt font còn lý do khác.
//! - Người dùng tắt tay một font thì thắng lý do `caseapp:*` (xem `deactivate.rs`); lần mở app sau bật lại.
//! - Xung đột (font cùng tên khác nội dung) → không bật, đếm vào `conflicts`, không hỏi giữa lúc làm việc.
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use fontatlas_store::Store;
use serde::Serialize;

use crate::engine::ActivationEngine;
use crate::service::{ActivationService, relock};
use crate::types::{ActivationError, FileOutcome};

/// Ân hạn mặc định giữa lúc app cuối của Case thoát và lúc gỡ font (mở lại nhanh không giật).
pub const DEFAULT_GRACE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy)]
enum CaseState {
    On,
    Leaving(Instant),
}

/// Trạng thái ân hạn trong bộ nhớ (không lưu đĩa: mở lại Font Atlas thì đối soát từ đầu).
#[derive(Debug, Default)]
pub struct AppSetTracker {
    cases: HashMap<i64, CaseState>,
}

impl AppSetTracker {
    pub fn new() -> Self {
        Self::default()
    }
    /// Số Case đang giữ font (bật hoặc đang trong ân hạn).
    pub fn held(&self) -> usize {
        self.cases.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSetEventKind {
    /// Một app của Case vừa chạy và font đã bật.
    On,
    /// App cuối đã thoát và ân hạn đã hết: font đã gỡ.
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSetEvent {
    pub collection_id: i64,
    pub kind: AppSetEventKind,
    /// Số file đổi trạng thái (bật mới / gỡ).
    pub changed: usize,
    /// Số file không bật được vì xung đột với font khác.
    pub conflicts: usize,
    /// Số file lỗi (thiếu file, engine từ chối, chưa tải về).
    pub failed: usize,
}

impl<E: ActivationEngine> ActivationService<E> {
    /// Đồng bộ font theo tập app đang chạy. `running` = bundle id; `now` do người gọi đưa (test dùng giờ giả).
    pub fn sync_app_sets(
        &self,
        store: &Mutex<Store>,
        running: &HashSet<String>,
        now: Instant,
        grace: Duration,
        tracker: &mut AppSetTracker,
    ) -> Result<Vec<AppSetEvent>, ActivationError> {
        let (bindings, with_reasons) = {
            let s = relock(store);
            (s.case_bindings()?, s.case_app_reason_collections()?)
        };
        let mut events = vec![];
        let bound: HashSet<i64> = bindings.iter().map(|b| b.collection_id).collect();

        for b in &bindings {
            let any = b.bundles.iter().any(|x| running.contains(x));
            let st = tracker.cases.get(&b.collection_id).copied();
            match (any, st) {
                (true, Some(CaseState::On)) => {}
                (true, Some(CaseState::Leaving(_))) => {
                    // Mở lại trong ân hạn: font vẫn đang bật, chỉ huỷ việc gỡ.
                    tracker.cases.insert(b.collection_id, CaseState::On);
                }
                (true, None) => {
                    let rep = self.activate_case_app(store, b.collection_id)?;
                    let (changed, conflicts, failed) = tally_on(&rep.results);
                    tracker.cases.insert(b.collection_id, CaseState::On);
                    events.push(AppSetEvent { collection_id: b.collection_id, kind: AppSetEventKind::On, changed, conflicts, failed });
                }
                (false, Some(CaseState::On)) => {
                    tracker.cases.insert(b.collection_id, CaseState::Leaving(now));
                }
                (false, Some(CaseState::Leaving(t))) => {
                    if now.saturating_duration_since(t) >= grace {
                        let rep = self.release_case_app(store, b.collection_id, &mut |_| {})?;
                        tracker.cases.remove(&b.collection_id);
                        let changed = rep.results.iter().filter(|r| r.outcome == FileOutcome::Deactivated).count();
                        let failed = rep.results.iter().filter(|r| matches!(r.outcome, FileOutcome::Failed { .. })).count();
                        events.push(AppSetEvent { collection_id: b.collection_id, kind: AppSetEventKind::Off, changed, conflicts: 0, failed });
                    }
                }
                (false, None) => {}
            }
        }

        // Dọn lý do còn sót mà tracker không biết: gỡ gắn app / xoá Case, hoặc lần chạy trước bị ngắt (crash) và app đã không còn chạy.
        for id in with_reasons {
            let stale = !bound.contains(&id) || (!tracker.cases.contains_key(&id));
            if stale {
                let rep = self.release_case_app(store, id, &mut |_| {})?;
                tracker.cases.remove(&id);
                let changed = rep.results.iter().filter(|r| r.outcome == FileOutcome::Deactivated).count();
                if changed > 0 {
                    events.push(AppSetEvent { collection_id: id, kind: AppSetEventKind::Off, changed, conflicts: 0, failed: 0 });
                }
            }
        }
        // Case bị bỏ gắn hoặc xoá: quên trạng thái.
        tracker.cases.retain(|id, _| bound.contains(id));
        Ok(events)
    }

    /// Lúc thoát Font Atlas: gỡ MỌI lý do `caseapp:*` (font session sắp bị gỡ; lần mở sau đồng bộ lại từ app đang chạy).
    pub fn release_all_case_apps(&self, store: &Mutex<Store>) -> Result<usize, ActivationError> {
        let ids = relock(store).case_app_reason_collections()?;
        let mut n = 0;
        for id in ids {
            n += self.release_case_app(store, id, &mut |_| {})?.results.len();
        }
        Ok(n)
    }
}

fn tally_on(results: &[crate::types::FileResult]) -> (usize, usize, usize) {
    let mut changed = 0;
    let mut conflicts = 0;
    let mut failed = 0;
    for r in results {
        match &r.outcome {
            FileOutcome::Activated => changed += 1,
            FileOutcome::NeedsConfirmation { .. } => conflicts += 1,
            FileOutcome::Failed { .. } | FileOutcome::MissingFile | FileOutcome::NotDownloaded => failed += 1,
            _ => {}
        }
    }
    (changed, conflicts, failed)
}
