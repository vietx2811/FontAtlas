//! Độ phủ ngôn ngữ (Hyperglot) theo face: lưu, hàng đợi tính bù, facet cho ô chọn ngôn ngữ.
//! Bảng `face_langs` / `face_lang_state` nằm ở library.db v2 (xem `schema::LIBRARY_V2`).
use std::path::PathBuf;

use rusqlite::{OptionalExtension, ToSql, params};

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::Store;
use crate::query::{FaceQuery, where_clause};

/// Một ngôn ngữ đạt ≥ near của một face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangRow {
    pub lang: String,
    pub have: u32,
    pub total: u32,
    /// "full" | "near"
    pub level: String,
}

#[derive(Debug, Clone)]
pub struct LangPending {
    pub key: FaceKey,
    pub face_id: i64,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LangProgress {
    pub done: u64,
    pub total: u64,
}

/// Số face theo ngôn ngữ trong một tập lọc: `full` = đủ; `near` = đạt ít nhất near (gồm cả full).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangCount {
    pub lang: String,
    pub full: u32,
    pub near: u32,
}

const LIVE: &str = "EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0)";

impl Store {
    /// Face còn có file trên đĩa mà chưa tính với `db_version`, theo thứ tự lưới mặc định (họ, weight).
    /// `limit` = số tối đa trả về.
    pub fn lang_pending(&self, db_version: &str, limit: usize) -> Result<Vec<LangPending>, StoreError> {
        let sql = format!(
            "SELECT fi.hash, f.face_index, f.face_id,
                    (SELECT l.path FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0
                      ORDER BY l.location_id LIMIT 1)
             FROM faces f JOIN files fi ON fi.file_id = f.file_id
             WHERE {LIVE}
               AND NOT EXISTS (SELECT 1 FROM face_lang_state s WHERE s.hash = fi.hash AND s.face_index = f.face_index
                               AND s.db_version = ?1)
             ORDER BY f.family_key, f.weight_class, f.is_italic, f.face_id
             LIMIT ?2"
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(params![db_version, limit as i64], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, u32>(1)?, r.get::<_, i64>(2)?, r.get::<_, Option<String>>(3)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (h, face_index, face_id, path) = r?;
            if let Some(p) = path {
                out.push(LangPending { key: FaceKey { hash: hash_from_blob(h)?, face_index }, face_id, path: PathBuf::from(p) });
            }
        }
        Ok(out)
    }

    /// Ghi kết quả của một face (thay toàn bộ hàng cũ) và đánh dấu đã tính với `db_version`.
    pub fn put_face_langs(&self, key: &FaceKey, db_version: &str, rows: &[LangRow]) -> Result<(), StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM face_langs WHERE hash = ?1 AND face_index = ?2", params![&key.hash[..], key.face_index])?;
        {
            let mut ins = tx.prepare(
                "INSERT INTO face_langs (hash, face_index, lang, have, total, level) VALUES (?1,?2,?3,?4,?5,?6)",
            )?;
            for r in rows {
                ins.execute(params![&key.hash[..], key.face_index, r.lang, r.have, r.total, r.level])?;
            }
        }
        tx.execute(
            "INSERT INTO face_lang_state (hash, face_index, db_version) VALUES (?1,?2,?3)
             ON CONFLICT(hash, face_index) DO UPDATE SET db_version = excluded.db_version",
            params![&key.hash[..], key.face_index, db_version],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Đã tính / tổng số face còn file trên đĩa. Hai phép đếm nhẹ (không JOIN): `done` lấy từ bảng dấu
    /// (hàng mồ côi được `prune_langs` dọn) và kẹp ≤ `total`.
    pub fn lang_progress(&self, db_version: &str) -> Result<LangProgress, StoreError> {
        let total: i64 = self.conn.query_row(&format!("SELECT count(*) FROM faces f WHERE {LIVE}"), [], |r| r.get(0))?;
        let done: i64 =
            self.conn.query_row("SELECT count(*) FROM face_lang_state WHERE db_version = ?1", [db_version], |r| r.get(0))?;
        Ok(LangProgress { done: done.clamp(0, total.max(0)) as u64, total: total.max(0) as u64 })
    }

    /// Facet khi KHÔNG có bộ lọc nào: một lượt quét chỉ mục `face_langs(lang, level)`, không JOIN `faces`.
    /// Đếm cả face đã mất file (khác `lang_counts`, vốn chỉ đếm face còn trên đĩa).
    pub fn lang_counts_unfiltered(&self) -> Result<Vec<LangCount>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT lang, SUM(level = 'full'), COUNT(*) FROM face_langs GROUP BY lang ORDER BY lang")?;
        let rows = st.query_map([], |r| {
            Ok(LangCount { lang: r.get(0)?, full: r.get::<_, i64>(1)? as u32, near: r.get::<_, i64>(2)? as u32 })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Facet: đếm face theo ngôn ngữ trên tập `q` (bỏ điều kiện `langs`), chỉ tính face đã có độ phủ.
    /// `scope` (như `query_faces_scoped`) thu hẹp tập face trước khi đếm.
    pub fn lang_counts(&self, q: &FaceQuery, scope: Option<&[(i64, f32)]>) -> Result<Vec<LangCount>, StoreError> {
        let mut q = q.clone();
        q.langs.clear();
        let (w, p) = where_clause(&q);
        let refs: Vec<&dyn ToSql> = p.iter().map(|v| v as &dyn ToSql).collect();
        let from = if scope.is_some() {
            "faces f JOIN files fi ON fi.file_id = f.file_id JOIN temp.scope sc ON sc.face_id = f.face_id"
        } else {
            "faces f JOIN files fi ON fi.file_id = f.file_id"
        };
        let sql = format!(
            "SELECT fl.lang, SUM(fl.level = 'full'), COUNT(*)
             FROM {from} JOIN face_langs fl ON fl.hash = fi.hash AND fl.face_index = f.face_index
             WHERE {w} GROUP BY fl.lang ORDER BY fl.lang"
        );
        if let Some(sc) = scope {
            self.load_scope(sc)?;
        }
        let out = (|| -> Result<Vec<LangCount>, StoreError> {
            let mut st = self.conn.prepare(&sql)?;
            let rows = st.query_map(refs.as_slice(), |r| {
                Ok(LangCount { lang: r.get(0)?, full: r.get::<_, i64>(1)? as u32, near: r.get::<_, i64>(2)? as u32 })
            })?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })();
        if scope.is_some() {
            self.clear_scope();
        }
        out
    }

    /// Đã có dấu "đã tính" cho face này với `db_version` chưa.
    pub fn lang_state_is(&self, key: &FaceKey, db_version: &str) -> Result<bool, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM face_lang_state WHERE hash = ?1 AND face_index = ?2 AND db_version = ?3",
                params![&key.hash[..], key.face_index, db_version],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// Khoá + đường dẫn (vị trí chưa missing) của một face theo `face_id`.
    pub fn face_file(&self, face_id: i64) -> Result<Option<(FaceKey, PathBuf)>, StoreError> {
        let r = self
            .conn
            .query_row(
                "SELECT fi.hash, f.face_index,
                        (SELECT l.path FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0
                          ORDER BY l.location_id LIMIT 1)
                 FROM faces f JOIN files fi ON fi.file_id = f.file_id WHERE f.face_id = ?1",
                [face_id],
                |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<String>>(2)?)),
            )
            .optional()?;
        match r {
            Some((h, face_index, Some(p))) => Ok(Some((FaceKey { hash: hash_from_blob(h)?, face_index }, PathBuf::from(p)))),
            _ => Ok(None),
        }
    }

    /// Xoá độ phủ của face không còn trong thư viện. Trả số hàng đã xoá.
    pub fn prune_langs(&self) -> Result<usize, StoreError> {
        let a = self.conn.execute(
            "DELETE FROM face_langs WHERE NOT EXISTS (SELECT 1 FROM files fi WHERE fi.hash = face_langs.hash)",
            [],
        )?;
        let b = self.conn.execute(
            "DELETE FROM face_lang_state WHERE NOT EXISTS (SELECT 1 FROM files fi WHERE fi.hash = face_lang_state.hash)",
            [],
        )?;
        Ok(a + b)
    }
}
