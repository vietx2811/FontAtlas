//! G7: font Variable (VF) và font static trùng tên PostScript — hai bản cùng bật thì app thiết kế thấy hai mục
//! giống nhau và tra theo tên PostScript luôn ra bản VF (đo bằng CoreText, xem
//! `docs/team/research/2026-11-12-variable-vs-static-names.md`).
//!
//! Chỉ ĐỌC library: không sửa file, không đổi tên. Tính theo yêu cầu từ các face còn thấy (không migration).

use std::collections::{BTreeSet, HashMap};

use crate::dup_resolve::HIDDEN_LOC;
use crate::error::StoreError;
use crate::open::Store;

/// Một face Variable và các face static trùng tên PostScript với nó.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameClash {
    pub vf_face_id: i64,
    /// Face static trùng tên (đã sắp tăng dần, không trùng lặp).
    pub static_face_ids: Vec<i64>,
    /// Các tên PostScript trùng (đã sắp, không trùng lặp).
    pub ps_names: Vec<String>,
    /// `true` khi trùng tên PostScript của CHÍNH face VF (instance mặc định); `false` khi chỉ trùng tên suy ra từ
    /// named instance (`<tiền tố>-<Kiểu không dấu cách>` hoặc `<PostScript>_<Kiểu>`; CoreText tạo tên theo hai dạng đó).
    pub exact: bool,
}

/// Tên PostScript mà named instance `style` của VF có PostScript `ps` thường mang (cả hai dạng CoreText dùng).
pub(crate) fn instance_ps_candidates(ps: &str, style: &str) -> Vec<String> {
    let compact: String = style.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return Vec::new();
    }
    let mut v = Vec::with_capacity(2);
    if let Some((prefix, _)) = ps.split_once('-') {
        if !prefix.is_empty() {
            v.push(format!("{prefix}-{compact}"));
        }
    }
    v.push(format!("{ps}_{compact}"));
    v
}

impl Store {
    /// Mọi face VF còn thấy (không bị ẩn) có ít nhất một face static còn thấy trùng tên PostScript, sắp theo `vf_face_id`.
    /// Hai face cùng file/hash không tính là trùng với nhau.
    pub fn name_clashes(&self) -> Result<Vec<NameClash>, StoreError> {
        // tên PostScript → các face static còn thấy
        let mut statics: HashMap<String, Vec<i64>> = HashMap::new();
        let visible = format!(
            "EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0 AND NOT {HIDDEN_LOC})"
        );
        {
            let mut st = self.conn.prepare(&format!(
                "SELECT f.face_id, f.postscript_name FROM faces f \
                 WHERE f.is_variable = 0 AND f.postscript_name IS NOT NULL AND f.postscript_name <> '' AND {visible}"
            ))?;
            for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
                let (id, ps) = r?;
                statics.entry(ps).or_default().push(id);
            }
        }
        if statics.is_empty() {
            return Ok(Vec::new());
        }
        let mut st = self.conn.prepare(&format!(
            "SELECT f.face_id, f.postscript_name, \
                    (SELECT a.metadata_json FROM analyses a WHERE a.hash = fi.hash AND a.face_index = f.face_index \
                     ORDER BY a.analyzed_at DESC, a.rowid DESC LIMIT 1) \
             FROM faces f JOIN files fi ON fi.file_id = f.file_id \
             WHERE f.is_variable = 1 AND f.postscript_name IS NOT NULL AND f.postscript_name <> '' AND {visible} \
             ORDER BY f.face_id"
        ))?;
        let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)))?;
        let mut out = Vec::new();
        for r in rows {
            let (vf_face_id, ps, meta) = r?;
            let mut ids: BTreeSet<i64> = BTreeSet::new();
            let mut names: BTreeSet<String> = BTreeSet::new();
            let mut exact = false;
            if let Some(v) = statics.get(&ps) {
                exact = true;
                ids.extend(v.iter().copied());
                names.insert(ps.clone());
            }
            let styles: Vec<String> = meta
                .as_deref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .and_then(|j| j.get("named_instances").cloned())
                .and_then(|n| serde_json::from_value::<Vec<String>>(n).ok())
                .unwrap_or_default();
            for style in styles {
                for cand in instance_ps_candidates(&ps, &style) {
                    if let Some(v) = statics.get(&cand) {
                        ids.extend(v.iter().copied());
                        names.insert(cand);
                    }
                }
            }
            if !ids.is_empty() {
                out.push(NameClash { vf_face_id, static_face_ids: ids.into_iter().collect(), ps_names: names.into_iter().collect(), exact });
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::instance_ps_candidates;

    #[test]
    fn candidates_cover_both_coretext_spellings() {
        assert_eq!(instance_ps_candidates("Roboto-Regular", "Medium Italic"), vec!["Roboto-MediumItalic", "Roboto-Regular_MediumItalic"]);
        assert_eq!(instance_ps_candidates("Oswald-Regular", "Light"), vec!["Oswald-Light", "Oswald-Regular_Light"]);
        assert_eq!(instance_ps_candidates("NoDash", "Bold"), vec!["NoDash_Bold"]);
        assert!(instance_ps_candidates("A-B", "  ").is_empty());
    }
}
