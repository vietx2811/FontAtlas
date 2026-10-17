//! Ghi kết quả phân tích của fontatlas-core vào library.db.
use std::path::PathBuf;

use fontatlas_core::{ANALYZER_VERSION, FaceReport, FileReport};
use rusqlite::{OptionalExtension, Transaction, params};
use serde::Serialize;

use crate::error::StoreError;
use crate::fold::vi_fold;
use crate::open::Store;

/// Danh tính một face: (BLAKE3 của file gốc, chỉ số face).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceKey {
    pub hash: [u8; 32],
    pub face_index: u32,
}

impl FaceKey {
    pub fn from_hex(hex: &str, face_index: u32) -> Result<FaceKey, StoreError> {
        Ok(FaceKey { hash: parse_hash(hex)?, face_index })
    }
    pub fn hash_hex(&self) -> String {
        hex_of(&self.hash)
    }
}

pub(crate) fn parse_hash(hex: &str) -> Result<[u8; 32], StoreError> {
    blake3::Hash::from_hex(hex)
        .map(|h| *h.as_bytes())
        .map_err(|_| StoreError::BadHash(hex.chars().take(80).collect()))
}

pub(crate) fn hex_of(hash: &[u8; 32]) -> String {
    blake3::Hash::from_bytes(*hash).to_hex().to_string()
}

/// Blob 32 byte đọc từ DB → mảng; sai độ dài thì lỗi chứ không panic.
pub(crate) fn hash_from_blob(b: Vec<u8>) -> Result<[u8; 32], StoreError> {
    b.as_slice().try_into().map_err(|_| StoreError::BadHash(format!("blob {} byte", b.len())))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub size: u64,
    pub mtime_ns: i64,
}

pub enum Analysis {
    Report(FileReport),
    Failed { code: String, detail: String },
    /// Hash đã có kết quả mới nhất: chỉ ghi vị trí.
    Known,
}

pub struct IngestItem {
    pub source_id: i64,
    pub file: ScannedFile,
    pub hash: [u8; 32],
    pub analysis: Analysis,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IngestSummary {
    pub faces_written: usize,
    pub failed: usize,
    pub relinked: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemFile {
    pub hash_hex: String,
    pub face_index: Option<u32>,
    pub stage: String,
    pub code: String,
    pub detail: Option<String>,
    pub attempts: u32,
    pub path: Option<String>,
}

/// Tên snake_case của enum core (trùng tên serde).
fn enum_str<T: Serialize>(x: &T) -> String {
    match serde_json::to_value(x) {
        Ok(serde_json::Value::String(s)) => s,
        _ => "unknown".to_string(),
    }
}

fn json<T: Serialize>(x: &T) -> String {
    // Kiểu thuần dữ liệu: không thể lỗi; vẫn tránh unwrap.
    serde_json::to_string(x).unwrap_or_else(|_| "null".to_string())
}

struct FaceCols {
    family: String,
    subfamily: Option<String>,
    is_italic: bool,
    family_key: String,
    search_text: String,
}

fn face_cols(f: &FaceReport) -> FaceCols {
    let m = &f.metadata;
    let nonempty = |o: &Option<String>| o.as_ref().filter(|s| !s.trim().is_empty()).cloned();
    let family = nonempty(&m.typographic_family)
        .or_else(|| nonempty(&m.family))
        .or_else(|| nonempty(&m.postscript_name))
        .unwrap_or_else(|| "(không tên)".to_string());
    let subfamily = nonempty(&m.typographic_subfamily).or_else(|| nonempty(&m.subfamily));
    let sub_key = subfamily.as_deref().map(vi_fold).unwrap_or_default();
    let is_italic = m.italic_angle.is_some_and(|a| a.abs() >= 0.5)
        || sub_key.contains("italic")
        || sub_key.contains("oblique");
    let mut parts: Vec<String> = Vec::new();
    for v in [
        &m.typographic_family,
        &m.family,
        &m.typographic_subfamily,
        &m.subfamily,
        &m.full_name,
        &m.postscript_name,
        &m.manufacturer,
        &m.designer,
    ]
    .into_iter()
    .flatten()
    {
        let k = vi_fold(v);
        if !k.is_empty() && !parts.contains(&k) {
            parts.push(k);
        }
    }
    FaceCols { family_key: vi_fold(&family), family, subfamily, is_italic, search_text: parts.join(" ") }
}

/// Ghi trạng thái pipeline; `attempts` tăng khi thất bại, về 0 khi khác.
#[allow(clippy::too_many_arguments)] // ánh xạ thẳng các cột của pipeline_state
pub(crate) fn set_state(
    tx: &Transaction,
    hash: &[u8; 32],
    face_index: i64,
    stage: &str,
    status: &str,
    version: &str,
    err: Option<(&str, &str)>,
    now: i64,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO pipeline_state(hash,face_index,stage,status,version,error_code,error_detail,attempts,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,CASE WHEN ?4='failed' THEN 1 ELSE 0 END,?8)
         ON CONFLICT(hash,face_index,stage) DO UPDATE SET status=excluded.status, version=excluded.version,
           error_code=excluded.error_code, error_detail=excluded.error_detail,
           attempts=CASE WHEN excluded.status='failed' THEN pipeline_state.attempts+1 ELSE 0 END,
           updated_at=excluded.updated_at",
        params![&hash[..], face_index, stage, status, version, err.map(|e| e.0), err.map(|e| e.1), now],
    )?;
    Ok(())
}

fn write_report(tx: &Transaction, file_id: i64, hash: &[u8; 32], r: &FileReport, now: i64) -> rusqlite::Result<usize> {
    tx.execute(
        "UPDATE files SET container=?1, num_faces=?2 WHERE file_id=?3",
        params![enum_str(&r.container), r.faces.len() as i64, file_id],
    )?;
    for f in &r.faces {
        let c = face_cols(f);
        let m = &f.metadata;
        let v = &f.vietnamese;
        tx.execute(
            "INSERT INTO faces(file_id,face_index,family_name,subfamily,full_name,postscript_name,manufacturer,designer,
                weight_class,width_class,is_italic,is_variable,outline_format,license_class,vi_status,supports_vi,
                vi_missing,num_glyphs,family_key,search_text)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)
             ON CONFLICT(file_id,face_index) DO UPDATE SET family_name=excluded.family_name, subfamily=excluded.subfamily,
                full_name=excluded.full_name, postscript_name=excluded.postscript_name, manufacturer=excluded.manufacturer,
                designer=excluded.designer, weight_class=excluded.weight_class, width_class=excluded.width_class,
                is_italic=excluded.is_italic, is_variable=excluded.is_variable, outline_format=excluded.outline_format,
                license_class=excluded.license_class, vi_status=excluded.vi_status, supports_vi=excluded.supports_vi,
                vi_missing=excluded.vi_missing, num_glyphs=excluded.num_glyphs, family_key=excluded.family_key,
                search_text=excluded.search_text",
            params![
                file_id,
                f.index as i64,
                c.family,
                c.subfamily,
                m.full_name,
                m.postscript_name,
                m.manufacturer,
                m.designer,
                m.weight_class,
                m.width_class,
                c.is_italic,
                !m.axes.is_empty(),
                enum_str(&m.outline_format),
                enum_str(&m.license_class),
                enum_str(&v.status),
                v.supports_vietnamese,
                v.missing.len() as i64,
                m.num_glyphs,
                c.family_key,
                c.search_text,
            ],
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO analyses(hash,face_index,analyzer_version,source,snapshot_id,metadata_json,
                vietnamese_json,anatomy_json,analyzed_at) VALUES (?1,?2,?3,'local',NULL,?4,?5,?6,?7)",
            params![&hash[..], f.index as i64, r.analyzer_version, json(m), json(v), json(&f.anatomy), now],
        )?;
        tx.execute(
            "DELETE FROM analyses WHERE hash=?1 AND face_index=?2 AND source='local' AND analyzer_version<>?3",
            params![&hash[..], f.index as i64, r.analyzer_version],
        )?;
        set_state(tx, hash, f.index as i64, "analysis", "done", r.analyzer_version, None, now)?;
    }
    tx.execute(
        "DELETE FROM faces WHERE file_id=?1 AND face_index >= ?2",
        params![file_id, r.faces.len() as i64],
    )?;
    set_state(tx, hash, -1, "load", "done", r.analyzer_version, None, now)?;
    Ok(r.faces.len())
}

impl Store {
    /// Ghi một lô trong MỘT transaction; lỗi bất kỳ → không ghi gì của lô.
    pub fn ingest(&mut self, items: Vec<IngestItem>, seen_at: i64) -> Result<IngestSummary, StoreError> {
        let tx = self.conn.transaction()?;
        let generation: i64 =
            tx.query_row("SELECT value FROM meta WHERE key='scan_generation'", [], |r| r.get(0))?;
        let mut sum = IngestSummary::default();
        for it in items {
            let h = &it.hash[..];
            if let Analysis::Report(r) = &it.analysis {
                if parse_hash(&r.hash_hex)? != it.hash {
                    return Err(StoreError::BadHash(format!("report {} ≠ {}", r.hash_hex, hex_of(&it.hash))));
                }
            }
            if !matches!(it.analysis, Analysis::Known) {
                tx.execute(
                    "INSERT INTO files(hash,size,first_seen_at) VALUES (?1,?2,?3) ON CONFLICT(hash) DO NOTHING",
                    params![h, it.file.size as i64, seen_at],
                )?;
            }
            let file_id: i64 = tx
                .query_row("SELECT file_id FROM files WHERE hash=?1", [h], |r| r.get(0))
                .optional()?
                .ok_or_else(|| StoreError::NotFound(format!("file {}", hex_of(&it.hash))))?;
            tx.execute(
                "INSERT INTO file_locations(file_id,source_id,path,size,mtime_ns,last_seen_at,missing,scan_gen)
                 VALUES (?1,?2,?3,?4,?5,?6,0,?7)
                 ON CONFLICT(path) DO UPDATE SET file_id=excluded.file_id, source_id=excluded.source_id,
                   size=excluded.size, mtime_ns=excluded.mtime_ns, last_seen_at=excluded.last_seen_at,
                   missing=0, scan_gen=excluded.scan_gen",
                params![
                    file_id,
                    it.source_id,
                    it.file.path.to_string_lossy(),
                    it.file.size as i64,
                    it.file.mtime_ns,
                    seen_at,
                    generation
                ],
            )?;
            match &it.analysis {
                Analysis::Report(r) => sum.faces_written += write_report(&tx, file_id, &it.hash, r, seen_at)?,
                Analysis::Failed { code, detail } => {
                    // lỗi đọc file (ổ rút giữa chừng, EIO) là tạm thời: để pending cho lần quét sau thử lại
                    let status = if code == "io" { "pending" } else { "failed" };
                    set_state(&tx, &it.hash, -1, "load", status, ANALYZER_VERSION, Some((code, detail)), seen_at)?;
                    sum.failed += 1;
                }
                Analysis::Known => sum.relinked += 1,
            }
        }
        tx.commit()?;
        Ok(sum)
    }

    /// File/face lỗi để UI hiện bảng "Font có vấn đề".
    pub fn problem_files(&self) -> Result<Vec<ProblemFile>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT p.hash, p.face_index, p.stage, p.error_code, p.error_detail, p.attempts,
                    (SELECT l.path FROM files fi JOIN file_locations l USING(file_id)
                      WHERE fi.hash = p.hash ORDER BY l.missing, l.location_id LIMIT 1)
             FROM pipeline_state p WHERE p.status = 'failed' ORDER BY p.updated_at DESC",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (h, fi, stage, code, detail, attempts, path) = row?;
            out.push(ProblemFile {
                hash_hex: hex_of(&hash_from_blob(h)?),
                face_index: u32::try_from(fi).ok(),
                stage,
                code: code.unwrap_or_default(),
                detail,
                attempts: attempts.max(0) as u32,
                path,
            });
        }
        Ok(out)
    }

    /// Đưa mọi mục lỗi về `pending` để lần quét sau chạy lại.
    pub fn retry_failed(&mut self) -> Result<usize, StoreError> {
        Ok(self.conn.execute("UPDATE pipeline_state SET status='pending' WHERE status='failed'", [])?)
    }
}
