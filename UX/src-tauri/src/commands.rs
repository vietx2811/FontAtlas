//! Lệnh Tauri: lớp mỏng, chỉ chuyển tham số rồi gọi `services::*`.
//!
//! `command_list!` là nguồn duy nhất cho `invoke_handler`, bảng `COMMANDS` và bản đồ TypeScript.
//! Mọi lệnh đụng Store/Index/embed là `async` (chạy trong `spawn_blocking`); lệnh `sync` chỉ cho
//! danh sách `SYNC_OK`.
use std::sync::Arc;

use tauri::State;

use tauri::ipc::Channel;

use crate::dto::activation::ActivationCapsDto;
use crate::dto::events::{BootEvent, ScanEvent};
use crate::dto::library::*;
use crate::dto::settings::FontManagerDto;
use crate::dto::settings::{BootStatusDto, LegacyStateDto, OnboardingDto};
use crate::services::{boot, library, onboarding, sources, system};
use crate::error::CmdError;
use crate::state::AppState;

pub struct CommandSig {
    pub name: &'static str,
    pub args: &'static [(&'static str, &'static str)],
    pub result: &'static str,
    pub is_async: bool,
}

/// Lệnh được phép đồng bộ: không đụng Store/Index/embed.
pub const SYNC_OK: &[&str] = &["startup_status", "scan_status", "preview_info", "activation_caps"];

/// Danh sách lệnh (nguồn duy nhất). Cú pháp mỗi dòng: `async|sync tên(đối số: kiểu, …) -> kiểu_kết_quả;`
/// Gọi `command_list!(__command_table)` để sinh `COMMANDS`, `command_list!(__command_handler)` cho handler.
#[macro_export]
macro_rules! command_list {
    ($cb:ident) => {
        $crate::$cb! {
            sync startup_status() -> Result<BootStatusDto, CmdError>;
            async boot_subscribe(on_event: Channel<BootEvent>) -> Result<(), CmdError>;
            async onboarding_state() -> Result<OnboardingDto, CmdError>;
            async set_onboarding_step(step: Option<String>) -> Result<(), CmdError>;
            async legacy_import() -> Result<LegacyStateDto, CmdError>;
            async legacy_retry() -> Result<LegacyStateDto, CmdError>;
            async list_sources() -> Result<Vec<SourceDto>, CmdError>;
            async add_source(path: String) -> Result<SourceDto, CmdError>;
            async remove_source(source_id: i64) -> Result<(), CmdError>;
            async rescan_sources(source_ids: Option<Vec<i64>>) -> Result<(), CmdError>;
            async scan_pause() -> Result<(), CmdError>;
            async scan_resume() -> Result<(), CmdError>;
            async scan_cancel() -> Result<(), CmdError>;
            sync scan_status() -> Result<ScanStatusDto, CmdError>;
            async scan_subscribe(on_event: Channel<ScanEvent>) -> Result<(), CmdError>;
            async query_faces(q: FaceQueryDto) -> Result<FacePageDto, CmdError>;
            async get_family_styles(family: String) -> Result<Vec<FamilyStyleDto>, CmdError>;
            async set_favorite(face: FaceKeyDto, on: bool) -> Result<(), CmdError>;
            async list_collections() -> Result<Vec<CollectionDto>, CmdError>;
            async create_collection(name: String, color: Option<String>) -> Result<CollectionDto, CmdError>;
            async rename_collection(id: i64, name: String) -> Result<(), CmdError>;
            async delete_collection(id: i64) -> Result<(), CmdError>;
            async add_to_collection(id: i64, faces: Vec<FaceKeyDto>) -> Result<(), CmdError>;
            async remove_from_collection(id: i64, faces: Vec<FaceKeyDto>) -> Result<(), CmdError>;
            async list_tags() -> Result<Vec<TagDto>, CmdError>;
            async tags_of(face: FaceKeyDto) -> Result<Vec<TagDto>, CmdError>;
            async create_tag(name: String) -> Result<TagDto, CmdError>;
            async tag_faces(tag_id: i64, faces: Vec<FaceKeyDto>) -> Result<(), CmdError>;
            async untag_faces(tag_id: i64, faces: Vec<FaceKeyDto>) -> Result<(), CmdError>;
            async list_problems() -> Result<Vec<ProblemDto>, CmdError>;
            async retry_problems() -> Result<u32, CmdError>;
            async reveal_in_finder(face: FaceKeyDto) -> Result<(), CmdError>;
            async backup_user_db() -> Result<String, CmdError>;
            async disk_free(target: String) -> Result<u64, CmdError>;
            async get_ui_state() -> Result<UiStateDto, CmdError>;
            async set_ui_state(state: UiStateDto) -> Result<(), CmdError>;
            async get_setting(key: String) -> Result<Option<String>, CmdError>;
            async set_setting(key: String, value: String) -> Result<(), CmdError>;
            async detect_font_managers() -> Result<Vec<FontManagerDto>, CmdError>;
            sync activation_caps() -> Result<ActivationCapsDto, CmdError>;
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __command_table {
    (@async async) => { true };
    (@async sync) => { false };
    ($( $kind:ident $name:ident ( $( $arg:ident : $ty:ty ),* ) -> $ret:ty ; )*) => {
        pub const COMMANDS: &[$crate::commands::CommandSig] = &[
            $( $crate::commands::CommandSig {
                name: stringify!($name),
                args: &[ $( (stringify!($arg), stringify!($ty)) ),* ],
                result: stringify!($ret),
                is_async: $crate::__command_table!(@async $kind),
            } ),*
        ];
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __command_handler {
    ($( $kind:ident $name:ident ( $( $arg:ident : $ty:ty ),* ) -> $ret:ty ; )*) => {
        tauri::generate_handler![ $( $crate::commands::$name ),* ]
    };
}

/// `invoke_handler` của app (cũng dùng trong test với `mock_builder`).
#[macro_export]
macro_rules! app_handler {
    () => {
        $crate::command_list!(__command_handler)
    };
}

command_list!(__command_table);

/// Chạy `f` ngoài luồng async của Tauri (SQLite, đĩa, CPU) rồi trả kết quả.
pub async fn blocking<T, F>(state: &State<'_, Arc<AppState>>, f: F) -> Result<T, CmdError>
where
    T: Send + 'static,
    F: FnOnce(&Arc<AppState>) -> Result<T, CmdError> + Send + 'static,
{
    let st = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || f(&st))
        .await
        .map_err(|e| CmdError::internal(format!("tác vụ nền lỗi: {e}")))?
}

#[tauri::command]
pub fn startup_status(state: State<'_, Arc<AppState>>) -> Result<BootStatusDto, CmdError> {
    Ok(boot::snapshot(&state))
}

#[tauri::command]
pub async fn boot_subscribe(state: State<'_, Arc<AppState>>, on_event: Channel<BootEvent>) -> Result<(), CmdError> {
    // người đến muộn nhận ngay pha hiện tại
    let _ = on_event.send(BootEvent::Phase { phase: boot::snapshot(&state).phase });
    state.hubs.boot.subscribe(Box::new(move |e| {
        let _ = on_event.send(e);
    }));
    Ok(())
}

#[tauri::command]
pub async fn onboarding_state(state: State<'_, Arc<AppState>>) -> Result<OnboardingDto, CmdError> {
    blocking(&state, |s| onboarding::onboarding_state(s)).await
}

#[tauri::command]
pub async fn set_onboarding_step(state: State<'_, Arc<AppState>>, step: Option<String>) -> Result<(), CmdError> {
    blocking(&state, move |s| onboarding::set_onboarding_step(s, step)).await
}

#[tauri::command]
pub async fn legacy_import(state: State<'_, Arc<AppState>>) -> Result<LegacyStateDto, CmdError> {
    blocking(&state, boot::legacy_import).await
}

#[tauri::command]
pub async fn legacy_retry(state: State<'_, Arc<AppState>>) -> Result<LegacyStateDto, CmdError> {
    blocking(&state, boot::legacy_import).await
}

macro_rules! cmd {
    ($(#[$m:meta])* $name:ident ( $($arg:ident : $ty:ty),* ) -> $ret:ty => |$s:ident| $body:expr) => {
        $(#[$m])*
        #[tauri::command]
        pub async fn $name(state: State<'_, Arc<AppState>> $(, $arg: $ty)*) -> Result<$ret, CmdError> {
            blocking(&state, move |$s| $body).await
        }
    };
}

cmd!(query_faces(q: FaceQueryDto) -> FacePageDto => |s| library::query_faces(s, &q));
cmd!(get_family_styles(family: String) -> Vec<FamilyStyleDto> => |s| library::get_family_styles(s, &family));
cmd!(set_favorite(face: FaceKeyDto, on: bool) -> () => |s| library::set_favorite(s, &face, on));
cmd!(list_collections() -> Vec<CollectionDto> => |s| library::list_collections(s));
cmd!(create_collection(name: String, color: Option<String>) -> CollectionDto => |s| library::create_collection(s, &name, color.as_deref()));
cmd!(rename_collection(id: i64, name: String) -> () => |s| library::rename_collection(s, id, &name));
cmd!(delete_collection(id: i64) -> () => |s| library::delete_collection(s, id));
cmd!(add_to_collection(id: i64, faces: Vec<FaceKeyDto>) -> () => |s| library::add_to_collection(s, id, &faces));
cmd!(remove_from_collection(id: i64, faces: Vec<FaceKeyDto>) -> () => |s| library::remove_from_collection(s, id, &faces));
cmd!(list_tags() -> Vec<TagDto> => |s| library::list_tags(s));
cmd!(tags_of(face: FaceKeyDto) -> Vec<TagDto> => |s| library::tags_of(s, &face));
cmd!(create_tag(name: String) -> TagDto => |s| library::create_tag(s, &name));
cmd!(tag_faces(tag_id: i64, faces: Vec<FaceKeyDto>) -> () => |s| library::tag_faces(s, tag_id, &faces));
cmd!(untag_faces(tag_id: i64, faces: Vec<FaceKeyDto>) -> () => |s| library::untag_faces(s, tag_id, &faces));
cmd!(list_problems() -> Vec<ProblemDto> => |s| library::list_problems(s));
cmd!(retry_problems() -> u32 => |s| library::retry_problems(s));
cmd!(reveal_in_finder(face: FaceKeyDto) -> () => |s| system::reveal_in_finder(s, &face, &|p| {
    tauri_plugin_opener::reveal_item_in_dir(p).map_err(|e| e.to_string())
}));
cmd!(backup_user_db() -> String => |s| system::backup_user_db(s));
cmd!(disk_free(target: String) -> u64 => |s| system::disk_free(s, &target));
cmd!(get_ui_state() -> UiStateDto => |s| system::get_ui_state(s));
cmd!(set_ui_state(state: UiStateDto) -> () => |s| system::set_ui_state(s, &state));
cmd!(get_setting(key: String) -> Option<String> => |s| system::get_setting(s, &key));
cmd!(set_setting(key: String, value: String) -> () => |s| system::set_setting(s, &key, &value));
cmd!(detect_font_managers() -> Vec<FontManagerDto> => |_s| Ok(system::detect_font_managers()));

#[tauri::command]
pub async fn list_sources(state: State<'_, Arc<AppState>>) -> Result<Vec<SourceDto>, CmdError> {
    blocking(&state, |s| sources::list_sources(s)).await
}

#[tauri::command]
pub async fn add_source(state: State<'_, Arc<AppState>>, path: String) -> Result<SourceDto, CmdError> {
    blocking(&state, move |s| sources::add_source(s, &path)).await
}

#[tauri::command]
pub async fn remove_source(state: State<'_, Arc<AppState>>, source_id: i64) -> Result<(), CmdError> {
    blocking(&state, move |s| sources::remove_source(s, source_id)).await
}

#[tauri::command]
pub async fn rescan_sources(state: State<'_, Arc<AppState>>, source_ids: Option<Vec<i64>>) -> Result<(), CmdError> {
    blocking(&state, move |s| sources::rescan_sources(s, source_ids)).await
}

#[tauri::command]
pub async fn scan_pause(state: State<'_, Arc<AppState>>) -> Result<(), CmdError> {
    state.scans.pause(&state);
    Ok(())
}

#[tauri::command]
pub async fn scan_resume(state: State<'_, Arc<AppState>>) -> Result<(), CmdError> {
    state.scans.resume(&state);
    Ok(())
}

#[tauri::command]
pub async fn scan_cancel(state: State<'_, Arc<AppState>>) -> Result<(), CmdError> {
    state.scans.cancel_current();
    Ok(())
}

#[tauri::command]
pub fn scan_status(state: State<'_, Arc<AppState>>) -> Result<ScanStatusDto, CmdError> {
    Ok(state.scans.status())
}

#[tauri::command]
pub async fn scan_subscribe(state: State<'_, Arc<AppState>>, on_event: Channel<ScanEvent>) -> Result<(), CmdError> {
    state.hubs.scan.subscribe(Box::new(move |e| {
        let _ = on_event.send(e);
    }));
    Ok(())
}

#[tauri::command]
pub fn activation_caps(state: State<'_, Arc<AppState>>) -> Result<ActivationCapsDto, CmdError> {
    Ok(ActivationCapsDto { method: state.activation.method().into(), supported: state.activation.supported() })
}

/// Kiểm kỷ luật async trên `table` và mã nguồn `src` của commands.rs; trả danh sách vi phạm.
pub fn async_violations(table: &[CommandSig], src: &str) -> Vec<String> {
    let mut v = Vec::new();
    for c in table {
        let sync_def = format!("pub fn {}(", c.name);
        let async_def = format!("pub async fn {}(", c.name);
        if !c.is_async && !SYNC_OK.contains(&c.name) {
            v.push(format!("{} khai báo sync nhưng không nằm trong SYNC_OK", c.name));
        }
        // `cmd!(tên(` sinh `pub async fn tên(`
        let via_macro = format!("cmd!({}(", c.name);
        if c.is_async && !src.contains(&async_def) && !src.contains(&via_macro) {
            v.push(format!("{} khai báo async trong bảng nhưng hàm không phải `pub async fn`", c.name));
        }
        if !c.is_async && !src.contains(&sync_def) {
            v.push(format!("{} khai báo sync trong bảng nhưng không thấy `pub fn`", c.name));
        }
    }
    // mọi `#[tauri::command]` trần phải đi cùng hàm trong SYNC_OK
    let lines: Vec<&str> = src.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        if l.trim() == "#[tauri::command]"
            && let Some(next) = lines.get(i + 1)
            && let Some(rest) = next.trim().strip_prefix("pub fn ")
        {
            let name = rest.split('(').next().unwrap_or("");
            if !SYNC_OK.contains(&name) {
                v.push(format!("{name}: lệnh đồng bộ ngoài SYNC_OK"));
            }
        }
    }
    v
}
