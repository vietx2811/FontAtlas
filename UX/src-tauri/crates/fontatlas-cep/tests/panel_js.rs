//! Chạy bộ test JS của panel (host.jsx + main.js trong vm của Node, với Illustrator giả). Không cần Illustrator.
//! Không có `node` trên máy: bỏ qua (ghi rõ), không làm đỏ; trên máy dev/CI của dự án thì `node` luôn có.
use std::process::Command;

#[test]
fn panel_js_tests_pass_under_node() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let Ok(node) = Command::new("node").arg("--version").output() else {
        eprintln!("bỏ qua: không có `node` trên PATH");
        return;
    };
    assert!(node.status.success());
    let out = Command::new("node").arg("--test").arg(format!("{dir}/jstests/panel.test.mjs")).output().expect("chạy node --test");
    assert!(out.status.success(), "test JS của panel đỏ:\n{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}
