//! Hàng đợi embedding trên `pipeline_state` (stage `embedding`), đọc/ghi `embeddings`, cài đặt embedding.
//! `version` của hàng đợi = raw_version (model + runtime + render); whitening không nằm trong hàng đợi.
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};

use crate::error::StoreError;
use crate::ingest::{FaceKey, hash_from_blob, set_state};
use crate::open::{LIBRARY_FILE, Store, now_ms};

pub const SETTING_EMBED_ENABLED: &str = "embed_enabled";
pub const SETTING_LICENSE_NOTICE_ACK: &str = "license_notice_ack";
const STAGE: &str = "embedding";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnqueueStats {
    pub pending: usize,
    pub already_done: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedJob {
    pub key: FaceKey,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobResult {
    Done { raw: Vec<f32>, white: Option<Vec<f32>> },
    Failed { code: &'static str, detail: String },
    Skipped { code: &'static str },
}

fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn from_blob(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn put_vec(c: &Connection, k: &FaceKey, version: &str, v: &[f32], now: i64) -> rusqlite::Result<()> {
    c.execute(
        "INSERT OR REPLACE INTO embeddings(hash,face_index,model_version,source,snapshot_id,dim,dtype,vec,created_at)
         VALUES (?1,?2,?3,'local',NULL,?4,'f32',?5,?6)",
        params![&k.hash[..], k.face_index, version, v.len() as i64, to_blob(v), now],
    )?;
    Ok(())
}

fn count(c: &Connection, version: &str) -> Result<usize, StoreError> {
    let n: i64 = c.query_row(
        "SELECT count(*) FROM embeddings WHERE model_version=?1 AND source='local'",
        [version],
        |r| r.get(0),
    )?;
    Ok(n as usize)
}

fn for_each(c: &Connection, version: &str, f: &mut dyn FnMut(FaceKey, &[f32])) -> Result<(), StoreError> {
    let mut st = c.prepare("SELECT hash, face_index, vec FROM embeddings WHERE model_version=?1 AND source='local'")?;
    let mut rows = st.query([version])?;
    while let Some(r) = rows.next()? {
        let key = FaceKey { hash: hash_from_blob(r.get(0)?)?, face_index: r.get::<_, i64>(1)? as u32 };
        let b: Vec<u8> = r.get(2)?;
        f(key, &from_blob(&b));
    }
    Ok(())
}

fn fetch(c: &Connection, version: &str, keys: &[FaceKey]) -> Result<Vec<Option<Vec<f32>>>, StoreError> {
    let mut st = c.prepare_cached(
        "SELECT vec FROM embeddings WHERE hash=?1 AND face_index=?2 AND model_version=?3 AND source='local'",
    )?;
    keys.iter()
        .map(|k| {
            let b: Option<Vec<u8>> =
                st.query_row(params![&k.hash[..], k.face_index, version], |r| r.get(0)).optional()?;
            Ok(b.map(|b| from_blob(&b)))
        })
        .collect()
}

impl Store {
    pub fn setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        Ok(self.conn.query_row("SELECT value FROM u.settings WHERE key=?1", [key], |r| r.get(0)).optional()?)
    }

    /// API người dùng: ghi `user.db`.
    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute("INSERT OR REPLACE INTO u.settings(key,value) VALUES (?1,?2)", [key, value])?;
        Ok(())
    }

    /// Mặc định bật; chỉ '0' là tắt.
    pub fn embed_enabled(&self) -> Result<bool, StoreError> {
        Ok(self.setting(SETTING_EMBED_ENABLED)?.as_deref() != Some("0"))
    }

    /// Xếp hàng mọi face chưa có vector `raw_version` (không lọc giấy phép). Tắt embedding → không làm gì.
    pub fn enqueue_embeddings(&mut self, raw_version: &str) -> Result<EnqueueStats, StoreError> {
        let mut stats = EnqueueStats::default();
        if !self.embed_enabled()? {
            return Ok(stats);
        }
        let now = now_ms();
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let rows: Vec<(Vec<u8>, i64, bool, Option<String>, Option<String>)> = {
            let mut st = tx.prepare(
                "SELECT fi.hash, fa.face_index,
                        EXISTS(SELECT 1 FROM embeddings e WHERE e.hash=fi.hash AND e.face_index=fa.face_index
                               AND e.model_version=?1 AND e.source='local'),
                        p.status, p.version
                 FROM faces fa JOIN files fi USING(file_id)
                 LEFT JOIN pipeline_state p ON p.hash=fi.hash AND p.face_index=fa.face_index AND p.stage='embedding'",
            )?;
            st.query_map([raw_version], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
                .collect::<Result<_, _>>()?
        };
        for (h, fi, has, status, version) in rows {
            let hash = hash_from_blob(h)?;
            if has {
                stats.already_done += 1;
                continue;
            }
            let same = version.as_deref() == Some(raw_version);
            let keep = same && matches!(status.as_deref(), Some("pending" | "running" | "skipped"));
            if keep {
                stats.pending += (status.as_deref() == Some("pending")) as usize;
                continue;
            }
            set_state(&tx, &hash, fi, STAGE, "pending", raw_version, None, now)?;
            stats.pending += 1;
        }
        tx.commit()?;
        Ok(stats)
    }

    /// Sau khi app tắt đột ngột: mọi dòng `running` về `pending`.
    pub fn reset_running_embeddings(&mut self) -> Result<usize, StoreError> {
        Ok(self.conn.execute(
            "UPDATE pipeline_state SET status='pending', updated_at=?1 WHERE stage='embedding' AND status='running'",
            [now_ms()],
        )?)
    }

    /// Lấy tối đa `n` face `pending` kèm đường dẫn còn tồn tại, đánh `running` (cùng transaction IMMEDIATE).
    pub fn claim_embedding_batch(&mut self, raw_version: &str, n: usize) -> Result<Vec<EmbedJob>, StoreError> {
        let now = now_ms();
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let cands: Vec<(Vec<u8>, i64)> = {
            let mut st = tx.prepare(
                "SELECT hash, face_index FROM pipeline_state
                 WHERE stage='embedding' AND status='pending' AND version=?1 ORDER BY updated_at, hash, face_index",
            )?;
            st.query_map([raw_version], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?
        };
        let mut out = Vec::new();
        for (h, fi) in cands {
            if out.len() >= n {
                break;
            }
            let hash = hash_from_blob(h)?;
            let path: Option<String> = tx
                .query_row(
                    "SELECT l.path FROM files f JOIN file_locations l USING(file_id)
                     WHERE f.hash=?1 AND l.missing=0 ORDER BY l.location_id LIMIT 1",
                    [&hash[..]],
                    |r| r.get(0),
                )
                .optional()?;
            match path {
                Some(p) => {
                    set_state(&tx, &hash, fi, STAGE, "running", raw_version, None, now)?;
                    out.push(EmbedJob { key: FaceKey { hash, face_index: fi as u32 }, path: PathBuf::from(p) });
                }
                None => set_state(&tx, &hash, fi, STAGE, "failed", raw_version, Some(("missing_file", "")), now)?,
            }
        }
        tx.commit()?;
        Ok(out)
    }

    /// Ghi kết quả của một lô. Chỉ ghi khi dòng còn `running`, đúng version và hash còn trong `files`.
    /// Trả số face đã ghi.
    pub fn finish_embeddings(
        &mut self,
        raw_version: &str,
        model_version: &str,
        results: &[(FaceKey, JobResult)],
    ) -> Result<usize, StoreError> {
        let now = now_ms();
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut written = 0;
        for (k, res) in results {
            let ok: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM pipeline_state WHERE hash=?1 AND face_index=?2 AND stage='embedding'
                                AND status='running' AND version=?3)
                    AND EXISTS(SELECT 1 FROM files WHERE hash=?1)",
                params![&k.hash[..], k.face_index, raw_version],
                |r| r.get(0),
            )?;
            if !ok {
                continue;
            }
            let fi = k.face_index as i64;
            match res {
                JobResult::Done { raw, white } => {
                    put_vec(&tx, k, raw_version, raw, now)?;
                    if let Some(w) = white {
                        put_vec(&tx, k, model_version, w, now)?;
                    }
                    set_state(&tx, &k.hash, fi, STAGE, "done", raw_version, None, now)?;
                }
                JobResult::Failed { code, detail } => {
                    set_state(&tx, &k.hash, fi, STAGE, "failed", raw_version, Some((code, detail)), now)?
                }
                JobResult::Skipped { code } => {
                    set_state(&tx, &k.hash, fi, STAGE, "skipped", raw_version, Some((code, "")), now)?
                }
            }
            written += 1;
        }
        tx.commit()?;
        Ok(written)
    }

    /// Huỷ: các face đang `running` của version này về `pending`.
    pub fn release_embeddings(&mut self, raw_version: &str, keys: &[FaceKey]) -> Result<(), StoreError> {
        let now = now_ms();
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        for k in keys {
            tx.execute(
                "UPDATE pipeline_state SET status='pending', updated_at=?4
                 WHERE hash=?1 AND face_index=?2 AND stage='embedding' AND version=?3 AND status='running'",
                params![&k.hash[..], k.face_index, raw_version, now],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Whiten lại từ vector thô (không chạy model) cho mọi face chưa có `model_version`.
    pub fn rewhiten(
        &mut self,
        raw_version: &str,
        model_version: &str,
        apply: &dyn Fn(&[f32]) -> Option<Vec<f32>>,
    ) -> Result<usize, StoreError> {
        let now = now_ms();
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let rows: Vec<(Vec<u8>, i64, Vec<u8>)> = {
            let mut st = tx.prepare(
                "SELECT e.hash, e.face_index, e.vec FROM embeddings e
                 WHERE e.model_version=?1 AND e.source='local'
                   AND NOT EXISTS(SELECT 1 FROM embeddings w WHERE w.hash=e.hash AND w.face_index=e.face_index
                                  AND w.model_version=?2 AND w.source='local')",
            )?;
            st.query_map([raw_version, model_version], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<_, _>>()?
        };
        let mut n = 0;
        for (h, fi, b) in rows {
            if let Some(w) = apply(&from_blob(&b)) {
                let k = FaceKey { hash: hash_from_blob(h)?, face_index: fi as u32 };
                put_vec(&tx, &k, model_version, &w, now)?;
                n += 1;
            }
        }
        tx.commit()?;
        Ok(n)
    }

    /// Xoá vector `local` của mọi version không nằm trong `keep`.
    pub fn gc_embeddings(&mut self, keep: &[&str]) -> Result<usize, StoreError> {
        let marks = vec!["?"; keep.len()].join(",");
        let sql = if keep.is_empty() {
            "DELETE FROM embeddings WHERE source='local'".to_string()
        } else {
            format!("DELETE FROM embeddings WHERE source='local' AND model_version NOT IN ({marks})")
        };
        Ok(self.conn.execute(&sql, rusqlite::params_from_iter(keep.iter()))?)
    }

    pub fn count_embeddings(&self, version: &str) -> Result<usize, StoreError> {
        count(&self.conn, version)
    }

    pub fn for_each_embedding(&self, version: &str, f: &mut dyn FnMut(FaceKey, &[f32])) -> Result<(), StoreError> {
        for_each(&self.conn, version, f)
    }

    pub fn embeddings_f32(&self, version: &str, keys: &[FaceKey]) -> Result<Vec<Option<Vec<f32>>>, StoreError> {
        fetch(&self.conn, version, keys)
    }
}

/// Kết nối chỉ đọc riêng tới `library.db` cho truy vấn (nạp chỉ mục, rerank f32) song song với writer (WAL).
pub struct EmbedReader {
    conn: Connection,
}

impl EmbedReader {
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open_with_flags(
            dir.join(LIBRARY_FILE),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(EmbedReader { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn count_embeddings(&self, version: &str) -> Result<usize, StoreError> {
        count(&self.conn, version)
    }

    pub fn for_each_embedding(&self, version: &str, f: &mut dyn FnMut(FaceKey, &[f32])) -> Result<(), StoreError> {
        for_each(&self.conn, version, f)
    }

    pub fn embeddings_f32(&self, version: &str, keys: &[FaceKey]) -> Result<Vec<Option<Vec<f32>>>, StoreError> {
        fetch(&self.conn, version, keys)
    }
}
