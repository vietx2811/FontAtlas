//! Settings → Tự bật font: trạng thái từng app thiết kế, cài/gỡ panel và script, nhật ký.
//! `build_status` là hàm thuần (đầu vào đã thu thập) để test không cần Adobe hay hệ thống thật.
use std::collections::HashMap;
use std::path::PathBuf;

use fontatlas_broker::{SETTING_AUTO_ACTIVATION, auto_enabled};
use fontatlas_cep::PanelStatus;
use fontatlas_indesign::{Product, Target, TargetStatus};

use crate::dto::auto::*;
use crate::error::CmdError;
use crate::state::AppState;

/// Mọi thứ `build_status` cần, đã đọc từ máy.
pub struct Inputs {
    pub enabled: bool,
    pub broker_running: bool,
    pub font_request_running: bool,
    /// Mã lỗi mở nguồn lần gần nhất (xem `AutoStatusDto::font_request_error`).
    pub font_request_error: Option<String>,
    pub signing_key_present: bool,
    /// Bản cô lập: từ chối cài/gỡ/xoá khoá (xem `is_isolated`).
    pub isolated_build: bool,
    /// Bản chạy với thư mục dữ liệu riêng: nguồn font request cố ý không mở.
    pub isolated: bool,
    /// Tên các app có trong `/Applications` (vd "Adobe Photoshop 2026").
    pub installed_apps: Vec<String>,
    /// Đường dẫn file chạy của mọi tiến trình đang chạy.
    pub running_exes: Vec<String>,
    pub cep: Option<PanelStatus>,
    pub indesign: Vec<(Target, TargetStatus)>,
    /// slug app → lần hỏi gần nhất (ms).
    pub last_activity: HashMap<String, i64>,
    pub managers: Vec<(String, bool)>,
}

struct AppDef {
    id: &'static str,
    label: &'static str,
    /// Tiền tố tên gói `.app` trong /Applications ("Adobe Photoshop " khớp "Adobe Photoshop 2026").
    prefix: &'static str,
    method: AutoMethodDto,
    evidence: AutoEvidenceDto,
}

const APPS: [AppDef; 6] = [
    AppDef { id: "photoshop", label: "Photoshop", prefix: "Adobe Photoshop", method: AutoMethodDto::FontRequest, evidence: AutoEvidenceDto::Measured },
    AppDef { id: "aftereffects", label: "After Effects", prefix: "Adobe After Effects", method: AutoMethodDto::FontRequest, evidence: AutoEvidenceDto::Untested },
    AppDef { id: "illustrator", label: "Illustrator", prefix: "Adobe Illustrator", method: AutoMethodDto::CepPanel, evidence: AutoEvidenceDto::Partial },
    AppDef { id: "indesign", label: "InDesign", prefix: "Adobe InDesign", method: AutoMethodDto::StartupScript, evidence: AutoEvidenceDto::Partial },
    AppDef { id: "incopy", label: "InCopy", prefix: "Adobe InCopy", method: AutoMethodDto::StartupScript, evidence: AutoEvidenceDto::Untested },
    AppDef { id: "affinity", label: "Affinity", prefix: "Affinity", method: AutoMethodDto::FontRequest, evidence: AutoEvidenceDto::Untested },
];

fn app_installed(d: &AppDef, installed: &[String]) -> bool {
    installed.iter().any(|n| n.starts_with(d.prefix))
}

/// Tiến trình đang chạy thuộc gói `.app` có tên bắt đầu bằng `prefix`.
fn app_running(d: &AppDef, exes: &[String]) -> bool {
    exes.iter().any(|p| p.split('/').any(|c| c.strip_suffix(".app").is_some_and(|n| n.starts_with(d.prefix))))
}

fn running_manager(i: &Inputs, name: &str) -> bool {
    i.running_exes.iter().any(|p| p.split('/').any(|c| c.strip_suffix(".app").is_some_and(|n| n.eq_ignore_ascii_case(name) || (name == "FontExplorer X" && n.starts_with("FontExplorer X")))))
}

pub fn build_status(i: &Inputs) -> AutoStatusDto {
    let apps = APPS
        .iter()
        .map(|d| {
            let installed_app = app_installed(d, &i.installed_apps);
            let running = app_running(d, &i.running_exes);
            let mut state: AutoAppStateDto;
            let (mut installable, mut installed, mut detail) = (false, false, None::<String>);
            match d.method {
                AutoMethodDto::FontRequest => {
                    if !i.enabled {
                        state = AutoAppStateDto::Off;
                    } else if i.font_request_running && i.font_request_error.as_deref() == Some("starved") {
                        // nguồn mở được nhưng không nhận request: app khác đang nhận trước (probe, xem services/fontreq::probe)
                        state = AutoAppStateDto::Error;
                        let names: Vec<&str> = i.managers.iter().filter(|(n, _)| running_manager(i, n)).map(|(n, _)| n.as_str()).collect();
                        detail = Some(if names.is_empty() {
                            "một app khác đang nhận yêu cầu font trước Font Atlas".to_string()
                        } else {
                            format!("{} đang nhận yêu cầu font trước Font Atlas", names.join(", "))
                        });
                    } else if i.font_request_running {
                        state = AutoAppStateDto::Ready;
                    } else if i.isolated {
                        state = AutoAppStateDto::IsolatedBuild;
                    } else {
                        state = AutoAppStateDto::Error;
                        detail = Some(
                            match i.font_request_error.as_deref() {
                                Some("timeout") => "nguồn font request chưa sẵn sàng (hệ thống bận); Font Atlas sẽ thử lại",
                                Some("refused") => "hệ thống từ chối nguồn font request; Font Atlas sẽ thử lại",
                                _ => "nguồn font request của hệ thống không mở được",
                            }
                            .into(),
                        );
                    }
                }
                AutoMethodDto::CepPanel => {
                    installable = true;
                    match &i.cep {
                        None => {
                            state = AutoAppStateDto::Error;
                            detail = Some("không xác định được thư mục extensions của Adobe".into());
                        }
                        Some(PanelStatus::NotInstalled) => state = AutoAppStateDto::NeedsInstall,
                        Some(PanelStatus::Installed { .. }) => {
                            installed = true;
                            state = AutoAppStateDto::Ready;
                        }
                        Some(PanelStatus::Stale { reason }) => {
                            installed = true;
                            state = AutoAppStateDto::Stale;
                            detail = Some(reason.clone());
                        }
                        Some(PanelStatus::Foreign) => state = AutoAppStateDto::Foreign,
                    }
                }
                AutoMethodDto::StartupScript => {
                    installable = true;
                    let product = if d.id == "incopy" { Product::InCopy } else { Product::InDesign };
                    let mine: Vec<_> = i.indesign.iter().filter(|(t, _)| t.product == product).collect();
                    let has = |s: &TargetStatus| mine.iter().any(|(_, x)| x == s);
                    installed = has(&TargetStatus::Installed) || has(&TargetStatus::Stale);
                    state = if mine.is_empty() {
                        AutoAppStateDto::WaitingFirstLaunch
                    } else if has(&TargetStatus::Foreign) {
                        AutoAppStateDto::Foreign
                    } else if has(&TargetStatus::Stale) {
                        AutoAppStateDto::Stale
                    } else if has(&TargetStatus::NotInstalled) {
                        AutoAppStateDto::NeedsInstall
                    } else {
                        AutoAppStateDto::Ready
                    };
                }
            }
            // Công tắc tắt thì cài sẵn vẫn im lặng (broker từ chối 503): báo Off để UI không hứa điều không xảy ra.
            if !i.enabled && d.method != AutoMethodDto::FontRequest && matches!(state, AutoAppStateDto::Ready) {
                state = AutoAppStateDto::Off;
            }
            // Đường qua broker cần broker chạy.
            if i.enabled && d.method != AutoMethodDto::FontRequest && matches!(state, AutoAppStateDto::Ready) && !i.broker_running {
                state = AutoAppStateDto::Error;
                detail = Some("broker nội bộ không chạy".into());
            }
            AutoAppDto {
                id: d.id.into(),
                label: d.label.into(),
                method: d.method,
                state,
                evidence: d.evidence,
                app_installed: installed_app,
                app_running: running,
                installable,
                installed,
                restart_needed: installable && running && matches!(state, AutoAppStateDto::NeedsInstall | AutoAppStateDto::Stale),
                detail,
                last_activity_ms: i.last_activity.get(d.id).copied(),
            }
        })
        .collect();
    let managers_running = |name: &str| running_manager(i, name);
    AutoStatusDto {
        enabled: i.enabled,
        broker_running: i.broker_running,
        font_request_running: i.font_request_running,
        font_request_error: i.font_request_error.clone(),
        signing_key_present: i.signing_key_present,
        isolated_build: i.isolated_build,
        apps,
        other_managers: i
            .managers
            .iter()
            .map(|(name, installed)| OtherManagerDto { name: name.clone(), installed: *installed, running: managers_running(name) })
            .collect(),
    }
}

// ---- thu thập đầu vào từ máy thật ----

fn installed_app_names() -> Vec<String> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(h) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(h).join("Applications"));
    }
    let mut out = vec![];
    for r in roots {
        // `/Applications/Adobe Photoshop 2026/Adobe Photoshop 2026.app` nằm trong thư mục cùng tên: lấy cả hai tầng.
        let Ok(rd) = std::fs::read_dir(&r) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if let Some(a) = n.strip_suffix(".app") {
                out.push(a.to_string());
            } else if n.starts_with("Adobe ") && e.path().is_dir() {
                out.push(n);
            }
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn running_exes() -> Vec<String> {
    let n = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    if n <= 0 {
        return vec![];
    }
    let mut pids = vec![0 as libc::pid_t; n as usize + 64];
    let got = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), (pids.len() * std::mem::size_of::<libc::pid_t>()) as libc::c_int) };
    if got <= 0 {
        return vec![];
    }
    pids.truncate(got as usize);
    pids.into_iter().filter(|p| *p > 0).filter_map(fontatlas_fontreq::exe_path_of_pid).collect()
}

#[cfg(not(target_os = "macos"))]
fn running_exes() -> Vec<String> {
    vec![]
}

pub fn status(state: &AppState) -> Result<AutoStatusDto, CmdError> {
    let store = state.store()?;
    let last_activity = state
        .with_store(|s| Ok(s.recent_auto_log(500)?))?
        .into_iter()
        .fold(HashMap::new(), |mut m: HashMap<String, i64>, r| {
            let e = m.entry(r.app).or_insert(r.at);
            *e = (*e).max(r.at);
            m
        });
    let inputs = Inputs {
        enabled: auto_enabled(store),
        // ping thật: worker chết thì cổng vẫn mở nhưng không ai trả lời
        broker_running: crate::services::broker::healthy(state),
        font_request_running: state.fontreq.lock().map(|g| g.is_some()).unwrap_or(false),
        font_request_error: state.fontreq_error.lock().ok().and_then(|e| e.clone()),
        signing_key_present: crate::services::cep::key_present(state),
        isolated_build: is_isolated(state),
        isolated: !state.system_hooks.load(std::sync::atomic::Ordering::SeqCst)
            || (std::env::var_os("FONTATLAS_DATA_DIR").is_some() && std::env::var_os("FONTATLAS_FONTREQ").is_none()),
        installed_apps: installed_app_names(),
        running_exes: running_exes(),
        cep: crate::services::cep::status(state),
        indesign: crate::services::indesign::status(state),
        last_activity,
        managers: crate::services::system::detect_font_managers().into_iter().map(|m| (m.name, m.installed)).collect(),
    };
    Ok(build_status(&inputs))
}

pub fn set_enabled(state: &AppState, on: bool) -> Result<AutoStatusDto, CmdError> {
    state.with_user(|u| Ok(u.set_setting(SETTING_AUTO_ACTIVATION, if on { "1" } else { "0" })?))?;
    status(state)
}

/// Cài/xoá khoá ghi vào HOME THẬT của người dùng (thư mục Adobe). Bản chạy cô lập (QA/test/`FONTATLAS_DATA_DIR`) dùng dữ liệu
/// tạm nhưng vẫn trỏ vào HOME thật, nên không được phép làm việc đó.
fn require_real_app(state: &AppState) -> Result<(), CmdError> {
    if is_isolated(state) {
        return Err(CmdError::new("isolated_build", "bản chạy với dữ liệu riêng (QA/test) không được cài vào thư mục Adobe thật"));
    }
    Ok(())
}

fn is_isolated(state: &AppState) -> bool {
    !state.system_hooks.load(std::sync::atomic::Ordering::SeqCst) || std::env::var_os("FONTATLAS_DATA_DIR").is_some()
}

/// Tự kiểm lại nguồn font request (nút "Kiểm tra lại" và khi mở Settings): một tiến trình con hỏi font ngẫu nhiên; chặn tối đa
/// `PROBE_WAIT` (4 s). Trả trạng thái mới (`fontRequestError == "starved"` nếu app khác nhận trước).
pub fn probe(state: &AppState) -> Result<AutoStatusDto, CmdError> {
    crate::services::fontreq::probe_now(state);
    status(state)
}

pub fn forget_key(state: &AppState) -> Result<AutoStatusDto, CmdError> {
    require_real_app(state)?;
    crate::services::cep::forget_key(state).map_err(|e| CmdError::new("io", e.to_string()))?;
    status(state)
}

pub fn install(state: &AppState, target: AutoTargetDto) -> Result<AutoStatusDto, CmdError> {
    require_real_app(state)?;
    match target {
        AutoTargetDto::Illustrator => {
            crate::services::cep::install(state).map_err(|e| CmdError::new("io", e.to_string()))?;
        }
        AutoTargetDto::Indesign => {
            let reports = crate::services::indesign::install(state);
            if let Some(err) = reports.iter().find_map(|r| r.result.as_ref().err()) {
                return Err(CmdError::new("io", err.clone()));
            }
        }
    }
    status(state)
}

pub fn remove(state: &AppState, target: AutoTargetDto) -> Result<AutoStatusDto, CmdError> {
    require_real_app(state)?;
    match target {
        AutoTargetDto::Illustrator => {
            crate::services::cep::remove(state).map_err(|e| CmdError::new("io", e.to_string()))?;
        }
        AutoTargetDto::Indesign => {
            crate::services::indesign::remove(state).map_err(|e| CmdError::new("io", e.to_string()))?;
        }
    }
    status(state)
}

pub fn log(state: &AppState, limit: Option<u32>) -> Result<Vec<AutoLogDto>, CmdError> {
    let n = limit.unwrap_or(50).clamp(1, 500) as usize;
    Ok(state.with_store(|s| Ok(s.recent_auto_log(n)?))?.into_iter().map(Into::into).collect())
}
