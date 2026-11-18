#!/usr/bin/env bash
# Tự kiểm «cloud tải xong» (07): app DEBUG, dữ liệu tạm, font thử. Q=<thư mục thử trên VXData> bash run08.sh <kịch bản js> [APP]
set -uo pipefail
Q="${Q:?}"; JS="${1:?kịch bản js}"; HERE="$(cd "$(dirname "$0")" && pwd)"
APP="${APP:-$Q/Font Atlas.app/Contents/MacOS/app}"; [ -x "$APP" ] || { echo "thiếu $APP"; exit 1; }
LOCK=/tmp/fontatlas-native-qa.lock
for _ in $(seq 1 600); do
  if mkdir "$LOCK" 2>/dev/null; then echo $$ > "$LOCK/pid"; break; fi
  h="$(cat "$LOCK/pid" 2>/dev/null || true)"; [ -n "$h" ] && ! kill -0 "$h" 2>/dev/null && { rm -rf "$LOCK"; continue; }
  sleep 3
done
[ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ] || { echo "không lấy được khoá QA"; exit 1; }
RUN="${RUN:-$Q/run-$(date +%H%M%S)}"; DATA="$RUN/data"; mkdir -p "$DATA"; : > "$RUN/ops.jsonl"; if [ -n "${KEEP_DATA:-}" ]; then sqlite3 "$DATA/user.db" "delete from settings where key like 'ui.qa.%';" 2>/dev/null; fi
T0=$(python3 -c 'import time;print(int(time.time()*1000))')
{ echo "window.__Q8_T0=$T0; window.__Q8_ONLY='${ONLY:-}';"; echo "(() => {"; cat "$HERE/lib8.js"; sed -e "s#__Q__#$Q#g" "$JS"; echo "})();"; } > "$RUN/qa08.js"
node --check "$RUN/qa08.js" || { rmdir "$LOCK"; exit 1; }
FONTATLAS_DATA_DIR="$DATA" FONTATLAS_QA_SCRIPT="$RUN/qa08.js" "$APP" > "$RUN/app.log" 2>&1 &
PID=$!
cleanup() { kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true; [ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ] && rm -rf "$LOCK"; true; }
trap cleanup EXIT
q() { sqlite3 -cmd '.timeout 4000' "file:$DATA/user.db?mode=ro" "$1" 2>/dev/null || true; }
: > "$RUN/done.list"
for _ in $(seq 1 $(( ${MAXWAIT:-300} * 2 ))); do
  sleep 1
  while IFS= read -r name; do
    grep -qxF "$name" "$RUN/done.list" && continue; echo "$name" >> "$RUN/done.list"
    case "$name" in sc-A) python3 "$HERE/cloudops.py" "$Q" A "$RUN/ops.jsonl" & ;; sc-B) python3 "$HERE/cloudops.py" "$Q" B "$RUN/ops.jsonl" & ;; sc-C) python3 "$HERE/cloudops.py" "$Q" C "$RUN/ops.jsonl" & ;; sc-D) python3 "$HERE/cloudops.py" "$Q" D "$RUN/ops.jsonl" & ;; esac
  done < <(q "select json_extract(value,'\$.step') from settings where key like 'ui.qa.%' and json_extract(value,'\$.step') like 'q8-wait-%'" | sed 's/^q8-wait-//')
  [ -n "$(q "select 1 from settings where key like 'ui.qa.%' and json_extract(value,'\$.step') in ('q8-done') limit 1")" ] && break
done
q "select value from settings where key like 'ui.qa.%' order by key" | python3 -c '
import sys, json
for l in sys.stdin:
    d = json.loads(l); print("OK  " if d["ok"] else "FAIL", "%4ss" % d.get("t", "?"), d["step"], json.dumps(d.get("detail"), ensure_ascii=False)[:1500])'
echo "run: $RUN"
