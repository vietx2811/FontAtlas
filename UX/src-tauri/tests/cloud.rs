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
