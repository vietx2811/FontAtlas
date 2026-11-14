//! B7: chuyển đăng ký của font đang bật sang đường dẫn mới (bản chép trong Organized Library, hoặc file đã dời).
//!
//! Thứ tự an toàn, KHÔNG có khoảng trống: **đăng ký đường mới trước**, ghi `active_path`, rồi mới gỡ đường cũ.
//! (Đã đo trên macOS: hai file cùng PostScript name đăng ký được cùng lúc; gỡ file trước thì file sau thay ngay.)
//! Lỗi ở bước đăng ký mới → giữ nguyên đường cũ, không đổi gì. Sổ "chờ gỡ" (`activation_relocate_pending` trong settings)
//! ghi đường cũ TRƯỚC khi gỡ; nếu app chết giữa hai bước, `reconcile` lần sau gỡ nốt (persistent không để đăng ký mồ côi).
//! Không bao giờ tự quyết: người gọi (người dùng bấm "Switch active fonts to the organized copies") chọn hash và gốc ưu tiên.
use std::path::Path;
use std::sync::Mutex;

use fontatlas_store::{ActivationScope, ActivationStatus, DesiredState, Store};
use serde::{Deserialize, Serialize};

use crate::engine::{ActivationEngine, Scope};
use crate::service::{ActivationService, Located, engine_failure, hex, locate, locate_where, now_ms, path_str, relock};
use crate::paths::canonical_path;
use crate::types::{ActivationError, ActivationReport, EngineError, FileOutcome, FileResult, Progress};

/// Khoá settings giữ các đường cũ đang chờ gỡ.
pub const PENDING_KEY: &str = "activation_relocate_pending";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Pending {
    path: String,
    persistent: bool,
}

fn read_pending(store: &Mutex<Store>) -> Vec<Pending> {
    relock(store).setting(PENDING_KEY).ok().flatten().and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default()
}

fn write_pending(store: &Mutex<Store>, list: &[Pending]) -> Result<(), ActivationError> {
    let v = if list.is_empty() { String::new() } else { serde_json::to_string(list).unwrap_or_default() };
    relock(store).set_setting(PENDING_KEY, &v)?;
    Ok(())
}

fn under(p: &Path, root: &Path) -> bool {
    p.starts_with(root) || canonical_path(p).is_ok_and(|c| canonical_path(root).is_ok_and(|r| c.starts_with(r)))
}

impl<E: ActivationEngine> ActivationService<E> {
    /// Chuyển đăng ký các file đang bật (`hashes`) sang đường dẫn mới.
    /// `prefer_under`: chỉ chấp nhận vị trí nằm dưới thư mục này (vd gốc Organized Library); `None` = vị trí dùng được đầu tiên.
    /// Kết quả mỗi file: `Activated` = đã chuyển; `AlreadyActive` = đã đúng đường mới; `AlreadyInactive` = không đang bật (bỏ qua);
    /// `MissingFile` / `NotDownloaded` / `Failed` = KHÔNG đổi gì, đường cũ vẫn đăng ký.
    pub fn relocate(
        &self,
        store: &Mutex<Store>,
        hashes: &[[u8; 32]],
        prefer_under: Option<&Path>,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let mut rep = ActivationReport::default();
        let total = hashes.len();
        for (i, h) in hashes.iter().enumerate() {
            rep.push(self.relocate_one(store, *h, prefer_under)?);
            progress(Progress { done: i + 1, total, hash_hex: hex(h) });
        }
        Ok(rep)
    }

    fn relocate_one(&self, store: &Mutex<Store>, hash: [u8; 32], prefer: Option<&Path>) -> Result<FileResult, ActivationError> {
        let mut res = FileResult { hash_hex: hex(&hash), path: None, requested_faces: vec![], extra_faces: vec![], outcome: FileOutcome::Busy, replaced: vec![], warning: None };
        let Some(_guard) = self.take(hash) else { return Ok(res) };
        let (row, target) = {
            let s = relock(store);
            (s.activation_row(&hash)?, s.file_target(&hash)?)
        };
        let Some(mut row) = row.filter(|r| r.state == DesiredState::Active && r.status == ActivationStatus::Registered) else {
            res.outcome = FileOutcome::AlreadyInactive;
            return Ok(res);
        };
        res.path = row.active_path.clone();
        let Some(target) = target else {
            res.outcome = FileOutcome::Failed { code: "not_found".into(), message: "không có file trong thư viện".into() };
            return Ok(res);
        };
        let located = match prefer {
            Some(root) => locate_where(&target, |l| under(&l.path, root)),
            None => locate(&target),
        };
        let new = match located {
            Located::Ready(p) => p,
            Located::NotDownloaded(_) => {
                res.outcome = FileOutcome::NotDownloaded;
                return Ok(res);
            }
            Located::Missing(_) => {
                res.outcome = FileOutcome::MissingFile;
                return Ok(res);
            }
            Located::Changed(_) => {
                res.outcome = FileOutcome::Failed { code: "content_changed".into(), message: "nội dung file mới không khớp hash".into() };
                return Ok(res);
            }
            Located::BadPath(_) => {
                res.outcome = FileOutcome::Failed { code: "bad_path".into(), message: "đường dẫn không biểu diễn được bằng UTF-8".into() };
                return Ok(res);
            }
        };
        let new_s = path_str(&new);
        if row.active_path.as_deref() == Some(new_s.as_str()) {
            res.outcome = FileOutcome::AlreadyActive;
            return Ok(res);
        }
        // 1) đăng ký đường mới TRƯỚC. Lỗi → không đổi gì.
        match self.engine().register(&new, row.scope) {
            Ok(()) | Err(EngineError::AlreadyRegistered) => {}
            Err(e) => {
                res.outcome = engine_failure(&e);
                return Ok(res);
            }
        }
        // 2) ghi sổ chờ gỡ (đường cũ) rồi đổi active_path: nếu chết ở đây, reconcile gỡ nốt đường cũ.
        let old = row.active_path.clone();
        let persistent = row.scope == ActivationScope::Persistent;
        if let Some(o) = &old {
            let mut list = read_pending(store);
            list.push(Pending { path: o.clone(), persistent });
            write_pending(store, &list)?;
        }
        row.active_path = Some(new_s.clone());
        row.status = ActivationStatus::Registered;
        row.last_error = None;
        row.verified_at = Some(now_ms());
        relock(store).put_activation(&row)?;
        res.path = Some(new_s);
        res.outcome = FileOutcome::Activated;
        // 3) gỡ đường cũ (lỗi → để reconcile thử lại, báo warning).
        if let Some(o) = old {
            let sc = if persistent { Scope::Persistent } else { Scope::Session };
            match self.engine().unregister(Path::new(&o), sc) {
                Ok(()) | Err(EngineError::NotRegistered) => {
                    let mut list = read_pending(store);
                    list.retain(|p| p.path != o);
                    write_pending(store, &list)?;
                }
                Err(e) => res.warning = Some(format!("chưa gỡ được đường cũ ({}); lần mở sau sẽ thử lại", e.code())),
            }
        }
        Ok(res)
    }

    /// Gỡ nốt các đường cũ còn trong sổ chờ gỡ (app chết giữa `relocate`), trừ đường đang là `active_path` của dòng nào đó.
    /// Gọi từ `reconcile`. Trả số đường đã gỡ.
    pub(crate) fn finish_pending_relocations(&self, store: &Mutex<Store>) -> usize {
        let list = read_pending(store);
        if list.is_empty() {
            return 0;
        }
        let in_use: Vec<String> = relock(store).activation_rows().map(|rs| rs.into_iter().filter_map(|r| r.active_path).collect()).unwrap_or_default();
        let (mut done, mut keep, mut n) = (vec![], vec![], 0);
        for p in list {
            if in_use.contains(&p.path) {
                done.push(p); // đã được dùng lại làm đường chính: không gỡ, bỏ khỏi sổ
                continue;
            }
            let sc = if p.persistent { Scope::Persistent } else { Scope::Session };
            match self.engine().unregister(Path::new(&p.path), sc) {
                Ok(()) | Err(EngineError::NotRegistered) => {
                    n += 1;
                    done.push(p);
                }
                Err(_) => keep.push(p),
            }
        }
        let _ = write_pending(store, &keep);
        n
    }
}
