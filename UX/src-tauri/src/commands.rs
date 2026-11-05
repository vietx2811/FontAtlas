//! Lệnh Tauri: lớp mỏng, chỉ chuyển tham số rồi gọi `services::*`.
//!
//! `command_list!` là nguồn duy nhất cho `invoke_handler`, bảng `COMMANDS` và bản đồ TypeScript.
//! Mọi lệnh đụng Store/Index/embed là `async` (chạy trong `spawn_blocking`); lệnh `sync` chỉ cho
//! danh sách `SYNC_OK`.
use std::sync::Arc;

use tauri::State;

use tauri::ipc::Channel;

use crate::dto::activation::ActivationCapsDto;
use crate::dto::events::BootEvent;
use crate::dto::settings::{BootStatusDto, LegacyStateDto, OnboardingDto};
use crate::services::{boot, onboarding};
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
        if c.is_async && !src.contains(&async_def) {
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
