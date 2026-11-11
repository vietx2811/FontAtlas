//! Danh mục tag hệ thống (taxonomy v2): phong cách (genre), mood, mục đích dùng. KHÔNG đo bằng máy; chỉ là tag người dùng
//! gán, lưu như tag thường với tên có tiền tố dành riêng `style:`, `mood:`, `use:` (không cần bảng mới).
//! Thuật ngữ giữ tiếng Anh; UI dịch phần chú giải.

/// Một tag hệ thống.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTag {
    /// Khoá ổn định (snake_case), phần sau tiền tố.
    pub key: &'static str,
    /// Nhãn hiển thị (tiếng Anh).
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTagGroup {
    /// Tiền tố namespace, không có dấu hai chấm: "style" | "mood" | "use".
    pub namespace: &'static str,
    pub label: &'static str,
    pub tags: &'static [SystemTag],
}

macro_rules! tags {
    ($(($k:expr, $l:expr)),* $(,)?) => { &[$(SystemTag { key: $k, label: $l }),*] };
}

pub const SYSTEM_TAG_GROUPS: &[SystemTagGroup] = &[
    SystemTagGroup {
        namespace: "style",
        label: "Style",
        tags: tags![
            ("art_deco", "Art Deco"), ("art_nouveau", "Art Nouveau"), ("wood_type", "Wood type"), ("western", "Western"),
            ("retro_vintage", "Retro / Vintage"), ("psychedelic", "Psychedelic"), ("brutalist", "Brutalist"), ("techno", "Techno"),
            ("medieval", "Medieval"), ("tuscan", "Tuscan"), ("distressed", "Distressed"), ("blobby", "Blobby"), ("wacky", "Wacky"),
        ],
    },
    SystemTagGroup {
        namespace: "mood",
        label: "Mood",
        tags: tags![
            ("business", "Business"), ("calm", "Calm"), ("elegant", "Elegant"), ("playful", "Playful"), ("friendly_cute", "Friendly / Cute"),
            ("loud_bold", "Loud / Bold"), ("vintage_retro", "Vintage / Retro"), ("futuristic_tech", "Futuristic / Tech"), ("rugged", "Rugged"),
            ("artistic", "Artistic"), ("formal", "Formal"), ("handmade", "Handmade"),
        ],
    },
    SystemTagGroup {
        namespace: "use",
        label: "Use",
        tags: tags![("editorial", "Editorial"), ("logo_branding", "Logo / Branding"), ("packaging", "Packaging"), ("signage", "Signage")],
    },
];

/// Tên tag lưu trong user.db: `namespace:key`.
pub fn tag_name(namespace: &str, key: &str) -> String {
    format!("{namespace}:{key}")
}

/// `Some(true)` = tên hợp lệ trong danh mục; `Some(false)` = dùng tiền tố dành riêng nhưng không có trong danh mục;
/// `None` = tag người dùng thường (không có tiền tố dành riêng).
pub fn check_name(name: &str) -> Option<bool> {
    let (ns, key) = name.split_once(':')?;
    let g = SYSTEM_TAG_GROUPS.iter().find(|g| g.namespace.eq_ignore_ascii_case(ns.trim()))?;
    Some(g.tags.iter().any(|t| t.key.eq_ignore_ascii_case(key.trim())))
}

/// Tên chuẩn hoá (chữ thường, không khoảng trắng thừa) cho tag hệ thống hợp lệ.
pub fn canonical_name(name: &str) -> Option<String> {
    let (ns, key) = name.split_once(':')?;
    let g = SYSTEM_TAG_GROUPS.iter().find(|g| g.namespace.eq_ignore_ascii_case(ns.trim()))?;
    let t = g.tags.iter().find(|t| t.key.eq_ignore_ascii_case(key.trim()))?;
    Some(tag_name(g.namespace, t.key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_well_formed() {
        let total: usize = SYSTEM_TAG_GROUPS.iter().map(|g| g.tags.len()).sum();
        assert_eq!(SYSTEM_TAG_GROUPS[1].tags.len(), 12, "12 mood tag");
        assert!(total >= 29);
        let mut seen = std::collections::HashSet::new();
        for g in SYSTEM_TAG_GROUPS {
            for t in g.tags {
                assert!(seen.insert(tag_name(g.namespace, t.key)), "trùng {}", t.key);
                assert!(t.key.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{}", t.key);
                assert!(!t.label.is_empty());
            }
        }
    }

    #[test]
    fn name_checks() {
        assert_eq!(check_name("style:art_deco"), Some(true));
        assert_eq!(check_name("Mood: Calm "), Some(true));
        assert_eq!(check_name("mood:spooky"), Some(false));
        assert_eq!(check_name("tet:2027"), None, "namespace lạ là tag người dùng");
        assert_eq!(check_name("plain"), None);
        assert_eq!(canonical_name("Style: ART_DECO"), Some("style:art_deco".into()));
        assert_eq!(canonical_name("style:nope"), None);
    }
}
