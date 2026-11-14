//! G7b: sổ bản sao đổi tên của VF. Bản sao là một font BÌNH THƯỜNG của thư viện (hash riêng), sổ này chỉ nối nó với bản gốc để
//! broker/Duplicates loại nó khỏi ứng viên và để thao tác lặp lại (idempotent) trả đúng bản đã có.
use std::collections::HashSet;

use rusqlite::{OptionalExtension, params};

use crate::error::StoreError;
use crate::open::{Store, now_ms};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedFont {
    pub hash: [u8; 32],
    pub derived_from: [u8; 32],
    pub path: String,
    pub license_restricted: bool,
}

fn h32(v: Vec<u8>) -> Result<[u8; 32], StoreError> {
    v.try_into().map_err(|_| StoreError::BadHash("hash không đủ 32 byte".into()))
}

/// Một lần gỡ bản sao (để Hoàn tác).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedRemoval {
    pub id: i64,
    pub hash: [u8; 32],
    pub derived_from: [u8; 32],
    pub original_path: String,
    pub trash_path: Option<String>,
    pub license_restricted: bool,
    pub undone: bool,
}

impl Store {
    /// Gỡ bản sao khỏi sổ + vị trí file của nó trong thư viện (+ `gc`), và ghi nhật ký gỡ. Không đụng file trên đĩa.
    pub fn derived_remove(&mut self, d: &DerivedFont, trash_path: Option<&str>) -> Result<i64, StoreError> {
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM u.derived_fonts WHERE hash=?1", [&d.hash[..]])?;
        tx.execute(
            "INSERT INTO u.derived_removals(hash, derived_from_hash, original_path, trash_path, license_restricted, removed_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![&d.hash[..], &d.derived_from[..], d.path, trash_path, d.license_restricted, now_ms()],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        self.conn.execute("DELETE FROM file_locations WHERE path=?1", [&d.path])?;
        self.gc()?;
        Ok(id)
    }

    pub fn derived_removal(&self, id: i64) -> Result<Option<DerivedRemoval>, StoreError> {
        self.conn
            .query_row(
                "SELECT id, hash, derived_from_hash, original_path, trash_path, license_restricted, undone_at IS NOT NULL FROM u.derived_removals WHERE id=?1",
                [id],
                |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, Vec<u8>>(2)?, r.get::<_, String>(3)?, r.get::<_, Option<String>>(4)?, r.get::<_, bool>(5)?, r.get::<_, bool>(6)?))
                },
            )
            .optional()?
            .map(|(id, h, f, original_path, trash_path, license_restricted, undone)| {
                Ok(DerivedRemoval { id, hash: h32(h)?, derived_from: h32(f)?, original_path, trash_path, license_restricted, undone })
            })
            .transpose()
    }

    pub fn derived_removal_mark_undone(&mut self, id: i64) -> Result<(), StoreError> {
        self.conn.execute("UPDATE u.derived_removals SET undone_at=?2 WHERE id=?1", params![id, now_ms()])?;
        Ok(())
    }

    /// Các bản sao hiện có của font gốc `from` (hiện tối đa một, nhưng trả danh sách để giao diện không phụ thuộc).
    pub fn derived_copies_of(&self, from: &[u8; 32]) -> Result<Vec<DerivedFont>, StoreError> {
        Ok(self.derived_for(from)?.into_iter().collect())
    }

    pub fn derived_record(&mut self, d: &DerivedFont) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO u.derived_fonts(hash, derived_from_hash, path, license_restricted, created_at) VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(derived_from_hash) DO UPDATE SET hash=excluded.hash, path=excluded.path, license_restricted=excluded.license_restricted",
            params![&d.hash[..], &d.derived_from[..], d.path, d.license_restricted, now_ms()],
        )?;
        Ok(())
    }

    /// Bản sao đổi tên đã có của font gốc `from`.
    pub fn derived_for(&self, from: &[u8; 32]) -> Result<Option<DerivedFont>, StoreError> {
        self.conn
            .query_row(
                "SELECT hash, derived_from_hash, path, license_restricted FROM u.derived_fonts WHERE derived_from_hash=?1",
                [&from[..]],
                |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, String>(2)?, r.get::<_, bool>(3)?)),
            )
            .optional()?
            .map(|(h, f, path, license_restricted)| Ok(DerivedFont { hash: h32(h)?, derived_from: h32(f)?, path, license_restricted }))
            .transpose()
    }

    /// Dòng sổ của chính bản sao `hash` (không phải của font gốc).
    pub fn derived_for_copy(&self, hash: &[u8; 32]) -> Result<Option<DerivedFont>, StoreError> {
        self.conn
            .query_row(
                "SELECT hash, derived_from_hash, path, license_restricted FROM u.derived_fonts WHERE hash=?1",
                [&hash[..]],
                |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, String>(2)?, r.get::<_, bool>(3)?)),
            )
            .optional()?
            .map(|(h, f, path, license_restricted)| Ok(DerivedFont { hash: h32(h)?, derived_from: h32(f)?, path, license_restricted }))
            .transpose()
    }

    pub fn is_derived(&self, hash: &[u8; 32]) -> Result<bool, StoreError> {
        Ok(self.conn.query_row("SELECT 1 FROM u.derived_fonts WHERE hash=?1", [&hash[..]], |_| Ok(())).optional()?.is_some())
    }

    /// Hash của font gốc mà `hash` được tạo ra từ đó (None nếu `hash` không phải bản sao).
    pub fn derived_from(&self, hash: &[u8; 32]) -> Result<Option<[u8; 32]>, StoreError> {
        self.conn
            .query_row("SELECT derived_from_hash FROM u.derived_fonts WHERE hash=?1", [&hash[..]], |r| r.get::<_, Vec<u8>>(0))
            .optional()?
            .map(h32)
            .transpose()
    }

    pub fn derived_hashes(&self) -> Result<HashSet<[u8; 32]>, StoreError> {
        let mut st = self.conn.prepare("SELECT hash FROM u.derived_fonts")?;
        let v = st.query_map([], |r| r.get::<_, Vec<u8>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        v.into_iter().map(h32).collect()
    }

    /// Vị trí tốt nhất để ĐỌC font `hash`: còn thấy, ưu tiên bản đã có trên máy. `(đường dẫn, cloud_only)`.
    pub fn best_location(&self, hash: &[u8; 32]) -> Result<Option<(String, bool)>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT l.path, l.cloud_only FROM file_locations l JOIN files fi ON fi.file_id = l.file_id
                  WHERE fi.hash=?1 AND l.missing=0 ORDER BY l.cloud_only, l.location_id LIMIT 1",
                [&hash[..]],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?)),
            )
            .optional()?)
    }

    /// "Họ Kiểu" của một face để hiển thị (vd "Oswald Regular"); None nếu không có trong thư viện.
    pub fn face_label(&self, hash: &[u8; 32], face_index: u32) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT f.family_name, f.subfamily FROM faces f JOIN files fi ON fi.file_id = f.file_id WHERE fi.hash=?1 AND f.face_index=?2",
                params![&hash[..], face_index],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
            )
            .optional()?
            .map(|(fam, sub)| match sub.filter(|s| !s.is_empty()) {
                Some(s) => format!("{fam} {s}"),
                None => fam,
            }))
    }

    /// `faces.is_variable` của một face (theo hash + index); None nếu không có trong thư viện.
    pub fn face_is_variable(&self, hash: &[u8; 32], face_index: u32) -> Result<Option<(i64, bool)>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT f.face_id, f.is_variable FROM faces f JOIN files fi ON fi.file_id = f.file_id WHERE fi.hash=?1 AND f.face_index=?2",
                params![&hash[..], face_index],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, bool>(1)?)),
            )
            .optional()?)
    }
}
