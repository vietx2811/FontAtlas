//! `ActivationService`: điều phối Store + engine.
//!
//! - Khoá `Store` theo từng file và nhả trước khi gọi engine / `progress`; không giữ suốt lô.
//! - Khoá thao tác theo hash (`in_flight`): hash đang bận → `Busy`.
//! - Ghi dòng "ý định" (`status=unknown`) trước khi gọi engine, để crash giữa chừng không để lại
//!   đăng ký mồ côi (`reconcile` xử lý dòng `unknown`).
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use fontatlas_store::{
    ActivationReason, ActivationRow, ActivationStatus, DesiredState, FaceKey, FileTarget, Store,
    add_activation_reason_tx, put_activation_tx, remove_activation_reason_tx,
};

use crate::engine::{ActivationEngine, Scope};
use crate::paths::{FileProbe, canonical_path, classify_owner, probe_file};
use crate::types::{
    ActivationError, ActivationReport, Conflict, ConflictPolicy, ENGINE_VERSION, EngineError, FileOutcome,
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

fn relock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
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
    Ready(PathBuf),
    Missing(Option<PathBuf>),
    NotDownloaded(PathBuf),
}

/// Vị trí đầu tiên `Ready` (đã canonical); không có thì phân biệt iCloud chưa tải / thiếu hẳn.
pub(crate) fn locate(t: &FileTarget) -> Located {
    let mut not_downloaded = None;
    for loc in &t.locations {
        match probe_file(&loc.path) {
            FileProbe::Ready => {
                if let Ok(c) = canonical_path(&loc.path) {
                    return Located::Ready(c);
                }
            }
            FileProbe::NotDownloaded => {
                not_downloaded.get_or_insert_with(|| loc.path.clone());
            }
            FileProbe::Missing => {}
        }
    }
    match not_downloaded {
        Some(p) => Located::NotDownloaded(p),
        None => Located::Missing(t.locations.first().map(|l| l.path.clone())),
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
        self.run(store, keys, ActivationReason::Manual, scope, ConflictPolicy::Ask, true, &mut |_| {})
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
        let scope = match scope {
            Some(s) => s,
            None => Self::default_scope(store)?,
        };
        self.run(store, keys, reason, scope, policy, false, progress)
    }

    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        store: &Mutex<Store>,
        keys: &[FaceKey],
        reason: ActivationReason,
        scope: Scope,
        policy: ConflictPolicy,
        dry_run: bool,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ActivationReport, ActivationError> {
        let groups = group(keys);
        let ours: HashSet<PathBuf> = relock(store)
            .activation_rows()?
            .into_iter()
            .filter(|r| r.status == ActivationStatus::Registered)
            .filter_map(|r| r.active_path.map(PathBuf::from))
            .collect();
        let mut batch = Batch {
            avail: self.engine.available_postscript_names().map_err(engine_err)?,
            ours,
            home: std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default(),
            seen: HashMap::new(),
        };
        let mut rep = ActivationReport::default();
        let total = groups.len();
        for (i, (hash, req)) in groups.into_iter().enumerate() {
            let r = self.one_file(store, &mut batch, hash, req, reason, scope, policy, dry_run)?;
            rep.push(r);
            progress(Progress { done: i + 1, total, hash_hex: hex(&hash) });
        }
        Ok(rep)
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
                out.push(Conflict { postscript_name: ps.to_string(), existing_path: existing, owner });
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
        };
        let pstr = path_str(&path);
        res.path = Some(pstr.clone());
        let names = self.names_of(&target, &path);

        // 3. Đã bật đúng chỗ, đúng scope → chỉ thêm lý do.
        let same = old.as_ref().is_some_and(|o| {
            o.state == DesiredState::Active
                && o.status == ActivationStatus::Registered
                && o.active_path.as_deref() == Some(pstr.as_str())
                && o.scope == scope
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
            if o.status == ActivationStatus::Registered && (op != &path || o.scope != scope) {
                match self.engine.unregister(op, o.scope) {
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
}
