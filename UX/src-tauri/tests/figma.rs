//! Nhập font từ Figma: parse_figma_import (thuần) + resolve_fonts (khớp tên với thư viện). Không đụng font thật.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::figma::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, figma, sources};
use app_lib::state::{AppDirs, AppState};
use base64::Engine;

const SIX: [&str; 6] = ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "libre_baskerville.ttf", "source_code_pro.ttf", "public_sans_vf.ttf"];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn lib(root: &Path, copies: &[&str]) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for (i, n) in copies.iter().enumerate() {
        let mut bytes = std::fs::read(fixture(n)).unwrap();
        if i >= SIX.len() || copies[..i].contains(n) {
            bytes.extend_from_slice(&(i as u64).to_le_bytes()); // bản trùng tên nhưng khác băm
        }
        std::fs::write(dir.join(format!("{i}_{n}")), bytes).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

fn name(f: &str, s: &str) -> FontNameDto {
    FontNameDto { family: f.into(), style: s.into() }
}

fn link(json: &str) -> String {
    format!("fontatlas://fonts?v=1&d={}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json))
}

const JSON: &str = r#"{"v":1,"src":"figma","file":"Menu v3","fonts":[["Playfair Display","Bold",0],["Inter","Regular",1]]}"#;

#[test]
fn parse_accepts_plain_json_and_link_and_gives_same_items() {
    let a = figma::parse_figma_import(JSON).unwrap();
    let b = figma::parse_figma_import(&link(JSON)).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.file.as_deref(), Some("Menu v3"));
    assert_eq!(a.items.len(), 2);
    assert_eq!((a.items[1].family.as_str(), a.items[1].style.as_str(), a.items[1].figma_missing), ("Inter", "Regular", true));
}

#[test]
fn parse_errors_carry_codes_not_prose() {
    let code = |s: &str| figma::parse_figma_import(s).unwrap_err().code;
    assert_eq!(code("   "), "import_empty");
    assert_eq!(code("không phải json"), "import_format");
    assert_eq!(code("fontatlas://fonts?v=1&d=@@@"), "import_format");
    assert_eq!(code("fontatlas://other?d=e30"), "import_format");
    assert_eq!(code(r#"{"v":9,"fonts":[]}"#), "import_version");
    let many: Vec<String> = (0..201).map(|i| format!(r#"["F{i}","Regular",0]"#)).collect();
    assert_eq!(code(&format!(r#"{{"v":1,"fonts":[{}]}}"#, many.join(","))), "import_too_many_fonts");
    let long = "x".repeat(129);
    assert_eq!(code(&format!(r#"{{"v":1,"fonts":[["{long}","Regular",0]]}}"#)), "import_format");
    assert_eq!(code(&format!("{}{}", " ".repeat(70_000), JSON)), "import_too_large");
}

#[test]
fn parse_drops_unknown_fields_and_keeps_vietnamese_names() {
    let r = figma::parse_figma_import(r#"{"v":1,"evil":"x","fonts":[["Be Vietnam Pro","Semi Bold",0]]}"#).unwrap();
    assert_eq!(r.items[0].family, "Be Vietnam Pro");
    assert_eq!(r.file, None);
}

#[test]
fn resolve_exact_family_only_none_in_input_order() {
    let d = tempfile::tempdir().unwrap();
    let st = lib(d.path(), &SIX);
    let out = figma::resolve_fonts(&st, &[name("Playfair Display", "Regular"), name("playfair  display", "Bold"), name("Inter", "Regular"), name("ROBOTO", "regular")]).unwrap();
    assert_eq!(out.iter().map(|m| m.index).collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert_eq!(out[0].status, MatchStatusDto::Exact);
    assert_eq!(out[0].faces.len(), 1);
    assert_eq!(out[1].status, MatchStatusDto::FamilyOnly, "có họ nhưng không có Bold: không được coi là exact");
    assert!(!out[1].faces.is_empty(), "family_only trả các style của họ để UI nêu rõ");
    assert_eq!(out[2].status, MatchStatusDto::None);
    assert!(out[2].faces.is_empty());
    assert_eq!(out[3].status, MatchStatusDto::Exact, "chuẩn hoá hoa thường/khoảng trắng");
}

#[test]
fn resolve_two_copies_is_ambiguous_and_never_picks_for_the_user() {
    let d = tempfile::tempdir().unwrap();
    let st = lib(d.path(), &["roboto.ttf", "roboto.ttf", "open_sans.ttf"]);
    let out = figma::resolve_fonts(&st, &[name("Roboto", "Regular")]).unwrap();
    assert_eq!(out[0].status, MatchStatusDto::Ambiguous);
    assert_eq!(out[0].faces.len(), 2);
}

#[test]
fn resolve_empty_and_too_many_inputs() {
    let d = tempfile::tempdir().unwrap();
    let st = lib(d.path(), &SIX);
    assert!(figma::resolve_fonts(&st, &[]).unwrap().is_empty());
    let many: Vec<FontNameDto> = (0..201).map(|i| name(&format!("F{i}"), "Regular")).collect();
    assert_eq!(figma::resolve_fonts(&st, &many).unwrap_err().code, "import_too_many_fonts");
}

#[test]
fn resolve_200_items_is_fast_on_a_360_face_library() {
    let d = tempfile::tempdir().unwrap();
    let names: Vec<&str> = (0..360).map(|i| SIX[i % 6]).collect();
    let st = lib(d.path(), &names);
    let items: Vec<FontNameDto> = (0..200).map(|i| name(["Roboto", "Open Sans", "Nope"][i % 3], "Regular")).collect();
    let t = std::time::Instant::now();
    let out = figma::resolve_fonts(&st, &items).unwrap();
    assert!(t.elapsed() < Duration::from_millis(50), "200 mục mất {:?}", t.elapsed());
    assert_eq!(out.len(), 200);
}
