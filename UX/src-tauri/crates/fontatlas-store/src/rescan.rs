//! Quét lại một nguồn theo diff (path, size, mtime) → BLAKE3 → chỉ phân tích hash chưa có kết quả mới.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use fontatlas_core::{ANALYZER_VERSION, analyze_paths};
use rayon::prelude::*;
use rusqlite::params;
use serde::Serialize;

use crate::error::StoreError;
use crate::ingest::{Analysis, IngestItem, ScannedFile, parse_hash};
use crate::open::{Store, now_ms};

pub const FONT_EXTENSIONS: [&str; 6] = ["ttf", "otf", "ttc", "otc", "woff", "woff2"];
pub const INGEST_BATCH: usize = 256;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RescanStatus {
    #[default]
    Ok,
    /// Gốc nguồn không đọc được (ổ ngoài đã rút…): không đánh missing gì.
    SourceUnavailable,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RescanStats {
    pub status: RescanStatus,
    pub seen: usize,
    pub unchanged: usize,
    pub relinked: usize,
    pub analyzed: usize,
    pub failed: usize,
    pub missing: usize,
    pub unreadable: Vec<(String, String)>,
}

fn is_font(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| FONT_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Đi cây thư mục: không theo symlink, bỏ mục ẩn, bỏ thư mục không đọc được; sắp theo đường dẫn.
pub fn walk_fonts(root: &Path) -> Vec<ScannedFile> {
    walk_fonts_report(root).0
}

/// Như `walk_fonts`, kèm các thư mục không đọc được (font bên trong không được coi là đã xoá).
fn walk_fonts_report(root: &Path) -> (Vec<ScannedFile>, Vec<PathBuf>) {
    let mut out = Vec::new();
    let mut failed = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            failed.push(dir);
            continue;
        };
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() && is_font(&p) {
                let Ok(m) = e.metadata() else { continue };
                let mtime_ns = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(0);
                out.push(ScannedFile { path: p, size: m.len(), mtime_ns });
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    (out, failed)
}

fn hash_file(p: &Path) -> std::io::Result<[u8; 32]> {
    let mut h = blake3::Hasher::new();
    h.update_reader(std::fs::File::open(p)?)?;
    Ok(*h.finalize().as_bytes())
}

fn root_available(root: &Path) -> bool {
    root.is_dir() && std::fs::read_dir(root).is_ok()
}

impl Store {
    pub fn rescan_source(&mut self, source_id: i64) -> Result<RescanStats, StoreError> {
        let src = self.source(source_id)?;
        let root = PathBuf::from(
            src.path
                .ok_or_else(|| StoreError::NotFound(format!("nguồn #{source_id} không có thư mục")))?,
        );
        let mut stats = RescanStats::default();
        if !root_available(&root) {
            stats.status = RescanStatus::SourceUnavailable;
            return Ok(stats);
        }
        let started = now_ms();
        let generation: i64 = self.conn.query_row(
            "UPDATE meta SET value = value + 1 WHERE key='scan_generation' RETURNING value",
            [],
            |r| r.get(0),
        )?;
        let (files, failed_dirs) = walk_fonts_report(&root);
        stats.seen = files.len();

        // vị trí đã biết của nguồn: path → (size, mtime, có kết quả mới nhất)
        let mut known: HashMap<String, (i64, i64, bool)> = HashMap::new();
        {
            let mut st = self.conn.prepare(
                "SELECT l.path, l.size, l.mtime_ns,
                        COALESCE(p.status IN ('done','failed') AND p.version = ?2, 0)
                 FROM file_locations l JOIN files fi USING(file_id)
                 LEFT JOIN pipeline_state p ON p.hash = fi.hash AND p.face_index = -1 AND p.stage = 'load'
                 WHERE l.source_id = ?1",
            )?;
            let rows = st.query_map(params![source_id, ANALYZER_VERSION], |r| {
                Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?, r.get(3)?)))
            })?;
            for row in rows {
                let (p, v) = row?;
                known.insert(p, v);
            }
        }
        let (unchanged, changed): (Vec<_>, Vec<_>) = files.into_iter().partition(|f| {
            known
                .get(f.path.to_string_lossy().as_ref())
                .is_some_and(|&(size, mtime, fresh)| fresh && size == f.size as i64 && mtime == f.mtime_ns)
        });
        {
            let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            for f in &unchanged {
                tx.execute(
                    "UPDATE file_locations SET last_seen_at=?2, scan_gen=?3, missing=0 WHERE path=?1",
                    params![f.path.to_string_lossy(), started, generation],
                )?;
            }
            tx.commit()?;
        }
        stats.unchanged = unchanged.len();

        // băm song song phần đổi; hash đã có kết quả mới nhất → chỉ nối vị trí
        let hashed: Vec<(ScannedFile, std::io::Result<[u8; 32]>)> =
            changed.into_par_iter().map(|f| {
                let h = hash_file(&f.path);
                (f, h)
            }).collect();
        let mut items: Vec<IngestItem> = Vec::new();
        let mut unreadable_known: Vec<String> = Vec::new();
        let mut to_analyze: Vec<(ScannedFile, [u8; 32])> = Vec::new();
        {
            let mut fresh = self.conn.prepare(
                "SELECT count(*) FROM pipeline_state WHERE hash=?1 AND face_index=-1 AND stage='load'
                   AND status IN ('done','failed') AND version=?2",
            )?;
            for (f, h) in hashed {
                match h {
                    Err(e) => {
                        let p = f.path.to_string_lossy().to_string();
                        if known.contains_key(&p) {
                            unreadable_known.push(p.clone());
                        }
                        stats.unreadable.push((p, format!("io: {}", e.kind())));
                    }
                    Ok(h) => {
                        let n: i64 = fresh.query_row(params![&h[..], ANALYZER_VERSION], |r| r.get(0))?;
                        if n > 0 {
                            items.push(IngestItem { source_id, file: f, hash: h, analysis: Analysis::Known });
                        } else {
                            to_analyze.push((f, h));
                        }
                    }
                }
            }
        }
        let paths: Vec<PathBuf> = to_analyze.iter().map(|(f, _)| f.path.clone()).collect();
        for ((f, h), (_, res)) in to_analyze.into_iter().zip(analyze_paths(&paths)) {
            let item = match res {
                // hash của report là chuẩn (file có thể đổi giữa lúc băm và lúc phân tích)
                Ok(r) => IngestItem { source_id, file: f, hash: parse_hash(&r.hash_hex)?, analysis: Analysis::Report(r) },
                Err(e) => IngestItem {
                    source_id,
                    file: f,
                    hash: h,
                    analysis: Analysis::Failed { code: e.code().to_string(), detail: e.to_string() },
                },
            };
            items.push(item);
        }
        let mut batch = Vec::with_capacity(INGEST_BATCH);
        for it in items {
            batch.push(it);
            if batch.len() == INGEST_BATCH {
                self.ingest_counting(std::mem::take(&mut batch), started, &mut stats)?;
            }
        }
        if !batch.is_empty() {
            self.ingest_counting(batch, started, &mut stats)?;
        }

        // file đã biết nhưng không đọc được lần này: vẫn coi là còn
        for p in &unreadable_known {
            self.conn.execute(
                "UPDATE file_locations SET scan_gen=?2 WHERE path=?1",
                params![p, generation],
            )?;
        }
        // ổ rút giữa lúc quét → không đánh missing gì
        if !root_available(&root) {
            stats.status = RescanStatus::SourceUnavailable;
            return Ok(stats);
        }
        let stale: Vec<(i64, String)> = {
            let mut st = self.conn.prepare(
                "SELECT location_id, path FROM file_locations WHERE source_id=?1 AND missing=0 AND scan_gen < ?2",
            )?;
            st.query_map(params![source_id, generation], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for (id, p) in stale {
            if failed_dirs.iter().any(|d| Path::new(&p).starts_with(d)) {
                continue; // nằm trong thư mục không đọc được: không biết đã xoá hay chưa
            }
            stats.missing += tx.execute("UPDATE file_locations SET missing=1 WHERE location_id=?1", [id])?;
        }
        tx.commit()?;
        Ok(stats)
    }

    fn ingest_counting(&mut self, items: Vec<IngestItem>, at: i64, stats: &mut RescanStats) -> Result<(), StoreError> {
        let analyzed = items.iter().filter(|i| matches!(i.analysis, Analysis::Report(_))).count();
        let sum = self.ingest(items, at)?;
        stats.analyzed += analyzed;
        stats.failed += sum.failed;
        stats.relinked += sum.relinked;
        Ok(())
    }

    /// Xoá nguồn (thao tác người dùng): vị trí của nguồn, bản ghi `u.sources`, rồi dọn rác.
    pub fn remove_source(&mut self, source_id: i64) -> Result<(), StoreError> {
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM file_locations WHERE source_id=?1", [source_id])?;
        tx.execute("DELETE FROM u.sources WHERE source_id=?1", [source_id])?;
        tx.commit()?;
        self.gc()?;
        Ok(())
    }

    /// Xoá file không còn vị trí nào (vị trí `missing` vẫn giữ file). Không đụng `u`.
    pub fn gc(&mut self) -> Result<usize, StoreError> {
        let tx = self.conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let n = tx.execute(
            "DELETE FROM files WHERE NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = files.file_id)",
            [],
        )?;
        // dữ liệu dẫn xuất khoá theo hash (không có FK): xoá cái mồ côi; dữ liệu cộng đồng giữ lại
        for t in ["analyses", "embeddings"] {
            tx.execute(
                &format!("DELETE FROM {t} WHERE source <> 'community' AND hash NOT IN (SELECT hash FROM files)"),
                [],
            )?;
        }
        tx.execute("DELETE FROM pipeline_state WHERE hash NOT IN (SELECT hash FROM files)", [])?;
        tx.commit()?;
        Ok(n)
    }
}
