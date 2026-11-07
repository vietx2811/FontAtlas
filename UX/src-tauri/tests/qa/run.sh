#!/usr/bin/env bash
# QA luồng chính trên app native thật (bản DEBUG, dữ liệu tạm; không bao giờ chạm thư viện thật).
# bash UX/src-tauri/tests/qa/run.sh   (cần đã chạy: cd UX && cargo tauri build --debug --bundles app)
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
UX="$(cd "$HERE/../../.." && pwd)"
APP="$UX/src-tauri/target/debug/bundle/macos/Font Atlas.app/Contents/MacOS/app"
test -x "$APP" || { echo "thiếu bản debug: cd UX && cargo tauri build --debug --bundles app"; exit 1; }
QA="$(mktemp -d)"; F="$UX/src-tauri/crates/fontatlas-core/tests/fixtures"
mkdir -p "$QA/A" "$QA/B" "$QA/C" "$QA/data"
cp "$F"/{roboto,roboto_italic_vf,open_sans,public_sans_vf}.ttf "$QA/A/"
cp "$F"/{playfair_display,libre_baskerville,libre_baskerville_italic_vf,fira_sans_regular}.ttf "$QA/B/"
cp "$F"/{bebas_neue,oswald_vf,dancing_script_vf,miltonian,source_code_pro}.ttf "$QA/C/"
cp "$F/roboto.ttf" "$QA/C/roboto_copy.ttf" # cùng nội dung với A/roboto.ttf: nhóm "Cùng tệp" cho bước duplicates-view
sed "s#__QA_DIR__#$QA#g" "$HERE/main_flows.js" > "$QA/qa.js"
# Lỗi cú pháp trong kịch bản làm QA "treo" 15 phút mà không ghi bước nào: kiểm trước khi mở app.
if command -v node >/dev/null; then node --check "$QA/qa.js" || { echo "qa.js lỗi cú pháp"; exit 1; }; fi
FONTATLAS_DATA_DIR="$QA/data" FONTATLAS_QA_SCRIPT="$QA/qa.js" "$APP" > "$QA/app.log" 2>&1 &
PID=$!
trap 'kill "$PID" 2>/dev/null || true' EXIT   # chỉ PID của chính mình
q() { sqlite3 "file:$QA/data/user.db?mode=ro" "$1" 2>/dev/null || true; }
# Tối đa 15 phút (máy bận chạy chậm hơn); kiểm mỗi 2 giây để dừng ngay khi script báo xong.
for _ in $(seq 1 450); do
  sleep 2
  case "$(q "select group_concat(value) from settings where key like 'ui.qa.%'")" in *'"done"'*|*'"error"'*|*onb-stuck*) break;; esac
done
for t in "$QA"/data/snap-*.tiff; do [ -f "$t" ] && sips -s format png "$t" --out "${t%.tiff}.png" >/dev/null && rm "$t"; done
echo "ảnh: $QA/data/snap-*.png"
q "select value from settings where key like 'ui.qa.%' order by key" | python3 -c '
import sys, json
fail = 0
for line in sys.stdin:
    d = json.loads(line); fail += not d["ok"]
    print("OK  " if d["ok"] else "FAIL", "%4ss" % d.get("t", "?"), d["step"], "" if d["ok"] else json.dumps(d["detail"], ensure_ascii=False)[:600])
sys.exit(1 if fail else 0)'
