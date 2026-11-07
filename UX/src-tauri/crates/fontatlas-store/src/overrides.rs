//! Ghi đè phân loại của người dùng (user.db v4): luôn ưu tiên hơn kết quả tự động.
use rusqlite::{OptionalExtension, params};

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{Store, now_ms};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationOverride {
    pub key: FaceKey,
    /// snake_case: serif | sans | mono | script | blackletter | symbol
    pub class: String,
    pub subclass: Option<String>,
    pub updated_at: i64,
}

impl Store {
    pub fn set_classification_override(&mut self, key: &FaceKey, class: &str, subclass: Option<&str>) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO u.classification_overrides(hash, face_index, class, subclass, updated_at) VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(hash, face_index) DO UPDATE SET class=excluded.class, subclass=excluded.subclass, updated_at=excluded.updated_at",
            params![&key.hash[..], key.face_index, class, subclass, now_ms()],
        )?;
        Ok(())
    }

    pub fn clear_classification_override(&mut self, key: &FaceKey) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM u.classification_overrides WHERE hash=?1 AND face_index=?2", params![&key.hash[..], key.face_index])?;
        Ok(())
    }

    pub fn classification_override(&self, key: &FaceKey) -> Result<Option<ClassificationOverride>, StoreError> {
        let r = self
            .conn
            .query_row(
                "SELECT class, subclass, updated_at FROM u.classification_overrides WHERE hash=?1 AND face_index=?2",
                params![&key.hash[..], key.face_index],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, i64>(2)?)),
            )
            .optional()?;
        Ok(r.map(|(class, subclass, updated_at)| ClassificationOverride { key: *key, class, subclass, updated_at }))
    }

    pub fn classification_overrides(&self) -> Result<Vec<ClassificationOverride>, StoreError> {
        let mut st = self.conn.prepare("SELECT hash, face_index, class, subclass, updated_at FROM u.classification_overrides ORDER BY hash, face_index")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, i64>(4)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (h, fi, class, subclass, updated_at) = row?;
            out.push(ClassificationOverride { key: FaceKey { hash: hash_from_blob(h)?, face_index: fi as u32 }, class, subclass, updated_at });
        }
        Ok(out)
    }
}
