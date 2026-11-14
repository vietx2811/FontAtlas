# SPIKE frontend-perf (code bỏ đi)
Báo cáo: docs/team/spikes/05_frontend_perf.md (repo chính).
- gen/      Rust (skrifa + tiny-skia): `cargo build --release && ./target/release/gen ../out "Hamburgefonts Ăn phở" 48`
- web/      Vite + React 19 + react-window 2: `?mode=a|b|b2|c|csvg&n=2000&auto=1`
- harness/  wk.swift (WKWebView) + run_all.py (ma trận, ghi runs.jsonl)
