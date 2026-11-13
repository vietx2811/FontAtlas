# Onboarding 2 mảng + kính thật (2026-11-13)

- **Yêu cầu:** mảng phải = minh hoạ blueprint phủ kín, đổi theo bước; mảng trái = nội dung trên kính macOS (trong suốt), 44/56, tối thiểu 1000 px.
- **Kính thật có dùng được không:** máy này macOS 27.2, SDK có `NSGlassEffectView` (macOS 26+). `objc2-app-kit 0.3.2` có binding (feature `NSGlassEffectView`) nên không cần FFI tay. Tauri không có `Effect` glass (chỉ vibrancy/Mica), nên phải gọi AppKit trực tiếp.
- **Cách làm:** lệnh `set_window_glass(on)` chèn một `NSGlassEffectView` DƯỚI webview (webview vốn trong suốt: `transparent:true`). Khi kính bật, gỡ vibrancy `sidebar` bằng `set_effects(None)` để hai lớp không chồng nhau; tắt kính thì trả đúng vibrancy `sidebar` như `tauri.conf`. Onboarding bật khi vào, tắt khi rời (cleanup) nên thư viện sau đó giữ nguyên diện mạo cũ.
- **Lùi an toàn:** macOS < 26 hoặc không phải macOS → lệnh trả `false`, vibrancy `sidebar` giữ nguyên; ngoài app native (trình duyệt/test/mock) mảng trái có nền đặc theo theme.
- **Tương phản:** nền sau cửa sổ có thể đen/trắng tuyền → mảng trái có lớp tint 78% màu sheet; chữ phụ/cảnh báo/liên kết dùng token riêng `--onb-*`. Test `onboardingContrast.test.ts` tính bằng số: mọi token >= 4.5:1 trên nền xấu nhất (đen hoặc trắng sau kính), cả sáng lẫn tối.
- **Mảng phải:** giấy kẻ ô vẽ bằng CSS (container query units) khớp lưới của hình vuông 480×480 ở giữa nên hình liền mạch với nền; đổi bước = remount (`key`) + `fa-art-in` 0,5 s; reduced-motion tắt mọi animation sẵn có trong `.fa-onb`.
- **Chưa kiểm:** hình dạng thật của kính (độ khúc xạ, sáng/tối theo nền) phải xem bằng native QA (ảnh onb-00..08); trong trình duyệt chỉ kiểm được bố cục và tương phản bằng số.
