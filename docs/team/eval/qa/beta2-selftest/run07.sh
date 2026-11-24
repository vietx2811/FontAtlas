#!/usr/bin/env bash
# Chạy tự kiểm beta.2 trên app DEBUG với dữ liệu tạm. Dùng:  Q=<thư mục thử trên VXData> bash run07.sh [pha,pha]
# Không bao giờ mở bản release. Chỉ dừng PID của chính mình. Giữ khoá QA native chung của máy (như tests/qa/run.sh).
set -uo pipefail
Q="${Q:?đặt Q=thư mục thử trên VXData}"
APPNAME="${APPNAME:-Calculator}"; APPID="${APPID:-com.apple.calculator}"
HERE="$(cd "$(dirname "$0")" && pwd)"
APP="$Q/target/debug/bundle/macos/Font Atlas.app/Contents/MacOS/app"
[ -x "$APP" ] || { echo "thiếu bản debug: $APP"; exit 1; }
LOCK=/tmp/fontatlas-native-qa.lock
for _ in $(seq 1 600); do
  if mkdir "$LOCK" 2>/dev/null; then echo $$ > "$LOCK/pid"; break; fi
  h="$(cat "$LOCK/pid" 2>/dev/null || true)"; [ -n "$h" ] && ! kill -0 "$h" 2>/dev/null && { rm -rf "$LOCK"; continue; }
  sleep 3
done
[ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ] || { echo "không lấy được khoá QA"; exit 1; }
# App thử (mặc định Calculator, không có tài liệu) mà chủ máy đang chạy thì không đụng tới. Chỉ chạy pha appset khi nó chưa chạy.
if { [ -z "${1:-}" ] || [[ ",${1:-}," == *",appset,"* ]]; } && pgrep -x "$APPNAME" >/dev/null; then echo "$APPNAME đang chạy (của chủ máy): dừng, không đụng tới"; rmdir "$LOCK" 2>/dev/null; exit 1; fi
RUN="$Q/run-$(date +%H%M%S)"; DATA="$RUN/data"; mkdir -p "$DATA" "$RUN"
rm -rf "$Q/OrgStore"
ONLY="${1:-}"
{ echo "window.__Q7_ONLY='$ONLY'; window.__Q7_APP='$APPID';"; sed -e "s#__Q__#$Q#g" "$HERE/qa07.js"; } > "$RUN/qa07.js"
node --check "$RUN/qa07.js" || { rmdir "$LOCK" 2>/dev/null; echo "lỗi cú pháp"; exit 1; }
FONTATLAS_DATA_DIR="$DATA" FONTATLAS_QA_SCRIPT="$RUN/qa07.js" "$APP" > "$RUN/app.log" 2>&1 &
PID=$!
cleanup() { kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true; [ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ] && rm -rf "$LOCK"; true; }
trap cleanup EXIT
q() { sqlite3 -cmd '.timeout 4000' "file:$DATA/user.db?mode=ro" "$1" 2>/dev/null || true; }
H="$RUN/handlers.log"
handler() { # $1 = tên điểm dừng; việc kiểm trên đĩa
  case "$1" in
    org-copied)
      L="$Q/OrgStore/Library"
      { echo "== cấu trúc kho"; (cd "$L" 2>/dev/null && find . -type f | sort | head -80)
        echo "== số file: $(find "$L" -type f 2>/dev/null | wc -l | tr -d ' ')"
        echo "== file không đúng dạng Class/Họ/file: $(cd "$L" && find . -type f | awk -F/ 'NF!=4' | wc -l | tr -d ' ')"
        echo "== file .part/tạm: $(find "$Q/OrgStore" \( -name "*.part" -o -name "*.tmp" \) 2>/dev/null | wc -l | tr -d ' ')"
        (cd "$Q/fonts" && find . -type f -exec shasum {} \; | sort -k2) > "$RUN/src-checksums.after"
        echo "== nguồn nguyên vẹn: $(diff -q "$Q/src-checksums.before" "$RUN/src-checksums.after" >/dev/null && echo CÓ || echo KHÔNG)"
        echo "== tên file small giữ nguyên: $(for f in "$Q"/fonts/small/*.ttf; do b="$(basename "$f")"; find "$L" -name "$b" | grep -q . || echo "THIẾU $b"; done | tr '\n' ' ')"
        echo "== nội dung bản chép khớp nguồn (small): $(for f in "$Q"/fonts/small/*.ttf; do b="$(basename "$f")"; c="$(find "$L" -name "$b" | head -1)"; cmp -s "$f" "$c" || echo "KHÁC $b"; done | tr '\n' ' ')"
        echo "== dung lượng kho: $(du -sh "$Q/OrgStore" | cut -f1)"; } >> "$H" ;;
    org-done)
      { echo "== sau resort-preview/snapshot"; echo "số file kho: $(find "$Q/OrgStore/Library" -type f | wc -l | tr -d ' ')"
        echo "gói khôi phục:"; find "$Q/OrgStore" -maxdepth 3 -iname '*snapshot*' -o -maxdepth 3 -iname '*recovery*' 2>/dev/null | head; ls "$Q/OrgStore" ; } >> "$H" ;;
    te-open-*) open -a "$APPNAME"; echo "$(date +%T) $1: mở $APPNAME" >> "$H" ;;
    te-quit-*) osascript -e "tell application \"$APPNAME\" to quit" >/dev/null 2>&1; echo "$(date +%T) $1: thoát $APPNAME" >> "$H" ;;
    vf-before-remove|vf-after-remove|vf-after-restore) { echo "== $1: bản sao đổi tên trong dữ liệu tạm"; find "$DATA" -iname '*VF*' -type f 2>/dev/null | head -5; } >> "$H" ;;
    *) echo "(không có việc cho $1)" >> "$H" ;;
  esac
}
: > "$RUN/done.list"
for _ in $(seq 1 900); do
  sleep 2
  while IFS= read -r name; do
    grep -qxF "$name" "$RUN/done.list" && continue; echo "$name" >> "$RUN/done.list"
    echo "[$(date +%T)] điểm dừng $name"; handler "$name"
    sqlite3 -cmd '.timeout 4000' "$DATA/user.db" "insert or replace into settings(key,value) values('qa07.go','$name');" 2>&1 | head -2
  done < <(q "select json_extract(value,'\$.step') from settings where key like 'ui.qa.%' and json_extract(value,'\$.step') like 'q7-wait-%'" | sed 's/^q7-wait-//')
  [ -n "$(q "select 1 from settings where key like 'ui.qa.%' and json_extract(value,'\$.step')='q7-done' limit 1")" ] && break
done
q "select value from settings where key like 'ui.qa.%' order by key" > "$RUN/results.jsonl"
python3 - "$RUN/results.jsonl" <<'P'
import sys, json
for l in open(sys.argv[1]):
    d = json.loads(l)
    if d["step"].startswith("q7-wait"): continue
    print("OK  " if d["ok"] else "FAIL", "%4ss" % d.get("t", "?"), d["step"], json.dumps(d.get("detail"), ensure_ascii=False)[:900])
P
echo "---- handlers"; cat "$H" 2>/dev/null; echo "run: $RUN"
