#!/usr/bin/env bash
# Smoke build macOS: dựng bản debug, mở với thư mục dữ liệu tạm, kiểm tiến trình sống và có library.db + user.db.
# Chạy từ bất kỳ đâu: bash UX/src-tauri/tests/smoke.sh
set -euo pipefail
UX="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$UX"
npm ci
npm run build
cargo tauri build --debug --bundles app
APP="$UX/src-tauri/target/debug/bundle/macos/Font Atlas.app"
test -d "$APP" || { echo "thiếu $APP"; exit 1; }
DATA="$(mktemp -d)"
FONTATLAS_DATA_DIR="$DATA" "$APP/Contents/MacOS/app" >"$DATA/stdout.log" 2>&1 &
PID=$!
trap 'kill "$PID" 2>/dev/null || true' EXIT   # chỉ PID của chính mình
sleep 10
kill -0 "$PID" || { echo "app thoát sớm"; tail -20 "$DATA/stdout.log"; exit 1; }
test -f "$DATA/library.db" && test -f "$DATA/user.db" || { echo "thiếu DB trong $DATA"; ls "$DATA"; exit 1; }
echo "smoke OK: pid=$PID data=$DATA"
