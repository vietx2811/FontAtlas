use app_lib::services::export::*;
use base64::Engine;

fn b64(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn tiny_png() -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
    v.extend_from_slice(b"rest-of-file");
    v
}

#[test]
fn decode_checks_magic_size_and_base64() {
    assert_eq!(decode_png(&b64(&tiny_png())).unwrap(), tiny_png());
    let bad_magic = decode_png(&b64(b"GIF89a....")).unwrap_err();
    assert_eq!(bad_magic.code, "invalid_arg");
    assert!(decode_png(&b64(b"\x89PN")).is_err(), "ngắn hơn chữ ký");
    assert_eq!(decode_png("***không phải base64***").unwrap_err().code, "invalid_arg");
    // đúng cỡ giới hạn thì qua, vượt 1 byte thì lỗi
    let mut at_cap = vec![0u8; MAX_PNG_BYTES];
    at_cap[..8].copy_from_slice(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']);
    assert!(decode_png(&b64(&at_cap)).is_ok());
    at_cap.push(0);
    let over = decode_png(&b64(&at_cap)).unwrap_err();
    assert_eq!(over.code, "invalid_arg");
    // chuỗi base64 khổng lồ bị chặn trước khi giải mã
    assert!(decode_png(&"A".repeat(MAX_PNG_BYTES * 2)).is_err());
}

#[test]
fn names_and_extension_are_forced_to_png() {
    assert_eq!(safe_name("Compare Roboto vs Playfair"), "Compare Roboto vs Playfair.png");
    assert_eq!(safe_name("a.png"), "a.png");
    assert_eq!(safe_name("../../etc/passwd"), "passwd.png");
    assert_eq!(safe_name("..\\x\\y:z*.PNG"), "yz.png");
    assert_eq!(safe_name("   "), "compare.png");
    assert_eq!(safe_name("\n\u{0}"), "compare.png");
    assert_eq!(force_png_ext(std::path::Path::new("/tmp/a")).to_str().unwrap(), "/tmp/a.png");
    assert_eq!(force_png_ext(std::path::Path::new("/tmp/a.PNG")).to_str().unwrap(), "/tmp/a.PNG");
    assert_eq!(force_png_ext(std::path::Path::new("/tmp/a.jpg")).to_str().unwrap(), "/tmp/a.jpg.png");
}

#[test]
fn save_writes_forced_png_and_cancel_is_none() {
    let d = tempfile::tempdir().unwrap();
    let target = d.path().join("out"); // người dùng gõ tên không đuôi
    let saved = save_png_with("x/y.png", &b64(&tiny_png()), &|name| {
        assert_eq!(name, "y.png");
        Some(target.clone())
    })
    .unwrap()
    .unwrap();
    assert!(saved.ends_with("out.png"), "{saved}");
    assert_eq!(std::fs::read(&saved).unwrap(), tiny_png());
    // huỷ → None, không ghi gì
    assert_eq!(save_png_with("n", &b64(&tiny_png()), &|_| None).unwrap(), None);
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
    // dữ liệu sai thì không bao giờ mở hộp thoại
    let called = std::cell::Cell::new(false);
    assert!(save_png_with("n", &b64(b"nope"), &|_| { called.set(true); None }).is_err());
    assert!(!called.get());
    // đích không ghi được → lỗi typed
    let err = save_png_with("n", &b64(&tiny_png()), &|_| Some(d.path().join("no/such/dir/f.png"))).unwrap_err();
    assert_eq!(err.code, "io");
}

#[test]
fn forced_extension_never_overwrites_an_unconfirmed_file() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("out.png"), b"quan trong").unwrap();
    // người dùng gõ "out" (không đuôi) → ta thêm .png nhưng out.png đã có: từ chối, không ghi đè
    let err = save_png_with("n", &b64(&tiny_png()), &|_| Some(d.path().join("out"))).unwrap_err();
    assert_eq!(err.code, "exists");
    assert_eq!(std::fs::read(d.path().join("out.png")).unwrap(), b"quan trong");
    // người dùng tự chọn đúng out.png (hộp thoại đã hỏi ghi đè) → được ghi
    assert!(save_png_with("n", &b64(&tiny_png()), &|_| Some(d.path().join("out.png"))).unwrap().is_some());
    assert_eq!(std::fs::read(d.path().join("out.png")).unwrap(), tiny_png());
}
