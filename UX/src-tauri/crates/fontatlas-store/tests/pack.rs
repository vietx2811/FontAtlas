//! B7 (nguyên mẫu): xuất/nhập gói thư viện trên thư mục tạm.
mod common;
use common::*;
use fontatlas_store::{PACK_FORMAT, SourceKind, Store, StoreError, import_pack, verify_pack};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

fn populated() -> (tempfile::TempDir, PathBuf, Store) {
    let d = tmp();
    let fonts = d.path().join("Organized");
    std::fs::create_dir_all(&fonts).unwrap();
    for n in ["roboto.ttf", "open_sans.ttf"] {
        std::fs::copy(core_fixture(n), fonts.join(n)).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Organized", Some(&fonts)).unwrap();
    s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    let hash: Vec<u8> = s.conn().query_row("SELECT hash FROM files ORDER BY file_id LIMIT 1", [], |r| r.get(0)).unwrap();
    let key = fontatlas_store::FaceKey { hash: hash.clone().try_into().unwrap(), face_index: 0 };
    let tag = s.tag("giữ").unwrap();
    s.tag_faces(tag, &[key]).unwrap();
    s.set_favorite(&key, true).unwrap();
    s.conn().execute("INSERT INTO u.settings(key, value) VALUES ('sample_text', 'Phở bò')", []).ok();
    s.conn()
        .execute(
            "INSERT INTO embeddings(hash,face_index,model_version,source,snapshot_id,dim,dtype,vec,created_at) VALUES (?1,0,'m1','local',NULL,2,'i8',x'0102',0)",
            [&hash[..]],
        )
        .unwrap();
    (d, fonts, s)
}

#[test]
fn export_then_import_on_a_fresh_data_dir_restores_everything_with_remapped_paths() {
    let (d, fonts, s) = populated();
    let pack = d.path().join("pack");
    let m = s.export_pack(&pack).unwrap();
    assert_eq!((m.format, m.counts.files, m.counts.embeddings, m.counts.sources), (PACK_FORMAT, 2, 1, 1));
    assert_eq!(m.source_roots, vec![fonts.to_string_lossy().to_string()]);
    assert_eq!(verify_pack(&pack).unwrap(), m);
    assert!(!pack.join(".writing").exists(), "không để rác tạm");
    drop(s);

    // "cài lại máy": thư viện font chuyển sang chỗ khác, dữ liệu app trống
    let moved = d.path().join("Moved");
    std::fs::rename(&fonts, &moved).unwrap();
    let fresh = d.path().join("newdb");
    let rep = import_pack(&pack, &fresh, &[(fonts.to_string_lossy().into(), moved.to_string_lossy().into())]).unwrap();
    assert_eq!((rep.remapped_locations, rep.remapped_sources), (2, 1));

    let (mut s2, open) = Store::open(&fresh).unwrap();
    assert!(!open.library_recreated);
    assert_eq!(count(&s2, "SELECT count(*) FROM faces"), m.counts.faces as i64);
    assert_eq!(count(&s2, "SELECT count(*) FROM embeddings"), 1);
    assert_eq!(count(&s2, "SELECT count(*) FROM u.face_tags"), 1);
    assert_eq!(count(&s2, "SELECT count(*) FROM u.favorites"), 1);
    assert_eq!(s2.sources().unwrap()[0].path.as_deref(), Some(moved.to_string_lossy().as_ref()));
    assert_eq!(count(&s2, &format!("SELECT count(*) FROM file_locations WHERE path LIKE '{}%'", moved.display())), 2);
    // quét lại nguồn đã đổi chỗ: mọi thứ khớp, không phân tích lại, không mất gì
    let src = s2.sources().unwrap()[0].id;
    let st = s2.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!((st.unchanged, st.analyzed, st.missing), (2, 0, 0));
    fts_ok(&s2);
}

#[test]
fn export_is_a_consistent_read_only_snapshot_and_refuses_a_non_empty_target() {
    let (d, _f, s) = populated();
    let pack = d.path().join("pack");
    let before = count(&s, "SELECT count(*) FROM files");
    s.export_pack(&pack).unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM files"), before, "xuất không đổi gì");
    assert!(matches!(s.export_pack(&pack), Err(StoreError::Conflict(_))), "không ghi đè gói có sẵn");
}

#[test]
fn a_corrupted_or_incomplete_pack_is_rejected_before_anything_is_written() {
    let (d, _f, s) = populated();
    let pack = d.path().join("pack");
    s.export_pack(&pack).unwrap();
    drop(s);
    let target = d.path().join("target");

    // lật một byte của library.db trong gói
    let lib = pack.join("library.db");
    let mut b = std::fs::read(&lib).unwrap();
    let n = b.len() / 2;
    b[n] ^= 0xff;
    std::fs::write(&lib, b).unwrap();
    assert!(matches!(import_pack(&pack, &target, &[]), Err(StoreError::Invalid(_))));
    assert!(!target.join("library.db").exists() && !target.join("user.db").exists());

    // thiếu manifest = gói chưa xuất xong
    std::fs::remove_file(pack.join("manifest.json")).unwrap();
    assert!(matches!(verify_pack(&pack), Err(StoreError::Invalid(_))));
}

#[test]
fn a_newer_pack_is_refused_and_existing_data_is_never_overwritten() {
    let (d, _f, s) = populated();
    let pack = d.path().join("pack");
    s.export_pack(&pack).unwrap();
    let existing = d.path().join("db"); // dữ liệu app đang dùng
    assert!(matches!(import_pack(&pack, &existing, &[]), Err(StoreError::Conflict(_))));
    let user_before = std::fs::read(existing.join("user.db")).unwrap_or_default();
    drop(s);
    let _ = user_before;

    // manifest tự nhận là định dạng mới hơn
    let mp = pack.join("manifest.json");
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&mp).unwrap()).unwrap();
    v["format"] = serde_json::json!(PACK_FORMAT + 1);
    std::fs::write(&mp, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(matches!(verify_pack(&pack), Err(StoreError::NewerSchema { db: "pack", .. })));
    v["format"] = serde_json::json!(PACK_FORMAT);
    v["user_schema"] = serde_json::json!(999);
    std::fs::write(&mp, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(matches!(verify_pack(&pack), Err(StoreError::NewerSchema { db: "user", .. })));
}

#[test]
fn restore_moves_existing_databases_aside_instead_of_deleting_them() {
    let (d, fonts, s) = populated();
    let pack = d.path().join("pack");
    s.export_pack(&pack).unwrap();
    drop(s);
    // lần mở đầu sau khi cài lại đã tạo DB rỗng
    let fresh = d.path().join("newdb");
    drop(Store::open(&fresh).unwrap());
    assert!(fresh.join("user.db").exists());
    let (rep, backup) = fontatlas_store::restore_pack(&pack, &fresh, &[]).unwrap();
    assert_eq!(rep.counts.files, 2);
    let b = backup.expect("DB cũ phải được dời, không xoá");
    assert!(b.join("user.db").exists() && b.join("library.db").exists());
    let (s2, _) = Store::open(&fresh).unwrap();
    assert_eq!(count(&s2, "SELECT count(*) FROM u.face_tags"), 1);
    assert_eq!(s2.sources().unwrap()[0].path.as_deref(), Some(fonts.to_string_lossy().as_ref()));

    // gói hỏng: không dời gì
    std::fs::write(pack.join("user.db"), b"hong").unwrap();
    assert!(fontatlas_store::restore_pack(&pack, &fresh, &[]).is_err());
    drop(s2);
    assert!(fresh.join("user.db").exists() && fresh.join("library.db").exists(), "gói hỏng: không dời gì");
}
