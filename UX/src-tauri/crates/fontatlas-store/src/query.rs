//! Truy vấn lưới thư viện: tìm, lọc, sắp xếp, phân trang phía backend.
use rusqlite::ToSql;
use rusqlite::types::Value;
use serde::{Deserialize, Serialize};

use crate::error::StoreError;
use crate::fold::{TextMatch, text_match};
use crate::ingest::{hash_from_blob, hex_of};
use crate::open::Store;

pub const DEFAULT_LIMIT: u32 = 100;
pub const MAX_LIMIT: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    #[default]
    Family,
    Weight,
    Added,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViFilter {
    /// vi_status = full
    Full,
    /// có đủ chữ dựng sẵn (supports_vi)
    Supported,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FaceQuery {
    pub text: Option<String>,
    pub vi: Option<ViFilter>,
    pub license: Vec<String>,
    pub variable: Option<bool>,
    pub italic: Option<bool>,
    pub source_ids: Vec<i64>,
    pub favorites_only: bool,
    pub collection_id: Option<i64>,
    pub tag_ids: Vec<i64>,
    pub include_missing: bool,
    pub sort: SortKey,
    pub descending: bool,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceRow {
    pub face_id: i64,
    pub hash_hex: String,
    pub face_index: u32,
    pub family: String,
    pub subfamily: Option<String>,
    pub full_name: Option<String>,
    pub weight_class: Option<u16>,
    pub width_class: Option<u16>,
    pub is_italic: bool,
    pub is_variable: bool,
    pub vi_status: String,
    pub supports_vi: bool,
    pub vi_missing: u32,
    pub license_class: String,
    pub path: Option<String>,
    /// Kích thước file gốc (byte) — cho ngân sách nạp FontFace.
    pub file_size: u64,
    pub missing: bool,
    pub is_favorite: bool,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacePage {
    pub total: u64,
    pub offset: u32,
    pub items: Vec<FaceRow>,
}

/// Điều kiện WHERE + tham số theo đúng thứ tự `?`.
fn where_clause(q: &FaceQuery) -> (String, Vec<Value>) {
    let mut w: Vec<String> = vec!["1=1".into()];
    let mut p: Vec<Value> = Vec::new();
    let in_list = |n: usize| vec!["?"; n].join(",");
    match q.text.as_deref().and_then(text_match) {
        Some(TextMatch::Fts(m)) => {
            w.push("f.face_id IN (SELECT rowid FROM faces_fts WHERE faces_fts MATCH ?)".into());
            p.push(Value::Text(m));
        }
        Some(TextMatch::Prefix { start, word }) => {
            w.push("(f.search_text LIKE ? ESCAPE '\\' OR f.search_text LIKE ? ESCAPE '\\')".into());
            p.push(Value::Text(start));
            p.push(Value::Text(word));
        }
        None => {}
    }
    match q.vi {
        Some(ViFilter::Full) => w.push("f.vi_status = 'full'".into()),
        Some(ViFilter::Supported) => w.push("f.supports_vi = 1".into()),
        None => {}
    }
    if !q.license.is_empty() {
        w.push(format!("f.license_class IN ({})", in_list(q.license.len())));
        p.extend(q.license.iter().map(|l| Value::Text(l.clone())));
    }
    if let Some(v) = q.variable {
        w.push("f.is_variable = ?".into());
        p.push(Value::Integer(v as i64));
    }
    if let Some(v) = q.italic {
        w.push("f.is_italic = ?".into());
        p.push(Value::Integer(v as i64));
    }
    if !q.source_ids.is_empty() {
        w.push(format!(
            "EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.source_id IN ({}))",
            in_list(q.source_ids.len())
        ));
        p.extend(q.source_ids.iter().map(|&i| Value::Integer(i)));
    }
    if q.favorites_only {
        w.push("EXISTS (SELECT 1 FROM u.favorites fv WHERE fv.hash = fi.hash AND fv.face_index = f.face_index)".into());
    }
    if let Some(c) = q.collection_id {
        w.push(
            "EXISTS (SELECT 1 FROM u.collection_items ci WHERE ci.collection_id = ? AND ci.hash = fi.hash \
             AND ci.face_index = f.face_index)"
                .into(),
        );
        p.push(Value::Integer(c));
    }
    for &t in &q.tag_ids {
        w.push(
            "EXISTS (SELECT 1 FROM u.face_tags ft WHERE ft.tag_id = ? AND ft.hash = fi.hash \
             AND ft.face_index = f.face_index)"
                .into(),
        );
        p.push(Value::Integer(t));
    }
    if !q.include_missing {
        w.push("EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0)".into());
    }
    (w.join(" AND "), p)
}

fn order_by(q: &FaceQuery) -> String {
    let dir = if q.descending { "DESC" } else { "ASC" };
    match q.sort {
        SortKey::Family => format!("f.family_key {dir}, f.weight_class, f.is_italic, f.face_id"),
        SortKey::Weight => format!("f.weight_class {dir}, f.family_key, f.face_id"),
        SortKey::Added => format!("fi.first_seen_at {dir}, f.face_id"),
    }
}

impl Store {
    pub fn query_faces(&self, q: &FaceQuery) -> Result<FacePage, StoreError> {
        let (w, p) = where_clause(q);
        let refs: Vec<&dyn ToSql> = p.iter().map(|v| v as &dyn ToSql).collect();
        let total: i64 = self.conn.query_row(
            &format!("SELECT count(*) FROM faces f JOIN files fi ON fi.file_id = f.file_id WHERE {w}"),
            refs.as_slice(),
            |r| r.get(0),
        )?;
        let limit = match q.limit {
            0 => DEFAULT_LIMIT,
            n => n.min(MAX_LIMIT),
        };
        let sql = format!(
            "SELECT f.face_id, fi.hash, f.face_index, f.family_name, f.subfamily, f.full_name, f.weight_class,
                    f.width_class, f.is_italic, f.is_variable, f.vi_status, f.supports_vi, f.vi_missing,
                    f.license_class,
                    (SELECT l.path FROM file_locations l WHERE l.file_id = f.file_id
                      ORDER BY l.missing, l.location_id LIMIT 1),
                    NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0),
                    EXISTS (SELECT 1 FROM u.favorites fv WHERE fv.hash = fi.hash AND fv.face_index = f.face_index),
                    fi.first_seen_at, fi.size
             FROM faces f JOIN files fi ON fi.file_id = f.file_id
             WHERE {w} ORDER BY {} LIMIT {limit} OFFSET {}",
            order_by(q),
            q.offset
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(refs.as_slice(), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                FaceRow {
                    face_id: r.get(0)?,
                    hash_hex: String::new(),
                    face_index: r.get(2)?,
                    family: r.get(3)?,
                    subfamily: r.get(4)?,
                    full_name: r.get(5)?,
                    weight_class: r.get(6)?,
                    width_class: r.get(7)?,
                    is_italic: r.get(8)?,
                    is_variable: r.get(9)?,
                    vi_status: r.get(10)?,
                    supports_vi: r.get(11)?,
                    vi_missing: r.get(12)?,
                    license_class: r.get(13)?,
                    path: r.get(14)?,
                    file_size: r.get::<_, i64>(18)?.max(0) as u64,
                    missing: r.get(15)?,
                    is_favorite: r.get(16)?,
                    added_at: r.get(17)?,
                },
            ))
        })?;
        let mut items = Vec::new();
        for row in rows {
            let (_, h, mut fr) = row?;
            fr.hash_hex = hex_of(&hash_from_blob(h)?);
            items.push(fr);
        }
        Ok(FacePage { total: total.max(0) as u64, offset: q.offset, items })
    }
}
