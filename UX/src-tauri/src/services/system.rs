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

/// Mở Finder tại thư mục của MỘT nguồn. Đường dẫn lấy từ Store theo `source_id` (webview không truyền path tuỳ ý).
/// Nguồn không có thư mục (hoặc thư mục không còn, vd ổ chưa mount / ổ cloud đang tắt) → `source_unavailable`, KHÔNG mở gì.
pub fn reveal_source(state: &AppState, source_id: i64, reveal: &dyn Fn(&StdPath) -> Result<(), String>) -> Result<(), CmdError> {
    let src = state.with_user(|u| Ok(u.source(source_id)?))?;
    let unavailable = |why: &str| {
        CmdError::new("source_unavailable", format!("thư mục của nguồn “{}” hiện không truy cập được ({why})", src.name))
            .with_detail(serde_json::json!({ "sourceId": source_id, "name": src.name }))
    };
    let Some(path) = src.path.as_deref() else { return Err(unavailable("nguồn không có thư mục")) };
    let root = StdPath::new(path);
    if !crate::services::sources::root_available(path) {
        return Err(unavailable("ổ chưa kết nối hoặc thư mục đã bị đổi tên"));
    }
    reveal(root).map_err(|e| CmdError::new("io", e).with_detail(serde_json::json!({ "source": "app", "op": "reveal" })))
}

/// Mở Finder tại MỘT đường dẫn cụ thể của nhóm same_file. Chỉ nhận đường dẫn đang có trong Store (so nguyên văn, hoặc sau
/// khi chuẩn hoá bằng `canonicalize` nếu tệp còn trên đĩa); đường dẫn lạ → `invalid_arg`: webview không được mở đường
/// dẫn tuỳ ý.
pub fn reveal_location(state: &AppState, path: &str, reveal: &dyn Fn(&StdPath) -> Result<(), String>) -> Result<(), CmdError> {
    let p = StdPath::new(path);
    if path.is_empty() || path.contains('\0') || !p.is_absolute() {
        return Err(CmdError::invalid_arg("đường dẫn không hợp lệ"));
    }
    let known = |s: &str| state.with_store(|st| Ok(st.has_location(s)?));
    // thử lần lượt: nguyên văn → chuẩn hoá chữ (bỏ `.`/`..`, không đụng đĩa) → canonicalize; chỉ mở đường dẫn ĐÃ LƯU
    let lexical = {
        let mut out = std::path::PathBuf::new();
        for c in p.components() {
            match c {
                std::path::Component::ParentDir => {
                    out.pop();
                }
                std::path::Component::CurDir => {}
                other => out.push(other.as_os_str()),
            }
        }
        out
    };
    let mut candidates = vec![path.to_string(), lexical.to_string_lossy().into_owned()];
    if let Ok(c) = std::fs::canonicalize(p) {
        candidates.push(c.to_string_lossy().into_owned());
    }
    let mut target = None;
    for c in candidates {
        if known(&c)? {
            target = Some(std::path::PathBuf::from(c));
            break;
        }
    }
    let Some(target) = target else {
        return Err(CmdError::invalid_arg("đường dẫn không nằm trong thư viện"));
    };
    reveal(&target).map_err(|e| CmdError::new("io", e).with_detail(serde_json::json!({ "source": "app", "op": "reveal" })))
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
