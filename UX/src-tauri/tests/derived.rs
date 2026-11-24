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
    assert!(Path::new(&c.path).starts_with(d.path().join("data/derived")), "ngoài Organized Library: <data>/derived: {}", c.path);
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
    assert!(Path::new(&c.path).starts_with(root.join("Library/Derived")), "{}", c.path);
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

// ---------- gỡ bản sao + hoàn tác ----------

use app_lib::dto::organized::RestoreViaDto;
use app_lib::services::derived::{derived_copies_of, remove_derived_copy_with, restore_derived_copy};

/// "Thùng rác" giả: chuyển file sang thư mục riêng (như macOS trả đường dẫn mới trong Thùng rác).
fn fake_trash(dir: PathBuf) -> impl Fn(&Path) -> Result<PathBuf, String> {
    move |p: &Path| {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let to = dir.join(p.file_name().unwrap());
        std::fs::rename(p, &to).map_err(|e| e.to_string())?;
        Ok(to)
    }
}

fn clash_with_copy() -> (tempfile::TempDir, Arc<AppState>, [u8; 32], fontatlas_store::FaceKey, String) {
    let d = tempfile::tempdir().unwrap();
    let (st, _dir) = scanned(d.path(), &["oswald_vf.ttf", "roboto.ttf"]);
    make_clash(&st);
    let vf = hash_of(&st, "Oswald");
    let (k, c) = derive_renamed_copy(&st, &vf).unwrap();
    (d, st, vf, k, c.path)
}

fn count_faces(st: &AppState) -> usize {
    library::query_faces(st, &FaceQueryDto { limit: 50, ..Default::default() }).unwrap().total as usize
}

#[test]
fn removing_a_copy_trashes_it_cleans_the_ledger_and_library_and_never_touches_the_original() {
    let (d, st, vf, k, path) = clash_with_copy();
    let orig_bytes = std::fs::read(d.path().join("Fonts/oswald_vf.ttf")).unwrap();
    assert_eq!(count_faces(&st), 3);
    let trash = d.path().join("FakeTrash");
    let face = FaceKeyDto::from(&k);

    let copies = derived_copies_of(&st, &FaceKeyDto::from(&fontatlas_store::FaceKey { hash: vf, face_index: 0 })).unwrap();
    assert_eq!((copies.len(), copies[0].face.clone(), copies[0].path.clone()), (1, face.clone(), path.clone()));
    assert!(copies[0].name.starts_with("Oswald VF"), "{}", copies[0].name);

    let r = remove_derived_copy_with(&st, &face, &fake_trash(trash.clone())).unwrap();
    assert!(r.trashed && !r.was_active);
    assert!(!Path::new(&path).exists(), "file đã rời thư mục bản sao");
    assert!(trash.join(Path::new(&path).file_name().unwrap()).is_file(), "nằm trong Thùng rác, không bị xoá hẳn");
    assert_eq!(count_faces(&st), 2, "bản sao rời khỏi thư viện");
    assert!(!is_derived(&st, &k.hash) && derived_from(&st, &k.hash).is_none());
    assert!(derived_copies_of(&st, &FaceKeyDto::from(&fontatlas_store::FaceKey { hash: vf, face_index: 0 })).unwrap().is_empty());
    assert_eq!(std::fs::read(d.path().join("Fonts/oswald_vf.ttf")).unwrap(), orig_bytes, "font gốc nguyên vẹn");
    assert_eq!(count_faces(&st), 2);
}

#[test]
fn undo_restores_from_the_trash_or_derives_again_when_the_trash_was_emptied() {
    let (d, st, vf, k, path) = clash_with_copy();
    let trash = d.path().join("FakeTrash");
    let face = FaceKeyDto::from(&k);

    // 1. còn trong Thùng rác → đưa về chỗ cũ
    let r1 = remove_derived_copy_with(&st, &face, &fake_trash(trash.clone())).unwrap();
    let back = restore_derived_copy(&st, r1.removal_id).unwrap();
    assert_eq!(back.via, RestoreViaDto::Trash);
    assert_eq!((back.face.clone(), back.path.clone()), (face.clone(), path.clone()));
    assert!(Path::new(&path).is_file() && is_derived(&st, &k.hash) && count_faces(&st) == 3);
    assert_eq!(restore_derived_copy(&st, r1.removal_id).unwrap_err().code, "already_undone");

    // 2. Thùng rác đã dọn → tạo lại từ font gốc (cùng tên, cùng hash vì cùng nội dung)
    let r2 = remove_derived_copy_with(&st, &face, &fake_trash(trash.clone())).unwrap();
    for e in std::fs::read_dir(&trash).unwrap().flatten() {
        std::fs::remove_file(e.path()).unwrap(); // người dùng "Dọn Thùng rác"
    }
    let again = restore_derived_copy(&st, r2.removal_id).unwrap();
    assert_eq!(again.via, RestoreViaDto::ReDerived);
    assert_eq!(again.face, face, "cùng nội dung ⇒ cùng hash");
    assert!(Path::new(&again.path).is_file() && is_derived(&st, &k.hash) && count_faces(&st) == 3);
    assert_eq!(derived_from(&st, &k.hash), Some(vf));

    // 3. gốc đã không còn trong thư viện và Thùng rác đã dọn: báo lỗi rõ, không tạo bậy
    let r3 = remove_derived_copy_with(&st, &face, &fake_trash(trash.clone())).unwrap();
    std::fs::remove_file(trash.join(Path::new(&path).file_name().unwrap())).unwrap();
    let src_id = sources::list_sources(&st).unwrap()[0].id;
    sources::remove_source(&st, src_id).unwrap();
    assert_eq!(restore_derived_copy(&st, r3.removal_id).unwrap_err().code, "not_in_library");
}

#[test]
fn only_real_copies_inside_the_copy_folder_can_be_removed() {
    let (d, st, vf, k, _path) = clash_with_copy();
    let trash = d.path().join("FakeTrash");
    // font thường: không phải bản sao
    let plain = FaceKeyDto::from(&fontatlas_store::FaceKey { hash: vf, face_index: 0 });
    assert_eq!(remove_derived_copy_with(&st, &plain, &fake_trash(trash.clone())).unwrap_err().code, "not_derived");
    // sổ bị trỏ nhầm vào file người dùng (ngoài thư mục bản sao): từ chối, file nguyên vẹn
    let user_file = d.path().join("Fonts/roboto.ttf");
    let stat = hash_of(&st, "Roboto");
    st.with_store(|s| {
        s.derived_record(&fontatlas_store::DerivedFont { hash: stat, derived_from: k.hash, path: user_file.to_string_lossy().into_owned(), license_restricted: false }).unwrap();
        Ok(())
    })
    .unwrap();
    let e = remove_derived_copy_with(&st, &FaceKeyDto::from(&fontatlas_store::FaceKey { hash: stat, face_index: 0 }), &fake_trash(trash.clone())).unwrap_err();
    assert_eq!(e.code, "not_allowed");
    assert!(user_file.is_file() && !trash.exists(), "không đụng file của người dùng");
}

// ---------- bản sao đang bật phải được tắt trước khi vào Thùng rác ----------

use app_lib::services::activation as act;
use fontatlas_activate::{ActivationEngine, ActivationService, EngineError, FakeEngine, Scope};
use std::collections::HashSet;

struct Shared(Arc<FakeEngine>);
impl ActivationEngine for Shared {
    fn method(&self) -> &'static str { self.0.method() }
    fn register(&self, p: &Path, s: Scope) -> Result<(), EngineError> { self.0.register(p, s) }
    fn unregister(&self, p: &Path, s: Scope) -> Result<(), EngineError> { self.0.unregister(p, s) }
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> { self.0.available_postscript_names() }
    fn providers(&self, ps: &str) -> Result<Vec<PathBuf>, EngineError> { self.0.providers(ps) }
    fn postscript_names_in_file(&self, p: &Path) -> Result<Vec<String>, EngineError> { self.0.postscript_names_in_file(p) }
}

#[test]
fn an_active_copy_is_deactivated_before_it_goes_to_the_trash() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    let dir = d.path().join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in ["oswald_vf.ttf", "roboto.ttf"] {
        std::fs::copy(core_fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    make_clash(&st);
    let vf = hash_of(&st, "Oswald");
    let (k, c) = derive_renamed_copy(&st, &vf).unwrap();
    let face = FaceKeyDto::from(&k);
    let cp = std::fs::canonicalize(&c.path).unwrap();
    eng.set_names_for(&cp, &["OswaldVF-Regular"]); // FakeEngine không đọc file: khai tên PostScript của bản sao
    let rep = act::activate(&st, std::slice::from_ref(&face), None, false, &mut |_| {}).unwrap();
    assert_eq!(rep.failed, 0, "{rep:?}");
    let active = |e: &FakeEngine| e.registrations().iter().filter(|(p, _)| *p == cp).count();
    assert!(active(&eng) > 0, "bản sao đang bật: {:?}", eng.registrations());

    let r = remove_derived_copy_with(&st, &face, &fake_trash(d.path().join("FakeTrash"))).unwrap();
    assert!(r.was_active && r.trashed);
    assert_eq!(active(&eng), 0, "đã tắt trước khi gỡ: không để đăng ký trỏ vào file đã đi");
    assert!(!Path::new(&c.path).exists());
}
