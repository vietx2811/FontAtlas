#!/usr/bin/env bash
# Chạy PHẦN CHUẨN BỊ của run.sh (FONTATLAS_QA_PREP_ONLY=1, không mở app) dưới set -euo pipefail thật, để lỗi kiểu «ls|head / mã thoát preflight giết script im lặng» bị bắt.
# bash tools/ci/test_qa_prep.sh
set -uo pipefail
cd "$(dirname "$0")/../.."
command -v python3 >/dev/null || { echo "SKIP không có python3 (patch_font_names.py cần)"; exit 0; }
[ "$(uname)" = Darwin ] || { echo "SKIP chỉ chạy trên macOS (run.sh dùng sed -i '' / base64 -D)"; exit 0; }
RUN=UX/src-tauri/tests/qa/run.sh; fail=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: muốn '$3', được '$2'"; fail=1; fi; }
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
export FONTATLAS_QA_NO_GUARD=1 FONTATLAS_QA_PREP_ONLY=1 FONTATLAS_QA_LOCK="$T/lock" FONTATLAS_QA_TMP="$T/base"
# 1) Đường thường: swift thật (nếu có) hoặc không có swift → chuẩn bị xong, in PREP-OK, nhả khoá.
out="$(bash $RUN 2>&1)"; rc=$?
check "chuẩn bị chạy hết (exit 0)" "$rc" 0
check "in PREP-OK" "$(grep -c '^PREP-OK ' <<<"$out")" 1
check "khoá được nhả" "$([ -d "$T/lock" ] && echo held || echo free)" free
d="$(sed -n 's/^PREP-OK //p' <<<"$out")"
check "đủ fixture E (12 bản)" "$(ls "$d"/E/*.ttf 2>/dev/null | wc -l | tr -d ' ')" 12
check "fixture đã đổi tên (FAQA)" "$([ -s "$d/patched_names.tsv" ] && grep -c FAQA "$d/patched_names.tsv" | awk '$1>0{print "yes"}')" yes
# 2) Preflight báo trùng (swift giả thoát 2) → phải IN thông báo và thoát 3, không chết im lặng.
mkdir -p "$T/bin"; cat > "$T/bin/swift" <<'SW'
#!/usr/bin/env bash
case "$1" in *qa_preflight_fonts.swift) echo "FONT-REGISTERED Roboto /x/y.ttf"; exit 2;; *) exit 0;; esac
SW
chmod +x "$T/bin/swift"
out="$(PATH="$T/bin:$PATH" bash $RUN 2>&1)"; rc=$?
check "preflight trùng → exit 3" "$rc" 3
check "preflight trùng → có in thông báo" "$(grep -c 'QA preflight: font fixture đã được đăng ký' <<<"$out")" 1
check "preflight trùng → khoá được nhả" "$([ -d "$T/lock" ] && echo held || echo free)" free
out="$(FONTATLAS_QA_ALLOW_REGISTERED=1 PATH="$T/bin:$PATH" bash $RUN 2>&1)"; rc=$?
check "ALLOW_REGISTERED=1 → vẫn chạy hết" "$rc" 0
exit $fail
