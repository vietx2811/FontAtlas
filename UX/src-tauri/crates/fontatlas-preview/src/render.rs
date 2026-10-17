//! Dàn chữ (cmap + advance) và vẽ PNG alpha-mask.
use crate::PreviewError;

/// Co cho vừa: top/bottom là biên lớn nhất của metric và mực thật (dấu chồng có thể vượt ascent).
pub fn fit(ascent: f32, descent: f32, ink_top: f32, ink_bottom: f32, height: u32, vpad: f32) -> (f32, f32) {
    let top = ascent.max(ink_top);
    let bottom = descent.min(ink_bottom);
    let avail = height as f32 - 2.0 * vpad;
    let k = (avail / (top - bottom).max(1e-3)).min(1.0);
    (k, vpad + top * k) // baseline tính từ mép trên canvas (y hướng xuống)
}

/// Số glyph giữ lại: dừng trước glyph đầu tiên có mép phải vượt (width - hpad); không cắt giữa glyph.
pub fn truncate_at(right_edges: &[f32], k: f32, width: u32, hpad: f32) -> usize {
    let limit = width as f32 - hpad;
    right_edges.iter().take_while(|&&r| hpad + r * k <= limit).count()
}

pub fn require_fit(keep: usize) -> Result<usize, PreviewError> {
    if keep == 0 { Err(PreviewError::NothingFits) } else { Ok(keep) }
}
