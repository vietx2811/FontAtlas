//! Trạng thái onboarding gộp cho giao diện (UX flows O1–O7).
use fontatlas_activate::{SETTING_ACTIVATION_KEEP, SETTING_ACTIVATION_ONBOARDED};
use fontatlas_store::SETTING_LICENSE_NOTICE_ACK;

use crate::dto::settings::OnboardingDto;
use crate::error::CmdError;
use crate::services::{boot, keys, model, system};
use crate::state::AppState;

pub fn onboarding_state(state: &AppState) -> Result<OnboardingDto, CmdError> {
    let (step, has_sources, ack, embed, onboarded, keep) = state.with_user(|u| {
        Ok((
            u.setting(keys::ONBOARDING_STEP)?.filter(|s| !s.is_empty()),
            !u.sources()?.is_empty(),
            u.setting(SETTING_LICENSE_NOTICE_ACK)?.as_deref() == Some("1"),
            u.embed_enabled()?,
            u.setting(SETTING_ACTIVATION_ONBOARDED)?.as_deref() == Some("1"),
            u.setting(SETTING_ACTIVATION_KEEP)?.as_deref() == Some("1"),
        ))
    })?;
    Ok(OnboardingDto {
        step,
        has_sources,
        legacy: boot::legacy_state(state),
        license_notice_ack: ack,
        embed_enabled: embed,
        model_state: model::model_state_label(state).into(),
        activation_supported: state.activation.supported(),
        activation_onboarded: onboarded,
        keep_active_on_exit: keep,
        font_managers: system::detect_font_managers(),
    })
}

/// `None` = onboarding xong.
pub fn set_onboarding_step(state: &AppState, step: Option<String>) -> Result<(), CmdError> {
    state.with_user(|u| Ok(u.set_setting(keys::ONBOARDING_STEP, step.as_deref().unwrap_or(""))?))
}
