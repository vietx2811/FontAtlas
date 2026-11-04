//! Trạng thái kích hoạt font trong user.db (schema `u`), Plan 7.
//!
//! Đơn vị là FILE (`hash`). Mỗi file có tập lý do (`manual`, `collection:<id>`);
//! bất biến: `state = active` ⇔ tập lý do không rỗng. Bảng này chỉ lưu; việc gọi hệ điều hành
//! nằm ở crate `fontatlas-activate`.
use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::ser::SerializeStruct;

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{Store, now_ms};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationScope {
    Session,
    Persistent,
}

impl ActivationScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivationScope::Session => "session",
            ActivationScope::Persistent => "persistent",
        }
    }
    pub fn parse(s: &str) -> Option<ActivationScope> {
        match s {
            "session" => Some(ActivationScope::Session),
            "persistent" => Some(ActivationScope::Persistent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DesiredState {
    Active,
    Inactive,
}

impl DesiredState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DesiredState::Active => "active",
            DesiredState::Inactive => "inactive",
        }
    }
    pub fn parse(s: &str) -> Option<DesiredState> {
        match s {
            "active" => Some(DesiredState::Active),
            "inactive" => Some(DesiredState::Inactive),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationStatus {
    Registered,
    Unregistered,
    Conflict,
    MissingFile,
    NotDownloaded,
    Error,
    Unknown,
}

impl ActivationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivationStatus::Registered => "registered",
            ActivationStatus::Unregistered => "unregistered",
            ActivationStatus::Conflict => "conflict",
            ActivationStatus::MissingFile => "missing_file",
            ActivationStatus::NotDownloaded => "not_downloaded",
            ActivationStatus::Error => "error",
            ActivationStatus::Unknown => "unknown",
        }
    }
    pub fn parse(s: &str) -> Option<ActivationStatus> {
        Some(match s {
            "registered" => ActivationStatus::Registered,
            "unregistered" => ActivationStatus::Unregistered,
            "conflict" => ActivationStatus::Conflict,
            "missing_file" => ActivationStatus::MissingFile,
            "not_downloaded" => ActivationStatus::NotDownloaded,
            "error" => ActivationStatus::Error,
            "unknown" => ActivationStatus::Unknown,
            _ => return None,
        })
    }
}

/// Vì sao một file đang được kích hoạt. `Ord`: `Manual` trước, collection theo id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActivationReason {
    Manual,
    Collection(i64),
}

impl ActivationReason {
    pub fn as_db(&self) -> String {
        match self {
            ActivationReason::Manual => "manual".into(),
            ActivationReason::Collection(id) => format!("collection:{id}"),
        }
    }
    pub fn parse_db(s: &str) -> Option<ActivationReason> {
        if s == "manual" {
            return Some(ActivationReason::Manual);
        }
        let id = s.strip_prefix("collection:")?;
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        id.parse().ok().map(ActivationReason::Collection)
    }
}

/// `{"kind":"manual"}` / `{"kind":"collection","collectionId":3}`. Viết tay vì serde
/// không gắn tag nội bộ được cho biến thể bọc một số nguyên.
impl Serialize for ActivationReason {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            ActivationReason::Manual => {
                let mut st = s.serialize_struct("ActivationReason", 1)?;
                st.serialize_field("kind", "manual")?;
                st.end()
            }
            ActivationReason::Collection(id) => {
                let mut st = s.serialize_struct("ActivationReason", 2)?;
                st.serialize_field("kind", "collection")?;
                st.serialize_field("collectionId", id)?;
                st.end()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationRow {
    /// Đã sắp xếp. Chỉ đọc: `put_activation` không ghi trường này.
    pub reasons: Vec<ActivationReason>,
    pub hash: [u8; 32],
    pub state: DesiredState,
    pub scope: ActivationScope,
    pub method: String,
    pub active_path: Option<String>,
    pub status: ActivationStatus,
    pub conflict_ps: Option<String>,
    pub last_error: Option<String>,
    pub activated_at: Option<i64>,
    pub verified_at: Option<i64>,
    /// `put_activation` luôn đặt bằng `now_ms()`.
    pub updated_at: i64,
    pub engine_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLocation {
    pub path: PathBuf,
    pub missing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceName {
    pub face_index: u32,
    pub postscript_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTarget {
    pub hash: [u8; 32],
    /// `missing = false` trước, rồi `last_seen_at` giảm dần.
    pub locations: Vec<FileLocation>,
    /// Theo `face_index`.
    pub faces: Vec<FaceName>,
}

/// (hash, các lý do đã bị xoá) — kết quả của `prune_collection_reasons`.
pub type PrunedReasons = Vec<([u8; 32], Vec<ActivationReason>)>;

const ROW_COLS: &str = "hash, state, scope, method, active_path, status, conflict_ps, last_error, \
                        activated_at, verified_at, updated_at, engine_version";

fn bad(col: &str, v: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        format!("activation_state.{col} không hợp lệ: {v}").into(),
    )
}

fn read_row(r: &rusqlite::Row) -> rusqlite::Result<ActivationRow> {
    let hash: Vec<u8> = r.get(0)?;
    let hash = hash_from_blob(hash).map_err(|e| bad("hash", &e.to_string()))?;
    let state: String = r.get(1)?;
    let scope: String = r.get(2)?;
    let status: String = r.get(5)?;
    Ok(ActivationRow {
        reasons: vec![],
        hash,
        state: DesiredState::parse(&state).ok_or_else(|| bad("state", &state))?,
        scope: ActivationScope::parse(&scope).ok_or_else(|| bad("scope", &scope))?,
        method: r.get(3)?,
        active_path: r.get(4)?,
        status: ActivationStatus::parse(&status).ok_or_else(|| bad("status", &status))?,
        conflict_ps: r.get(6)?,
        last_error: r.get(7)?,
        activated_at: r.get(8)?,
        verified_at: r.get(9)?,
        updated_at: r.get(10)?,
        engine_version: r.get(11)?,
    })
}

fn reasons_of(conn: &Connection, hash: &[u8; 32]) -> Result<Vec<ActivationReason>, StoreError> {
    let mut st = conn.prepare_cached("SELECT reason FROM u.activation_reasons WHERE hash=?1")?;
    let mut v = st
        .query_map([&hash[..]], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .filter_map(|s| ActivationReason::parse_db(s))
        .collect::<Vec<_>>();
    v.sort();
    Ok(v)
}

fn row_exists(conn: &Connection, hash: &[u8; 32]) -> Result<bool, StoreError> {
    Ok(conn
        .query_row("SELECT 1 FROM u.activation_state WHERE hash=?1", [&hash[..]], |_| Ok(()))
        .optional()?
        .is_some())
}

/// Upsert một dòng trong transaction có sẵn. `ON CONFLICT DO UPDATE` — không dùng
/// `INSERT OR REPLACE`, vì REPLACE xoá dòng cũ và FK cascade sẽ xoá sạch lý do.
pub fn put_activation_tx(tx: &Connection, row: &ActivationRow) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO u.activation_state (hash, state, scope, method, active_path, status, conflict_ps, \
           last_error, activated_at, verified_at, updated_at, engine_version)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT(hash) DO UPDATE SET
           state=excluded.state, scope=excluded.scope, method=excluded.method,
           active_path=excluded.active_path, status=excluded.status, conflict_ps=excluded.conflict_ps,
           last_error=excluded.last_error, activated_at=excluded.activated_at,
           verified_at=excluded.verified_at, updated_at=excluded.updated_at,
           engine_version=excluded.engine_version",
        params![
            &row.hash[..],
            row.state.as_str(),
            row.scope.as_str(),
            row.method,
            row.active_path,
            row.status.as_str(),
            row.conflict_ps,
            row.last_error,
            row.activated_at,
            row.verified_at,
            now_ms(),
            row.engine_version,
        ],
    )?;
    Ok(())
}

/// `true` nếu lý do mới. Dòng `activation_state` phải có trước, thiếu → `NotFound`.
pub fn add_activation_reason_tx(tx: &Connection, hash: &[u8; 32], reason: ActivationReason) -> Result<bool, StoreError> {
    if !row_exists(tx, hash)? {
        return Err(StoreError::NotFound(format!("activation_state {}", blake3_hex(hash))));
    }
    let n = tx.execute(
        "INSERT OR IGNORE INTO u.activation_reasons(hash, reason, added_at) VALUES (?1,?2,?3)",
        params![&hash[..], reason.as_db(), now_ms()],
    )?;
    Ok(n == 1)
}

/// Gỡ một lý do; trả các lý do còn lại (đã sắp xếp).
pub fn remove_activation_reason_tx(
    tx: &Connection,
    hash: &[u8; 32],
    reason: ActivationReason,
) -> Result<Vec<ActivationReason>, StoreError> {
    tx.execute(
        "DELETE FROM u.activation_reasons WHERE hash=?1 AND reason=?2",
        params![&hash[..], reason.as_db()],
    )?;
    reasons_of(tx, hash)
}

fn blake3_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

impl Store {
    pub fn activation_rows(&self) -> Result<Vec<ActivationRow>, StoreError> {
        let mut st = self.conn.prepare(&format!("SELECT {ROW_COLS} FROM u.activation_state ORDER BY hash"))?;
        let mut rows = st.query_map([], read_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        for r in &mut rows {
            r.reasons = reasons_of(&self.conn, &r.hash)?;
        }
        Ok(rows)
    }

    pub fn activation_row(&self, hash: &[u8; 32]) -> Result<Option<ActivationRow>, StoreError> {
        let row = self
            .conn
            .query_row(
                &format!("SELECT {ROW_COLS} FROM u.activation_state WHERE hash=?1"),
                [&hash[..]],
                read_row,
            )
            .optional()?;
        match row {
            Some(mut r) => {
                r.reasons = reasons_of(&self.conn, hash)?;
                Ok(Some(r))
            }
            None => Ok(None),
        }
    }

    pub fn put_activation(&mut self, row: &ActivationRow) -> Result<(), StoreError> {
        put_activation_tx(&self.conn, row)
    }

    /// Xoá dòng; lý do đi theo (FK cascade).
    pub fn delete_activation(&mut self, hash: &[u8; 32]) -> Result<bool, StoreError> {
        Ok(self.conn.execute("DELETE FROM u.activation_state WHERE hash=?1", [&hash[..]])? == 1)
    }

    pub fn add_activation_reason(&mut self, hash: &[u8; 32], reason: ActivationReason) -> Result<bool, StoreError> {
        add_activation_reason_tx(&self.conn, hash, reason)
    }

    pub fn remove_activation_reason(
        &mut self,
        hash: &[u8; 32],
        reason: ActivationReason,
    ) -> Result<Vec<ActivationReason>, StoreError> {
        remove_activation_reason_tx(&self.conn, hash, reason)
    }

    pub fn clear_activation_reasons(&mut self, hash: &[u8; 32]) -> Result<usize, StoreError> {
        Ok(self.conn.execute("DELETE FROM u.activation_reasons WHERE hash=?1", [&hash[..]])?)
    }

    pub fn hashes_with_reason(&self, reason: ActivationReason) -> Result<Vec<[u8; 32]>, StoreError> {
        let mut st = self.conn.prepare("SELECT hash FROM u.activation_reasons WHERE reason=?1 ORDER BY hash")?;
        let blobs = st.query_map([reason.as_db()], |r| r.get::<_, Vec<u8>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        blobs.into_iter().map(hash_from_blob).collect()
    }

    /// Chạy `f` trong một transaction; `Err` → rollback.
    pub fn with_tx<R>(
        &mut self,
        f: impl FnOnce(&rusqlite::Transaction) -> Result<R, StoreError>,
    ) -> Result<R, StoreError> {
        let tx = self.conn.transaction()?;
        let r = f(&tx)?;
        tx.commit()?;
        Ok(r)
    }

    /// Xoá `collection:<id>` khi collection không còn, hoặc file không còn face nào trong đó.
    /// Trả (hash, các lý do đã xoá) theo thứ tự hash.
    pub fn prune_collection_reasons(&mut self) -> Result<PrunedReasons, StoreError> {
        let tx = self.conn.transaction()?;
        let stale: Vec<(Vec<u8>, String)> = {
            let mut st = tx.prepare(
                "SELECT r.hash, r.reason FROM u.activation_reasons r
                 WHERE r.reason LIKE 'collection:%'
                   AND NOT EXISTS (
                     SELECT 1 FROM u.collection_items ci
                     WHERE ci.hash = r.hash
                       AND ci.collection_id = CAST(substr(r.reason, 12) AS INTEGER))
                 ORDER BY r.hash",
            )?;
            st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut out: PrunedReasons = vec![];
        for (blob, reason) in stale {
            tx.execute("DELETE FROM u.activation_reasons WHERE hash=?1 AND reason=?2", params![blob, reason])?;
            let h = hash_from_blob(blob)?;
            let Some(r) = ActivationReason::parse_db(&reason) else { continue };
            match out.last_mut() {
                Some((last, v)) if *last == h => v.push(r),
                _ => out.push((h, vec![r])),
            }
        }
        tx.commit()?;
        for (_, v) in &mut out {
            v.sort();
        }
        Ok(out)
    }

    /// Mọi vị trí và tên PostScript của một file; `None` nếu `files` không có hash.
    pub fn file_target(&self, hash: &[u8; 32]) -> Result<Option<FileTarget>, StoreError> {
        let Some(file_id) = self
            .conn
            .query_row("SELECT file_id FROM main.files WHERE hash=?1", [&hash[..]], |r| r.get::<_, i64>(0))
            .optional()?
        else {
            return Ok(None);
        };
        let mut st = self.conn.prepare(
            "SELECT path, missing FROM main.file_locations WHERE file_id=?1
             ORDER BY missing, last_seen_at DESC, path",
        )?;
        let locations = st
            .query_map([file_id], |r| {
                Ok(FileLocation { path: PathBuf::from(r.get::<_, String>(0)?), missing: r.get::<_, i64>(1)? != 0 })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut st = self
            .conn
            .prepare("SELECT face_index, postscript_name FROM main.faces WHERE file_id=?1 ORDER BY face_index")?;
        let faces = st
            .query_map([file_id], |r| Ok(FaceName { face_index: r.get(0)?, postscript_name: r.get(1)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Some(FileTarget { hash: *hash, locations, faces }))
    }

    /// Các face của một collection; `NotFound` nếu collection không tồn tại.
    pub fn collection_faces(&self, collection_id: i64) -> Result<Vec<FaceKey>, StoreError> {
        let exists = self
            .conn
            .query_row("SELECT 1 FROM u.collections WHERE collection_id=?1", [collection_id], |_| Ok(()))
            .optional()?
            .is_some();
        if !exists {
            return Err(StoreError::NotFound(format!("collection #{collection_id}")));
        }
        let mut st = self.conn.prepare(
            "SELECT hash, face_index FROM u.collection_items WHERE collection_id=?1 ORDER BY added_at, hash, face_index",
        )?;
        let rows = st
            .query_map([collection_id], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, u32>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().map(|(b, i)| Ok(FaceKey { hash: hash_from_blob(b)?, face_index: i })).collect()
    }
}
