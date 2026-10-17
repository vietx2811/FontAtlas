//! Di chuyển từ DB cũ (`fontatlas.db`, bảng `fonts` 35 cột) sang user.db.
//! Bước 1 `import_legacy`: chép dữ liệu người dùng vào `u.legacy_ids` (một transaction).
//! Bước 2 `resolve_legacy` (sau rescan): nối theo đường dẫn → favorite/collection/tag. Chỉ INSERT OR IGNORE.
use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::error::StoreError;
use crate::fold::vi_fold;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{Store, now_ms};
use crate::user::{SourceKind, norm_path, tag_in};

pub const LEGACY_DB_FILE: &str = "fontatlas.db";

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImport {
    pub fonts: usize,
    pub sources: usize,
    pub collections: usize,
    pub recovered_collections: usize,
    pub bad_tags: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyResolve {
    pub resolved: usize,
    pub unresolved: usize,
    pub ambiguous_faces: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Payload {
    name: Option<String>,
    family: Option<String>,
    subfamily: Option<String>,
    favorite: bool,
    collection_id: Option<String>,
    tags: Vec<String>,
    date_added: Option<String>,
}

struct OldFont {
    id: String,
    path: String,
    payload: Payload,
    bad_tags: bool,
}

fn lerr(e: impl std::fmt::Display) -> StoreError {
    StoreError::Legacy(e.to_string())
}

fn columns(c: &Connection, table: &str) -> Result<HashSet<String>, StoreError> {
    let mut st = c.prepare(&format!("PRAGMA table_info({table})")).map_err(lerr)?;
    let v = st
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(lerr)?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(lerr)?;
    Ok(v)
}

/// Danh sách cột cho SELECT: cột thiếu → NULL.
fn select_list(have: &HashSet<String>, want: &[&str]) -> String {
    want.iter()
        .map(|c| if have.contains(*c) { (*c).to_string() } else { "NULL".to_string() })
        .collect::<Vec<_>>()
        .join(", ")
}

fn text(v: ValueRef) -> Option<String> {
    match v {
        ValueRef::Text(t) => Some(String::from_utf8_lossy(t).to_string()),
        ValueRef::Integer(i) => Some(i.to_string()),
        _ => None,
    }
}

fn truthy(v: ValueRef) -> bool {
    match v {
        ValueRef::Integer(i) => i != 0,
        ValueRef::Real(f) => f != 0.0,
        ValueRef::Text(t) => matches!(t, b"1" | b"true"),
        _ => false,
    }
}

/// NULL → không tag, không lỗi; chuỗi JSON mảng chuỗi → tag; còn lại → lỗi (đếm).
fn tags_of(v: ValueRef) -> (Vec<String>, bool) {
    match v {
        ValueRef::Null => (vec![], false),
        ValueRef::Text(t) => match serde_json::from_slice::<Vec<String>>(t) {
            Ok(v) => (v, false),
            Err(_) => (vec![], true),
        },
        _ => (vec![], true),
    }
}

struct OldDb {
    fonts: Vec<OldFont>,
    sources: Vec<(String, String, String, Option<String>)>, // id, name, type, path
    collections: Vec<(String, String, Option<String>)>,       // id, name, color
}

fn read_old(path: &Path) -> Result<OldDb, StoreError> {
    let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(lerr)?;
    let fc = columns(&c, "fonts")?;
    if !fc.contains("id") || !fc.contains("file_path") {
        return Err(StoreError::Legacy("không có bảng fonts(id, file_path)".into()));
    }
    let sel = select_list(
        &fc,
        &["id", "file_path", "name", "family_name", "subfamily", "tags", "collection_id", "is_favorite", "date_added"],
    );
    let mut fonts = Vec::new();
    {
        let mut st = c.prepare(&format!("SELECT {sel} FROM fonts ORDER BY id")).map_err(lerr)?;
        let mut rows = st.query([]).map_err(lerr)?;
        while let Some(r) = rows.next().map_err(lerr)? {
            let g = |i: usize| r.get_ref(i).map_err(lerr);
            let (Some(id), Some(path)) = (text(g(0)?), text(g(1)?)) else { continue };
            let (tags, bad) = tags_of(g(5)?);
            fonts.push(OldFont {
                id,
                path,
                bad_tags: bad,
                payload: Payload {
                    name: text(g(2)?),
                    family: text(g(3)?),
                    subfamily: text(g(4)?),
                    tags,
                    collection_id: text(g(6)?).filter(|s| !s.is_empty()),
                    favorite: truthy(g(7)?),
                    date_added: text(g(8)?),
                },
            });
        }
    }
    let mut sources = Vec::new();
    let sc = columns(&c, "sources")?;
    if sc.contains("id") && sc.contains("type") {
        let sel = select_list(&sc, &["id", "name", "type", "path"]);
        let mut st = c.prepare(&format!("SELECT {sel} FROM sources ORDER BY id")).map_err(lerr)?;
        let mut rows = st.query([]).map_err(lerr)?;
        while let Some(r) = rows.next().map_err(lerr)? {
            let g = |i: usize| r.get_ref(i).map_err(lerr);
            if let (Some(id), Some(kind)) = (text(g(0)?), text(g(2)?)) {
                let name = text(g(1)?).unwrap_or_else(|| id.clone());
                sources.push((id, name, kind, text(g(3)?)));
            }
        }
    }
    let mut collections = Vec::new();
    let cc = columns(&c, "collections")?;
    if cc.contains("id") {
        let sel = select_list(&cc, &["id", "name", "color"]);
        let mut st = c.prepare(&format!("SELECT {sel} FROM collections ORDER BY id")).map_err(lerr)?;
        let mut rows = st.query([]).map_err(lerr)?;
        while let Some(r) = rows.next().map_err(lerr)? {
            let g = |i: usize| r.get_ref(i).map_err(lerr);
            if let Some(id) = text(g(0)?) {
                let name = text(g(1)?).unwrap_or_else(|| id.clone());
                collections.push((id, name, text(g(2)?)));
            }
        }
    }
    Ok(OldDb { fonts, sources, collections })
}

fn nfc(s: &str) -> String {
    s.nfc().collect()
}

impl Store {
    /// Chép dữ liệu người dùng từ DB cũ (mở chỉ đọc) vào user.db. Chạy lại an toàn.
    pub fn import_legacy(&mut self, old_db: &Path) -> Result<LegacyImport, StoreError> {
        let old = read_old(old_db)?;
        let now = now_ms();
        let mut rep = LegacyImport { fonts: old.fonts.len(), ..Default::default() };
        let tx = self.conn.transaction()?;

        // nguồn: bỏ nguồn thư mục không có path và nguồn lồng trong nguồn khác
        for (id, name, kind, path) in &old.sources {
            let kind = SourceKind::parse(kind).unwrap_or(SourceKind::Directory);
            if path.is_none() && matches!(kind, SourceKind::Directory | SourceKind::System) {
                continue;
            }
            let exact: Option<i64> = tx
                .query_row(
                    "SELECT source_id FROM u.sources WHERE kind=?1 AND path IS ?2",
                    params![kind.as_str(), path],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(sid) = exact {
                tx.execute(
                    "UPDATE u.sources SET legacy_id=?2 WHERE source_id=?1 AND legacy_id IS NULL",
                    params![sid, id],
                )?;
                continue;
            }
            if let Some(p) = path {
                let new = norm_path(Path::new(p));
                let mut st = tx.prepare("SELECT path FROM u.sources WHERE path IS NOT NULL")?;
                let others = st.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
                let nested = others.iter().map(|o| norm_path(Path::new(o))).any(|o| o.starts_with(&new) || new.starts_with(&o));
                if nested {
                    continue;
                }
            }
            tx.execute(
                "INSERT INTO u.sources(legacy_id,kind,name,path,created_at) VALUES (?1,?2,?3,?4,?5)
                 ON CONFLICT DO NOTHING",
                params![id, kind.as_str(), name, path, now],
            )?;
        }
        for (id, ..) in &old.sources {
            rep.sources += tx.query_row("SELECT count(*) FROM u.sources WHERE legacy_id=?1", [id], |r| r.get::<_, i64>(0))? as usize;
        }

        for (id, name, color) in &old.collections {
            tx.execute(
                "INSERT INTO u.collections(legacy_id,name,color,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)
                 ON CONFLICT(legacy_id) DO NOTHING",
                params![id, name, color, now],
            )?;
        }
        rep.collections = old.collections.len();
        let known: HashSet<&str> = old.collections.iter().map(|c| c.0.as_str()).collect();
        let mut orphan: Vec<&str> = old
            .fonts
            .iter()
            .filter_map(|f| f.payload.collection_id.as_deref())
            .filter(|c| !known.contains(c))
            .collect();
        orphan.sort_unstable();
        orphan.dedup();
        for (n, id) in orphan.iter().enumerate() {
            tx.execute(
                "INSERT INTO u.collections(legacy_id,name,created_at,updated_at) VALUES (?1,?2,?3,?3)
                 ON CONFLICT(legacy_id) DO NOTHING",
                params![id, format!("Đã khôi phục ({})", n + 1), now],
            )?;
        }
        rep.recovered_collections = orphan.len();

        for f in &old.fonts {
            rep.bad_tags += f.bad_tags as usize;
            tx.execute(
                "INSERT INTO u.legacy_ids(legacy_id,legacy_path,payload) VALUES (?1,?2,?3)
                 ON CONFLICT(legacy_id) DO NOTHING",
                params![f.id, f.path, serde_json::to_string(&f.payload).unwrap_or_else(|_| "{}".into())],
            )?;
        }
        let log = serde_json::json!({ "source": old_db.to_string_lossy(), "importedAt": now, "result": &rep });
        tx.execute(
            "INSERT OR REPLACE INTO u.settings(key,value) VALUES ('legacy_import', ?1)",
            [log.to_string()],
        )?;
        tx.commit()?;
        Ok(rep)
    }

    /// Nối các dòng cũ chưa có hash với vị trí hiện có (so đường dẫn sau NFC), rồi áp dữ liệu người dùng.
    pub fn resolve_legacy(&mut self) -> Result<LegacyResolve, StoreError> {
        let mut rep = LegacyResolve::default();
        let now = now_ms();
        let tx = self.conn.transaction()?;
        let pending: Vec<(String, String, String)> = {
            let mut st = tx.prepare("SELECT legacy_id, legacy_path, payload FROM u.legacy_ids WHERE hash IS NULL")?;
            st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let locs: HashMap<String, (Vec<u8>, i64)> = {
            let mut st = tx.prepare(
                "SELECT l.path, fi.hash, fi.file_id FROM file_locations l JOIN files fi USING(file_id) WHERE l.missing = 0",
            )?;
            st.query_map([], |r| Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?))))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .map(|(p, v)| (nfc(&p), v))
                .collect()
        };
        for (lid, lpath, payload) in pending {
            let Some((hash, file_id)) = locs.get(&nfc(&lpath)) else { continue };
            let hash = hash_from_blob(hash.clone())?;
            let p: Payload = serde_json::from_str(&payload).unwrap_or_default();
            let faces: Vec<(i64, String, Option<String>)> = {
                let mut st = tx.prepare("SELECT face_index, family_name, subfamily FROM faces WHERE file_id=?1 ORDER BY face_index")?;
                st.query_map([file_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            let face_index = if faces.len() <= 1 {
                0
            } else {
                let fam = p.family.as_deref().map(vi_fold);
                let sub = p.subfamily.as_deref().map(vi_fold);
                match faces.iter().find(|(_, f, s)| Some(vi_fold(f)) == fam && s.as_deref().map(vi_fold) == sub) {
                    Some((i, ..)) => *i,
                    None => {
                        rep.ambiguous_faces += 1;
                        0
                    }
                }
            };
            let h = &hash[..];
            if p.favorite {
                tx.execute(
                    "INSERT OR IGNORE INTO u.favorites(hash,face_index,created_at) VALUES (?1,?2,?3)",
                    params![h, face_index, now],
                )?;
            }
            if let Some(cid) = &p.collection_id {
                let col: Option<i64> = tx
                    .query_row("SELECT collection_id FROM u.collections WHERE legacy_id=?1", [cid], |r| r.get(0))
                    .optional()?;
                if let Some(col) = col {
                    tx.execute(
                        "INSERT OR IGNORE INTO u.collection_items(collection_id,hash,face_index,added_at) VALUES (?1,?2,?3,?4)",
                        params![col, h, face_index, now],
                    )?;
                }
            }
            for t in &p.tags {
                if let Some(tid) = tag_in(&tx, t, now)? {
                    tx.execute(
                        "INSERT OR IGNORE INTO u.face_tags(tag_id,hash,face_index,added_at) VALUES (?1,?2,?3,?4)",
                        params![tid, h, face_index, now],
                    )?;
                }
            }
            rep.resolved += tx.execute(
                "UPDATE u.legacy_ids SET hash=?2, face_index=?3, resolved_at=?4 WHERE legacy_id=?1 AND hash IS NULL",
                params![lid, h, face_index, now],
            )?;
        }
        rep.unresolved =
            tx.query_row("SELECT count(*) FROM u.legacy_ids WHERE hash IS NULL", [], |r| r.get::<_, i64>(0))? as usize;
        tx.commit()?;
        Ok(rep)
    }

    /// Danh tính mới của một id cũ (chuyển tiếp API); None nếu chưa nối.
    pub fn legacy_face(&self, legacy_id: &str) -> Result<Option<FaceKey>, StoreError> {
        let row: Option<(Vec<u8>, u32)> = self
            .conn
            .query_row(
                "SELECT hash, face_index FROM u.legacy_ids WHERE legacy_id=?1 AND hash IS NOT NULL",
                [legacy_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(h, i)| Ok(FaceKey { hash: hash_from_blob(h)?, face_index: i })).transpose()
    }
}
