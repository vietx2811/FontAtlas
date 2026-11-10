//! Tự bật font theo yêu cầu của app thiết kế (0.3.0): app nào hỏi, tìm font trong thư viện, nhật ký.
//!
//! Đơn vị vẫn là FILE (`hash`); lý do là `auto:<app>` trong `activation_reasons` (xem `ActivationReason::Auto`).
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::error::StoreError;
use crate::fold::vi_fold;
use crate::ingest::{FaceKey, hash_from_blob};
use crate::open::{Store, now_ms};

/// Số dòng nhật ký giữ lại.
pub const AUTO_LOG_KEEP: i64 = 500;
/// Cửa sổ chống ghi trùng nhật ký (ms).
pub const AUTO_LOG_DEDUPE_MS: i64 = 60_000;

/// App đã biết. Tập đóng để lý do `auto:<slug>` luôn hợp lệ; app lạ gộp vào `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoApp {
    Photoshop,
    AfterEffects,
    Illustrator,
    Indesign,
    Incopy,
    Sketch,
    Affinity,
    Figma,
    Other,
}

impl AutoApp {
    pub const ALL: [AutoApp; 9] = [
        AutoApp::Photoshop,
        AutoApp::AfterEffects,
        AutoApp::Illustrator,
        AutoApp::Indesign,
        AutoApp::Incopy,
        AutoApp::Sketch,
        AutoApp::Affinity,
        AutoApp::Figma,
        AutoApp::Other,
    ];

    pub fn slug(&self) -> &'static str {
        match self {
            AutoApp::Photoshop => "photoshop",
            AutoApp::AfterEffects => "aftereffects",
            AutoApp::Illustrator => "illustrator",
            AutoApp::Indesign => "indesign",
            AutoApp::Incopy => "incopy",
            AutoApp::Sketch => "sketch",
            AutoApp::Affinity => "affinity",
            AutoApp::Figma => "figma",
            AutoApp::Other => "other",
        }
    }

    pub fn from_slug(s: &str) -> Option<AutoApp> {
        AutoApp::ALL.into_iter().find(|a| a.slug() == s)
    }

    /// Nhận tên app người ta gửi (slug, tên hiển thị, bundle id): chữ thường, bỏ ký tự không phải chữ cái.
    /// Không nhận ra → `Other`.
    pub fn parse_lenient(s: &str) -> AutoApp {
        let k: String = s.chars().filter(|c| c.is_ascii_alphabetic()).flat_map(char::to_lowercase).collect();
        let has = |w: &str| k.contains(w);
        if has("photoshop") {
            AutoApp::Photoshop
        } else if has("aftereffects") {
            AutoApp::AfterEffects
        } else if has("illustrator") {
            AutoApp::Illustrator
        } else if has("indesign") {
            AutoApp::Indesign
        } else if has("incopy") {
            AutoApp::Incopy
        } else if has("sketch") {
            AutoApp::Sketch
        } else if has("affinity") {
            AutoApp::Affinity
        } else if has("figma") {
            AutoApp::Figma
        } else {
            AutoApp::Other
        }
    }
}

/// Một dòng nhật ký cần ghi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoLogEntry {
    pub app: AutoApp,
    pub doc: Option<String>,
    pub requested: String,
    pub outcome: String,
    pub hash: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLogRow {
    pub id: i64,
    pub at: i64,
    pub app: String,
    pub doc: Option<String>,
    pub requested: String,
    pub outcome: String,
    pub hash_hex: Option<String>,
}

fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// Gọn: chữ thường, bỏ khoảng trắng và dấu gạch (so style "Semi Bold" = "SemiBold" = "semibold").
fn squash(s: &str) -> String {
    vi_fold(s).chars().filter(|c| c.is_alphanumeric()).collect()
}

impl Store {
    /// Face có đúng PostScript name này và còn ít nhất một vị trí file chưa mất. Thứ tự ổn định (hash, face).
    pub fn faces_by_postscript(&self, ps: &str) -> Result<Vec<FaceKey>, StoreError> {
        let mut st = self.conn.prepare_cached(
            "SELECT f.hash, fa.face_index FROM main.faces fa JOIN main.files f ON f.file_id = fa.file_id
             WHERE fa.postscript_name = ?1
               AND EXISTS (SELECT 1 FROM main.file_locations l WHERE l.file_id = f.file_id AND l.missing = 0)
             ORDER BY f.hash, fa.face_index",
        )?;
        let rows = st
            .query_map([ps], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, u32>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().map(|(b, i)| Ok(FaceKey { hash: hash_from_blob(b)?, face_index: i })).collect()
    }

    /// Face theo family + style. Family so sau `vi_fold`; style so sau khi gọn (rỗng = "Regular").
    /// Chỉ trả khi khớp đúng cả hai: không đoán gần đúng, vì bật nhầm font còn tệ hơn báo thiếu.
    pub fn faces_by_family_style(&self, family: &str, style: &str) -> Result<Vec<FaceKey>, StoreError> {
        let want = {
            let w = squash(style);
            if w.is_empty() { "regular".to_string() } else { w }
        };
        let mut st = self.conn.prepare_cached(
            "SELECT f.hash, fa.face_index, fa.subfamily FROM main.faces fa JOIN main.files f ON f.file_id = fa.file_id
             WHERE fa.family_key = ?1
               AND EXISTS (SELECT 1 FROM main.file_locations l WHERE l.file_id = f.file_id AND l.missing = 0)
             ORDER BY f.hash, fa.face_index",
        )?;
        let rows = st
            .query_map([vi_fold(family)], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<String>>(2)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut out = vec![];
        for (b, i, sub) in rows {
            let have = sub.as_deref().map(squash).filter(|s| !s.is_empty()).unwrap_or_else(|| "regular".into());
            if have == want {
                out.push(FaceKey { hash: hash_from_blob(b)?, face_index: i });
            }
        }
        Ok(out)
    }

    /// Ghi một dòng nhật ký và giữ `AUTO_LOG_KEEP` dòng mới nhất. App hỏi lặp lại (vd Photoshop hỏi lại mỗi lần vẽ
    /// lại chữ) thì không ghi thêm nếu đã có dòng y hệt (app, tên, kết quả) trong `AUTO_LOG_DEDUPE_MS`.
    /// Trả `true` nếu có ghi.
    pub fn log_auto(&mut self, e: &AutoLogEntry) -> Result<bool, StoreError> {
        let now = now_ms();
        let dup = self
            .conn
            .query_row(
                "SELECT 1 FROM u.auto_activation_log WHERE app=?1 AND requested=?2 AND outcome=?3 AND at > ?4 LIMIT 1",
                params![e.app.slug(), e.requested, e.outcome, now - AUTO_LOG_DEDUPE_MS],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if dup {
            return Ok(false);
        }
        self.conn.execute(
            "INSERT INTO u.auto_activation_log(at, app, doc, requested, outcome, hash) VALUES (?1,?2,?3,?4,?5,?6)",
            params![now, e.app.slug(), e.doc, e.requested, e.outcome, e.hash.as_ref().map(|h| &h[..])],
        )?;
        self.conn.execute(
            "DELETE FROM u.auto_activation_log WHERE id <= (SELECT max(id) FROM u.auto_activation_log) - ?1",
            [AUTO_LOG_KEEP],
        )?;
        Ok(true)
    }

    /// Mới nhất trước.
    pub fn recent_auto_log(&self, limit: usize) -> Result<Vec<AutoLogRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT id, at, app, doc, requested, outcome, hash FROM u.auto_activation_log ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = st
            .query_map([limit as i64], |r| {
                let h: Option<Vec<u8>> = r.get(6)?;
                Ok(AutoLogRow {
                    id: r.get(0)?,
                    at: r.get(1)?,
                    app: r.get(2)?,
                    doc: r.get(3)?,
                    requested: r.get(4)?,
                    outcome: r.get(5)?,
                    hash_hex: h.and_then(|b| hash_from_blob(b).ok()).map(|h| hex(&h)),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_slug_roundtrip_and_lenient_parse() {
        for a in AutoApp::ALL {
            assert_eq!(AutoApp::from_slug(a.slug()), Some(a));
        }
        assert_eq!(AutoApp::parse_lenient("Adobe Photoshop 2026"), AutoApp::Photoshop);
        assert_eq!(AutoApp::parse_lenient("com.adobe.illustrator"), AutoApp::Illustrator);
        assert_eq!(AutoApp::parse_lenient("Adobe After Effects"), AutoApp::AfterEffects);
        assert_eq!(AutoApp::parse_lenient("InDesign"), AutoApp::Indesign);
        assert_eq!(AutoApp::parse_lenient("Affinity Designer 2"), AutoApp::Affinity);
        assert_eq!(AutoApp::parse_lenient("rm -rf; photoshop'--"), AutoApp::Photoshop);
        assert_eq!(AutoApp::parse_lenient("TextEdit"), AutoApp::Other);
        assert_eq!(AutoApp::from_slug("auto:evil"), None);
    }

    #[test]
    fn squash_ignores_case_space_and_dash() {
        assert_eq!(squash("Semi Bold"), squash("SemiBold"));
        assert_eq!(squash("Bold-Italic"), squash("bold italic"));
    }
}
