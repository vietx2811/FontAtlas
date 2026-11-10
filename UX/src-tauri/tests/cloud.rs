//! Ổ cloud: nhận diện nguồn theo đường dẫn (DTO có `cloudProvider`).
use std::sync::Arc;

use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, sources};
use app_lib::state::{AppDirs, AppState};

fn state(root: &std::path::Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    st
}

#[test]
fn source_under_cloudstorage_reports_its_provider() {
    let d = tempfile::tempdir().unwrap();
    let st = state(d.path());
    let onedrive = d.path().join("Library/CloudStorage/OneDrive-CongTy/Fonts");
    let icloud = d.path().join("Library/Mobile Documents/com~apple~CloudDocs/Fonts");
    let local = d.path().join("Fonts");
    for p in [&onedrive, &icloud, &local] {
        std::fs::create_dir_all(p).unwrap();
    }
    let a = sources::add_source(&st, onedrive.to_str().unwrap()).unwrap();
    let b = sources::add_source(&st, icloud.to_str().unwrap()).unwrap();
    let c = sources::add_source(&st, local.to_str().unwrap()).unwrap();
    assert_eq!(a.cloud_provider.as_deref(), Some("one_drive"));
    assert_eq!(b.cloud_provider.as_deref(), Some("icloud"));
    assert_eq!(c.cloud_provider, None);
    let listed = sources::list_sources(&st).unwrap();
    assert_eq!(listed.len(), 3);
    assert_eq!(listed.iter().filter(|s| s.cloud_provider.is_some()).count(), 2);
}

// ---- xem trước font cloud: công tắc "Tự tải font trên cloud để xem trước" ----
use std::time::Duration;

use app_lib::dto::model::SettingsPatchDto;
use app_lib::services::preview::StoreLocator;
use app_lib::services::settings;
use fontatlas_preview::FaceLocator;

fn with_one_font(root: &std::path::Path) -> (Arc<AppState>, String, std::path::PathBuf) {
    let st = state(root);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures/roboto.ttf"),
        dir.join("roboto.ttf"),
    )
    .unwrap();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let hash: String = st
        .with_store(|s| Ok(s.conn().query_row("SELECT lower(hex(hash)) FROM files", [], |r| r.get(0)).unwrap()))
        .unwrap();
    (st, hash, dir.join("roboto.ttf"))
}

fn mark_cloud_only(st: &AppState) {
    st.with_store(|s| {
        s.conn().execute("UPDATE file_locations SET cloud_only = 1", []).unwrap();
        Ok(())
    })
    .unwrap();
}

#[test]
fn setting_defaults_on_and_round_trips() {
    let d = tempfile::tempdir().unwrap();
    let st = state(d.path());
    assert!(settings::get_settings(&st).unwrap().cloud_preview_download, "mặc định BẬT");
    let off = settings::set_settings(&st, SettingsPatchDto { cloud_preview_download: Some(false), ..Default::default() }).unwrap();
    assert!(!off.cloud_preview_download);
    assert!(!settings::get_settings(&st).unwrap().cloud_preview_download);
    let on = settings::set_settings(&st, SettingsPatchDto { cloud_preview_download: Some(true), ..Default::default() }).unwrap();
    assert!(on.cloud_preview_download);
}

#[test]
fn preview_locator_reads_cloud_copies_only_when_the_setting_is_on() {
    let d = tempfile::tempdir().unwrap();
    let (st, hash, path) = with_one_font(d.path());
    let loc = StoreLocator::open(&st.dirs.data).unwrap();
    assert_eq!(loc.locate(&hash, 0), Some(path.clone()), "bản trên máy: luôn đọc được");
    mark_cloud_only(&st);
    assert_eq!(loc.locate(&hash, 0), Some(path), "bật (mặc định): font cloud được đọc, sẽ tải lười");
    settings::set_settings(&st, SettingsPatchDto { cloud_preview_download: Some(false), ..Default::default() }).unwrap();
    assert_eq!(loc.locate(&hash, 0), None, "tắt: bỏ qua bản cloud_only, không tải gì");
}

#[test]
fn source_dto_counts_faces_that_are_only_placeholders() {
    let d = tempfile::tempdir().unwrap();
    let (st, _hash, _path) = with_one_font(d.path());
    assert_eq!(sources::list_sources(&st).unwrap()[0].cloud_only_count, 0);
    mark_cloud_only(&st);
    let s = &sources::list_sources(&st).unwrap()[0];
    assert_eq!((s.cloud_only_count, s.face_count), (1, 1));
}

// ---- cổng tải cũng áp cho lệnh đọc cmap (missing_chars) ----
use app_lib::dto::library::FaceKeyDto;
use app_lib::services::preview::missing_chars_with;
use fontatlas_preview::PreviewError;

struct Fake { path: std::path::PathBuf, verdict: fn() -> Result<(), PreviewError> }
impl FaceLocator for Fake {
    fn locate(&self, _h: &str, _i: u32) -> Option<std::path::PathBuf> { Some(self.path.clone()) }
    fn before_read(&self, _p: &std::path::Path) -> Result<(), PreviewError> { (self.verdict)() }
}
fn face() -> FaceKeyDto { FaceKeyDto { hash_hex: "ab".repeat(32), face_index: 0 } }

#[test]
fn missing_chars_does_not_open_the_file_when_the_cloud_gate_refuses() {
    // đường dẫn không tồn tại: nếu bị mở thì lỗi sẽ là invalid_font/io, không phải cloud_timeout
    let f = Fake { path: "/khong/co/font.ttf".into(), verdict: || Err(PreviewError::CloudTimeout) };
    let e = missing_chars_with(&f, &face(), &["a".to_string()]).unwrap_err();
    assert_eq!(e.code, "cloud_timeout");
}

#[test]
fn missing_chars_reads_after_the_gate_lets_it_through() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures/roboto.ttf");
    let f = Fake { path: p, verdict: || Ok(()) };
    let r = missing_chars_with(&f, &face(), &["a".to_string(), "\u{10FFFE}".to_string()]);
    assert!(r.is_ok(), "{r:?}");
}
