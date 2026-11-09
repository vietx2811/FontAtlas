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
use crate::paths::{FileProbe, canonical_path, classify_owner, probe_file, same_content};
use crate::types::{
    ActivationError, ActivationReport, ActivationStateDto, Conflict, ConflictPolicy, ENGINE_VERSION, EngineError, FileOutcome,
    FileResult, Owner, Progress, SETTING_ACTIVATION_KEEP,
};

pub struct ActivationService<E: ActivationEngine> {
    engine: E,
    in_flight: Mutex<HashSet<[u8; 32]>>,
}

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
        Self { engine, in_flight: Mutex::new(HashSet::new()) }
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
        self.run(store, keys, ActivationReason::Manual, scope, false, ConflictPolicy::Ask, true, &mut |_| {})
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
        self.run(store, keys, reason, scope, explicit, policy, false, progress)
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
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let groups = group(keys);
        let mut batch = self.new_batch(store)?;
        let mut rep = ActivationReport::default();
        let total = groups.len();
        for (i, (hash, req)) in groups.into_iter().enumerate() {
            let r = self.one_file(store, &mut batch, hash, req, reason, scope, explicit, policy, dry_run)?;
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
                out.push(Conflict { postscript_name: ps.to_string(), existing_path: existing, owner, same_file });
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
    ) -> Result<FileResult, ActivationError> {
        let mut res = FileResult {
            hash_hex: hex(&hash),
            path: None,
            requested_faces,
            extra_faces: vec![],
            outcome: FileOutcome::Busy,
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
        let path = match locate(&target) {
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
                        return Ok(res);
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
                res.outcome = FileOutcome::Activated;
            }
            Err(e) => {
                let failure = engine_failure(&e);
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
        Ok(res)
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
