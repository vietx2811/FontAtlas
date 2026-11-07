//! Đọc bản phân tích đã lưu, vị trí file, xem trước DB cũ, sao lưu user.db theo yêu cầu.
use std::collections::HashMap;
use std::path::PathBuf;

use fontatlas_core::ANALYZER_VERSION;
use rusqlite::{OptionalExtension, params};

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{BACKUP_DIR, Store, now_ms};

/// Một (face, đường dẫn) cho việc nhóm trùng lặp.
#[derive(Debug, Clone)]
pub struct DupRow {
    pub face_id: i64,
    pub key: FaceKey,
    pub family: String,
    pub subfamily: Option<String>,
    pub postscript: Option<String>,
    pub weight: Option<u16>,
    pub italic: bool,
    /// sfnt | collection | woff1 | woff2
    pub container: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct StoredAnalysis {
    pub face_id: i64,
    pub key: FaceKey,
    pub metadata_json: String,
    pub vietnamese_json: String,
    pub anatomy_json: String,
    pub missing: bool,
    pub source_ids: Vec<i64>,
    pub container: Option<String>,
    pub file_size: u64,
    pub analyzer_version: String,
}

/// Bản local mới nhất của mỗi face (bất kể analyzer_version), kèm tình trạng file.
const LATEST: &str = "
SELECT f.face_id, fi.hash, f.face_index, a.metadata_json, a.vietnamese_json, a.anatomy_json,
       a.analyzer_version, fi.container, fi.size, f.file_id,
       NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0)
FROM faces f
JOIN files fi ON fi.file_id = f.file_id
JOIN analyses a ON a.hash = fi.hash AND a.face_index = f.face_index AND a.source = 'local'
WHERE a.rowid = (SELECT a2.rowid FROM analyses a2
                 WHERE a2.hash = a.hash AND a2.face_index = a.face_index AND a2.source = 'local'
                 ORDER BY a2.analyzed_at DESC, a2.rowid DESC LIMIT 1)";

impl Store {
    fn latest_rows(&self, filter: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<(StoredAnalysis, i64)>, StoreError> {
        let mut st = self.conn.prepare(&format!("{LATEST} {filter}"))?;
        let mut rows = st.query(args)?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            let hash = hash_from_blob(r.get(1)?)?;
            out.push((
                StoredAnalysis {
                    face_id: r.get(0)?,
                    key: FaceKey { hash, face_index: r.get::<_, i64>(2)? as u32 },
                    metadata_json: r.get(3)?,
                    vietnamese_json: r.get(4)?,
                    anatomy_json: r.get(5)?,
                    analyzer_version: r.get(6)?,
                    container: r.get(7)?,
                    file_size: r.get::<_, i64>(8)? as u64,
                    missing: r.get(10)?,
                    source_ids: Vec::new(),
                },
                r.get(9)?,
            ));
        }
        Ok(out)
    }

    fn sources_by_file(&self) -> Result<HashMap<i64, Vec<i64>>, StoreError> {
        let mut m: HashMap<i64, Vec<i64>> = HashMap::new();
        let mut st = self.conn.prepare("SELECT DISTINCT file_id, source_id FROM file_locations ORDER BY file_id, source_id")?;
        for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))? {
            let (f, s) = r?;
            m.entry(f).or_default().push(s);
        }
        Ok(m)
    }

    /// Mỗi face một dòng: bản phân tích local MỚI NHẤT có sẵn, bất kể analyzer_version
    /// (Index vẫn dùng được sau khi đổi ANALYZER_VERSION, cho tới khi phân tích lại xong).
    pub fn for_each_latest_analysis(&self, f: &mut dyn FnMut(StoredAnalysis)) -> Result<usize, StoreError> {
        let srcs = self.sources_by_file()?;
        let rows = self.latest_rows("", &[])?;
        let n = rows.len();
        for (mut a, file_id) in rows {
            a.source_ids = srcs.get(&file_id).cloned().unwrap_or_default();
            f(a);
        }
        Ok(n)
    }

    /// Số face có bản phân tích mới nhất khác ANALYZER_VERSION (0 = đã phân tích lại xong).
    pub fn stale_analysis_count(&self) -> Result<u64, StoreError> {
        let rows = self.latest_rows("AND a.analyzer_version <> ?1", &[&ANALYZER_VERSION])?;
        Ok(rows.len() as u64)
    }

    pub fn analysis_of(&self, key: &FaceKey) -> Result<Option<StoredAnalysis>, StoreError> {
        let srcs = self.sources_by_file()?;
        let rows = self.latest_rows("AND fi.hash = ?1 AND f.face_index = ?2", &[&&key.hash[..], &key.face_index])?;
        Ok(rows.into_iter().next().map(|(mut a, file_id)| {
            a.source_ids = srcs.get(&file_id).cloned().unwrap_or_default();
            a
        }))
    }

    /// `face_id` của mọi face trong thư viện (kể cả chưa có bản phân tích).
    pub fn all_face_ids(&self) -> Result<Vec<i64>, StoreError> {
        let mut st = self.conn.prepare("SELECT face_id FROM faces")?;
        Ok(st.query_map([], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Tra face_id theo khoá (bỏ khoá không có).
    pub fn face_ids_of(&self, keys: &[FaceKey]) -> Result<Vec<(FaceKey, i64)>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT f.face_id FROM faces f JOIN files fi ON fi.file_id = f.file_id WHERE fi.hash = ?1 AND f.face_index = ?2",
        )?;
        let mut out = Vec::new();
        for k in keys {
            if let Some(id) = st.query_row(params![&k.hash[..], k.face_index], |r| r.get(0)).optional()? {
                out.push((*k, id));
            }
        }
        Ok(out)
    }

    /// `face_id` của mọi face đang nằm trong collection (cho lọc ứng viên Pairing theo collection).
    pub fn collection_face_ids(&self, collection_id: i64) -> Result<Vec<i64>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT f.face_id FROM faces f JOIN files fi ON fi.file_id = f.file_id
             JOIN u.collection_items ci ON ci.hash = fi.hash AND ci.face_index = f.face_index
             WHERE ci.collection_id = ?1",
        )?;
        let ids = st.query_map([collection_id], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(ids)
    }

    /// Mọi (face, vị trí chưa missing) của thư viện, để nhóm trùng lặp (cùng nội dung ở nhiều đường dẫn,
    /// cùng PostScript name khác hash, cùng family+style khác định dạng). Không có face nào chỉ còn vị trí missing.
    pub fn duplicate_rows(&self) -> Result<Vec<DupRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT f.face_id, fi.hash, f.face_index, f.family_name, f.subfamily, f.postscript_name,
                    f.weight_class, f.is_italic, fi.container, l.path
             FROM faces f
             JOIN files fi ON fi.file_id = f.file_id
             JOIN file_locations l ON l.file_id = fi.file_id AND l.missing = 0
             ORDER BY f.face_id, l.location_id",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, u32>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<u16>>(6)?,
                r.get::<_, bool>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, String>(9)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (face_id, hash, face_index, family, subfamily, postscript, weight, italic, container, path) = r?;
            let Ok(hash) = <[u8; 32]>::try_from(hash.as_slice()) else { continue };
            out.push(DupRow { face_id, key: FaceKey { hash, face_index }, family, subfamily, postscript, weight, italic, container, path });
        }
        Ok(out)
    }

    /// Vị trí chưa missing có location_id nhỏ nhất.
    pub fn face_path(&self, key: &FaceKey) -> Result<Option<PathBuf>, StoreError> {
        let p: Option<String> = self
            .conn
            .query_row(
                "SELECT l.path FROM file_locations l JOIN files fi ON fi.file_id = l.file_id
                 WHERE fi.hash = ?1 AND l.missing = 0 ORDER BY l.location_id LIMIT 1",
                [&key.hash[..]],
                |r| r.get(0),
            )
            .optional()?;
        Ok(p.map(PathBuf::from))
    }

    /// Sao lưu user.db (VACUUM INTO) vào `<dir>/backups/user-<ms>.db`; trả đường dẫn.
    pub fn backup_user_db(&self) -> Result<PathBuf, StoreError> {
        let dir = self.dir().join(BACKUP_DIR);
        std::fs::create_dir_all(&dir)?;
        let p = dir.join(format!("user-{}.db", now_ms()));
        self.conn.execute("VACUUM u INTO ?1", [p.to_string_lossy()])?;
        Ok(p)
    }
}
