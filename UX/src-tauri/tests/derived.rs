//! G7b: bản sao đổi tên của VF trùng tên với static — chỉ khi trùng thật, gốc nguyên vẹn, idempotent, vào Library/Derived hoặc <data>/derived.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::derived::{DeriveError, derive_renamed_copy, derived_from, is_derived};
use app_lib::services::{boot, duplicates, library, organized, sources};
use app_lib::state::{AppDirs, AppState};

fn core_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn scanned(root: &Path, names: &[&str]) -> (Arc<AppState>, PathBuf) {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in names {
        std::fs::copy(core_fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    (st, dir)
}

fn hash_of(st: &AppState, family: &str) -> [u8; 32] {
    let q = FaceQueryDto { text: Some(family.into()), limit: 50, ..Default::default() };
    let r = library::query_faces(st, &q).unwrap().items.into_iter().find(|r| r.family == family).unwrap();
    fontatlas_store::FaceKey::from_hex(&r.hash_hex, 0).unwrap().hash
}

/// Dựng một cặp trùng tên PostScript: file static (Roboto) được gán PS name của VF Oswald trong DB.
fn make_clash(st: &AppState) {
    let ps: String = st
        .with_store(|s| Ok(s.conn().query_row("SELECT postscript_name FROM faces WHERE family_name='Oswald'", [], |r| r.get(0)).unwrap()))
        .unwrap();
    st.with_store(|s| {
        s.conn().execute("UPDATE faces SET postscript_name=?1 WHERE family_name='Roboto'", [&ps]).unwrap();
        Ok(())
    })
    .unwrap();
}

#[test]
fn derives_a_renamed_copy_only_for_a_real_clash_without_touching_the_original() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["oswald_vf.ttf", "roboto.ttf"]);
    let (vf, stat) = (hash_of(&st, "Oswald"), hash_of(&st, "Roboto"));
    let before = std::fs::read(dir.join("oswald_vf.ttf")).unwrap();

    // chưa trùng ⇒ không tạo
    assert!(matches!(derive_renamed_copy(&st, &vf), Err(DeriveError::NoClash)));
    assert!(matches!(derive_renamed_copy(&st, &stat), Err(DeriveError::NotVariable)));
    assert!(matches!(derive_renamed_copy(&st, &[7; 32]), Err(DeriveError::NotInLibrary)));
    assert!(!d.path().join("data/derived").exists(), "không tạo gì khi không trùng");

    make_clash(&st);
    let (key, c) = derive_renamed_copy(&st, &vf).unwrap();
    assert!(c.created && !c.license_restricted);
    assert!(c.path.starts_with(&*d.path().join("data/derived").to_string_lossy()), "ngoài Organized Library: <data>/derived: {}", c.path);
    assert!(c.path.ends_with("oswald_vf-VF.ttf"));
    assert_eq!(std::fs::read(dir.join("oswald_vf.ttf")).unwrap(), before, "file gốc nguyên vẹn");
    assert!(!std::fs::read_dir(d.path().join("data/derived")).unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".partial")));

    // bản sao là một font bình thường của thư viện, nối với gốc qua sổ
    assert!(is_derived(&st, &key.hash) && !is_derived(&st, &vf));
    assert_eq!(derived_from(&st, &key.hash), Some(vf));
    let q = FaceQueryDto { limit: 50, ..Default::default() };
    let fams: Vec<String> = library::query_faces(&st, &q).unwrap().items.into_iter().map(|r| r.family).collect();
    assert!(fams.contains(&"Oswald VF".to_string()) && fams.len() == 3, "{fams:?}");
    // sau khi nạp: tên mới không trùng static, Duplicates không coi là bản trùng
    let ps_new: String = st
        .with_store(|s| Ok(s.conn().query_row("SELECT postscript_name FROM faces WHERE family_name='Oswald VF'", [], |r| r.get(0)).unwrap()))
        .unwrap();
    assert!(ps_new.starts_with("OswaldVF-"), "{ps_new}");
    assert_eq!(st.with_store(|s| Ok(s.name_clashes().unwrap().len())).unwrap(), 1, "chỉ còn cặp gốc VF↔static; bản sao không trùng");
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    // (cặp gốc VF↔static cùng PS name vẫn là một nhóm "versions" như trước; bản sao đổi tên không được lọt vào nhóm nào)
    let groups = duplicates::duplicate_groups(&st, &q).unwrap();
    assert!(groups.iter().all(|g| g.faces.iter().all(|m| m.row.family != "Oswald VF")), "bản sao lọt vào nhóm trùng lặp");

    // chi tiết font: bản sao biết nó là bản sao của ai; font thường thì không
    {
        use app_lib::dto::analysis::LangDto;
        use app_lib::services::analysis;
        assert!(wait_rebuilt(&st, Duration::from_secs(60)));
        let det = analysis::get_face(&st, &FaceKeyDto::from(&key), LangDto::Vi).unwrap();
        let from = det.derived_from.expect("bản sao có derivedFrom");
        assert_eq!(from.face_key.hash_hex, FaceKeyDto::from(&fontatlas_store::FaceKey { hash: vf, face_index: 0 }).hash_hex);
        assert!(from.name.starts_with("Oswald"), "{}", from.name);
        let orig = analysis::get_face(&st, &from.face_key, LangDto::Vi).unwrap();
        assert!(orig.derived_from.is_none());
    }
    // idempotent: lần hai trả lại đúng bản đã có
    let (key2, c2) = derive_renamed_copy(&st, &vf).unwrap();
    assert!(!c2.created);
    assert_eq!((key2, c2.path), (key, c.path));
    assert_eq!(std::fs::read_dir(d.path().join("data/derived")).unwrap().count(), 1);
}

#[test]
fn with_an_organized_library_the_copy_goes_to_library_derived_and_survives_a_resort() {
    let d = tempfile::tempdir().unwrap();
    let (st, _dir) = scanned(d.path(), &["oswald_vf.ttf", "roboto.ttf"]);
    make_clash(&st);
    let root = d.path().join("Organized");
    organized::create(&st, root.to_str().unwrap()).unwrap();
    let vf = hash_of(&st, "Oswald");
    let (_k, c) = derive_renamed_copy(&st, &vf).unwrap();
    assert!(c.path.starts_with(&*root.join("Library/Derived").to_string_lossy()), "{}", c.path);
    assert!(Path::new(&c.path).is_file());
    assert!(!d.path().join("data/derived").exists());
    // "Sắp xếp lại" chỉ đụng file có trong sổ chép: Derived/ nguyên chỗ
    assert_eq!(organized::resort_preview(&st).unwrap().total, 0);
}

#[test]
fn unsupported_containers_are_refused_with_a_stable_code() {
    let e: app_lib::error::CmdError = DeriveError::UnsupportedContainer("ttc").into();
    assert_eq!(e.code, "unsupported_container");
    assert_eq!(e.detail.unwrap()["container"], "ttc");
    assert_eq!(app_lib::error::CmdError::from(DeriveError::NotDownloaded).code, "not_downloaded");
    assert_eq!(app_lib::error::CmdError::from(DeriveError::DiskFull).code, "disk_full");
}
