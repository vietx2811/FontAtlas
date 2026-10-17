//! Cache preview trên đĩa: một file SQLite, hai vùng LRU theo byte (chữ mặc định / tuỳ chỉnh).
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{DEFAULT_SAMPLE, PreviewError, RENDERER_VERSION, normalize_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey(pub [u8; 32]);

/// Khoá = BLAKE3(renderer, hash, face, px, chữ mẫu đã chuẩn hoá NFC).
pub fn cache_key(hash_hex: &str, face_index: u32, px: u32, text: &str) -> CacheKey {
    let s = format!("{RENDERER_VERSION}\0{hash_hex}\0{face_index}\0{px}\0{}", normalize_text(text));
    CacheKey(*blake3::hash(s.as_bytes()).as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pool {
    Default,
    Custom,
}
impl Pool {
    fn id(self) -> i64 {
        match self { Pool::Default => 0, Pool::Custom => 1 }
    }
    fn idx(self) -> usize {
        self.id() as usize
    }
}

pub fn pool_for(text: &str) -> Pool {
    if normalize_text(text) == DEFAULT_SAMPLE { Pool::Default } else { Pool::Custom }
}

#[derive(Debug, Clone, Copy)]
pub struct Budgets {
    pub default_bytes: u64,
    pub custom_bytes: u64,
}
impl Budgets {
    pub const DEFAULT: Budgets = Budgets { default_bytes: 64 << 20, custom_bytes: 32 << 20 };
    fn of(&self, p: Pool) -> u64 {
        match p { Pool::Default => self.default_bytes, Pool::Custom => self.custom_bytes }
    }
}

const DDL: &str = "
CREATE TABLE IF NOT EXISTS previews(
  id INTEGER PRIMARY KEY,
  key BLOB NOT NULL UNIQUE,
  pool INTEGER NOT NULL,
  png BLOB NOT NULL,
  bytes INTEGER NOT NULL,
  tick INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS previews_lru ON previews(pool, tick);
";

pub struct PreviewCache {
    conn: Connection,
    budgets: Budgets,
    totals: [u64; 2],
    /// Thứ tự LRU bằng bộ đếm tăng dần (không dùng đồng hồ → test tất định).
    tick: i64,
}

impl PreviewCache {
    pub fn open(path: &Path, budgets: Budgets) -> Result<PreviewCache, PreviewError> {
        let mut c = match Self::try_open(path, budgets) {
            Ok(c) => c,
            Err(_) => {
                // File hỏng: đổi tên để còn chẩn đoán, rồi tạo mới (cache dựng lại được).
                rename_broken(path);
                Self::try_open(path, budgets)?
            }
        };
        for p in [Pool::Default, Pool::Custom] {
            if c.totals[p.idx()] > budgets.of(p) {
                c.evict(p)?;
            }
        }
        Ok(c)
    }

    fn try_open(path: &Path, budgets: Budgets) -> Result<PreviewCache, PreviewError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(DDL)?;
        let mut totals = [0u64; 2];
        {
            let mut st = conn.prepare("SELECT pool, total(bytes) FROM previews GROUP BY pool")?;
            let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)))?;
            for row in rows {
                let (p, t) = row?;
                if (0..2).contains(&p) {
                    totals[p as usize] = t as u64;
                }
            }
        }
        let tick: i64 = conn.query_row("SELECT coalesce(max(tick), 0) FROM previews", [], |r| r.get(0))?;
        Ok(PreviewCache { conn, budgets, totals, tick })
    }

    pub fn get(&mut self, k: &CacheKey) -> Result<Option<Vec<u8>>, PreviewError> {
        let png: Option<Vec<u8>> = self
            .conn
            .query_row("SELECT png FROM previews WHERE key = ?1", params![&k.0[..]], |r| r.get(0))
            .optional()?;
        if png.is_some() {
            self.tick += 1;
            self.conn.execute("UPDATE previews SET tick = ?1 WHERE key = ?2", params![self.tick, &k.0[..]])?;
        }
        Ok(png)
    }

    pub fn put(&mut self, k: &CacheKey, pool: Pool, png: &[u8]) -> Result<(), PreviewError> {
        if png.len() as u64 > self.budgets.of(pool) {
            return Ok(()); // lớn hơn cả ngân sách vùng → không lưu
        }
        self.tick += 1;
        let tx = self.conn.transaction()?;
        let old: Option<(i64, i64)> = tx
            .query_row("SELECT pool, bytes FROM previews WHERE key = ?1", params![&k.0[..]], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        tx.execute(
            "INSERT INTO previews(key, pool, png, bytes, tick) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(key) DO UPDATE SET pool=excluded.pool, png=excluded.png, bytes=excluded.bytes, tick=excluded.tick",
            params![&k.0[..], pool.id(), png, png.len() as i64, self.tick],
        )?;
        tx.commit()?;
        if let Some((op, ob)) = old {
            if (0..2).contains(&op) {
                let t = &mut self.totals[op as usize];
                *t = t.saturating_sub(ob as u64);
            }
        }
        self.totals[pool.idx()] += png.len() as u64;
        if self.totals[pool.idx()] > self.budgets.of(pool) {
            self.evict(pool)?;
        }
        Ok(())
    }

    /// Đẩy cũ nhất của vùng cho tới khi tổng ≤ 90% ngân sách.
    fn evict(&mut self, pool: Pool) -> Result<(), PreviewError> {
        let target = self.budgets.of(pool) * 9 / 10;
        let tx = self.conn.transaction()?;
        'outer: while self.totals[pool.idx()] > target {
            let batch: Vec<(i64, i64)> = {
                let mut st = tx.prepare("SELECT id, bytes FROM previews WHERE pool = ?1 ORDER BY tick LIMIT 64")?;
                let rows = st.query_map(params![pool.id()], |r| Ok((r.get(0)?, r.get(1)?)))?;
                rows.collect::<Result<_, _>>()?
            };
            if batch.is_empty() {
                self.totals[pool.idx()] = 0;
                break;
            }
            for (id, bytes) in batch {
                tx.execute("DELETE FROM previews WHERE id = ?1", params![id])?;
                let t = &mut self.totals[pool.idx()];
                *t = t.saturating_sub(bytes as u64);
                if *t <= target {
                    break 'outer;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn total_bytes(&self, pool: Pool) -> u64 {
        self.totals[pool.idx()]
    }

    pub fn len(&self) -> Result<u64, PreviewError> {
        Ok(self.conn.query_row("SELECT count(*) FROM previews", [], |r| r.get::<_, i64>(0))? as u64)
    }

    pub fn is_empty(&self) -> Result<bool, PreviewError> {
        Ok(self.len()? == 0)
    }
}

/// `previews.db` → `previews.broken-{ms}.db` (kèm -wal, -shm nếu có).
fn rename_broken(path: &Path) {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "previews".into());
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let target = dir.join(format!("{stem}.broken-{ms}.db"));
    let with = |p: &Path, suf: &str| PathBuf::from(format!("{}{suf}", p.display()));
    let _ = std::fs::rename(path, &target);
    for suf in ["-wal", "-shm"] {
        let src = with(path, suf);
        if src.exists() {
            let _ = std::fs::rename(&src, with(&target, suf));
        }
    }
}
