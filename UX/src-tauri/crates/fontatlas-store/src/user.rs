//! Dữ liệu người dùng trong user.db (schema `u`): nguồn, favorite, collection, tag.
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use unicode_normalization::UnicodeNormalization;

use crate::error::StoreError;
use crate::fold::vi_fold;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{Store, now_ms};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Directory,
    System,
    GoogleFonts,
    AdobeFonts,
    Community,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Directory => "directory",
            SourceKind::System => "system",
            SourceKind::GoogleFonts => "google_fonts",
            SourceKind::AdobeFonts => "adobe_fonts",
            SourceKind::Community => "community",
        }
    }
    pub fn parse(s: &str) -> Option<SourceKind> {
        Some(match s {
            "directory" => SourceKind::Directory,
            "system" => SourceKind::System,
            "google_fonts" => SourceKind::GoogleFonts,
            "adobe_fonts" => SourceKind::AdobeFonts,
            "community" => SourceKind::Community,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: i64,
    pub kind: SourceKind,
    pub name: String,
    pub path: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub item_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub face_count: u32,
}

/// Chuẩn hoá để so lồng nhau: canonicalize (lỗi thì giữ nguyên) → NFC.
pub(crate) fn norm_path(p: &Path) -> PathBuf {
    let c = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    PathBuf::from(c.to_string_lossy().nfc().collect::<String>())
}

/// Lấy hoặc tạo tag theo `vi_fold(name)`; tên rỗng sau gập → None.
pub(crate) fn tag_in(conn: &Connection, name: &str, now: i64) -> Result<Option<i64>, StoreError> {
    let key = vi_fold(name);
    if key.is_empty() {
        return Ok(None);
    }
    conn.execute(
        "INSERT INTO u.tags(name,name_key,created_at) VALUES (?1,?2,?3) ON CONFLICT(name_key) DO NOTHING",
        params![name.trim(), key, now],
    )?;
    Ok(Some(conn.query_row("SELECT tag_id FROM u.tags WHERE name_key=?1", [key], |r| r.get(0))?))
}

fn row_source(r: &rusqlite::Row) -> rusqlite::Result<(i64, String, String, Option<String>, bool, i64)> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
}

fn to_source(t: (i64, String, String, Option<String>, bool, i64)) -> Source {
    Source {
        id: t.0,
        kind: SourceKind::parse(&t.1).unwrap_or(SourceKind::Directory),
        name: t.2,
        path: t.3,
        enabled: t.4,
        created_at: t.5,
    }
}

const SOURCE_COLS: &str = "source_id, kind, name, path, enabled, created_at";

impl Store {
    /// Thêm nguồn. Trùng (kind, path) → id cũ. Lồng trong/chứa nguồn khác → `NestedSource`.
    pub fn add_source(&mut self, kind: SourceKind, name: &str, path: Option<&Path>) -> Result<i64, StoreError> {
        let tx = self.conn.transaction()?;
        let path_s = path.map(|p| p.to_string_lossy().to_string());
        if let Some(id) = tx
            .query_row(
                "SELECT source_id FROM u.sources WHERE kind=?1 AND path IS ?2",
                params![kind.as_str(), path_s],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
        {
            return Ok(id);
        }
        if let Some(p) = path {
            let new = norm_path(p);
            let mut st = tx.prepare("SELECT source_id, kind, path FROM u.sources WHERE path IS NOT NULL")?;
            let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
            for row in rows {
                let (id, k, ep) = row?;
                let old = norm_path(Path::new(&ep));
                if old == new && k == kind.as_str() {
                    return Ok(id);
                }
                if old.starts_with(&new) || new.starts_with(&old) {
                    return Err(StoreError::NestedSource { existing_id: id });
                }
            }
        }
        tx.execute(
            "INSERT INTO u.sources(kind,name,path,created_at) VALUES (?1,?2,?3,?4)",
            params![kind.as_str(), name, path_s, now_ms()],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    pub fn sources(&self) -> Result<Vec<Source>, StoreError> {
        let mut st = self.conn.prepare(&format!("SELECT {SOURCE_COLS} FROM u.sources ORDER BY source_id"))?;
        let v = st.query_map([], row_source)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(v.into_iter().map(to_source).collect())
    }

    pub fn source(&self, id: i64) -> Result<Source, StoreError> {
        self.conn
            .query_row(&format!("SELECT {SOURCE_COLS} FROM u.sources WHERE source_id=?1"), [id], row_source)
            .optional()?
            .map(to_source)
            .ok_or_else(|| StoreError::NotFound(format!("nguồn #{id}")))
    }

    pub fn set_favorite(&mut self, key: &FaceKey, on: bool) -> Result<(), StoreError> {
        if on {
            self.conn.execute(
                "INSERT OR IGNORE INTO u.favorites(hash,face_index,created_at) VALUES (?1,?2,?3)",
                params![&key.hash[..], key.face_index, now_ms()],
            )?;
        } else {
            self.conn.execute(
                "DELETE FROM u.favorites WHERE hash=?1 AND face_index=?2",
                params![&key.hash[..], key.face_index],
            )?;
        }
        Ok(())
    }

    pub fn favorites(&self) -> Result<Vec<FaceKey>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT hash, face_index FROM u.favorites ORDER BY created_at, hash, face_index")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, u32>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (h, i) = row?;
            out.push(FaceKey { hash: hash_from_blob(h)?, face_index: i });
        }
        Ok(out)
    }

    pub fn create_collection(&mut self, name: &str, color: Option<&str>) -> Result<i64, StoreError> {
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO u.collections(name,color,created_at,updated_at) VALUES (?1,?2,?3,?3)",
            params![name, color, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_collection(&mut self, id: i64, name: &str) -> Result<(), StoreError> {
        let n = self.conn.execute(
            "UPDATE u.collections SET name=?2, updated_at=?3 WHERE collection_id=?1",
            params![id, name, now_ms()],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound(format!("collection #{id}")));
        }
        Ok(())
    }

    /// Xoá collection và lý do kích hoạt `collection:<id>` của nó trong cùng một transaction.
    /// Font hết lý do vẫn đang đăng ký trong hệ thống cho tới khi service gỡ
    /// (`ActivationService::on_collection_deleted`, hoặc `reconcile` lần mở app sau).
    pub fn delete_collection(&mut self, id: i64) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM u.activation_reasons WHERE reason = ?1", [format!("collection:{id}")])?;
        tx.execute("DELETE FROM u.collections WHERE collection_id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_to_collection(&mut self, id: i64, keys: &[FaceKey]) -> Result<usize, StoreError> {
        let now = now_ms();
        let tx = self.conn.transaction()?;
        let mut n = 0;
        for k in keys {
            n += tx.execute(
                "INSERT OR IGNORE INTO u.collection_items(collection_id,hash,face_index,added_at) VALUES (?1,?2,?3,?4)",
                params![id, &k.hash[..], k.face_index, now],
            )?;
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn remove_from_collection(&mut self, id: i64, keys: &[FaceKey]) -> Result<usize, StoreError> {
        let tx = self.conn.transaction()?;
        let mut n = 0;
        for k in keys {
            n += tx.execute(
                "DELETE FROM u.collection_items WHERE collection_id=?1 AND hash=?2 AND face_index=?3",
                params![id, &k.hash[..], k.face_index],
            )?;
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn collections(&self) -> Result<Vec<Collection>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT c.collection_id, c.name, c.color,
                    (SELECT count(*) FROM u.collection_items i WHERE i.collection_id = c.collection_id)
             FROM u.collections c ORDER BY c.position, c.name, c.collection_id",
        )?;
        let v = st
            .query_map([], |r| {
                Ok(Collection { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, item_count: r.get(3)? })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(v)
    }

    pub fn tag(&mut self, name: &str) -> Result<i64, StoreError> {
        tag_in(&self.conn, name, now_ms())?.ok_or_else(|| StoreError::NotFound("tag rỗng".into()))
    }

    pub fn tag_faces(&mut self, tag_id: i64, keys: &[FaceKey]) -> Result<usize, StoreError> {
        let now = now_ms();
        let tx = self.conn.transaction()?;
        let mut n = 0;
        for k in keys {
            n += tx.execute(
                "INSERT OR IGNORE INTO u.face_tags(tag_id,hash,face_index,added_at) VALUES (?1,?2,?3,?4)",
                params![tag_id, &k.hash[..], k.face_index, now],
            )?;
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn untag_faces(&mut self, tag_id: i64, keys: &[FaceKey]) -> Result<usize, StoreError> {
        let tx = self.conn.transaction()?;
        let mut n = 0;
        for k in keys {
            n += tx.execute(
                "DELETE FROM u.face_tags WHERE tag_id=?1 AND hash=?2 AND face_index=?3",
                params![tag_id, &k.hash[..], k.face_index],
            )?;
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn tags(&self) -> Result<Vec<Tag>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT t.tag_id, t.name, (SELECT count(*) FROM u.face_tags ft WHERE ft.tag_id = t.tag_id)
             FROM u.tags t ORDER BY t.name_key",
        )?;
        let v = st
            .query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, face_count: r.get(2)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(v)
    }

    pub fn tags_of(&self, key: &FaceKey) -> Result<Vec<Tag>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT t.tag_id, t.name, (SELECT count(*) FROM u.face_tags x WHERE x.tag_id = t.tag_id)
             FROM u.tags t JOIN u.face_tags ft ON ft.tag_id = t.tag_id
             WHERE ft.hash = ?1 AND ft.face_index = ?2 ORDER BY t.name_key",
        )?;
        let v = st
            .query_map(params![&key.hash[..], key.face_index], |r| {
                Ok(Tag { id: r.get(0)?, name: r.get(1)?, face_count: r.get(2)? })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(v)
    }
}
