//! Ground truth phân loại từ google/fonts: `tags/all/families.csv` (tag điểm ≥ 50) + `category` trong METADATA.pb.
use std::collections::{BTreeMap, BTreeSet};

use fontatlas_core::classify::Class;

pub const MIN_SCORE: u32 = 50;

/// Đáp án: `primary` (dòng của ma trận nhầm lẫn), `accept` (mọi class chấp nhận là đúng), `sub` (nếu rõ).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Truth {
    pub primary: Class,
    pub accept: BTreeSet<Class>,
    /// Các subclass chấp nhận (rỗng = không đánh giá subclass).
    pub sub: BTreeSet<&'static str>,
    /// Tag hình thái từ tag GF (stencil, inline_outline, rounded, fat_face, pixel, decorative).
    pub traits: BTreeSet<&'static str>,
    /// GF xếp vào DISPLAY theo mục đích dùng.
    pub display_purpose: bool,
    /// Họ DISPLAY không có tag hình dáng nào: không biết đáp án class → loại khỏi bảng class.
    pub shape_unknown: bool,
}

/// `family,,/Tag/Path,score` → (family, tag, score). Tên họ có thể chứa dấu phẩy nên cắt từ phải.
pub fn parse_tag_line(line: &str) -> Option<(String, String, u32)> {
    let mut it = line.rsplitn(3, ',');
    let score: u32 = it.next()?.trim().parse().ok()?;
    let tag = it.next()?.to_string();
    let rest = it.next()?;
    if !tag.starts_with('/') {
        return None;
    }
    Some((rest.trim_end_matches(',').to_string(), tag, score))
}

pub fn load_tags(csv: &str) -> BTreeMap<String, Vec<(String, u32)>> {
    let mut m: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    for l in csv.lines() {
        if let Some((f, t, s)) = parse_tag_line(l)
            && s >= MIN_SCORE
        {
            m.entry(f).or_default().push((t, s));
        }
    }
    m
}

fn tag_class(t: &str) -> Option<Class> {
    let top = t.split('/').nth(1)?;
    Some(match top {
        "Serif" | "Slab" => Class::Serif,
        "Sans" => Class::Sans,
        "Script" => Class::Script,
        "Monospace" => Class::Mono,
        _ => return t.eq_ignore_ascii_case("/Theme/Blackletter").then_some(Class::Blackletter),
    })
}

fn tag_sub(t: &str) -> Option<&'static str> {
    Some(match t {
        "/Serif/Humanist Venetian" | "/Serif/Old Style Garalde" => "humanist_oldstyle",
        "/Serif/Transitional" => "transitional",
        "/Serif/Didone" | "/Serif/Modern" | "/Serif/Scotch" => "didone",
        "/Slab/Humanist" | "/Slab/Geometric" | "/Slab/Clarendon" => "slab",
        "/Sans/Humanist" => "humanist",
        "/Sans/Geometric" => "geometric",
        "/Sans/Grotesque" => "grotesque",
        "/Sans/Neo Grotesque" => "neo_grotesque",
        "/Script/Formal" => "formal",
        "/Script/Handwritten" => "handwritten",
        "/Theme/Brush" => "casual_brush",
        _ => return None,
    })
}

fn tag_trait(t: &str) -> Option<&'static str> {
    Some(match t {
        "/Theme/Stencil" => "stencil",
        "/Theme/Inline" | "/Theme/Shaded" => "inline_outline",
        "/Sans/Rounded" => "rounded",
        "/Serif/Fat Face" => "fat_face",
        "/Theme/Pixel" => "pixel",
        "/Theme/Wacky" | "/Theme/Distressed" | "/Theme/Woodtype" | "/Theme/Blobby" | "/Theme/Tuscan" | "/Theme/Art Deco" | "/Theme/Art Nouveau" => "decorative",
        _ => return None,
    })
}

/// `category` của METADATA.pb: SERIF | SANS_SERIF | DISPLAY | HANDWRITING | MONOSPACE.
/// Display là mục đích dùng, không phải class: họ DISPLAY được xếp theo tag hình dáng (nếu có).
pub fn truth(category: &str, tags: &[(String, u32)]) -> Option<Truth> {
    let cat = match category {
        "SERIF" => Some(Class::Serif),
        "SANS_SERIF" => Some(Class::Sans),
        "MONOSPACE" => Some(Class::Mono),
        "HANDWRITING" => Some(Class::Script),
        "DISPLAY" => None,
        _ => return None,
    };
    let tag_classes: BTreeSet<Class> = tags.iter().filter_map(|(t, _)| tag_class(t)).collect();
    let traits: BTreeSet<&'static str> = tags.iter().filter_map(|(t, _)| tag_trait(t)).collect();
    let display_purpose = cat.is_none();
    let mut accept: BTreeSet<Class> = BTreeSet::new();
    let primary;
    let mut shape_unknown = false;
    if tag_classes.contains(&Class::Blackletter) {
        primary = Class::Blackletter;
        accept.insert(Class::Blackletter);
    } else if cat.is_none() {
        // DISPLAY: chấp nhận mọi class hình dáng được tag; không có tag → không biết đáp án
        accept.extend(tag_classes.iter().copied());
        match tag_classes.iter().next() {
            Some(c) => primary = *c,
            None => {
                primary = Class::Uncertain;
                shape_unknown = true;
            }
        }
    } else if cat == Some(Class::Mono) || tag_classes.contains(&Class::Mono) {
        primary = Class::Mono;
        accept.insert(Class::Mono);
    } else {
        let cat = cat?;
        // tag khác class với category = mâu thuẫn → bỏ
        if tag_classes.iter().any(|c| *c != cat) {
            return None;
        }
        primary = cat;
        accept.insert(cat);
    }
    let subs: BTreeSet<&'static str> = tags.iter().filter_map(|(t, _)| tag_sub(t)).collect();
    // Có đúng một nhãn con (cùng class) thì mới đánh giá subclass
    let sub = if primary == Class::Mono {
        // Mono của GF mang tag hình dạng của họ gốc: Sans → sans_mono; Serif/Slab (không có Sans) → serif_mono
        let has = |p: &str| tags.iter().any(|(t, _)| t.starts_with(p));
        match (has("/Sans/"), has("/Serif/") || has("/Slab/")) {
            (true, false) => BTreeSet::from(["sans_mono"]),
            (false, true) => BTreeSet::from(["serif_mono"]),
            _ => BTreeSet::new(),
        }
    } else if subs.len() == 1 && !shape_unknown {
        subs
    } else {
        BTreeSet::new()
    };
    Some(Truth { primary, accept, sub, traits, display_purpose, shape_unknown })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(v: &[&str]) -> Vec<(String, u32)> {
        v.iter().map(|x| (x.to_string(), 100)).collect()
    }

    #[test]
    fn parses_lines_with_commas_in_names() {
        assert_eq!(parse_tag_line("Roboto,,/Sans/Neo Grotesque,100"), Some(("Roboto".into(), "/Sans/Neo Grotesque".into(), 100)));
        assert_eq!(parse_tag_line("Foo, Bar,,/Serif/Didone,60").unwrap().0, "Foo, Bar");
        assert!(parse_tag_line("// comment").is_none());
    }

    #[test]
    fn sans_with_geometric_tag() {
        let g = truth("SANS_SERIF", &t(&["/Sans/Geometric"])).unwrap();
        assert_eq!((g.primary, g.sub.iter().copied().collect::<Vec<_>>()), (Class::Sans, vec!["geometric"]));
    }

    #[test]
    fn conflict_between_category_and_tags_is_dropped() {
        assert!(truth("SERIF", &t(&["/Sans/Humanist"])).is_none());
    }

    #[test]
    fn display_is_purpose_not_class() {
        let g = truth("DISPLAY", &t(&["/Sans/Grotesque"])).unwrap();
        assert!(g.display_purpose && g.accept.contains(&Class::Sans) && !g.shape_unknown);
        let u = truth("DISPLAY", &t(&["/Theme/Wacky"])).unwrap();
        assert!(u.shape_unknown && u.traits.contains("decorative"));
    }

    #[test]
    fn rounded_sans_is_sans_with_rounded_trait() {
        let g = truth("SANS_SERIF", &t(&["/Sans/Rounded"])).unwrap();
        assert_eq!(g.accept.iter().copied().collect::<Vec<_>>(), vec![Class::Sans]);
        assert!(g.traits.contains("rounded"));
    }

    #[test]
    fn mono_subclass_comes_from_shape_family_tags() {
        assert_eq!(truth("MONOSPACE", &t(&["/Sans/Geometric"])).unwrap().sub.iter().copied().collect::<Vec<_>>(), vec!["sans_mono"]);
        assert_eq!(truth("MONOSPACE", &t(&["/Slab/Humanist"])).unwrap().sub.iter().copied().collect::<Vec<_>>(), vec!["serif_mono"]);
        assert!(truth("MONOSPACE", &t(&[])).unwrap().sub.is_empty());
    }

    #[test]
    fn blackletter_tag_wins() {
        assert_eq!(truth("DISPLAY", &t(&["/Theme/Blackletter"])).unwrap().primary, Class::Blackletter);
    }
}
