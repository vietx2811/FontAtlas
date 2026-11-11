//! Năm phát hành ước lượng (library v5): tính từ `Years` đã lưu trong bản phân tích, không đọc lại file font.
use fontatlas_core::extras::{ReleaseYearSource, Years, release_year};
use rusqlite::{Connection, params};

use crate::error::StoreError;

/// Năm hiện tại (UTC) để kiểm tra "năm tương lai".
pub(crate) fn current_year() -> u16 {
    // unix → giây từ 1904-01-01 (epoch của `head`)
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    fontatlas_core::extras::year_from_longdatetime(unix + 2_082_844_800).unwrap_or(2026)
}

/// Giá trị cột `(release_year, release_year_src)` cho một `Years`: `none` khi không có nguồn hợp lệ.
pub(crate) fn columns(y: &Years, now_year: u16) -> (Option<i64>, &'static str) {
    match release_year(y, now_year) {
        Some((v, src)) => (Some(i64::from(v)), src.as_str()),
        None => (None, "none"),
    }
}

const BACKFILL_KEY: &str = "release_backfill";
const BATCH: i64 = 2000;

/// Điền `release_year` MỘT LẦN sau khi nâng lên v5, từ bản phân tích local mới nhất có `extras` (không đọc file font).
/// Chạy theo lô `BATCH` face (`face_id` tăng dần), mỗi lô một transaction, đọc thẳng `$.extras.years` bằng `json_extract`
/// nên không nạp `metadata_json` vào RAM. Xong thì ghi cờ `meta.release_backfill` nên các lần mở sau không quét lại; face mà
/// bản phân tích chưa có `extras` (analyzer < core-0.8.0) giữ NULL tới lần phân tích lại (ingest ghi cột). Trả số face đã điền.
pub(crate) fn backfill(conn: &Connection) -> Result<usize, StoreError> {
    let done: Option<i64> = conn.query_row("SELECT value FROM meta WHERE key = ?1", [BACKFILL_KEY], |r| r.get(0)).ok();
    if done.is_some() {
        return Ok(0);
    }
    let now_year = current_year();
    let mut last = 0i64;
    let mut n = 0;
    loop {
        let rows: Vec<(i64, Option<String>)> = {
            let mut st = conn.prepare_cached(
                "SELECT f.face_id, json_extract(a.metadata_json, '$.extras.years')
                   FROM faces f JOIN files fi ON fi.file_id = f.file_id
                   JOIN analyses a ON a.hash = fi.hash AND a.face_index = f.face_index AND a.source = 'local'
                  WHERE f.face_id > ?1 AND f.release_year_src IS NULL
                    AND a.rowid = (SELECT a2.rowid FROM analyses a2 WHERE a2.hash = a.hash AND a2.face_index = a.face_index
                                    AND a2.source = 'local' ORDER BY a2.analyzed_at DESC, a2.rowid DESC LIMIT 1)
                  ORDER BY f.face_id LIMIT ?2",
            )?;
            st.query_map(params![last, BATCH], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?
        };
        let Some(&(max_id, _)) = rows.last() else { break };
        let tx = conn.unchecked_transaction()?;
        {
            let mut up = tx.prepare_cached("UPDATE faces SET release_year = ?2, release_year_src = ?3 WHERE face_id = ?1")?;
            for (id, years) in &rows {
                let Some(years) = years else { continue };
                let y: Years = serde_json::from_str(years).unwrap_or_default();
                let (year, src) = columns(&y, now_year);
                up.execute(params![id, year, src])?;
                n += 1;
            }
        }
        tx.commit()?;
        last = max_id;
    }
    conn.execute("INSERT OR REPLACE INTO meta(key, value) VALUES (?1, 1)", [BACKFILL_KEY])?;
    Ok(n)
}

/// Dùng ở test/DTO: nguồn lưu → enum.
#[allow(dead_code)]
pub(crate) fn parse_src(s: &str) -> Option<ReleaseYearSource> {
    ReleaseYearSource::parse(s)
}

/// Nguồn của ngày tạo file: `cloud_sync` khi file nằm dưới thư mục ổ cloud (birth time có thể là lúc đồng bộ, không phải lúc
/// tạo font); `disk` còn lại.
pub(crate) fn created_src(path: &std::path::Path) -> &'static str {
    if crate::cloud::cloud_provider_of(path).is_some() { "cloud_sync" } else { "disk" }
}
