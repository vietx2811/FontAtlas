//! Truy vấn lưới thư viện: tìm, lọc, sắp xếp, phân trang phía backend.
use rusqlite::{OptionalExtension, ToSql};
use rusqlite::types::Value;
use serde::{Deserialize, Serialize};

use crate::error::StoreError;
use crate::fold::{TextMatch, text_match};
use crate::ingest::{hash_from_blob, hex_of};
use crate::dup_resolve::HIDDEN_LOC;
use crate::open::Store;

pub const DEFAULT_LIMIT: u32 = 100;
pub const MAX_LIMIT: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    /// Không chọn sort: như Tên (Family) nhưng khi có chữ tìm vẫn xếp khớp đầu từ trước (hành vi gốc của mọi lời gọi tìm chữ).
    #[default]
    Auto,
    /// Tên tường minh: xếp THUẦN, không xếp hạng khớp.
    Family,
    Weight,
    Added,
    /// Năm phát hành ước lượng (`release_year`); face không có năm luôn xếp cuối bất kể chiều.
    Released,
    /// Ngày tạo file sớm nhất trong các vị trí còn thấy; không có thì xếp cuối bất kể chiều.
    FileCreated,
    /// Điểm khớp trong scope (cao trước); không có scope: khớp đầu từ trước, rồi theo tên (chiều không áp dụng).
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
    /// Gồm cả face đã bị ẩn bởi "giải quyết trùng lặp" (mặc định: không). Trang Trùng lặp / Đã giải quyết dùng true.
    pub include_hidden: bool,
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
    /// usWeightClass, khoảng đóng trong 100..=900 (giá trị ghi 1–9 coi như ×100).
    pub weight_class: Option<[u16; 2]>,
    /// usWidthClass, khoảng đóng trong 1..=9.
    pub width_class: Option<[u16; 2]>,
    /// Font Variable cũng khớp `weight_class` khi dải trục `wght` giao khoảng chọn.
    pub var_weight: bool,
    /// Như trên cho `wdth` (quy % về class theo bảng OS/2, xem `WIDTH_PCT`).
    pub var_width: bool,
    /// Xếp theo NHÓM family: mỗi family liền nhau (không bao giờ bị xé), thứ tự nhóm theo `sort`, trong nhóm Weight
    /// tăng → thẳng trước nghiêng → tên. `family_groups` trả đúng thứ tự này.
    pub group_by_family: bool,
}

/// `wdth` (%) ứng với usWidthClass 1..=9 theo OS/2.
pub const WIDTH_PCT: [f64; 9] = [50.0, 62.5, 75.0, 87.5, 100.0, 112.5, 125.0, 150.0, 200.0];

/// Khoảng `wdth` (%) mà các class a..=b phủ: ranh giới là trung điểm giữa hai class liền kề.
pub fn width_pct_span(a: u16, b: u16) -> (f64, f64) {
    let (a, b) = (a.clamp(1, 9) as usize - 1, b.clamp(1, 9) as usize - 1);
    let lo = if a == 0 { WIDTH_PCT[0] } else { (WIDTH_PCT[a - 1] + WIDTH_PCT[a]) / 2.0 };
    let hi = if b == 8 { WIDTH_PCT[8] } else { (WIDTH_PCT[b] + WIDTH_PCT[b + 1]) / 2.0 };
    (lo, hi)
}

/// usWeightClass chuẩn hoá về 100..=900: font cũ ghi 1–9 → ×100; 0/NULL → NULL; giá trị ngoài (50, 950, 1000)
/// kẹp vào biên; giá trị lẻ (250) giữ nguyên. Bộ lọc và histogram dùng chung biểu thức này.
pub(crate) const WEIGHT_NORM: &str = "(CASE WHEN f.weight_class BETWEEN 1 AND 9 THEN f.weight_class * 100 \
     WHEN f.weight_class > 0 THEN min(900, max(100, f.weight_class)) END)";
/// usWidthClass hợp lệ 1..=9; 0 hoặc lớn hơn 9 → NULL.
pub(crate) const WIDTH_NORM: &str = "(CASE WHEN f.width_class BETWEEN 1 AND 9 THEN f.width_class END)";

/// Điều kiện lớp weight: instance mặc định nằm trong khoảng, hoặc (nếu `var`) dải `wght` giao khoảng.
pub(crate) fn weight_cond(r: [u16; 2], var: bool, p: &mut Vec<Value>) -> String {
    let (a, b) = (r[0].min(r[1]) as i64, r[0].max(r[1]) as i64);
    p.push(Value::Integer(a));
    p.push(Value::Integer(b));
    let base = format!("{WEIGHT_NORM} BETWEEN ? AND ?");
    if !var {
        return format!("({base})");
    }
    p.push(Value::Integer(b));
    p.push(Value::Integer(a));
    format!("({base} OR (f.wght_min IS NOT NULL AND f.wght_min <= ? AND f.wght_max >= ?))")
}

/// `wdth` (%) giao khoảng class [a, b]: khoảng nửa mở [lo, hi) — giá trị đúng ranh giới thuộc class bên trên —
/// riêng class 9 đóng ở 200.
fn wdth_intersects(lo: f64, hi: f64, hi_closed: bool, p: &mut Vec<Value>) -> String {
    p.push(Value::Real(hi));
    p.push(Value::Real(lo));
    format!("f.wdth_min IS NOT NULL AND f.wdth_min {} ? AND f.wdth_max >= ?", if hi_closed { "<=" } else { "<" })
}

pub(crate) fn width_cond(r: [u16; 2], var: bool, p: &mut Vec<Value>) -> String {
    let (a, b) = (r[0].min(r[1]).max(1), r[0].max(r[1]).min(9));
    p.push(Value::Integer(a as i64));
    p.push(Value::Integer(b as i64));
    let base = format!("{WIDTH_NORM} BETWEEN ? AND ?");
    if !var {
        return format!("({base})");
    }
    let (lo, hi) = width_pct_span(a, b);
    format!("({base} OR ({}))", wdth_intersects(lo, hi, b == 9, p))
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
            && self.weight_class.is_none()
            && self.width_class.is_none()
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
    /// Mọi bản còn thấy đều là placeholder trên ổ cloud (chưa tải về): chưa dùng được ngay.
    pub on_cloud_only: bool,
    pub is_favorite: bool,
    pub added_at: i64,
    /// Năm phát hành ước lượng (xem `fontatlas_core::extras::release_year`); `None` = không rõ hoặc chưa tính.
    pub release_year: Option<u16>,
    /// `copyright` | `head_created` | `head_modified`; `None` khi `release_year` là `None`.
    pub release_year_source: Option<String>,
    /// Ngày tạo file sớm nhất (ms từ epoch) trong các vị trí chưa missing; `None` = không đọc được / chưa quét lại.
    pub file_created_at: Option<i64>,
    /// `disk` | `cloud_sync` (birth time của file cloud có thể là lúc đồng bộ); `None` khi `file_created_at` là `None`.
    pub file_created_source: Option<String>,
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
        let hid = if q.include_hidden { String::new() } else { format!(" AND NOT {HIDDEN_LOC}") };
        w.push(format!("EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0{hid})"));
    } else if !q.include_hidden {
        // vẫn xét cả vị trí missing, nhưng face mà MỌI vị trí đều bị ẩn thì loại
        w.push(format!(
            "(NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND {HIDDEN_LOC}) \
             OR EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND NOT {HIDDEN_LOC}))"
        ));
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
    if let Some(r) = q.weight_class {
        let c = weight_cond(r, q.var_weight, &mut p);
        w.push(c);
    }
    if let Some(r) = q.width_class {
        let c = width_cond(r, q.var_width, &mut p);
        w.push(c);
    }
    (w.join(" AND "), p)
}

/// `Auto` (sort vắng mặt) và `Match` không scope: khớp đầu từ đứng trước khớp chuỗi con, rồi theo tên. Sort tường minh
/// (Tên/Ngày/Weight) xếp THUẦN theo trường, không xếp hạng khớp. `Match` có scope (Gần đây / Đang bật) giữ thứ tự đưa vào.
/// Trả kèm tham số.
fn order_by(q: &FaceQuery, scoped: bool) -> (String, Vec<Value>) {
    if q.group_by_family {
        return grouped_order(q, scoped, true);
    }
    let rank = match q.text.as_deref().and_then(text_match) {
        Some(TextMatch::Fts { start, word, .. }) if q.sort == SortKey::Auto || (q.sort == SortKey::Match && !scoped) => Some((
            "CASE WHEN f.search_text LIKE ? ESCAPE '\\' OR f.search_text LIKE ? ESCAPE '\\' THEN 0 ELSE 1 END, "
                .to_string(),
            vec![Value::Text(start), Value::Text(word)],
        )),
        _ => None,
    };
    let (head, params) = rank.unwrap_or_default();
    (format!("{head}{}", user_order(q, scoped)), params)
}

/// Thứ tự theo nhóm family. `window = true`: dùng cho từng hàng face (cực trị của family tính bằng hàm cửa sổ);
/// `false`: dùng cho truy vấn GROUP BY family_key (cực trị là hàm gộp) — cả hai cho cùng thứ tự nhóm.
/// Khoá nhóm: family → tên; weight/added/released/file_created → cực trị của family theo chiều (nhỏ nhất khi tăng,
/// lớn nhất khi giảm; không có giá trị → cuối); match có scope → điểm cao nhất; còn lại → tên (khớp đầu từ lên trước).
fn grouped_order(q: &FaceQuery, scoped: bool, window: bool) -> (String, Vec<Value>) {
    let agg = |func: &str, expr: &str| {
        if window {
            format!("{func}({expr}) OVER (PARTITION BY f.family_key)")
        } else {
            format!("{func}({expr})")
        }
    };
    let dir = if q.descending { "DESC" } else { "ASC" };
    let ext = if q.descending { "MAX" } else { "MIN" };
    let mut params = Vec::new();
    let mut head = String::new();
    if let Some(TextMatch::Fts { start, word, .. }) = q.text.as_deref().and_then(text_match)
        && (q.sort == SortKey::Auto || (q.sort == SortKey::Match && !scoped))
    {
        head = format!(
            "{}, ",
            agg("MIN", "CASE WHEN f.search_text LIKE ? ESCAPE '\\' OR f.search_text LIKE ? ESCAPE '\\' THEN 0 ELSE 1 END")
        );
        params = vec![Value::Text(start), Value::Text(word)];
    }
    let nulls_last = |k: String| format!("({k} IS NULL), {k} {dir}");
    let key = match q.sort {
        SortKey::Weight => nulls_last(agg(ext, "f.weight_class")),
        SortKey::Added => nulls_last(agg(ext, "fi.first_seen_at")),
        SortKey::Released => nulls_last(agg(ext, "f.release_year")),
        SortKey::FileCreated => nulls_last(agg(ext, FILE_CREATED_SQL)),
        SortKey::Match if scoped => format!("{} DESC", agg("MAX", "sc.score")),
        SortKey::Auto | SortKey::Family | SortKey::Match => format!("f.family_key {dir}"),
    };
    (format!("{head}{key}, f.family_key"), params)
}

/// Ngày tạo file sớm nhất của face (các vị trí chưa missing).
/// Vị trí bị ẩn bởi quyết định Trùng lặp không tính (cùng luật với cột `file_created_at` trả về).
const FILE_CREATED_SQL: &str = "(SELECT MIN(l.created_ns) FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0 \
     AND NOT EXISTS (SELECT 1 FROM u.dup_resolutions dr WHERE dr.loser_path = l.path AND dr.undone_at IS NULL))";

fn user_order(q: &FaceQuery, scoped: bool) -> String {
    let dir = if q.descending { "DESC" } else { "ASC" };
    match q.sort {
        SortKey::Auto | SortKey::Family => format!("f.family_key {dir}, f.weight_class, f.is_italic, f.face_id"),
        SortKey::Weight => format!("f.weight_class {dir}, f.family_key, f.face_id"),
        SortKey::Added => format!("fi.first_seen_at {dir}, f.face_id"),
        // không có giá trị: luôn cuối (kể cả khi giảm dần), rồi cùng bậc theo Family → Weight → nghiêng
        SortKey::Released => format!("(f.release_year IS NULL), f.release_year {dir}, f.family_key, f.weight_class, f.is_italic, f.face_id"),
        SortKey::FileCreated => format!("({FILE_CREATED_SQL} IS NULL), {FILE_CREATED_SQL} {dir}, f.family_key, f.weight_class, f.is_italic, f.face_id"),
        SortKey::Match if scoped => "sc.score DESC, f.family_key, f.face_id".to_string(),
        SortKey::Match => "f.family_key, f.weight_class, f.is_italic, f.face_id".to_string(),
    }
}

/// Hai chuỗi 9 nấc của `class_histogram`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClassHistogram {
    /// Mỗi face một lần, ở class của instance mặc định.
    pub default_bins: [u32; 9],
    /// Như `default_bins` + font Variable đếm vào mọi nấc dải trục của nó phủ.
    pub span_bins: [u32; 9],
}

/// Một nhóm family trong kết quả `family_groups`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyGroup {
    /// Tên hiển thị (tên family của một face trong nhóm).
    pub family: String,
    /// Vị trí face đầu của nhóm trong thứ tự `query_faces` với `group_by_family = true`.
    pub start: u32,
    /// Số face của family trong truy vấn (đang khớp bộ lọc).
    pub count: u32,
    /// = `count`: số kiểu khớp bộ lọc (chip "4/9 kiểu").
    pub match_count: u32,
    /// Tổng số kiểu của family trong thư viện, KHÔNG áp bộ lọc (chỉ tính face còn hiện diện).
    pub style_total: u32,
}

/// Trục rời rạc của OS/2 có histogram theo nấc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassAxis {
    Weight,
    Width,
}

impl Store {
    /// Histogram 9 nấc (usWeightClass 100..900 hoặc usWidthClass 1..9; index 0 = nấc đầu) trên tập `q`
    /// BỎ điều kiện chính trục đó. Hai chuỗi:
    ///
    /// - `default_bins`: mỗi face đếm một lần ở class của instance mặc định;
    /// - `span_bins`: như trên, nhưng font Variable còn được đếm vào MỌI nấc mà dải trục `wght`/`wdth` của nó phủ
    ///   (đúng tập mà bộ lọc với `varAxisMatch` bật sẽ khớp).
    ///
    /// Weight lẻ (250) làm tròn về trăm gần nhất; face không có giá trị bị bỏ qua.
    pub fn class_histogram(&self, q: &FaceQuery, scope: Option<&[(i64, f32)]>, axis: ClassAxis) -> Result<ClassHistogram, StoreError> {
        let mut q = q.clone();
        let (col, vmin, vmax) = match axis {
            ClassAxis::Weight => {
                q.weight_class = None;
                (WEIGHT_NORM, "f.wght_min", "f.wght_max")
            }
            ClassAxis::Width => {
                q.width_class = None;
                (WIDTH_NORM, "f.wdth_min", "f.wdth_max")
            }
        };
        let (w, p) = where_clause(&q);
        let refs: Vec<&dyn ToSql> = p.iter().map(|v| v as &dyn ToSql).collect();
        let from = if scope.is_some() {
            "faces f JOIN files fi ON fi.file_id = f.file_id JOIN temp.scope sc ON sc.face_id = f.face_id"
        } else {
            "faces f JOIN files fi ON fi.file_id = f.file_id"
        };
        let sql = format!(
            "SELECT {col}, {vmin}, {vmax}, count(*) FROM {from} WHERE {w} AND {col} IS NOT NULL GROUP BY {col}, {vmin}, {vmax}"
        );
        if let Some(sc) = scope {
            self.load_scope(sc)?;
        }
        let out = (|| -> Result<ClassHistogram, StoreError> {
            let mut st = self.conn.prepare(&sql)?;
            let rows = st.query_map(refs.as_slice(), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Option<f64>>(1)?, r.get::<_, Option<f64>>(2)?, r.get::<_, i64>(3)?))
            })?;
            let mut h = ClassHistogram::default();
            for r in rows {
                let (v, lo, hi, n) = r?;
                let k = match axis {
                    ClassAxis::Weight => ((v as f64 / 100.0).round() as i64).clamp(1, 9),
                    ClassAxis::Width => v,
                };
                if !(1..=9).contains(&k) {
                    continue;
                }
                let n = n as u32;
                h.default_bins[k as usize - 1] += n;
                let mut covered = [false; 9];
                covered[k as usize - 1] = true;
                if let (Some(lo), Some(hi)) = (lo, hi) {
                    for c in 1..=9u16 {
                        covered[c as usize - 1] |= match axis {
                            ClassAxis::Weight => lo <= c as f64 * 100.0 && c as f64 * 100.0 <= hi,
                            ClassAxis::Width => {
                                let (a, b) = width_pct_span(c, c);
                                (if c == 9 { lo <= b } else { lo < b }) && hi >= a
                            }
                        };
                    }
                }
                for (i, c) in covered.iter().enumerate() {
                    if *c {
                        h.span_bins[i] += n;
                    }
                }
            }
            Ok(h)
        })();
        if scope.is_some() {
            self.clear_scope();
        }
        out
    }

    /// Các nhóm family của tập `q` (cùng bộ lọc/scope với `query_faces`), theo đúng thứ tự `query_faces` khi
    /// `group_by_family = true` (bỏ qua `q.group_by_family`, `offset`, `limit`): nhóm kế tiếp bắt đầu đúng nơi
    /// nhóm trước kết thúc, một family không bao giờ thành hai nhóm. Một truy vấn GROUP BY, không tải face.
    pub fn family_groups(&self, q: &FaceQuery, scope: Option<&[(i64, f32)]>) -> Result<Vec<FamilyGroup>, StoreError> {
        let scoped = scope.is_some();
        let (w, p) = where_clause(q);
        let (order, order_params) = grouped_order(q, scoped, false);
        let from = if scoped {
            "faces f JOIN files fi ON fi.file_id = f.file_id JOIN temp.scope sc ON sc.face_id = f.face_id"
        } else {
            "faces f JOIN files fi ON fi.file_id = f.file_id"
        };
        // tham số: WHERE trước, ORDER BY sau (đúng thứ tự `?` trong câu lệnh)
        let all: Vec<&dyn ToSql> = p.iter().chain(order_params.iter()).map(|v| v as &dyn ToSql).collect();
        let sql = format!(
            "SELECT f.family_key, MIN(f.family_name), count(*) FROM {from} WHERE {w} GROUP BY f.family_key ORDER BY {order}"
        );
        if let Some(sc) = scope {
            self.load_scope(sc)?;
        }
        let out = (|| -> Result<Vec<(String, String, u32)>, StoreError> {
            let mut st = self.conn.prepare(&sql)?;
            let rows = st.query_map(all.as_slice(), |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)? as u32)))?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })();
        if scope.is_some() {
            self.clear_scope();
        }
        let groups = out?;
        let totals = self.family_style_totals()?;
        let mut start = 0u32;
        Ok(groups
            .into_iter()
            .map(|(key, family, count)| {
                let g = FamilyGroup {
                    style_total: totals.get(&key).copied().unwrap_or(count).max(count),
                    family,
                    start,
                    count,
                    match_count: count,
                };
                start += count;
                g
            })
            .collect())
    }

    /// `family_key → số kiểu` trên toàn thư viện (face còn ít nhất một vị trí hiện diện, chưa bị ẩn).
    fn family_style_totals(&self) -> Result<std::collections::HashMap<String, u32>, StoreError> {
        let mut st = self.conn.prepare(&format!(
            "SELECT f.family_key, count(*) FROM faces f WHERE EXISTS \
             (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0 AND NOT {HIDDEN_LOC}) \
             GROUP BY f.family_key"
        ))?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u32)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// `face_id` của mọi face khớp `q` (và `scope` nếu có) — không phân trang, không sắp xếp: dùng để lập tập
    /// "được phép" một lần cho các phép thống kê.
    pub fn face_ids_matching(&self, q: &FaceQuery, scope: Option<&[(i64, f32)]>) -> Result<Vec<i64>, StoreError> {
        let (w, p) = where_clause(q);
        let refs: Vec<&dyn ToSql> = p.iter().map(|v| v as &dyn ToSql).collect();
        let from = if scope.is_some() {
            "faces f JOIN files fi ON fi.file_id = f.file_id JOIN temp.scope sc ON sc.face_id = f.face_id"
        } else {
            "faces f JOIN files fi ON fi.file_id = f.file_id"
        };
        if let Some(sc) = scope {
            self.load_scope(sc)?;
        }
        let out = (|| -> Result<Vec<i64>, StoreError> {
            let mut st = self.conn.prepare(&format!("SELECT f.face_id FROM {from} WHERE {w}"))?;
            let rows = st.query_map(refs.as_slice(), |r| r.get::<_, i64>(0))?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })();
        if scope.is_some() {
            self.clear_scope();
        }
        out
    }

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
        let (mut order, order_params) = order_by(q, scoped);
        if q.group_by_family {
            order.push_str(", f.weight_class, f.is_italic, f.full_name, f.face_id");
        }
        // đường dẫn / trạng thái missing / cloud của hàng KHÔNG tính vị trí đã ẩn (trừ khi chủ ý lấy cả bản đã ẩn)
        let nh = if q.include_hidden { String::new() } else { format!(" AND NOT {HIDDEN_LOC}") };
        let limit = match q.limit {
            0 => DEFAULT_LIMIT,
            n => n.min(MAX_LIMIT),
        };
        let sql = format!(
            "SELECT f.face_id, fi.hash, f.face_index, f.family_name, f.subfamily, f.full_name, f.weight_class,
                    f.width_class, f.is_italic, f.is_variable, f.vi_status, f.supports_vi, f.vi_missing,
                    f.license_class,
                    (SELECT l.path FROM file_locations l WHERE l.file_id = f.file_id{nh}
                      ORDER BY l.missing, l.location_id LIMIT 1),
                    NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0{nh}),
                    EXISTS (SELECT 1 FROM u.favorites fv WHERE fv.hash = fi.hash AND fv.face_index = f.face_index),
                    fi.first_seen_at, fi.size,
                    (EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0 AND l.cloud_only = 1{nh})
                     AND NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0 AND l.cloud_only = 0{nh})),
                    f.release_year, f.release_year_src,
                    (SELECT MIN(l.created_ns) FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0{nh}),
                    (SELECT l.created_src FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0 AND l.created_ns IS NOT NULL{nh}
                      ORDER BY l.created_ns, l.location_id LIMIT 1)
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
                    on_cloud_only: r.get(19)?,
                    is_favorite: r.get(16)?,
                    added_at: r.get(17)?,
                    release_year: r.get::<_, Option<i64>>(20)?.and_then(|y| u16::try_from(y).ok()),
                    release_year_source: r.get::<_, Option<String>>(21)?.filter(|s| s != "none"),
                    file_created_at: r.get::<_, Option<i64>>(22)?.map(|ns| ns / 1_000_000),
                    file_created_source: r.get(23)?,
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
