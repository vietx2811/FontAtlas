//! Kính thật của macOS (Liquid Glass, `NSGlassEffectView`, macOS 26+) cho màn hướng dẫn đầu tiên.
//!
//! Lớp kính được chèn DƯỚI webview (webview trong suốt nên mảng không phủ nền sẽ lộ kính). Khi bật kính, vibrancy
//! `sidebar` của cửa sổ được gỡ để hai lớp không chồng nhau; tắt kính thì trả lại đúng vibrancy ban đầu.
//! Hệ không có `NSGlassEffectView` (macOS < 26) hoặc không phải macOS: trả `false`, giữ nguyên vibrancy `sidebar`.
use std::ffi::c_void;

/// Tag mà `window-vibrancy` gán cho NSVisualEffectView của cửa sổ.
#[cfg(target_os = "macos")]
const VIBRANCY_TAG: isize = 91376254;

/// `true` = kính đang hiện sau lệnh này.
#[cfg(target_os = "macos")]
pub fn set_glass(ns_window: *mut c_void, on: bool) -> bool {
    use objc2::{ClassType, MainThreadMarker};
    use objc2::runtime::NSObjectProtocol;
    use objc2_app_kit::{NSAutoresizingMaskOptions, NSGlassEffectView, NSGlassEffectViewStyle, NSView, NSVisualEffectView, NSWindow, NSWindowOrderingMode};
    let Some(mtm) = MainThreadMarker::new() else { return false };
    if ns_window.is_null() || objc2::runtime::AnyClass::get(c"NSGlassEffectView").is_none() {
        return false;
    }
    // SAFETY: con trỏ NSWindow do Tauri cấp, còn sống suốt lệnh này và ta đang ở luồng chính.
    let win: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
    let Some(content) = win.contentView() else { return false };
    let existing: Vec<_> = content.subviews().iter().filter(|v| v.isKindOfClass(NSGlassEffectView::class())).collect();
    if !on {
        for v in existing {
            v.removeFromSuperview();
        }
        return false;
    }
    // Gỡ vibrancy `sidebar` do Tauri gắn (window-vibrancy đánh tag cố định): `set_effects(None)` của Tauri không gỡ nó trên macOS.
    for v in content.subviews().iter().filter(|v| v.isKindOfClass(NSVisualEffectView::class()) && v.tag() == VIBRANCY_TAG) {
        v.removeFromSuperview();
    }
    if !existing.is_empty() {
        return true;
    }
    let glass = NSGlassEffectView::new(mtm);
    glass.setStyle(NSGlassEffectViewStyle::Regular);
    glass.setFrame(content.bounds());
    glass.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
    let as_view: &NSView = &glass;
    content.addSubview_positioned_relativeTo(as_view, NSWindowOrderingMode::Below, None);
    true
}

#[cfg(not(target_os = "macos"))]
pub fn set_glass(_ns_window: *mut c_void, _on: bool) -> bool {
    false
}

static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

/// Gọi một lần ở `setup` để lệnh `set_window_glass` tìm được cửa sổ chính (lệnh không nhận cửa sổ làm tham số).
pub fn init(app: &tauri::AppHandle) {
    let _ = APP.set(app.clone());
}

/// Bật/tắt kính cho cửa sổ `main`; gỡ vibrancy `sidebar` khi kính hiện, trả lại khi tắt. `true` = kính đang hiện.
pub fn apply(on: bool) -> bool {
    use tauri::utils::config::WindowEffectsConfig;
    use tauri::utils::{WindowEffect, WindowEffectState};
    use tauri::Manager;
    let Some(win) = APP.get().and_then(|a| a.get_webview_window("main")) else { return false };
    let Ok(ns) = win.ns_window() else { return false };
    let shown = set_glass(ns, on);
    let sidebar = WindowEffectsConfig { effects: vec![WindowEffect::Sidebar], state: Some(WindowEffectState::FollowsWindowActiveState), radius: None, color: None };
    let _ = win.set_effects(if shown { None } else { Some(sidebar) });
    shown
}

/// QA bản debug: đếm lớp kính và vibrancy trong contentView của cửa sổ chính.
#[cfg(all(debug_assertions, target_os = "macos"))]
pub fn probe(resize: Option<(f64, f64)>) -> Option<crate::dto::settings::GlassProbeDto> {
    use objc2::ClassType;
    use objc2::runtime::NSObjectProtocol;
    use objc2_app_kit::{NSGlassEffectView, NSVisualEffectView, NSWindow};
    use tauri::Manager;
    let win = APP.get()?.get_webview_window("main")?;
    if let Some((w, h)) = resize {
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
    }
    let ns = win.ns_window().ok()?;
    // SAFETY: như `set_glass`; chỉ đọc, và `probe` chạy ở luồng chính (lệnh sync).
    let w: &NSWindow = unsafe { &*(ns as *const NSWindow) };
    let content = w.contentView()?;
    let subs = content.subviews();
    let glass: Vec<_> = subs.iter().filter(|v| v.isKindOfClass(NSGlassEffectView::class())).collect();
    let fx = subs.iter().filter(|v| v.isKindOfClass(NSVisualEffectView::class())).count();
    let b = content.bounds();
    let f = glass.first().map(|g| g.frame());
    Some(crate::dto::settings::GlassProbeDto {
        glass_count: glass.len() as u32,
        visual_effect_count: fx as u32,
        glass_frame: f.map(|r| vec![r.origin.x, r.origin.y, r.size.width, r.size.height]).unwrap_or_default(),
        content_bounds: vec![b.size.width, b.size.height],
        subviews: subs.iter().map(|v| format!("{}#{}", v.class().name().to_string_lossy(), v.tag())).collect(),
    })
}
