//! Cặp font đã lưu (user.db v3).
use rusqlite::{Row, params};

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{Store, now_ms};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPair {
    pub id: i64,
    pub a: FaceKey,
    pub a_role: String,
    pub b: FaceKey,
    pub b_role: String,
    pub mode: Option<String>,
    pub preset: Option<String>,
    pub sample_text: Option<String>,
    pub collection_id: Option<i64>,
    pub note: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPair {
    pub a: FaceKey,
    pub a_role: String,
    pub b: FaceKey,
    pub b_role: String,
    pub mode: Option<String>,
    pub preset: Option<String>,
    pub sample_text: Option<String>,
    pub collection_id: Option<i64>,
    pub note: Option<String>,
}

fn from_row(r: &Row) -> Result<SavedPair, StoreError> {
    Ok(SavedPair {
        id: r.get(0)?,
        a: FaceKey { hash: hash_from_blob(r.get(1)?)?, face_index: r.get::<_, i64>(2)? as u32 },
        a_role: r.get(3)?,
        b: FaceKey { hash: hash_from_blob(r.get(4)?)?, face_index: r.get::<_, i64>(5)? as u32 },
        b_role: r.get(6)?,
        mode: r.get(7)?,
        preset: r.get(8)?,
        sample_text: r.get(9)?,
        collection_id: r.get(10)?,
        note: r.get(11)?,
        created_at: r.get(12)?,
    })
}

impl Store {
    pub fn save_pair(&mut self, p: &NewPair) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO u.pairs(a_hash,a_face,a_role,b_hash,b_face,b_role,mode,preset,sample_text,collection_id,note,created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                &p.a.hash[..], p.a.face_index, p.a_role, &p.b.hash[..], p.b.face_index, p.b_role,
                p.mode, p.preset, p.sample_text, p.collection_id, p.note, now_ms()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// `collection_id = None` → mọi cặp; mới nhất trước.
    pub fn pairs(&self, collection_id: Option<i64>) -> Result<Vec<SavedPair>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT pair_id,a_hash,a_face,a_role,b_hash,b_face,b_role,mode,preset,sample_text,collection_id,note,created_at
             FROM u.pairs WHERE (?1 IS NULL OR collection_id = ?1) ORDER BY created_at DESC, pair_id DESC",
        )?;
        let mut rows = st.query([collection_id])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push(from_row(r)?);
        }
        Ok(out)
    }

    pub fn delete_pair(&mut self, id: i64) -> Result<(), StoreError> {
        let n = self.conn.execute("DELETE FROM u.pairs WHERE pair_id = ?1", [id])?;
        if n == 0 {
            return Err(StoreError::NotFound(format!("cặp #{id}")));
        }
        Ok(())
    }
}
