#!/usr/bin/env bash
# Tự kiểm beta.5 (07): bản DEBUG, dữ liệu tạm trên VXData, chỉ font fixture. Q=<thư mục thử> APP=<đường tới binary debug> bash run09.sh [pha,pha]
set -uo pipefail
Q="${Q:?}"; APP="${APP:?}"; HERE="$(cd "$(dirname "$0")" && pwd)"; SHOTS="${SHOTS:?đặt SHOTS=thư mục ảnh}"
[ -x "$APP" ] || { echo "thiếu bản debug: $APP"; exit 1; }
LOCK=/tmp/fontatlas-native-qa.lock
for _ in $(seq 1 900); do
  if mkdir "$LOCK" 2>/dev/null; then echo $$ > "$LOCK/pid"; break; fi
  h="$(cat "$LOCK/pid" 2>/dev/null || true)"; [ -n "$h" ] && ! kill -0 "$h" 2>/dev/null && { rm -rf "$LOCK"; continue; }
  sleep 3
done
[ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ] || { echo "không lấy được khoá QA"; exit 1; }
RUN="$Q/run-$(date +%H%M%S)"; DATA="$RUN/data"; mkdir -p "$DATA" "$RUN" "$SHOTS"
rm -rf "$Q/OrgStore"; : > "$RUN/handlers.log"
python3 "$HERE/cloudreset.py" "$Q" >/dev/null 2>&1
ONLY="${1:-}"
{ echo "window.__Q9_ONLY='$ONLY';"; sed -e "s#__Q__#$Q#g" "$HERE/qa09.js"; } > "$RUN/qa09.js"
node --check "$RUN/qa09.js" || { rmdir "$LOCK" 2>/dev/null; echo "lỗi cú pháp"; exit 1; }
UNREG="${UNREG:-$HERE/../../../../../UX/src-tauri/tests/qa/unregister_qa_fonts.swift}"
swift "$UNREG" "$Q/fonts" >/dev/null 2>&1 || true
FONTATLAS_DATA_DIR="$DATA" FONTATLAS_QA_SCRIPT="$RUN/qa09.js" "$APP" > "$RUN/app.log" 2>&1 &
PID=$!
cleanup() { kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
  [ -f "$RUN/cloudsim.pid" ] && kill "$(cat "$RUN/cloudsim.pid")" 2>/dev/null
  swift "$UNREG" "$Q/fonts" >/dev/null 2>&1 || true
  [ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ] && rm -rf "$LOCK"; true; }
trap cleanup EXIT
q() { sqlite3 -cmd '.timeout 4000' "file:$DATA/user.db?mode=ro" "$1" 2>/dev/null || true; }
H="$RUN/handlers.log"; CT="$HERE/ctlist.swift"
ctcheck() { echo "== $1 $(date +%T): CoreText đăng ký dưới fonts/:"; swift "$CT" "$Q/fonts" 2>&1 | sed "s#$Q/##" | sort | tr '\n' ' '; echo; }
handler() {
  case "$1" in
    act-on) { ctcheck act-on; ( system_profiler SPFontsDataType -json 2>/dev/null | python3 -c "import sys,json;d=json.load(sys.stdin);print('system_profiler: ', sorted({f['path'].replace('$Q/','') for x in d['SPFontsDataType'] for f in [x] if '$Q/fonts' in x.get('path','')}))" > "$RUN/sysprof-on.txt" 2>&1 & ) ; } >> "$H" ;;
    act-off) ctcheck act-off >> "$H" ;;
    act-reloc) ctcheck act-reloc >> "$H" ;;
    act-end) ctcheck act-end >> "$H" ;;
    cloud-start) ( python3 "$HERE/cloudsim.py" "$Q" > "$RUN/cloudsim.log" 2>&1 & CSP=$!; echo $CSP > "$RUN/cloudsim.pid" ); echo "$(date +%T) cloud-start: mô phỏng tải 40 file (stub→thật, 1,2 s/file)" >> "$H" ;;
    org-before) { echo "== org-before: file nguồn big: $(ls "$Q/fonts/big" | wc -l)"; (cd "$Q/fonts" && find . -type f -exec shasum {} \; | sort -k2) > "$RUN/src.before"; } >> "$H" ;;
    org-cancelled) { L="$Q/OrgStore"; echo "== org-cancelled: file trong kho: $(find "$L" -type f ! -name '.*' 2>/dev/null | wc -l | tr -d ' ')"
        echo "   file .part/.tmp/dở: $(find "$L" \( -name '*.part' -o -name '*.tmp' -o -name '*.partial' \) 2>/dev/null | wc -l | tr -d ' ')"
        (cd "$Q/fonts" && find . -type f -exec shasum {} \; | sort -k2) > "$RUN/src.after"
        echo "   nguồn nguyên vẹn: $(diff -q "$RUN/src.before" "$RUN/src.after" >/dev/null && echo CÓ || echo KHÔNG)"
        echo "   bản chép khớp nguồn: $(cd "$L" && find . -type f ! -name '.*' | head -400 | while read f; do b=$(basename "$f"); s=$(find "$Q/fonts" -name "$b" | head -1); cmp -s "$f" "$s" || echo "KHÁC $b"; done | head -5 | tr '\n' ' ')"; } >> "$H" ;;
    resort-before|resort-applied|resort-undone|resort-cancelled|resort-final) { L="$Q/OrgR/Library"; echo "== $1 $(date +%T): ttf=$(find "$L" -name '*.ttf' | wc -l | tr -d ' ') partial=$(find "$Q/OrgR" \( -name '*.part' -o -name '*.tmp' \) | wc -l | tr -d ' ') phân bố: $(cd "$L" && for d in */; do echo -n "$d$(find "$d" -name '*.ttf' | wc -l | tr -d ' ') "; done)"
        [ "$1" = resort-before ] && (cd "$Q/fonts" && find . -type f -exec shasum {} \; | sort -k2) > "$RUN/src.rb"
        (cd "$Q/fonts" && find . -type f -exec shasum {} \; | sort -k2) > "$RUN/src.now"; echo "   nguồn nguyên vẹn: $(diff -q "$RUN/src.rb" "$RUN/src.now" >/dev/null 2>&1 && echo CÓ || echo KHÔNG/NA)"
        echo "   .ttf khác bản gốc: $(cd "$L" && find . -name '*.ttf' -print0 | while IFS= read -r -d '' f; do b=$(basename "$f"); s=$(find "$Q/fonts/lib" -name "$b" | head -1); cmp -s "$f" "$s" || echo x; done | wc -l | tr -d ' ')"; } >> "$H" ;;
    *) echo "(không có việc cho $1)" >> "$H" ;;
  esac
}
: > "$RUN/done.list"
for _ in $(seq 1 1500); do
  sleep 2
  while IFS= read -r name; do
    grep -qxF "$name" "$RUN/done.list" && continue; echo "$name" >> "$RUN/done.list"; echo "[$(date +%T)] điểm dừng $name"; handler "$name"
  done < <(q "select json_extract(value,'\$.step') from settings where key like 'ui.qa.%' and json_extract(value,'\$.step') like 'q9-wait-%'" | sed 's/^q9-wait-//')
  [ -n "$(q "select 1 from settings where key like 'ui.qa.%' and json_extract(value,'\$.step') in ('q9-done','q9-error') limit 1")" ] && break
done
for t in "$DATA"/snap-*.tiff; do [ -f "$t" ] && b="$(basename "${t%.tiff}")" && sips -s format png "$t" --out "$SHOTS/${b#snap-}.png" >/dev/null 2>&1; done
q "select value from settings where key like 'ui.qa.%' order by key" > "$RUN/results.jsonl"
python3 - "$RUN/results.jsonl" <<'P'
import sys, json
for l in open(sys.argv[1]):
    d = json.loads(l)
    if d["step"].startswith("q9-wait"): continue
    print("OK  " if d["ok"] else "FAIL", "%4ss" % d.get("t", "?"), d["step"], json.dumps(d.get("detail"), ensure_ascii=False)[:1400])
P
echo "---- handlers"; cat "$H" 2>/dev/null; echo "run: $RUN"
