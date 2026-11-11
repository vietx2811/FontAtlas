//! Settings → Tự bật font: trạng thái dựng từ đầu vào giả (không cần Adobe, không đụng thư mục Adobe thật).
use std::collections::HashMap;
use std::path::PathBuf;

use app_lib::dto::auto::*;
use app_lib::services::auto::{Inputs, build_status};
use fontatlas_cep::PanelStatus;
use fontatlas_indesign::{Product, Target, TargetStatus};

fn base() -> Inputs {
    Inputs {
        enabled: true,
        broker_running: true,
        font_request_running: true,
        isolated: false,
        installed_apps: vec!["Adobe Photoshop 2026".into(), "Adobe Illustrator".into(), "Adobe InDesign 2027".into()],
        running_exes: vec![],
        cep: Some(PanelStatus::NotInstalled),
        indesign: vec![],
        last_activity: HashMap::new(),
        managers: vec![("RightFont".into(), true), ("FontBase".into(), false)],
    }
}

fn app<'a>(s: &'a AutoStatusDto, id: &str) -> &'a AutoAppDto {
    s.apps.iter().find(|a| a.id == id).unwrap()
}

fn target(p: Product, v: &str) -> Target {
    Target { product: p, version: v.into(), locale: "en_US".into(), dir: PathBuf::from("/x") }
}

#[test]
fn six_apps_in_fixed_order_with_methods_and_evidence() {
    let s = build_status(&base());
    let ids: Vec<_> = s.apps.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["photoshop", "aftereffects", "illustrator", "indesign", "incopy", "affinity"]);
    assert_eq!(app(&s, "photoshop").method, AutoMethodDto::FontRequest);
    assert_eq!(app(&s, "photoshop").evidence, AutoEvidenceDto::Measured);
    assert_eq!(app(&s, "illustrator").method, AutoMethodDto::CepPanel);
    assert_eq!(app(&s, "indesign").method, AutoMethodDto::StartupScript);
    assert_eq!(app(&s, "aftereffects").evidence, AutoEvidenceDto::Untested);
    assert!(app(&s, "photoshop").app_installed && !app(&s, "aftereffects").app_installed);
    assert!(app(&s, "illustrator").app_installed, "tên không có năm vẫn khớp tiền tố");
}

#[test]
fn font_request_apps_follow_the_switch_and_the_source() {
    let mut i = base();
    assert_eq!(app(&build_status(&i), "photoshop").state, AutoAppStateDto::Ready);
    i.enabled = false;
    assert_eq!(app(&build_status(&i), "photoshop").state, AutoAppStateDto::Off);
    i.enabled = true;
    i.font_request_running = false;
    let s = build_status(&i);
    assert_eq!(app(&s, "photoshop").state, AutoAppStateDto::Error);
    assert!(app(&s, "photoshop").detail.is_some());
    i.isolated = true;
    assert_eq!(app(&build_status(&i), "photoshop").state, AutoAppStateDto::IsolatedBuild);
    assert!(!app(&build_status(&i), "photoshop").installable);
}

#[test]
fn illustrator_panel_states() {
    let mut i = base();
    let st = |i: &Inputs| build_status(i).apps.into_iter().find(|a| a.id == "illustrator").unwrap();
    let a = st(&i);
    assert_eq!((a.state, a.installable, a.installed), (AutoAppStateDto::NeedsInstall, true, false));
    i.cep = Some(PanelStatus::Installed { version: "0.3.0".into() });
    let a = st(&i);
    assert_eq!((a.state, a.installed), (AutoAppStateDto::Ready, true));
    i.cep = Some(PanelStatus::Stale { reason: "khác phiên bản".into() });
    let a = st(&i);
    assert_eq!((a.state, a.installed, a.detail.as_deref()), (AutoAppStateDto::Stale, true, Some("khác phiên bản")));
    i.cep = Some(PanelStatus::Foreign);
    assert_eq!(st(&i).state, AutoAppStateDto::Foreign);
    i.cep = None;
    assert_eq!(st(&i).state, AutoAppStateDto::Error);
}

#[test]
fn installed_panel_goes_quiet_when_switch_off_and_errors_without_broker() {
    let mut i = base();
    i.cep = Some(PanelStatus::Installed { version: "x".into() });
    i.broker_running = false;
    let a = build_status(&i).apps.into_iter().find(|a| a.id == "illustrator").unwrap();
    assert_eq!(a.state, AutoAppStateDto::Error);
    i.enabled = false;
    let a = build_status(&i).apps.into_iter().find(|a| a.id == "illustrator").unwrap();
    assert_eq!(a.state, AutoAppStateDto::Off, "công tắc tắt: broker từ chối, UI báo Tắt");
}

#[test]
fn indesign_and_incopy_are_reported_separately_and_wait_for_first_launch() {
    let mut i = base();
    let s = build_status(&i);
    assert_eq!(app(&s, "indesign").state, AutoAppStateDto::WaitingFirstLaunch);
    assert_eq!(app(&s, "incopy").state, AutoAppStateDto::WaitingFirstLaunch);
    i.indesign = vec![
        (target(Product::InDesign, "21.0"), TargetStatus::Installed),
        (target(Product::InDesign, "22.0"), TargetStatus::NotInstalled),
        (target(Product::InCopy, "21.0"), TargetStatus::Installed),
    ];
    let s = build_status(&i);
    assert_eq!(app(&s, "indesign").state, AutoAppStateDto::NeedsInstall, "còn một phiên bản chưa cài");
    assert!(app(&s, "indesign").installed);
    assert_eq!(app(&s, "incopy").state, AutoAppStateDto::Ready);
    i.indesign[1].1 = TargetStatus::Stale;
    assert_eq!(app(&build_status(&i), "indesign").state, AutoAppStateDto::Stale);
    i.indesign[1].1 = TargetStatus::Foreign;
    assert_eq!(app(&build_status(&i), "indesign").state, AutoAppStateDto::Foreign);
}

#[test]
fn running_app_and_restart_hint() {
    let mut i = base();
    i.running_exes = vec!["/Applications/Adobe Illustrator/Adobe Illustrator.app/Contents/MacOS/Adobe Illustrator".into(), "/Applications/RightFont.app/Contents/MacOS/RightFont".into()];
    let s = build_status(&i);
    let a = app(&s, "illustrator");
    assert!(a.app_running && a.restart_needed, "chưa cài mà app đang chạy → cài xong phải khởi động lại");
    assert!(!app(&s, "photoshop").app_running);
    assert!(!app(&s, "photoshop").restart_needed, "đường font request không cần khởi động lại app");
    i.cep = Some(PanelStatus::Installed { version: "x".into() });
    assert!(!app(&build_status(&i), "illustrator").restart_needed);
}

#[test]
fn other_managers_report_installed_and_running() {
    let mut i = base();
    i.running_exes = vec!["/Applications/RightFont.app/Contents/MacOS/RightFont".into()];
    let s = build_status(&i);
    assert_eq!(s.other_managers[0], OtherManagerDto { name: "RightFont".into(), installed: true, running: true });
    assert_eq!(s.other_managers[1], OtherManagerDto { name: "FontBase".into(), installed: false, running: false });
}

#[test]
fn last_activity_comes_from_the_log() {
    let mut i = base();
    i.last_activity.insert("photoshop".into(), 1_700_000_000_000);
    let s = build_status(&i);
    assert_eq!(app(&s, "photoshop").last_activity_ms, Some(1_700_000_000_000));
    assert_eq!(app(&s, "illustrator").last_activity_ms, None);
}

#[test]
fn switch_and_log_through_the_real_state() {
    use app_lib::services::{activation::NoActivation, auto, boot};
    use app_lib::state::{AppDirs, AppState};
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    // chỉ ĐỌC máy thật (thư mục ứng dụng, tiến trình, extensions của Adobe); không cài gì
    let s = auto::status(&st).unwrap();
    assert!(s.enabled, "mặc định bật");
    assert!(!s.font_request_running, "AppState dựng trong test không mở nguồn font request của cả phiên đăng nhập");
    assert!(!s.broker_running, "NoActivation không hỗ trợ → không mở broker");
    assert_eq!(s.apps.len(), 6);
    let off = auto::set_enabled(&st, false).unwrap();
    assert!(!off.enabled);
    assert_eq!(off.apps[0].state, AutoAppStateDto::Off);
    assert!(auto::set_enabled(&st, true).unwrap().enabled);
    assert!(auto::log(&st, None).unwrap().is_empty());
    assert!(auto::log(&st, Some(0)).unwrap().is_empty(), "giới hạn 0 được nâng lên 1, không lỗi");
}
