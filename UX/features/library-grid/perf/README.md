# Harness S2 — lưới thư viện trên WKWebView

Test hồi quy hiệu năng cho tiêu chí S2 (spec 1.5): lưới 5.000 font cuộn 60 FPS, bộ nhớ WebContent < 400 MB, 0% ô trống khi cuộn chậm.

```bash
cd UX
npm run perf:s2                  # previewMode = 'mask' (mặc định)
npm run perf:s2 -- --mode img    # phương án dự phòng <img> + invert
```

- Yêu cầu: macOS (WKWebView, `footprint`), Xcode Command Line Tools (`swiftc`), Rust, Node. Không chạy trên CI Linux; CI chạy job `s2` trên `macos-14` ở chế độ báo cáo.
- Bước chạy: dựng preview cho `/System/Library/Fonts` + `/Library/Fonts` bằng `dump_previews` (lần đầu) → Vite mock backend ở cổng 5199 → WKWebView mở `bench.html` 3 lần → trung vị.
- Kịch bản (giống spike 05): cuộn nhanh 4.000 px/s 6 s, cuộn chậm 600 px/s 6 s, nghỉ 1,5 s, đổi chữ mẫu.
- Ngưỡng: `fps ≥ 58`, `frame_p95 ≤ 20 ms`, `slow_blank_pct = 0`, `proc_mb < 400`. Không nới ngưỡng; nếu trượt, ghi số vào báo cáo.
- Kết quả thô: `perf/.data/runs.jsonl`.
