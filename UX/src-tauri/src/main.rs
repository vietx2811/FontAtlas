// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Tiến trình con của probe sức khoẻ nguồn font request: hỏi một font rồi thoát, không khởi động app.
    if app_lib::services::fontreq::run_probe_child_if_asked() {
        return;
    }
    app_lib::run();
}
