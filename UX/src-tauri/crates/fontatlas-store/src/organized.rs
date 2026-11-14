//! B7 Organized Library (tầng dữ liệu): sổ bản chép, ứng viên để chép, nhật ký di chuyển có hoàn tác.
//! Logic chọn thư mục theo class/họ nằm ở app (cần Index để biết class hiệu lực); ở đây chỉ có SQL và các thao tác nguyên tử.
use std::collections::BTreeMap;

use rusqlite::{OptionalExtension, params};

use crate::error::StoreError;
use crate::ingest::FaceKey;
use crate::open::{Store, now_ms};

/// Một font (theo hash) có thể chép vào Organized Library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizeCandidate {
    pub hash: [u8; 32],
    pub size: i64,
    /// Vị trí nguồn chọn để chép: ưu tiên bản đã có trên máy, nằm NGOÀI thư viện có tổ chức.
    pub src_path: String,
    pub cloud_only: bool,
    /// Họ và `face_id` của các face; face đầu (index nhỏ nhất) quyết định thư mục.
    pub family: String,
    pub face_ids: Vec<i64>,
    /// Đã có vị trí nằm trong thư viện có tổ chức (đã chép hoặc người dùng tự bỏ vào).
    pub in_library: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizedFile {
    pub hash: [u8; 32],
    pub dest_path: String,
    pub src_path: String,
    pub class_key: String,
    pub family: String,
    pub copied_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizedMove {
    pub id: i64,
    pub batch_id: i64,
    pub hash: [u8; 32],
    pub from_path: String,
    pub to_path: String,
    pub undone: bool,
}

fn blob_hash(v: Vec<u8>) -> Result<[u8; 32], StoreError> {
    v.try_into().map_err(|_| StoreError::BadHash("hash không đủ 32 byte".into()))
}

impl Store {
    /// Mọi font (theo hash) đang hiện diện, kèm vị trí nên chép từ đó. `library_prefix`: thư mục của thư viện có tổ chức (không dấu `/` cuối).
    pub fn organize_candidates(&self, library_prefix: &str) -> Result<Vec<OrganizeCandidate>, StoreError> {
        let under = |p: &str| p.strip_prefix(library_prefix).is_some_and(|r| r.starts_with('/'));
        let mut st = self.conn.prepare(
            "SELECT fi.file_id, fi.hash, fi.size, l.path, l.cloud_only, f.face_id, f.face_index, f.family_name
               FROM files fi
               JOIN file_locations l ON l.file_id = fi.file_id AND l.missing = 0
               JOIN faces f ON f.file_id = fi.file_id
              ORDER BY fi.file_id, l.cloud_only, l.location_id, f.face_index, f.face_id",
        )?;
        let mut by_file: BTreeMap<i64, OrganizeCandidate> = BTreeMap::new();
        let mut rows = st.query([])?;
        while let Some(r) = rows.next()? {
            let (file_id, hash, size, path, cloud, face_id, family): (i64, Vec<u8>, i64, String, bool, i64, String) =
                (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(7)?);
            let c = match by_file.get_mut(&file_id) {
                Some(c) => c,
                None => by_file.entry(file_id).or_insert(OrganizeCandidate {
                    hash: blob_hash(hash)?,
                    size,
                    src_path: String::new(),
                    cloud_only: true,
                    family,
                    face_ids: Vec::new(),
                    in_library: false,
                }),
            };
            if under(&path) {
                c.in_library = true;
            } else if c.src_path.is_empty() {
                c.src_path = path;
                c.cloud_only = cloud;
            }
            if !c.face_ids.contains(&face_id) {
                c.face_ids.push(face_id);
            }
        }
        // font chỉ có vị trí trong thư viện: vẫn trả về (in_library) nhưng không có nguồn để chép
        Ok(by_file.into_values().collect())
    }

    pub fn organized_files(&self) -> Result<Vec<OrganizedFile>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT hash, dest_path, src_path, class_key, family, copied_at FROM u.organized_files ORDER BY dest_path")?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (h, dest_path, src_path, class_key, family, copied_at) = row?;
            out.push(OrganizedFile { hash: blob_hash(h)?, dest_path, src_path, class_key, family, copied_at });
        }
        Ok(out)
    }

    pub fn organized_file(&self, hash: &[u8; 32]) -> Result<Option<OrganizedFile>, StoreError> {
        Ok(self.organized_files()?.into_iter().find(|f| f.hash == *hash))
    }

    /// Ghi (hoặc thay) bản chép của `hash`.
    pub fn organized_record(&mut self, f: &OrganizedFile) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO u.organized_files(hash, dest_path, src_path, class_key, family, copied_at) VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(hash) DO UPDATE SET dest_path=excluded.dest_path, src_path=excluded.src_path,
               class_key=excluded.class_key, family=excluded.family, copied_at=excluded.copied_at",
            params![&f.hash[..], f.dest_path, f.src_path, f.class_key, f.family, f.copied_at],
        )?;
        Ok(())
    }

    pub fn organized_forget(&mut self, hash: &[u8; 32]) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM u.organized_files WHERE hash=?1", [&hash[..]])?;
        Ok(())
    }

    /// Đường dẫn đích đã bị bản chép nào chiếm chưa?
    pub fn organized_dest_taken(&self, dest: &str) -> Result<bool, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT 1 FROM u.organized_files WHERE dest_path=?1", [dest], |_| Ok(()))
            .optional()?
            .is_some())
    }

    /// Ghi một lần di chuyển ĐÃ xảy ra trên đĩa: cập nhật sổ, vị trí file trong thư viện và nhật ký (một giao dịch mỗi DB; sổ trước).
    pub fn organized_apply_move(&mut self, batch_id: i64, hash: &[u8; 32], from: &str, to: &str, class_key: &str, family: &str) -> Result<(), StoreError> {
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE u.organized_files SET dest_path=?2, class_key=?3, family=?4 WHERE hash=?1",
            params![&hash[..], to, class_key, family],
        )?;
        tx.execute(
            "INSERT INTO u.organized_moves(batch_id, hash, from_path, to_path, moved_at) VALUES (?1,?2,?3,?4,?5)",
            params![batch_id, &hash[..], from, to, now_ms()],
        )?;
        tx.commit()?;
        self.conn.execute("UPDATE file_locations SET path=?2 WHERE path=?1", params![from, to])?;
        Ok(())
    }

    pub fn organized_next_batch(&self) -> Result<i64, StoreError> {
        Ok(self.conn.query_row("SELECT COALESCE(MAX(batch_id), 0) + 1 FROM u.organized_moves", [], |r| r.get(0))?)
    }

    pub fn organized_moves(&self, batch_id: i64) -> Result<Vec<OrganizedMove>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT id, batch_id, hash, from_path, to_path, undone_at IS NOT NULL FROM u.organized_moves WHERE batch_id=?1 ORDER BY id")?;
        let rows = st.query_map([batch_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get::<_, Vec<u8>>(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, batch_id, h, from_path, to_path, undone) = row?;
            out.push(OrganizedMove { id, batch_id, hash: blob_hash(h)?, from_path, to_path, undone });
        }
        Ok(out)
    }

    /// Lô "Sắp xếp lại" mới nhất chưa hoàn tác hết.
    pub fn organized_last_batch(&self) -> Result<Option<i64>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT MAX(batch_id) FROM u.organized_moves WHERE undone_at IS NULL", [], |r| r.get::<_, Option<i64>>(0))?)
    }

    /// Đánh dấu một lần di chuyển đã hoàn tác trên đĩa và đưa sổ + vị trí về chỗ cũ.
    pub fn organized_mark_undone(&mut self, m: &OrganizedMove, class_key: &str, family: &str) -> Result<(), StoreError> {
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("UPDATE u.organized_moves SET undone_at=?2 WHERE id=?1", params![m.id, now_ms()])?;
        tx.execute(
            "UPDATE u.organized_files SET dest_path=?2, class_key=?3, family=?4 WHERE hash=?1",
            params![&m.hash[..], m.from_path, class_key, family],
        )?;
        tx.commit()?;
        self.conn.execute("UPDATE file_locations SET path=?2 WHERE path=?1", params![m.to_path, m.from_path])?;
        Ok(())
    }

    /// Face (theo hash) → key; tiện cho app.
    pub fn face_keys_of_hash(&self, hash: &[u8; 32]) -> Result<Vec<FaceKey>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT f.face_index FROM faces f JOIN files fi ON fi.file_id = f.file_id WHERE fi.hash=?1 ORDER BY f.face_index",
        )?;
        let v = st.query_map([&hash[..]], |r| Ok(FaceKey { hash: *hash, face_index: r.get(0)? }))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(v)
    }
}
