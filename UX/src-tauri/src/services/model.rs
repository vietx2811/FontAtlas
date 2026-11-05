//! Trạng thái model SigLIP (Task 16 mở rộng: tải, huỷ, ETA).
use fontatlas_embed::{ModelStatus, ModelStore, bundled_manifest};

use crate::state::AppState;

/// Nhãn trạng thái model cho onboarding: missing | paused | ready | corrupt | unpublished.
pub fn model_state_label(state: &AppState) -> &'static str {
    let m = bundled_manifest();
    if m.source.url.trim().is_empty() {
        return "unpublished";
    }
    match ModelStore::new(state.dirs.models.clone(), m).status() {
        ModelStatus::Missing => "missing",
        ModelStatus::Partial { .. } => "paused",
        ModelStatus::Ready(_) => "ready",
        ModelStatus::Corrupt => "corrupt",
    }
}
