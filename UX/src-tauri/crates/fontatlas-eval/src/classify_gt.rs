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
        "/Theme/Stencil" => "stencil",
        "/Theme/Inline" => "inline_outline",
        "/Serif/Fat Face" => "fat_face",
        _ => return None,
    })
}

/// `category` của METADATA.pb: SERIF | SANS_SERIF | DISPLAY | HANDWRITING | MONOSPACE.
pub fn truth(category: &str, tags: &[(String, u32)]) -> Option<Truth> {
    let cat = match category {
        "SERIF" => Class::Serif,
        "SANS_SERIF" => Class::Sans,
        "MONOSPACE" => Class::Mono,
        "HANDWRITING" => Class::Script,
        "DISPLAY" => Class::Display,
        _ => return None,
    };
    let tag_classes: BTreeSet<Class> = tags.iter().filter_map(|(t, _)| tag_class(t)).collect();
    let mut accept: BTreeSet<Class> = BTreeSet::new();
    let primary;
    if tag_classes.contains(&Class::Blackletter) {
        primary = Class::Blackletter;
        accept.insert(Class::Blackletter);
    } else if cat == Class::Display {
        // Display của GF thường mang cả tag hình dạng (Sans/Serif/Script): chấp nhận cả hai
        primary = Class::Display;
        accept.insert(Class::Display);
        accept.extend(tag_classes.iter().copied());
    } else if cat == Class::Mono || tag_classes.contains(&Class::Mono) {
        primary = Class::Mono;
        accept.insert(Class::Mono);
    } else {
        // SERIF / SANS_SERIF / HANDWRITING: tag khác class với category (trừ Display/Rounded/Fat Face) = mâu thuẫn → bỏ
        let other: Vec<&Class> = tag_classes.iter().filter(|c| **c != cat).collect();
        let only_ok = tags.iter().all(|(t, _)| tag_class(t).is_none_or(|c| c == cat) || t == "/Sans/Rounded" || t == "/Serif/Fat Face");
        if !other.is_empty() && !only_ok {
            return None;
        }
        primary = cat;
        accept.insert(cat);
        if tags.iter().any(|(t, _)| t == "/Sans/Rounded" || t == "/Serif/Fat Face") {
            accept.insert(Class::Display);
        }
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
    } else if subs.len() == 1 && primary != Class::Display {
        subs
    } else {
        BTreeSet::new()
    };
    Some(Truth { primary, accept, sub })
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
    fn display_accepts_shape_class_and_has_no_subclass_eval() {
        let g = truth("DISPLAY", &t(&["/Sans/Grotesque"])).unwrap();
        assert!(g.accept.contains(&Class::Display) && g.accept.contains(&Class::Sans));
        assert!(g.sub.is_empty());
    }

    #[test]
    fn rounded_sans_accepts_display() {
        let g = truth("SANS_SERIF", &t(&["/Sans/Rounded"])).unwrap();
        assert!(g.accept.contains(&Class::Display) && g.accept.contains(&Class::Sans));
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
