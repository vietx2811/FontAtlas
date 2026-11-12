mod common;
use std::collections::{BTreeMap, BTreeSet};

use common::*;
use fontatlas_eval::taxonomy_v2::{
    CLASSES, KNOWN_GAPS, Pred, Quality, Truth, load_labels, map_old_classifier, report, subclasses,
    truths,
};

fn labels() -> Vec<fontatlas_eval::taxonomy_v2::Label> {
    load_labels(&eval_dir().join("taxonomy_v2_labels.csv")).unwrap()
}

#[test]
fn label_table_uses_only_defined_classes_and_subclasses() {
    for l in labels() {
        assert!(
            CLASSES.contains(&l.class.as_str()),
            "{}: class {}",
            l.family,
            l.class
        );
        if let Some(s) = &l.subclass {
            assert!(
                subclasses(&l.class).contains(&s.as_str()),
                "{}: subclass {}/{}",
                l.family,
                l.class,
                s
            );
        }
        assert_eq!(l.license, "ofl", "{}: chỉ họ OFL", l.family);
        if l.source == fontatlas_eval::taxonomy_v2::Source::GfTag && l.quality == Quality::Gf {
            assert!(
                !l.gf_tags.is_empty(),
                "{}: source GF tag mà không có tag",
                l.family
            );
        }
    }
}

#[test]
fn every_subclass_has_five_families_except_the_documented_gaps() {
    let mut n: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for l in labels() {
        if let Some(s) = l.subclass {
            n.entry((l.class, s)).or_default().insert(l.family);
        }
    }
    let mut thin: BTreeSet<(String, String)> = BTreeSet::new();
    for c in CLASSES {
        for s in subclasses(c) {
            if n.get(&(c.to_string(), s.to_string()))
                .map_or(0, |v| v.len())
                < 5
            {
                thin.insert((c.to_string(), s.to_string()));
            }
        }
    }
    let known: BTreeSet<(String, String)> = KNOWN_GAPS
        .iter()
        .map(|(c, s)| (c.to_string(), s.to_string()))
        .collect();
    assert_eq!(
        thin, known,
        "ô thiếu họ phải khớp KNOWN_GAPS (và README giải thích)"
    );
}

#[test]
fn a_family_keeps_one_class_and_merges_subclasses() {
    let t = truths(&labels()).unwrap();
    let jb = t.iter().find(|t| t.family == "JetBrains Mono").unwrap();
    assert_eq!(jb.class, "mono");
    let subs: BTreeSet<&str> = jb.subs.iter().map(|(s, _)| s.as_str()).collect();
    assert_eq!(subs, BTreeSet::from(["sans_mono", "coding"]));
    // một họ không được xuất hiện hai lần
    let names: BTreeSet<&str> = t.iter().map(|t| t.family.as_str()).collect();
    assert_eq!(names.len(), t.len());
}

#[test]
fn conflicting_classes_for_one_family_are_rejected() {
    let mut l = labels();
    let mut x = l[0].clone();
    x.class = if x.class == "sans" {
        "serif".into()
    } else {
        "sans".into()
    };
    x.subclass = None;
    l.push(x);
    assert!(truths(&l).is_err());
}

#[test]
fn fonts_manifest_lists_exactly_the_families_in_the_table() {
    let manifest = include_str!("../../../../../docs/team/eval/taxonomy_v2/fonts.sha256");
    let in_manifest: BTreeSet<String> = manifest
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .map(|p| p.split('/').take(2).collect::<Vec<_>>().join("/"))
        .collect();
    let in_table: BTreeSet<String> = labels()
        .iter()
        .map(|l| format!("{}/{}", l.license, l.gf_dir))
        .collect();
    assert_eq!(in_manifest, in_table);
    for line in manifest.lines() {
        let h = line.split_whitespace().next().unwrap();
        assert!(
            h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()),
            "{line}"
        );
    }
}

#[test]
fn small_foot_serif_fixture_group_has_five_ofl_serifs() {
    let g: Vec<_> = labels()
        .into_iter()
        .filter(|l| l.boundary.as_deref() == Some("small_foot_serif"))
        .collect();
    assert!(g.len() >= 5, "{} họ", g.len());
    assert!(g.iter().all(|l| l.class == "serif" && l.license == "ofl"));
}

#[test]
fn old_classifier_maps_to_new_taxonomy() {
    let p = |c: &str, s: Option<&str>, t: &[&str]| {
        let t: Vec<String> = t.iter().map(|x| x.to_string()).collect();
        map_old_classifier(c, s, &t)
    };
    let e = |c: &str, s: Option<&str>| Pred {
        class: c.into(),
        subclass: s.map(Into::into),
    };
    assert_eq!(p("serif", Some("slab"), &[]), e("slab", None));
    assert_eq!(
        p("serif", Some("humanist_oldstyle"), &[]),
        e("serif", Some("venetian_oldstyle"))
    );
    assert_eq!(
        p("serif", Some("glyphic"), &[]),
        e("serif", Some("glyphic"))
    );
    assert_eq!(
        p("sans", Some("geometric"), &[]),
        e("sans", Some("geometric"))
    );
    assert_eq!(p("sans", None, &["rounded"]), e("sans", Some("rounded")));
    assert_eq!(
        p("script", Some("formal"), &[]),
        e("script", Some("formal_copperplate"))
    );
    assert_eq!(
        p("script", Some("casual_brush"), &[]),
        e("script", Some("brush"))
    );
    assert_eq!(
        p("script", Some("handwritten"), &[]),
        e("handwriting", Some("casual_print"))
    );
    assert_eq!(
        p("mono", Some("serif_mono"), &[]),
        e("mono", Some("serif_mono"))
    );
    assert_eq!(p("blackletter", None, &[]), e("blackletter", None));
    assert_eq!(p("uncertain", None, &[]), e("uncertain", None));
    // trait cấu trúc thắng class cũ
    assert_eq!(
        p("sans", Some("geometric"), &["stencil"]),
        e("display", Some("stencil"))
    );
    assert_eq!(
        p("serif", None, &["pixel"]),
        e("display", Some("pixel_bitmap"))
    );
    assert_eq!(
        p("sans", None, &["inline_outline"]),
        e("display", Some("inline_outline"))
    );
}

fn t(family: &str, class: &str, subs: &[(&str, Quality)], boundary: Option<&str>) -> Truth {
    Truth {
        family: family.into(),
        gf_dir: family.to_lowercase(),
        class: class.into(),
        subs: subs.iter().map(|(s, q)| (s.to_string(), *q)).collect(),
        secondary: None,
        quality: subs.iter().map(|x| x.1).max().unwrap_or(Quality::Gf),
        boundary: boundary.map(Into::into),
    }
}

fn pr(c: &str, s: Option<&str>) -> Pred {
    Pred {
        class: c.into(),
        subclass: s.map(Into::into),
    }
}

#[test]
fn report_counts_class_and_subclass_confusions() {
    let rows = vec![
        (
            t("A", "serif", &[("didone", Quality::Gf)], None),
            pr("serif", Some("didone")),
        ),
        (
            t("B", "serif", &[("didone", Quality::Gf)], None),
            pr("serif", Some("transitional")),
        ),
        (
            t(
                "C",
                "serif",
                &[("glyphic", Quality::Visual)],
                Some("small_serif_risk"),
            ),
            pr("sans", Some("geometric")),
        ),
        (
            t("D", "sans", &[("humanist", Quality::Gf)], None),
            pr("uncertain", None),
        ),
        (
            t("E", "script", &[("signature", Quality::Weak)], None),
            pr("script", Some("brush")),
        ),
    ];
    let md = report(&rows);
    // class: gate rows A,B,C,D → 2 đúng, 1 sai (C), 1 Uncertain (D)
    assert!(md.contains("| 2 | 1 | 1 |"), "{md}");
    // cột: serif slab sans script handwriting mono display blackletter symbol uncertain
    assert!(
        md.contains("| serif | 2 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |"),
        "{md}"
    );
    assert!(
        md.contains("| sans | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |"),
        "{md}"
    );
    // subclass didone: A đúng, B nhầm sang transitional; glyphic bị sai class
    assert!(
        md.contains("| serif/didone | 2 | 1 | transitional×1 |"),
        "{md}"
    );
    assert!(
        md.contains("| serif/glyphic | 1 | 0 | sans/geometric×1 |"),
        "{md}"
    );
    // boundary
    assert!(
        md.contains("| small_serif_risk | C | serif | sans | ✗ |"),
        "{md}"
    );
    // dòng weak không vào cổng nhưng được nhắc
    assert!(md.contains("weak"), "{md}");
}
