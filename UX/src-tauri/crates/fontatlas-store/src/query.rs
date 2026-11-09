//! Truy vấn lưới thư viện: tìm, lọc, sắp xếp, phân trang phía backend.
use rusqlite::{OptionalExtension, ToSql};
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
    /// Điểm khớp trong scope (cao trước); không có scope thì như Family.
    Match,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViFilter {
    /// vi_status = full
    Full,
    /// có đủ chữ dựng sẵn (supports_vi)
    Supported,
}

/// Lọc theo ngôn ngữ (Hyperglot): `near_ok` = chấp nhận mức near (ngược lại chỉ full).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LangFilter {
    pub code: String,
    pub near_ok: bool,
}

/// Mức độ phủ đã lưu của một face với một ngôn ngữ (chỉ có khi ≥ near).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LangHit {
    pub level: String,
    pub have: u32,
    pub total: u32,
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
    /// Ngôn ngữ cần có (xem `lang_any`); chỉ face đã tính độ phủ mới khớp.
    pub langs: Vec<LangFilter>,
    /// true = OR giữa các ngôn ngữ, false = AND.
    pub lang_any: bool,
    /// Ngôn ngữ để điền `FaceRow::lang_hit` (chỉ báo trên thẻ).
    pub badge_lang: Option<String>,
}

impl FaceQuery {
    /// Không có điều kiện lọc nào (bỏ qua sort/phân trang/`badge_lang`/`langs`): tập = cả thư viện.
    pub fn is_unfiltered(&self) -> bool {
        self.text.as_deref().and_then(text_match).is_none()
            && self.vi.is_none()
            && self.license.is_empty()
            && self.variable.is_none()
            && self.italic.is_none()
            && self.source_ids.is_empty()
            && !self.favorites_only
            && self.collection_id.is_none()
            && self.tag_ids.is_empty()
    }
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
    /// Độ phủ đã lưu với `FaceQuery::badge_lang` (None: không hỏi, chưa tính, hoặc dưới near).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang_hit: Option<LangHit>,
    /// Face đã được tính độ phủ (với bất kỳ `db_version`); chỉ đặt khi có `badge_lang`.
    /// `lang_computed && lang_hit.is_none()` = đã tính, dưới near.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub lang_computed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacePage {
    pub total: u64,
    pub offset: u32,
    pub items: Vec<FaceRow>,
}

/// Điều kiện WHERE + tham số theo đúng thứ tự `?`.
pub(crate) fn where_clause(q: &FaceQuery) -> (String, Vec<Value>) {
    let mut w: Vec<String> = vec!["1=1".into()];
    let mut p: Vec<Value> = Vec::new();
    let in_list = |n: usize| vec!["?"; n].join(",");
    match q.text.as_deref().and_then(text_match) {
        Some(TextMatch::Fts { phrase, .. }) => {
            w.push("f.face_id IN (SELECT rowid FROM faces_fts WHERE faces_fts MATCH ?)".into());
            p.push(Value::Text(phrase));
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
    if !q.langs.is_empty() {
        let one = |lf: &LangFilter, p: &mut Vec<Value>| {
            p.push(Value::Text(lf.code.clone()));
            format!(
                "EXISTS (SELECT 1 FROM face_langs fl WHERE fl.hash = fi.hash AND fl.face_index = f.face_index \
                 AND fl.lang = ?{})",
                if lf.near_ok { "" } else { " AND fl.level = 'full'" }
            )
        };
        let parts: Vec<String> = q.langs.iter().map(|lf| one(lf, &mut p)).collect();
        w.push(if q.lang_any { format!("({})", parts.join(" OR ")) } else { parts.join(" AND ") });
    }
    (w.join(" AND "), p)
}

/// Khớp đầu từ đứng trước khớp chuỗi con, rồi mới tới sort người dùng chọn. Trả kèm tham số.
fn order_by(q: &FaceQuery, scoped: bool) -> (String, Vec<Value>) {
    let rank = match q.text.as_deref().and_then(text_match) {
        _ if q.sort == SortKey::Match => None,
        Some(TextMatch::Fts { start, word, .. }) => Some((
            "CASE WHEN f.search_text LIKE ? ESCAPE '\\' OR f.search_text LIKE ? ESCAPE '\\' THEN 0 ELSE 1 END, "
                .to_string(),
            vec![Value::Text(start), Value::Text(word)],
        )),
        _ => None,
    };
    let (head, params) = rank.unwrap_or_default();
    (format!("{head}{}", user_order(q, scoped)), params)
}

fn user_order(q: &FaceQuery, scoped: bool) -> String {
    let dir = if q.descending { "DESC" } else { "ASC" };
    match q.sort {
        SortKey::Family => format!("f.family_key {dir}, f.weight_class, f.is_italic, f.face_id"),
        SortKey::Weight => format!("f.weight_class {dir}, f.family_key, f.face_id"),
        SortKey::Added => format!("fi.first_seen_at {dir}, f.face_id"),
        SortKey::Match if scoped => "sc.score DESC, f.family_key, f.face_id".to_string(),
        SortKey::Match => "f.family_key, f.weight_class, f.is_italic, f.face_id".to_string(),
    }
}

impl Store {
    /// Nạp `temp.scope` (thay nội dung cũ). Gọi `clear_scope` khi xong.
    pub(crate) fn load_scope(&self, sc: &[(i64, f32)]) -> Result<(), StoreError> {
        self.conn.execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS scope(face_id INTEGER PRIMARY KEY, score REAL NOT NULL);
             DELETE FROM temp.scope;",
        )?;
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut ins = tx.prepare_cached("INSERT OR REPLACE INTO temp.scope(face_id, score) VALUES (?1, ?2)")?;
            for &(id, score) in sc {
                ins.execute(rusqlite::params![id, score as f64])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn clear_scope(&self) {
        let _ = self.conn.execute("DELETE FROM temp.scope", []);
    }

    pub fn query_faces(&self, q: &FaceQuery) -> Result<FacePage, StoreError> {
        self.query_faces_scoped(q, None)
    }

    /// `scope` = (face_id, điểm). Nạp vào `temp.scope` rồi JOIN (không dùng `IN (…)`: 50k tham số
    /// vượt giới hạn SQLite). `total` đếm trong scope.
    pub fn query_faces_scoped(&self, q: &FaceQuery, scope: Option<&[(i64, f32)]>) -> Result<FacePage, StoreError> {
        let scoped = scope.is_some();
        if let Some(sc) = scope {
            self.load_scope(sc)?;
        }
        let page = self.query_faces_inner(q, scoped);
        if scoped {
            let _ = self.conn.execute("DELETE FROM temp.scope", []);
        }
        page
    }

    fn query_faces_inner(&self, q: &FaceQuery, scoped: bool) -> Result<FacePage, StoreError> {
        let from = if scoped {
            "faces f JOIN files fi ON fi.file_id = f.file_id JOIN temp.scope sc ON sc.face_id = f.face_id"
        } else {
            "faces f JOIN files fi ON fi.file_id = f.file_id"
        };
        let (w, p) = where_clause(q);
        let refs: Vec<&dyn ToSql> = p.iter().map(|v| v as &dyn ToSql).collect();
        let total: i64 = self.conn.query_row(
            &format!("SELECT count(*) FROM {from} WHERE {w}"),
            refs.as_slice(),
            |r| r.get(0),
        )?;
        let (order, order_params) = order_by(q, scoped);
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
             FROM {from}
             WHERE {w} ORDER BY {order} LIMIT {limit} OFFSET {}",
            q.offset
        );
        let all: Vec<&dyn ToSql> = refs.iter().copied().chain(order_params.iter().map(|v| v as &dyn ToSql)).collect();
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(all.as_slice(), |r| {
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
                    lang_hit: None,
                    lang_computed: false,
                },
            ))
        })?;
        let mut items = Vec::new();
        let mut badge = match q.badge_lang.as_deref() {
            Some(_) => Some(self.conn.prepare(
                "SELECT level, have, total FROM face_langs WHERE hash = ?1 AND face_index = ?2 AND lang = ?3",
            )?),
            None => None,
        };
        let mut computed = match q.badge_lang {
            Some(_) => Some(self.conn.prepare_cached("SELECT 1 FROM face_lang_state WHERE hash = ?1 AND face_index = ?2")?),
            None => None,
        };
        for row in rows {
            let (_, h, mut fr) = row?;
            let hash = hash_from_blob(h)?;
            fr.hash_hex = hex_of(&hash);
            if let (Some(st), Some(lang)) = (badge.as_mut(), q.badge_lang.as_deref()) {
                fr.lang_hit = st
                    .query_row(rusqlite::params![&hash[..], fr.face_index, lang], |r| {
                        Ok(LangHit { level: r.get(0)?, have: r.get(1)?, total: r.get(2)? })
                    })
                    .optional()?;
            }
            if let Some(st) = computed.as_mut() {
                fr.lang_computed = fr.lang_hit.is_some() || st.exists(rusqlite::params![&hash[..], fr.face_index])?;
            }
            items.push(fr);
        }
        Ok(FacePage { total: total.max(0) as u64, offset: q.offset, items })
    }
}
