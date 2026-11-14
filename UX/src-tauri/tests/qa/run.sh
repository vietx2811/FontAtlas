#!/usr/bin/env bash
# QA luồng chính trên app native thật (bản DEBUG, dữ liệu tạm; không bao giờ chạm thư viện thật).
# bash UX/src-tauri/tests/qa/run.sh   (cần đã chạy: cd UX && cargo tauri build --debug --bundles app)
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
UX="$(cd "$HERE/../../.." && pwd)"
APP="$UX/src-tauri/target/debug/bundle/macos/Font Atlas.app/Contents/MacOS/app"
test -x "$APP" || { echo "thiếu bản debug: cd UX && cargo tauri build --debug --bundles app"; exit 1; }
# QA phải chạy qua CI cục bộ (khoá + hàng đợi) và đúng bundle của SHA đang checkout: bundle nằm ở target/ DÙNG CHUNG nên run khác có thể đã ghi đè nó.
# Chạy lại riêng: bash tools/ci/local_ci.sh <ref> --qa-only. Bỏ qua kiểm (chỉ khi tự biết mình đang làm gì): FONTATLAS_QA_NO_GUARD=1.
if [ "${FONTATLAS_QA_NO_GUARD:-}" != 1 ]; then
  [ "${FONTATLAS_CI_RUN:-}" = 1 ] || { echo "run.sh phải chạy qua khoá CI: bash tools/ci/local_ci.sh <ref> --qa-only (hoặc FONTATLAS_QA_NO_GUARD=1)"; exit 1; }
  STAMP="$UX/src-tauri/target/debug/bundle/.built-sha"   # ghi bởi local_ci sau khi build xong
  WANT="$(git -C "$UX" rev-parse HEAD 2>/dev/null || true)"
  HAVE="$(cat "$STAMP" 2>/dev/null || true)"
  echo "bundle SHA: ${HAVE:-không có}; mã nguồn SHA: ${WANT:-?}"
  [ -n "$WANT" ] && [ "$HAVE" = "$WANT" ] || { echo "bundle không khớp SHA đang checkout → từ chối chạy QA (build lại: bash tools/ci/local_ci.sh <ref>)"; exit 1; }
fi
echo "tải máy: $(uptime | sed 's/.*load averages*: //')"
# Chỉ MỘT lượt QA native tại một thời điểm trên máy: font đăng ký phiên (CoreText) của app đang chạy là toàn người dùng, nên lượt QA khác
# (CI cục bộ, worktree khác) làm font của mình thành "đã có sẵn · ứng dụng khác" và dọn không sạch (conflict-clean left:5, available-*).
LOCK="${FONTATLAS_QA_LOCK:-/tmp/fontatlas-native-qa.lock}"
for _ in $(seq 1 600); do
  if mkdir "$LOCK" 2>/dev/null; then echo $$ > "$LOCK/pid"; break; fi
  holder="$(cat "$LOCK/pid" 2>/dev/null || true)"
  if [ -n "$holder" ] && ! kill -0 "$holder" 2>/dev/null; then rm -rf "$LOCK"; continue; fi   # người giữ khoá đã chết
  [ $((_ % 30)) -eq 1 ] && echo "chờ lượt QA native khác (pid ${holder:-?}) xong…"
  sleep 3
done
[ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ] || { echo "không lấy được khoá QA native sau 30 phút"; exit 1; }
release_lock() { [ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ] && rm -rf "$LOCK"; true; }
trap release_lock EXIT
# Dọn đăng ký font còn sót từ lượt QA trước bị ngắt giữa chừng (persistent scope sống qua tiến trình). Chỉ file dưới thư mục QA có qa.js.
sweep_qa_fonts() { command -v swift >/dev/null && swift "$HERE/unregister_qa_fonts.swift" 2>/dev/null || true; }
sweep_qa_fonts
# Thư mục tạm của QA (ảnh chụp + dữ liệu, ~40 MB/lượt) đặt ở ổ dữ liệu khi có, không ở ổ hệ thống (đang thiếu chỗ).
QA_BASE="${FONTATLAS_QA_TMP:-}"
[ -z "$QA_BASE" ] && [ -d /Volumes/VXData/Users/VietX ] && QA_BASE="/Volumes/VXData/Users/VietX/fontatlas-tmp/qa"
if [ -n "$QA_BASE" ]; then
  mkdir -p "$QA_BASE"
  # Dọn lượt QA cũ hơn 7 ngày, chỉ thư mục có qa.js do chính script này tạo.
  find "$QA_BASE" -maxdepth 1 -name 'qa.*' -type d -mtime +7 -exec test -f '{}/qa.js' \; -exec rm -rf '{}' + 2>/dev/null || true
  QA="$(mktemp -d "$QA_BASE/qa.XXXXXX")"
else
  QA="$(mktemp -d)"
fi
F="$UX/src-tauri/crates/fontatlas-core/tests/fixtures"
mkdir -p "$QA/A" "$QA/B" "$QA/C" "$QA/E" "$QA/G" "$QA/H" "$QA/OrgRoot" "$QA/data"
cp "$F"/{roboto,roboto_italic_vf,open_sans,public_sans_vf}.ttf "$QA/A/"
cp "$F"/{playfair_display,libre_baskerville,libre_baskerville_italic_vf,fira_sans_regular}.ttf "$QA/B/"
cp "$F"/{bebas_neue,oswald_vf,dancing_script_vf,miltonian,source_code_pro}.ttf "$QA/C/"
# E (chỉ thêm ở bước cuối của QA): 12 bản SỬA 2 byte của font đã có (cùng tên PostScript, khác nội dung) để có xung đột kích hoạt thật.
for n in roboto roboto_italic_vf open_sans public_sans_vf playfair_display libre_baskerville fira_sans_regular bebas_neue oswald_vf dancing_script_vf miltonian source_code_pro; do cp "$F/$n.ttf" "$QA/E/$n.ttf"; printf 'QA' >> "$QA/E/$n.ttf"; done
# G (cuối QA): bản chép một font hệ thống (giống hệt từng byte) → trạng thái "Có sẵn trên máy". Không có thì QA bỏ qua bước.
SYS="/System/Library/Fonts/Supplemental/Academy Engraved LET Fonts.ttf"; [ -f "$SYS" ] && cp "$SYS" "$QA/G/academy_copy.ttf" || true
cp "$F/roboto_vf.ttf" "$QA/H/" # H (cuối QA): VF Roboto trùng PostScript "Roboto-Regular" với roboto.ttf static → bước «Giữ cả hai» G7b
cp "$F/roboto.ttf" "$QA/C/roboto_copy.ttf" # cùng nội dung với A/roboto.ttf: nhóm "Cùng tệp" cho bước duplicates-view
sed -e "s#__QA_DIR__#$QA#g" -e "s#__QA_LANG__#${QA_LANG:-vi}#g" "$HERE/main_flows.js" > "$QA/qa.js"
# QA_LANG=en: các bộ chọn theo aria-label viết bằng tiếng Việt đổi sang đúng chuỗi tiếng Anh của app (khoá i18n tương ứng), để các bước vẫn tìm được phần tử và chụp ảnh bản tiếng Anh.
if [ "${QA_LANG:-vi}" = "en" ]; then
  for pair in 'Thanh bên|Sidebar' 'Cài đặt|Settings' 'Giao diện sáng/tối|Light/dark theme' 'Chi tiết font|Font details' 'Tương tự|Similar' 'Tên dự án|Project name' 'Phân loại|Class' 'Giải phẫu|Anatomy' 'Ghép cặp|Pairing'; do
    sed -i '' "s#aria-label=\"${pair%%|*}\"#aria-label=\"${pair##*|}\"#g" "$QA/qa.js"
  done
  sed -i '' "s#'Xem family'#'View family'#g" "$QA/qa.js"
fi
# Lỗi cú pháp trong kịch bản làm QA "treo" 15 phút mà không ghi bước nào: kiểm trước khi mở app.
if command -v node >/dev/null; then node --check "$QA/qa.js" || { echo "qa.js lỗi cú pháp"; exit 1; }; fi
FONTATLAS_DATA_DIR="$QA/data" FONTATLAS_QA_SCRIPT="$QA/qa.js" "$APP" > "$QA/app.log" 2>&1 &
PID=$!
trap 'kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true; sweep_qa_fonts; release_lock' EXIT   # chỉ PID của chính mình
q() { sqlite3 "file:$QA/data/user.db?mode=ro" "$1" 2>/dev/null || true; }
# Tối đa 15 phút (máy bận chạy chậm hơn); kiểm mỗi 2 giây để dừng ngay khi script báo xong.
for _ in $(seq 1 450); do
  sleep 2
  # Dừng khi script ghi bước kết thúc. So TÊN BƯỚC (json_extract), không so chuỗi: chi tiết của bước thường có thể chứa chữ "error".
  [ -n "$(q "select 1 from settings where key like 'ui.qa.%' and json_extract(value,'\$.step') in ('done','error','onb-stuck') limit 1")" ] && break
done
for t in "$QA"/data/snap-*.tiff; do [ -f "$t" ] && sips -s format png "$t" --out "${t%.tiff}.png" >/dev/null && rm "$t"; done
echo "ảnh: $QA/data/snap-*.png"
for th in light dark; do b64="$(q "select value from settings where key like 'ui.qapng.$th.%' order by key" | tr -d '\n')"; [ -n "$b64" ] && printf '%s' "$b64" | base64 -D > "$QA/data/png-$th.png" && echo "ảnh PNG xuất ($th): $QA/data/png-$th.png"; done
q "select value from settings where key like 'ui.qa.%' order by key" | python3 -c '
import sys, json
fail = 0
seen = set()
for line in sys.stdin:
    d = json.loads(line); fail += not d["ok"]
    print("OK  " if d["ok"] else "FAIL", "%4ss" % d.get("t", "?"), d["step"], "" if d["ok"] else json.dumps(d.get("detail"), ensure_ascii=False)[:600])
    seen.add(d["step"])
if "done" not in seen:
    print("FAIL    ?s done (script không chạy tới bước cuối — QA dừng sớm, app treo hoặc hết 15 phút)"); fail += 1
sys.exit(1 if fail else 0)'
