//! Việc của hệ thống: nhận diện trình quản lý font đã cài.
use std::path::{Path, PathBuf};

use crate::dto::settings::FontManagerDto;

/// (tên hiển thị, tên bundle trong `/Applications`)
const MANAGERS: [(&str, &str); 5] = [
    ("RightFont", "RightFont.app"),
    ("FontBase", "FontBase.app"),
    ("Typeface", "Typeface.app"),
    ("Suitcase Fusion", "Suitcase Fusion.app"),
    ("FontExplorer X", "FontExplorer X Pro.app"),
];

/// Chỉ xem có cài không (`/Applications`, `~/Applications`); `activeCount` do Plan 7 cung cấp sau.
pub fn detect_font_managers_in(roots: &[PathBuf]) -> Vec<FontManagerDto> {
    MANAGERS
        .iter()
        .map(|(name, bundle)| FontManagerDto {
            name: (*name).into(),
            installed: roots.iter().any(|r| r.join(bundle).exists()),
            active_count: None,
        })
        .collect()
}

pub fn detect_font_managers() -> Vec<FontManagerDto> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(Path::new(&home).join("Applications"));
    }
    detect_font_managers_in(&roots)
}

// ---- lệnh hệ thống ----

use std::path::Path as StdPath;

use fontatlas_store::FaceKey;

use crate::dto::library::{FaceKeyDto, UiStateDto};
use crate::error::CmdError;
use crate::state::AppState;

pub const UI_STATE_KEY: &str = "ui.state";
pub const MAX_SETTING_BYTES: usize = 64 * 1024;

/// Mở Finder tại file của face; đường dẫn lấy từ Store (frontend không truyền path).
/// `reveal` là hàm mở thật (`tauri_plugin_opener::reveal_item_in_dir`), tách ra để test.
pub fn reveal_in_finder(
    state: &AppState,
    face: &FaceKeyDto,
    reveal: &dyn Fn(&StdPath) -> Result<(), String>,
) -> Result<(), CmdError> {
    let k = FaceKey::try_from(face)?;
    let path = state
        .with_store(|s| Ok(s.face_path(&k)?))?
        .ok_or_else(|| CmdError::not_found("font không còn vị trí nào trên đĩa"))?;
    reveal(&path).map_err(|e| CmdError::new("io", e).with_detail(serde_json::json!({ "source": "app", "op": "reveal" })))
}

pub fn backup_user_db(state: &AppState) -> Result<String, CmdError> {
    state.with_user(|u| Ok(u.backup_user_db()?.to_string_lossy().into_owned()))
}

pub fn disk_free(state: &AppState, target: &str) -> Result<u64, CmdError> {
    let dir = match target {
        "models" => &state.dirs.models,
        "data" => &state.dirs.data,
        other => return Err(CmdError::invalid_arg(format!("target không hợp lệ: {other}"))),
    };
    fontatlas_embed::disk_free(dir).map_err(|e| CmdError::app_io("disk_free", &e))
}

/// Khoá setting chung cho giao diện: `ui.` + 1–60 ký tự `[a-z0-9_.-]`.
pub fn check_ui_key(key: &str) -> Result<(), CmdError> {
    let ok = key.strip_prefix("ui.").is_some_and(|r| {
        (1..=60).contains(&r.len()) && r.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-'))
    });
    if ok { Ok(()) } else { Err(CmdError::invalid_arg(format!("khoá setting không hợp lệ: {key}"))) }
}

pub fn get_setting(state: &AppState, key: &str) -> Result<Option<String>, CmdError> {
    check_ui_key(key)?;
    state.with_user(|u| Ok(u.setting(key)?))
}

pub fn set_setting(state: &AppState, key: &str, value: &str) -> Result<(), CmdError> {
    check_ui_key(key)?;
    if value.len() > MAX_SETTING_BYTES {
        return Err(CmdError::invalid_arg(format!("giá trị vượt {MAX_SETTING_BYTES} byte")));
    }
    state.with_user(|u| Ok(u.set_setting(key, value)?))
}

pub fn get_ui_state(state: &AppState) -> Result<UiStateDto, CmdError> {
    let raw = state.with_user(|u| Ok(u.setting(UI_STATE_KEY)?))?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default())
}

pub fn set_ui_state(state: &AppState, ui: &UiStateDto) -> Result<(), CmdError> {
    let json = serde_json::to_string(ui).map_err(|e| CmdError::internal(e.to_string()))?;
    set_setting(state, UI_STATE_KEY, &json)
}
