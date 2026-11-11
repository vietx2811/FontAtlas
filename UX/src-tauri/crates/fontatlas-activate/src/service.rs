//! `ActivationService`: điều phối Store + engine.
//!
//! - Khoá `Store` theo từng file và nhả trước khi gọi engine / `progress`; không giữ suốt lô.
//! - Khoá thao tác theo hash (`in_flight`): hash đang bận → `Busy`.
//! - Ghi dòng "ý định" (`status=unknown`) trước khi gọi engine, để crash giữa chừng không để lại
//!   đăng ký mồ côi (`reconcile` xử lý dòng `unknown`).
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use fontatlas_store::{
    ActivationReason, ActivationRow, ActivationStatus, DesiredState, FaceKey, FileTarget, Store,
    add_activation_reason_tx, put_activation_tx, remove_activation_reason_tx,
};

use serde::Serialize;

use crate::engine::{ActivationEngine, Scope};
use crate::paths::{
    CLOUD_DOWNLOAD_TIMEOUT, EnsureLocalError, FileProbe, canonical_path, classify_owner, ensure_local, probe_file, replace_tier, same_content,
};
use crate::types::{ConfirmedConflict, 
    ActivationError, ActivationReport, ActivationStateDto, Conflict, ConflictPolicy, ENGINE_VERSION, EngineError, FileOutcome,
    FileResult, Owner, Progress, ReplaceTier, ReplacedFont, RestoreOutcome, SETTING_ACTIVATION_KEEP,
};

/// Đưa một file cloud về máy trước khi bật (mặc định `ensure_local` với `CLOUD_DOWNLOAD_TIMEOUT`).
type EnsureLocalFn = Box<dyn Fn(&Path) -> Result<(), EnsureLocalError> + Send + Sync>;

pub struct ActivationService<E: ActivationEngine> {
    engine: E,
    in_flight: Mutex<HashSet<[u8; 32]>>,
    ensure_local: EnsureLocalFn,
    /// Như `ensure_local` nhưng cho app đang đợi trong callback (lý do `auto:*`): hạn chờ ngắn hơn nhiều.
    ensure_local_auto: EnsureLocalFn,
}

/// Một app thiết kế (Photoshop…) đang đợi trong callback: không chờ tải cloud quá lâu. Con số này là ước đoán
/// (chưa đo Photoshop chịu được bao lâu, đo ở L2).
pub const AUTO_DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// Giữ một hash trong `in_flight`; nhả khi drop (kể cả khi panic).
pub(crate) struct InFlight<'a> {
    set: &'a Mutex<HashSet<[u8; 32]>>,
    hash: [u8; 32],
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        relock(self.set).remove(&self.hash);
    }
}

pub(crate) fn relock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Vị trí dùng được của một file.
pub(crate) enum Located {
    /// Canonical, UTF-8, và nội dung khớp `hash` của file trong thư viện.
    Ready(PathBuf),
    Missing(Option<PathBuf>),
    NotDownloaded(PathBuf),
    /// Có file nhưng nội dung không còn khớp hash (bị thay / ghi đè): không đăng ký dưới hash cũ.
    Changed(PathBuf),
    /// Đường dẫn canonical không biểu diễn được bằng UTF-8 (cột `active_path` là TEXT): từ chối rõ ràng.
    BadPath(PathBuf),
}

/// Vị trí đầu tiên dùng được (đã canonical, UTF-8, đúng hash); không có thì phân biệt
/// nội dung đã đổi / đường dẫn lạ / iCloud chưa tải / thiếu hẳn.
pub(crate) fn locate(t: &FileTarget) -> Located {
    let (mut changed, mut bad, mut not_downloaded) = (None, None, None);
    for loc in &t.locations {
        match probe_file(&loc.path) {
            FileProbe::Ready => {
                let Ok(c) = canonical_path(&loc.path) else { continue };
                if c.to_str().is_none() {
                    bad.get_or_insert(c);
                    continue;
                }
                match std::fs::read(&c) {
                    Ok(bytes) if blake3::hash(&bytes).as_bytes() == &t.hash => return Located::Ready(c),
                    Ok(_) => {
                        changed.get_or_insert(c);
                    }
                    Err(_) => {}
                }
            }
            FileProbe::NotDownloaded => {
                not_downloaded.get_or_insert_with(|| loc.path.clone());
            }
            FileProbe::Missing => {}
        }
    }
    if let Some(p) = changed {
        Located::Changed(p)
    } else if let Some(p) = bad {
        Located::BadPath(p)
    } else if let Some(p) = not_downloaded {
        Located::NotDownloaded(p)
    } else {
        Located::Missing(t.locations.first().map(|l| l.path.clone()))
    }
}

pub(crate) fn engine_failure(e: &EngineError) -> FileOutcome {
    FileOutcome::Failed { code: format!("engine.{}", e.code()), message: e.to_string() }
}

pub(crate) fn engine_err(e: EngineError) -> ActivationError {
    match e {
        EngineError::Unsupported => ActivationError::Unsupported,
        e => ActivationError::Engine(e),
    }
}

/// Trạng thái dùng chung trong một lô.
struct Batch {
    avail: HashSet<String>,
    ours: HashSet<PathBuf>,
    home: PathBuf,
    /// PS name → file đã kích hoạt trước trong cùng lô (phát hiện trùng trong lô).
    seen: HashMap<String, PathBuf>,
    /// (file của ta, file của nhà cung cấp) → cùng nội dung? TTC nhiều PS name trỏ cùng một provider: so byte một lần.
    same: RefCell<HashMap<(PathBuf, PathBuf), bool>>,
}

/// Font đã dùng được trên máy nhờ một file GIỐNG HỆT do bên khác cung cấp (hệ thống, FontBase, RightFont…).
/// Font Atlas không đăng ký gì cho nó; trạng thái này tính lại, không lưu thành dòng kích hoạt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    pub hash_hex: String,
    pub owner: Owner,
    pub path: String,
}

/// Các hash đã từng được thấy là "có sẵn" — tính lại khi mở app / đối soát để font quay về "chưa bật" khi bên kia gỡ.
const SETTING_AVAILABLE_HASHES: &str = "activation_available_hashes";

/// Mọi conflict đều là cùng file → font đã dùng được; trả owner + đường dẫn của bên cung cấp đầu tiên.
fn available_from(conflicts: &[Conflict]) -> Option<(Owner, String)> {
    let first = conflicts.first()?;
    conflicts.iter().all(|c| c.same_file).then(|| (first.owner, first.existing_path.clone()))
}

/// Gom face theo hash, giữ thứ tự xuất hiện đầu tiên.
fn group(keys: &[FaceKey]) -> Vec<([u8; 32], Vec<u32>)> {
    let mut out: Vec<([u8; 32], Vec<u32>)> = vec![];
    for k in keys {
        match out.iter_mut().find(|(h, _)| *h == k.hash) {
            Some((_, v)) => {
                if !v.contains(&k.face_index) {
                    v.push(k.face_index);
                }
            }
            None => out.push((k.hash, vec![k.face_index])),
        }
    }
    for (_, v) in &mut out {
        v.sort_unstable();
    }
    out
}

impl<E: ActivationEngine> ActivationService<E> {
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            in_flight: Mutex::new(HashSet::new()),
            ensure_local: Box::new(|p| ensure_local(p, CLOUD_DOWNLOAD_TIMEOUT)),
            ensure_local_auto: Box::new(|p| ensure_local(p, AUTO_DOWNLOAD_TIMEOUT)),
        }
    }

    /// Thay cách tải cho yêu cầu từ app (`auto:*`) — test hoặc đổi hạn chờ.
    pub fn with_auto_ensure_local(mut self, f: impl Fn(&Path) -> Result<(), EnsureLocalError> + Send + Sync + 'static) -> Self {
        self.ensure_local_auto = Box::new(f);
        self
    }

    /// Thay cách tải file cloud (test dataless không cần ổ cloud thật).
    pub fn with_ensure_local(mut self, f: impl Fn(&Path) -> Result<(), EnsureLocalError> + Send + Sync + 'static) -> Self {
        self.ensure_local = Box::new(f);
        self
    }

    pub fn engine(&self) -> &E {
        &self.engine
    }

    /// Mọi dòng kích hoạt kèm lý do, cho UI.
    pub fn state(&self, store: &Mutex<Store>) -> Result<Vec<ActivationStateDto>, ActivationError> {
        Ok(relock(store)
            .activation_rows()?
            .into_iter()
            .map(|r| ActivationStateDto {
                hash_hex: hex(&r.hash),
                state: r.state,
                scope: r.scope,
                status: r.status,
                reasons: r.reasons,
                active_path: r.active_path,
                conflict_ps: r.conflict_ps,
                last_error: r.last_error,
            })
            .collect())
    }

    /// Scope mặc định theo `SETTING_ACTIVATION_KEEP`.
    pub fn default_scope(store: &Mutex<Store>) -> Result<Scope, ActivationError> {
        let v = relock(store).setting(SETTING_ACTIVATION_KEEP)?;
        Ok(if v.as_deref() == Some("1") { Scope::Persistent } else { Scope::Session })
    }

    pub(crate) fn take(&self, hash: [u8; 32]) -> Option<InFlight<'_>> {
        // Không dùng `then_some(InFlight{..})`: đối số được tạo ngay cả khi hash đang bận, và drop của nó
        // vừa xoá nhầm hash của thao tác khác vừa khoá lại `in_flight` khi guard tạm còn giữ (deadlock).
        let inserted = relock(&self.in_flight).insert(hash);
        if inserted { Some(InFlight { set: &self.in_flight, hash }) } else { None }
    }

    /// Kiểm tra xung đột, không đăng ký, không ghi DB. Không xung đột → `Ready`.
    pub fn preflight(&self, store: &Mutex<Store>, keys: &[FaceKey]) -> Result<ActivationReport, ActivationError> {
        let scope = Self::default_scope(store)?;
        self.run(store, keys, ActivationReason::Manual, scope, false, ConflictPolicy::Ask, true, None, &mut |_| {})
    }

    pub fn activate_faces(
        &self,
        store: &Mutex<Store>,
        keys: &[FaceKey],
        scope: Option<Scope>,
        policy: ConflictPolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        self.activate_files(store, keys, ActivationReason::Manual, scope, policy, progress)
    }

    /// Bật font vì một app thiết kế vừa hỏi: lý do `auto:<app>`, luôn scope session, xung đột → không bật
    /// (`NeedsConfirmation`, không có ai để hỏi lúc này). Font đã bật ở scope persistent thì chỉ thêm lý do.
    pub fn activate_auto(
        &self,
        store: &Mutex<Store>,
        keys: &[FaceKey],
        app: fontatlas_store::AutoApp,
    ) -> Result<ActivationReport, ActivationError> {
        self.run(store, keys, ActivationReason::Auto(app), Scope::Session, true, ConflictPolicy::Ask, false, None, &mut |_| {})
    }

    pub fn activate_collection(
        &self,
        store: &Mutex<Store>,
        collection_id: i64,
        scope: Option<Scope>,
        policy: ConflictPolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let keys = relock(store).collection_faces(collection_id)?;
        self.activate_files(store, &keys, ActivationReason::Collection(collection_id), scope, policy, progress)
    }

    fn activate_files(
        &self,
        store: &Mutex<Store>,
        keys: &[FaceKey],
        reason: ActivationReason,
        scope: Option<Scope>,
        policy: ConflictPolicy,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let explicit = scope.is_some();
        let scope = match scope {
            Some(s) => s,
            None => Self::default_scope(store)?,
        };
        self.run(store, keys, reason, scope, explicit, policy, false, None, progress)
    }

    /// "Thay bằng bản này": chỉ thay đúng những xung đột người dùng đã thấy và xác nhận. Tập xung đột của một file đổi
    /// so với `confirmed` (bản cũ mới xuất hiện / biến mất giữa lúc hỏi và lúc bấm) → `replace_conflicts_changed`.
    pub fn replace_faces(
        &self,
        store: &Mutex<Store>,
        keys: &[FaceKey],
        scope: Option<Scope>,
        confirmed: &[ConfirmedConflict],
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let explicit = scope.is_some();
        let scope = match scope {
            Some(s) => s,
            None => Self::default_scope(store)?,
        };
        self.run(store, keys, ActivationReason::Manual, scope, explicit, ConflictPolicy::Replace, false, Some(confirmed), progress)
    }

    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        store: &Mutex<Store>,
        keys: &[FaceKey],
        reason: ActivationReason,
        scope: Scope,
        explicit: bool,
        policy: ConflictPolicy,
        dry_run: bool,
        confirmed: Option<&[ConfirmedConflict]>,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let groups = group(keys);
        let mut batch = self.new_batch(store)?;
        let mut rep = ActivationReport::default();
        let total = groups.len();
        for (i, (hash, req)) in groups.into_iter().enumerate() {
            let r = self.one_file(store, &mut batch, hash, req, reason, scope, explicit, policy, dry_run, confirmed)?;
            rep.push(r);
            progress(Progress { done: i + 1, total, hash_hex: hex(&hash) });
        }
        Ok(rep)
    }

    fn new_batch(&self, store: &Mutex<Store>) -> Result<Batch, ActivationError> {
        let ours: HashSet<PathBuf> = relock(store)
            .activation_rows()?
            .into_iter()
            .filter(|r| r.status == ActivationStatus::Registered)
            .filter_map(|r| r.active_path.map(PathBuf::from))
            .collect();
        Ok(Batch {
            avail: self.engine.available_postscript_names().map_err(engine_err)?,
            ours,
            home: std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default(),
            seen: HashMap::new(),
            same: RefCell::new(HashMap::new()),
        })
    }

    /// PS name của file: từ library.db; face thiếu tên thì đọc thẳng từ file.
    fn names_of(&self, t: &FileTarget, path: &Path) -> Vec<String> {
        let mut names: Vec<String> = t.faces.iter().filter_map(|f| f.postscript_name.clone()).collect();
        if t.faces.iter().any(|f| f.postscript_name.is_none()) {
            for n in self.engine.postscript_names_in_file(path).unwrap_or_default() {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        names
    }

    fn conflicts(
        &self,
        batch: &Batch,
        names: &[String],
        path: &Path,
        old_path: Option<&Path>,
    ) -> Result<Vec<Conflict>, ActivationError> {
        let mut out: Vec<Conflict> = vec![];
        let mut push = |ps: &str, p: &Path, owner: Owner| {
            let existing = path_str(p);
            if !out.iter().any(|c| c.postscript_name == ps && c.existing_path == existing) {
                let same_file = *batch
                    .same
                    .borrow_mut()
                    .entry((path.to_path_buf(), p.to_path_buf()))
                    .or_insert_with(|| same_content(path, p));
                out.push(Conflict { postscript_name: ps.to_string(), existing_path: existing, owner, same_file, replace_tier: tier_of(owner, p, &batch.home) });
            }
        };
        for name in names {
            if let Some(p) = batch.seen.get(name).filter(|p| p.as_path() != path) {
                push(name, p, Owner::FontAtlas);
            }
            if !batch.avail.contains(name) {
                continue;
            }
            for p in self.engine.providers(name).map_err(engine_err)? {
                if p == path || Some(p.as_path()) == old_path {
                    continue;
                }
                push(name, &p, classify_owner(&p, &batch.home, &batch.ours));
            }
        }
        Ok(out)
    }

    #[allow(clippy::too_many_arguments)]
    fn one_file(
        &self,
        store: &Mutex<Store>,
        batch: &mut Batch,
        hash: [u8; 32],
        requested_faces: Vec<u32>,
        reason: ActivationReason,
        scope: Scope,
        explicit: bool,
        policy: ConflictPolicy,
        dry_run: bool,
        confirmed: Option<&[ConfirmedConflict]>,
    ) -> Result<FileResult, ActivationError> {
        let mut res = FileResult {
            hash_hex: hex(&hash),
            path: None,
            requested_faces,
            extra_faces: vec![],
            outcome: FileOutcome::Busy,
            replaced: vec![],
            warning: None,
        };
        let Some(_guard) = self.take(hash) else { return Ok(res) };
        let (target, old) = {
            let s = relock(store);
            (s.file_target(&hash)?, s.activation_row(&hash)?)
        };
        let Some(target) = target else {
            res.outcome = FileOutcome::Failed { code: "not_found".into(), message: format!("không có file {}", res.hash_hex) };
            return Ok(res);
        };
        res.extra_faces =
            target.faces.iter().map(|f| f.face_index).filter(|i| !res.requested_faces.contains(i)).collect();
        // font còn là placeholder trên ổ cloud: tải về trước (có hạn chót), không bao giờ treo; lỗi trả về rõ ràng
        let mut located = locate(&target);
        if let Located::NotDownloaded(p) = &located {
            let fetch = if matches!(reason, ActivationReason::Auto(_)) { &self.ensure_local_auto } else { &self.ensure_local };
            match fetch(p) {
                Ok(()) => {
                    // file vừa về máy: ghi cờ cloud_only=0 ngay (giao diện, hàng phân tích lại dựa vào cờ này)
                    let _ = relock(store).refresh_cloud_flag(p);
                    located = locate(&target);
                }
                Err(EnsureLocalError::PlaceholderStub) => {} // chỉ có stub .icloud: giữ outcome NotDownloaded
                Err(EnsureLocalError::Missing) => located = Located::Missing(Some(p.clone())),
                Err(e) => {
                    res.path = Some(path_str(p));
                    res.outcome = FileOutcome::Failed { code: e.code().into(), message: e.message() };
                    return Ok(res);
                }
            }
        }
        let path = match located {
            Located::Ready(p) => p,
            Located::Missing(p) => {
                res.path = p.map(|p| path_str(&p));
                res.outcome = FileOutcome::MissingFile;
                return Ok(res);
            }
            Located::NotDownloaded(p) => {
                res.path = Some(path_str(&p));
                res.outcome = FileOutcome::NotDownloaded;
                return Ok(res);
            }
            Located::Changed(p) => {
                res.path = Some(path_str(&p));
                res.outcome = FileOutcome::Failed {
                    code: "file_changed".into(),
                    message: "nội dung file đã đổi so với thư viện; hãy quét lại nguồn".into(),
                };
                return Ok(res);
            }
            Located::BadPath(p) => {
                res.path = Some(path_str(&p));
                res.outcome = FileOutcome::Failed {
                    code: "path_not_utf8".into(),
                    message: "đường dẫn file không phải UTF-8 nên không thể kích hoạt".into(),
                };
                return Ok(res);
            }
        };
        let pstr = path_str(&path);
        res.path = Some(pstr.clone());
        let names = self.names_of(&target, &path);

        // 3. Đã bật đúng chỗ → chỉ thêm lý do. Scope hiệu lực = cao nhất trong các lý do: một lần bật KHÔNG tường minh
        //    (vd bật collection chứa face này) không hạ "Giữ bật" xuống "phiên này"; một lần tường minh chỉ nâng, không hạ
        //    (hạ là việc của `set_scope`).
        let same = old.as_ref().is_some_and(|o| {
            o.state == DesiredState::Active
                && o.status == ActivationStatus::Registered
                && o.active_path.as_deref() == Some(pstr.as_str())
                && (!explicit || o.scope == scope || o.scope == Scope::Persistent)
        });
        if same {
            if !dry_run {
                relock(store).add_activation_reason(&hash, reason)?;
                for n in &names {
                    batch.seen.entry(n.clone()).or_insert_with(|| path.clone());
                }
            }
            res.outcome = FileOutcome::AlreadyActive;
            return Ok(res);
        }

        // 4–5. Preflight trước, gỡ sau.
        let old_path = old.as_ref().and_then(|o| o.active_path.as_ref()).map(PathBuf::from);
        let conflicts = self.conflicts(batch, &names, &path, old_path.as_deref())?;
        // Mọi bản trùng đều là cùng file (cùng từng byte) → đã dùng được: không đăng ký, không ghi dòng kích hoạt.
        if let Some((owner, existing)) = available_from(&conflicts) {
            if !dry_run {
                self.remember_available(store, &hash, true)?;
            }
            res.outcome = FileOutcome::AlreadyAvailable { owner, path: existing };
            return Ok(res);
        }
        if dry_run {
            res.outcome =
                if conflicts.is_empty() { FileOutcome::Ready } else { FileOutcome::NeedsConfirmation { conflicts } };
            return Ok(res);
        }
        if !conflicts.is_empty() && policy == ConflictPolicy::Ask {
            res.outcome = FileOutcome::NeedsConfirmation { conflicts };
            return Ok(res);
        }
        // "Tắt bản cũ và bật bản này": gỡ các bản đăng ký xung đột (nếu ma trận cho phép) TRƯỚC khi đăng ký bản của ta.
        let mut removed: Vec<Removed> = vec![];
        if let (ConflictPolicy::Replace, Some(ok)) = (policy, confirmed) {
            let mut now: Vec<&str> = conflicts.iter().map(|c| c.existing_path.as_str()).collect();
            let mut seen: Vec<&str> = ok.iter().filter(|c| c.hash == hash).map(|c| c.existing_path.as_str()).collect();
            now.sort_unstable();
            now.dedup();
            seen.sort_unstable();
            seen.dedup();
            if now != seen {
                res.outcome = FileOutcome::Failed {
                    code: "replace_conflicts_changed".into(),
                    message: "các bản đang cài đã khác so với lúc bạn xác nhận; hãy xem lại rồi thử lại".into(),
                };
                return Ok(res);
            }
        }
        if policy == ConflictPolicy::Replace && !conflicts.is_empty() {
            match self.remove_conflicts(store, &hash, &conflicts, &batch.home) {
                Ok(r) => removed = r,
                Err(outcome) => {
                    res.outcome = outcome;
                    return Ok(res);
                }
            }
        }

        // 6–7. Mọi lỗi từ đây (kể cả lỗi store) phải trả lại các bản đã gỡ: `tail` chạy trong closure để gom nhánh lỗi một chỗ.
        let mut tail = || -> Result<(), ActivationError> {
            // 6. Đăng ký cũ ở chỗ khác / scope khác → gỡ.
            if let (Some(o), Some(op)) = (&old, &old_path) {
                // Unknown/Error cũng có thể đang đăng ký (crash / gỡ lỗi): gỡ ở cả hai scope, trừ đúng
                // cặp (path, scope) sắp đăng ký. `NotRegistered` coi như đã gỡ.
                let scopes = match o.status {
                    ActivationStatus::Registered => vec![o.scope],
                    ActivationStatus::Unknown | ActivationStatus::Error => vec![Scope::Session, Scope::Persistent],
                    _ => vec![],
                };
                for s in scopes {
                    if op == &path && s == scope {
                        continue;
                    }
                    match self.engine.unregister(op, s) {
                        Ok(()) | Err(EngineError::NotRegistered) => {}
                        Err(e) => {
                            res.outcome = engine_failure(&e);
                            if !removed.is_empty() && !self.give_back(store, &removed) {
                                res.outcome = rollback_failed(e.code());
                                res.replaced = removed.iter().filter_map(|r| r.journal.clone()).collect();
                            }
                            return Ok(());
                        }
                    }
                }
            }

            // 7. Ý định trước (một transaction), rồi mới gọi engine.
            let mut row = ActivationRow {
                reasons: vec![],
                hash,
                state: DesiredState::Active,
                scope,
                method: self.engine.method().to_string(),
                active_path: Some(pstr.clone()),
                status: ActivationStatus::Unknown,
                conflict_ps: old.as_ref().and_then(|o| o.conflict_ps.clone()),
                last_error: None,
                activated_at: old.as_ref().and_then(|o| o.activated_at),
                verified_at: old.as_ref().and_then(|o| o.verified_at),
                updated_at: 0,
                engine_version: ENGINE_VERSION,
            };
            let added_new = relock(store).with_tx(|tx| {
                put_activation_tx(tx, &row)?;
                add_activation_reason_tx(tx, &hash, reason)
            })?;

            match self.engine.register(&path, scope) {
                Ok(()) | Err(EngineError::AlreadyRegistered) => {
                    let now = now_ms();
                    row.status = ActivationStatus::Registered;
                    row.conflict_ps = conflicts.first().map(|c| c.postscript_name.clone());
                    row.activated_at = Some(now);
                    row.verified_at = Some(now);
                    relock(store).put_activation(&row)?;
                    for n in &names {
                        batch.seen.entry(n.clone()).or_insert_with(|| path.clone());
                    }
                    batch.ours.insert(path.clone());
                    self.remember_available(store, &hash, false)?;
                    res.replaced = removed.iter().filter_map(|r| r.journal.clone()).collect();
                    res.outcome = FileOutcome::Activated;
                    // bật xong mà bản khác vẫn cung cấp font: vẫn là Activated, kèm cảnh báo (font của ta đang bật thật)
                    if !removed.is_empty() && !self.replace_took_effect(&conflicts, &path) {
                        res.warning = Some("replace_not_effective".into());
                    }
                }
                Err(e) => {
                    let mut failure = engine_failure(&e);
                    // đã gỡ bản cũ mà bật bản mới hỏng → trả bản cũ lại đúng URL + scope, bỏ dòng sổ. Trả lại hỏng thì GIỮ sổ.
                    if !self.give_back(store, &removed) {
                        failure = rollback_failed(e.code());
                        res.replaced = removed.iter().filter_map(|r| r.journal.clone()).collect();
                    }
                    relock(store).with_tx(|tx| {
                        // Lý do đã có từ trước thì vẫn còn: giữ state=Active.
                        let none_left = added_new && remove_activation_reason_tx(tx, &hash, reason)?.is_empty();
                        if none_left {
                            row.state = DesiredState::Inactive;
                        }
                        row.status = ActivationStatus::Error;
                        row.last_error = Some(format!("engine.{}", e.code()));
                        put_activation_tx(tx, &row)
                    })?;
                    res.outcome = failure;
                }
            }
            Ok(())
        };
        match tail() {
            Ok(()) => Ok(res),
            Err(e) if !removed.is_empty() => {
                // lỗi store giữa chừng: đưa hệ thống về như cũ (gỡ bản của ta nếu đã đăng ký, trả bản cũ), báo qua kết quả
                let _ = self.engine.unregister(&path, scope);
                if self.give_back(store, &removed) {
                    res.outcome = FileOutcome::Failed { code: e.code().into(), message: e.to_string() };
                } else {
                    res.outcome = rollback_failed(e.code());
                    res.replaced = removed.iter().filter_map(|r| r.journal.clone()).collect();
                }
                Ok(res)
            }
            Err(e) => Err(e),
        }
    }

    fn remember_available(&self, store: &Mutex<Store>, hash: &[u8; 32], on: bool) -> Result<(), ActivationError> {
        let mut st = relock(store);
        let mut set = read_remembered(&st)?;
        let h = hex(hash);
        let changed = if on { set.insert(h) } else { set.remove(&h) };
        if changed {
            write_remembered(&mut st, &set)?;
        }
        Ok(())
    }

    /// Font nào trong `hashes` (cộng các hash đã nhớ từ trước) đang dùng được nhờ file giống hệt của bên khác?
    /// Tính lại từ đầu từ hệ điều hành mỗi lần gọi, rồi cập nhật tập "đã nhớ" (bên kia gỡ font → bỏ khỏi tập).
    /// Font Atlas đang tự bật thì không tính (đã có dòng kích hoạt).
    pub fn availability(&self, store: &Mutex<Store>, hashes: &[[u8; 32]]) -> Result<Vec<Availability>, ActivationError> {
        let mut want: Vec<[u8; 32]> = hashes.to_vec();
        for h in read_remembered(&relock(store))? {
            if let Some(b) = unhex(&h) {
                want.push(b);
            }
        }
        want.sort_unstable();
        want.dedup();
        let batch = self.new_batch(store)?;
        let mut out = vec![];
        for hash in want {
            let (target, old) = {
                let s = relock(store);
                (s.file_target(&hash)?, s.activation_row(&hash)?)
            };
            if old.as_ref().is_some_and(|o| o.state == DesiredState::Active && o.status == ActivationStatus::Registered) {
                continue;
            }
            let Some(target) = target else { continue };
            let Located::Ready(path) = locate(&target) else { continue };
            let names = self.names_of(&target, &path);
            let conflicts = self.conflicts(&batch, &names, &path, None)?;
            if let Some((owner, existing)) = available_from(&conflicts) {
                out.push(Availability { hash_hex: hex(&hash), owner, path: existing });
            }
        }
        let mut st = relock(store);
        let now: std::collections::BTreeSet<String> = out.iter().map(|a| a.hash_hex.clone()).collect();
        if now != read_remembered(&st)? {
            write_remembered(&mut st, &now)?;
        }
        Ok(out)
    }
}

fn read_remembered(st: &Store) -> Result<std::collections::BTreeSet<String>, ActivationError> {
    Ok(st
        .setting(SETTING_AVAILABLE_HASHES)?
        .map(|v| v.split(',').filter(|x| !x.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default())
}

fn write_remembered(st: &mut Store, set: &std::collections::BTreeSet<String>) -> Result<(), ActivationError> {
    let v: Vec<&str> = set.iter().map(String::as_str).collect();
    st.set_setting(SETTING_AVAILABLE_HASHES, &v.join(","))?;
    Ok(())
}

fn unhex(h: &str) -> Option<[u8; 32]> {
    if h.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(h.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}


/// Một bản đăng ký xung đột đã gỡ trong lúc "Thay thế" (để trả lại nếu bước sau hỏng).
struct Removed {
    path: PathBuf,
    scopes: Vec<Scope>,
    /// Bản ghi sổ khôi phục (None với bản do chính Font Atlas đăng ký: người dùng bật lại từ thư viện).
    journal: Option<ReplacedFont>,
    /// Với bản do chính Font Atlas đăng ký: các dòng kích hoạt trước khi tắt (để bật lại đúng như cũ nếu bước sau hỏng).
    own: Vec<ActivationRow>,
}

/// Tier chặt nhất giữa chủ sở hữu đã phân loại và tier tính lại trên đường dẫn (canonical hoá).
fn tier_of(owner: Owner, p: &Path, home: &Path) -> ReplaceTier {
    use ReplaceTier::*;
    let canon = canonical_path(p).unwrap_or_else(|_| p.to_path_buf());
    let again = classify_owner(&canon, home, &HashSet::new());
    let a = replace_tier(owner, p, home);
    let b = replace_tier(if owner == Owner::FontAtlas { owner } else { again }, &canon, home);
    match (a, b) {
        (Protected, _) | (_, Protected) => Protected,
        (Guide, _) | (_, Guide) => Guide,
        _ => Replace,
    }
}

fn rollback_failed(cause: &str) -> FileOutcome {
    FileOutcome::Failed {
        code: "replace_rollback_failed".into(),
        message: format!("không bật được bản mới ({cause}) và cũng không trả lại được bản cũ; bản cũ nằm trong mục \"Đã thay thế\" để khôi phục"),
    }
}

impl<E: ActivationEngine> ActivationService<E> {
    /// Sau khi gỡ + đăng ký: không còn file nào khác (ngoài bản của ta) cung cấp các PS name đã xung đột?
    /// Không hỏi được hệ điều hành → coi như hiệu lực (không có bằng chứng ngược lại).
    fn replace_took_effect(&self, conflicts: &[Conflict], ours: &Path) -> bool {
        let ours = canonical_path(ours).unwrap_or_else(|_| ours.to_path_buf());
        let mut names: Vec<&str> = conflicts.iter().map(|c| c.postscript_name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        names.into_iter().all(|n| match self.engine.providers(n) {
            Ok(ps) => ps.iter().all(|p| canonical_path(p).unwrap_or_else(|_| p.clone()) == ours),
            Err(_) => true,
        })
    }

    /// Gỡ mọi bản xung đột, theo ma trận. Lỗi → `Err(outcome)` và mọi thứ đã gỡ được trả lại (không trả được → giữ sổ).
    /// Gỡ/trả/khôi phục luôn dùng URL ĐÃ ĐĂNG KÝ gốc (từ `providers`); canonical chỉ dùng để gom nhóm và kiểm tier.
    fn remove_conflicts(
        &self,
        store: &Mutex<Store>,
        new_hash: &[u8; 32],
        conflicts: &[Conflict],
        home: &Path,
    ) -> Result<Vec<Removed>, FileOutcome> {
        let fail = |code: &str, message: String| FileOutcome::Failed { code: code.into(), message };
        // gom theo đường dẫn CANONICAL: một bản có thể trùng nhiều PS name, hoặc hiện ra dưới nhiều tên (symlink/firmlink)
        struct Group {
            canon: PathBuf,
            owner: Owner,
            names: Vec<String>,
            raws: Vec<PathBuf>,
        }
        let mut groups: Vec<Group> = vec![];
        for c in conflicts {
            let raw = PathBuf::from(&c.existing_path);
            let canon = canonical_path(&raw).unwrap_or_else(|_| raw.clone());
            match groups.iter_mut().find(|g| g.canon == canon) {
                Some(g) => {
                    g.names.push(c.postscript_name.clone());
                    if c.owner != Owner::FontAtlas {
                        g.owner = c.owner;
                    }
                    if !g.raws.contains(&raw) {
                        g.raws.push(raw);
                    }
                }
                None => groups.push(Group { canon, owner: c.owner, names: vec![c.postscript_name.clone()], raws: vec![raw] }),
            }
        }
        // tier chặt nhất trên canonical VÀ từng URL gốc
        let tier_for = |g: &Group| -> ReplaceTier {
            if g.owner == Owner::FontAtlas {
                return ReplaceTier::Replace;
            }
            let mut t = tier_of(g.owner, &g.canon, home);
            for r in &g.raws {
                t = match (t, tier_of(g.owner, r, home)) {
                    (ReplaceTier::Protected, _) | (_, ReplaceTier::Protected) => ReplaceTier::Protected,
                    (ReplaceTier::Guide, _) | (_, ReplaceTier::Guide) => ReplaceTier::Guide,
                    _ => ReplaceTier::Replace,
                };
            }
            t
        };
        // 1. Ma trận: Protected thắng Guide; một bản không được phép là dừng cả lần, chưa đụng gì.
        let tiers: Vec<ReplaceTier> = groups.iter().map(&tier_for).collect();
        if let Some(i) = tiers.iter().position(|t| *t == ReplaceTier::Protected) {
            return Err(fail(
                "replace_protected",
                format!("{} nằm trong thư mục font của hệ thống hoặc của bạn; Font Atlas không tắt hay di chuyển font ở đó", groups[i].canon.display()),
            ));
        }
        if let Some(i) = tiers.iter().position(|t| *t == ReplaceTier::Guide) {
            return Err(fail("replace_guide_only", format!("{} do app khác quản lý; hãy tắt font trong app đó rồi thử lại", groups[i].canon.display())));
        }
        // 2. Gỡ từng bản. Không biết bản cũ đăng ký ở scope nào → thử cả hai; 201 (chưa đăng ký ở scope đó) là bình thường.
        let mut done: Vec<Removed> = vec![];
        let abort = |this: &Self, done: &[Removed], out: FileOutcome| -> FileOutcome {
            if this.give_back(store, done) { out } else { rollback_failed("trả bản cũ") }
        };
        for g in &groups {
            if g.owner == Owner::FontAtlas {
                // chính ta đăng ký (lô này hoặc trước đó): tắt qua luồng tắt bình thường để dòng kích hoạt khớp thực tế;
                // ghi lại để bước sau hỏng thì bật lại đúng như cũ
                match self.turn_off_our_own(store, &g.canon) {
                    Ok(rows) => done.push(Removed { path: g.canon.clone(), scopes: vec![], journal: None, own: rows }),
                    Err(o) => return Err(abort(self, &done, o)),
                }
                continue;
            }
            // kiểm lại tier NGAY trước khi gỡ (đường dẫn có thể đã đổi từ lúc phân loại)
            if tier_for(g) != ReplaceTier::Replace {
                return Err(abort(self, &done, fail("replace_protected", format!("{} không được phép gỡ", g.canon.display()))));
            }
            let mut any = false;
            for raw in &g.raws {
                let mut scopes = vec![];
                for s in [Scope::Persistent, Scope::Session] {
                    match self.engine.unregister(raw, s) {
                        Ok(()) => scopes.push(s),
                        Err(EngineError::NotRegistered) => {}
                        Err(e) => {
                            let back = self.give_back_one(raw, &scopes);
                            let out = abort(self, &done, engine_failure(&e));
                            return Err(if back { out } else { rollback_failed(e.code()) });
                        }
                    }
                }
                if scopes.is_empty() {
                    continue; // tên khác của cùng file có thể là cái đã đăng ký
                }
                any = true;
                // ghi sổ NGAY sau khi gỡ: nếu bước sau hỏng và không trả lại được thì bản cũ vẫn khôi phục được từ sổ
                let entry = fontatlas_store::NewReplaced {
                    hash: *new_hash,
                    old_path: path_str(raw),
                    owner: g.owner.as_str().to_string(),
                    scopes: scopes.clone(),
                    postscript_names: g.names.clone(),
                };
                let journal = match relock(store).add_replaced(&entry) {
                    Ok(id) => Some(ReplacedFont { id, path: path_str(raw), owner: g.owner, scopes: scopes.clone() }),
                    Err(e) => {
                        let back = self.give_back_one(raw, &scopes);
                        let out = abort(self, &done, fail("store", e.to_string()));
                        return Err(if back { out } else { rollback_failed("ghi sổ") });
                    }
                };
                done.push(Removed { path: raw.clone(), scopes, journal, own: vec![] });
            }
            if !any {
                return Err(abort(
                    self,
                    &done,
                    fail("replace_not_registered", format!("{} không phải font do app đăng ký qua hệ thống nên Font Atlas không tắt được", g.canon.display())),
                ));
            }
        }
        Ok(done)
    }

    /// Tắt font do chính Font Atlas đang bật ở `path` (so theo đường dẫn canonical), chỉ gỡ lý do thủ công (Ask): font còn
    /// lý do từ collection thì KHÔNG đụng (lỗi rõ). Trả các dòng kích hoạt trước khi tắt để bật lại nếu bước sau hỏng.
    fn turn_off_our_own(&self, store: &Mutex<Store>, path: &Path) -> Result<Vec<ActivationRow>, FileOutcome> {
        let rows = relock(store).activation_rows().map_err(|e| FileOutcome::Failed { code: "store".into(), message: e.to_string() })?;
        let want = canonical_path(path).unwrap_or_else(|_| path.to_path_buf());
        let rows: Vec<ActivationRow> = rows
            .into_iter()
            .filter(|r| r.status == ActivationStatus::Registered)
            .filter(|r| r.active_path.as_deref().is_some_and(|a| canonical_path(Path::new(a)).unwrap_or_else(|_| PathBuf::from(a)) == want))
            .collect();
        if rows.is_empty() {
            return Err(FileOutcome::Failed {
                code: "replace_own_not_found".into(),
                message: format!("không tìm thấy font Font Atlas đang bật ở {}", path.display()),
            });
        }
        let keys: Vec<FaceKey> = rows.iter().map(|r| FaceKey { hash: r.hash, face_index: 0 }).collect();
        let bad = match self.deactivate_faces(store, &keys, crate::types::DeactivatePolicy::Ask, &mut |_| {}) {
            Ok(rep) => rep.results.into_iter().map(|r| r.outcome).find(|o| {
                matches!(o, FileOutcome::Failed { .. } | FileOutcome::HeldByCollections { .. } | FileOutcome::KeptActive { .. })
            }),
            Err(e) => Some(FileOutcome::Failed { code: e.code().into(), message: e.to_string() }),
        };
        match bad {
            None => Ok(rows),
            Some(o) => {
                // dừng cả lượt: bật lại mọi thứ vừa tắt, giữ nguyên lý do (kể cả của collection)
                self.give_back_own(store, &rows);
                Err(match o {
                    f @ FileOutcome::Failed { .. } => f,
                    _ => FileOutcome::Failed {
                        code: "replace_own_held".into(),
                        message: format!("{} đang được một collection của Font Atlas giữ bật; hãy tắt nó trong collection trước", path.display()),
                    },
                })
            }
        }
    }

    /// Bật lại các font của Font Atlas vừa bị tắt trong lượt thay: đăng ký lại đúng scope + khôi phục dòng và lý do.
    fn give_back_own(&self, store: &Mutex<Store>, rows: &[ActivationRow]) -> bool {
        let mut all = true;
        for r in rows {
            let Some(p) = r.active_path.as_deref() else { continue };
            match self.engine.register(Path::new(p), r.scope) {
                Ok(()) | Err(EngineError::AlreadyRegistered) => {}
                Err(_) => {
                    all = false;
                    continue;
                }
            }
            let mut row = r.clone();
            row.state = DesiredState::Active;
            row.status = ActivationStatus::Registered;
            row.last_error = None;
            let mut st = relock(store);
            if st.put_activation(&row).is_err() {
                all = false;
                continue;
            }
            for reason in &r.reasons {
                let _ = st.add_activation_reason(&r.hash, *reason);
            }
        }
        all
    }

    /// Đăng ký lại `path` ở các scope; true khi tất cả OK / đã có.
    fn give_back_one(&self, path: &Path, scopes: &[Scope]) -> bool {
        let mut all = true;
        for s in scopes {
            match self.engine.register(path, *s) {
                Ok(()) | Err(EngineError::AlreadyRegistered) => {}
                Err(_) => all = false,
            }
        }
        all
    }

    /// Trả lại các bản đã gỡ (đúng URL + scope). Dòng sổ chỉ bị xoá khi trả lại thành công; hỏng → giữ sổ, trả false.
    fn give_back(&self, store: &Mutex<Store>, removed: &[Removed]) -> bool {
        let mut all = true;
        for r in removed {
            if !r.own.is_empty() {
                all &= self.give_back_own(store, &r.own);
                continue;
            }
            if self.give_back_one(&r.path, &r.scopes) {
                if let Some(j) = &r.journal {
                    let _ = relock(store).delete_replaced(j.id);
                }
            } else {
                all = false;
            }
        }
        all
    }

    /// "Khôi phục bản cũ": đăng ký lại bản đã gỡ (cùng URL + scope), rồi tắt bản của Font Atlas đã thay nó
    /// (chỉ khi không còn dòng sổ nào khác của cùng font; collection còn giữ thì bản của ta ở lại).
    /// Tắt bản của ta hỏng → gỡ lại phần vừa đăng ký, KHÔNG đánh dấu đã khôi phục.
    pub fn restore_replaced(&self, store: &Mutex<Store>, id: i64) -> Result<RestoreOutcome, ActivationError> {
        let row = relock(store).get_replaced(id)?.ok_or_else(|| ActivationError::NotFound(format!("sổ khôi phục #{id}")))?;
        if row.restored_at.is_some() {
            return Ok(RestoreOutcome::AlreadyRestored);
        }
        let path = PathBuf::from(&row.old_path);
        match probe_file(&path) {
            FileProbe::Missing => return Ok(RestoreOutcome::MissingFile),
            FileProbe::NotDownloaded => return Ok(RestoreOutcome::NotDownloaded),
            FileProbe::Ready => {}
        }
        // đăng ký bản cũ trước; hỏng giữa chừng → gỡ các scope vừa đăng ký, chưa đổi gì
        let mut fresh: Vec<Scope> = vec![];
        for s in &row.scopes {
            match self.engine.register(&path, *s) {
                Ok(()) => fresh.push(*s),
                Err(EngineError::AlreadyRegistered) => {}
                Err(e) => {
                    for f in &fresh {
                        let _ = self.engine.unregister(&path, *f);
                    }
                    return Err(engine_err(e));
                }
            }
        }
        let others = relock(store).unrestored_replaced()?.iter().any(|r| r.hash == row.hash && r.id != id);
        if !others {
            // Ask (không Force): lý do bật từ collection không bị xoá; collection còn giữ thì bản của ta ở lại
            let key = FaceKey { hash: row.hash, face_index: 0 };
            let failed = match self.deactivate_faces(store, &[key], crate::types::DeactivatePolicy::Ask, &mut |_| {}) {
                Ok(rep) => rep.results.into_iter().find_map(|r| match r.outcome {
                    FileOutcome::Failed { message, .. } => Some(EngineError::Rejected { code: 0, message }),
                    _ => None,
                }),
                Err(e) => {
                    for f in &fresh {
                        let _ = self.engine.unregister(&path, *f);
                    }
                    return Err(e);
                }
            };
            if let Some(e) = failed {
                for f in &fresh {
                    let _ = self.engine.unregister(&path, *f);
                }
                return Err(engine_err(e));
            }
        }
        relock(store).mark_replaced_restored(id)?;
        Ok(RestoreOutcome::Restored)
    }
}
