//! Cài đặt chung: đọc/ghi có kiểm; đổi `embedEnabled` bật/tắt worker, `keepActiveOnExit` đổi scope kích hoạt.
use std::sync::Arc;

use fontatlas_activate::SETTING_ACTIVATION_KEEP;
use fontatlas_store::{SETTING_EMBED_ENABLED, SETTING_LICENSE_NOTICE_ACK, Store};
use unicode_normalization::UnicodeNormalization;

use crate::dto::model::{SettingsDto, SettingsPatchDto};
use crate::error::CmdError;
use crate::services::keys;
use crate::state::AppState;

pub const DEFAULT_SAMPLE: &str = "Phở bò Hà Nội";
pub const MAX_SAMPLE_CHARS: usize = 64;

fn flag(u: &Store, key: &str, default: bool) -> Result<bool, CmdError> {
    Ok(match u.setting(key)?.as_deref() {
        Some("1") | Some("true") => true,
        Some("0") | Some("false") => false,
        _ => default,
    })
}

pub fn get_settings(state: &AppState) -> Result<SettingsDto, CmdError> {
    state.with_user(|u| {
        Ok(SettingsDto {
            embed_enabled: u.embed_enabled()?,
            license_notice_ack: flag(u, SETTING_LICENSE_NOTICE_ACK, false)?,
            ui_lang: u.setting(keys::UI_LANG)?.filter(|l| l == "vi" || l == "en").unwrap_or_else(|| "vi".into()),
            sample_text: u.setting(keys::SAMPLE_TEXT)?.unwrap_or_else(|| DEFAULT_SAMPLE.into()),
            project_vi_default: flag(u, keys::PROJECT_VI_DEFAULT, true)?,
            keep_active_on_exit: u.setting(SETTING_ACTIVATION_KEEP)?.as_deref() == Some("1"),
            rescan_on_launch: flag(u, keys::RESCAN_ON_LAUNCH, true)?,
            cloud_preview_download: flag(u, keys::CLOUD_PREVIEW_DOWNLOAD, true)?,
        })
    })
}

fn b(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

pub fn set_settings(state: &Arc<AppState>, p: SettingsPatchDto) -> Result<SettingsDto, CmdError> {
    if let Some(l) = &p.ui_lang
        && l != "vi" && l != "en"
    {
        return Err(CmdError::invalid_arg(format!("uiLang không hợp lệ: {l}")));
    }
    state.with_user(|u| {
        if let Some(v) = p.embed_enabled {
            u.set_setting(SETTING_EMBED_ENABLED, b(v))?;
        }
        if let Some(v) = p.license_notice_ack {
            u.set_setting(SETTING_LICENSE_NOTICE_ACK, b(v))?;
        }
        if let Some(l) = &p.ui_lang {
            u.set_setting(keys::UI_LANG, l)?;
        }
        if let Some(t) = &p.sample_text {
            let t: String = t.nfc().collect::<String>().trim().chars().take(MAX_SAMPLE_CHARS).collect();
            u.set_setting(keys::SAMPLE_TEXT, &t)?;
        }
        if let Some(v) = p.project_vi_default {
            u.set_setting(keys::PROJECT_VI_DEFAULT, b(v))?;
        }
        if let Some(v) = p.rescan_on_launch {
            u.set_setting(keys::RESCAN_ON_LAUNCH, b(v))?;
        }
        if let Some(v) = p.cloud_preview_download {
            u.set_setting(keys::CLOUD_PREVIEW_DOWNLOAD, b(v))?;
        }
        Ok(())
    })?;
    if let Some(v) = p.keep_active_on_exit {
        crate::services::activation::set_keep(state, v)?;
    }
    if let Some(on) = p.embed_enabled {
        if on {
            crate::services::embed::ensure_started(state)?;
        } else {
            state.embed.stop();
        }
    }
    get_settings(state)
}
