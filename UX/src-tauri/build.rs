fn main() {
    tauri_build::build();

    // Windows (W1): file test tích hợp liên kết tauri nhưng không nhận manifest Common-Controls v6 mà tauri-build chỉ nhúng cho binary app
    // → chết lúc khởi động (0xc0000139). Nhúng manifest riêng chỉ cho test; không đụng build mac/linux và không đụng binary app.
    let target = |k: &str| std::env::var(k).unwrap_or_default();
    if target("CARGO_CFG_TARGET_OS") == "windows" && target("CARGO_CFG_TARGET_ENV") == "msvc" {
        let manifest = std::path::Path::new(&target("CARGO_MANIFEST_DIR")).join("windows-test.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}", manifest.display());
    }
}
