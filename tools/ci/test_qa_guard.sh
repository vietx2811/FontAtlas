#!/usr/bin/env bash
# Thử các chốt của native QA: run.sh từ chối ngoài CI / khi bundle lệch SHA; local_ci --qa-only từ chối khi bundle không phải của SHA.
#   bash tools/ci/test_qa_guard.sh
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"; cd "$ROOT"
T="$(mktemp -d)"; export FONTATLAS_CI_HOME="$T/ci"
trap 'rm -rf "$T"; git worktree prune' EXIT
fail=0; check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: muốn '$3', được '$2'"; fail=1; fi; }

# Cây giả: <T>/repo/UX/src-tauri/{tests/qa/run.sh, target/debug/bundle/macos/Font Atlas.app/Contents/MacOS/app}
R="$T/repo"; mkdir -p "$R/UX/src-tauri/tests/qa" "$R/UX/src-tauri/target/debug/bundle/macos/Font Atlas.app/Contents/MacOS"
cp UX/src-tauri/tests/qa/run.sh "$R/UX/src-tauri/tests/qa/"; printf '#!/bin/sh\\n' > "$R/UX/src-tauri/target/debug/bundle/macos/Font Atlas.app/Contents/MacOS/app"; chmod +x "$R/UX/src-tauri/target/debug/bundle/macos/Font Atlas.app/Contents/MacOS/app"
( cd "$R" && git init -q && git add -A && git -c user.email=t@t -c user.name=t commit -qm x )
RUN="$R/UX/src-tauri/tests/qa/run.sh"; SHA="$(git -C "$R" rev-parse HEAD)"; STAMP="$R/UX/src-tauri/target/debug/bundle/.built-sha"
out="$(bash "$RUN" 2>&1)"; check "ngoài CI bị từ chối" "$(grep -c 'phải chạy qua khoá CI' <<<"$out")" 1
out="$(FONTATLAS_CI_RUN=1 bash "$RUN" 2>&1)"; check "thiếu stamp bị từ chối" "$(grep -c 'không khớp SHA' <<<"$out")" 1
echo deadbeef > "$STAMP"
out="$(FONTATLAS_CI_RUN=1 bash "$RUN" 2>&1)"; check "stamp lệch bị từ chối" "$(grep -c 'không khớp SHA' <<<"$out")" 1
echo "$SHA" > "$STAMP"
out="$(FONTATLAS_CI_RUN=1 FONTATLAS_QA_LOCK="$T/qalock" timeout 20 bash "$RUN" 2>&1)"
check "stamp khớp qua chốt" "$(grep -c 'từ chối' <<<"$out")" 0
check "log tải máy" "$(grep -c '^tải máy:' <<<"$out")" 1

# local_ci --qa-only: bundle của repo CI chưa có stamp → thoát 2, không chạy gì.
bash tools/ci/local_ci.sh HEAD --qa-only >/dev/null 2>&1; check "--qa-only thiếu bundle thoát 2" "$?" 2
exit $fail
