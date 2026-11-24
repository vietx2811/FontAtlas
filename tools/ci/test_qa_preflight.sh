#!/usr/bin/env bash
# Thử qa_preflight_fonts.swift và unregister_qa_fonts.swift (chỉ đọc CoreText; không mở app). bash tools/ci/test_qa_preflight.sh
set -uo pipefail
cd "$(dirname "$0")/../.."
Q=UX/src-tauri/tests/qa; fail=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: muốn '$3', được '$2'"; fail=1; fi; }
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
SYS="$(ls /System/Library/Fonts/Supplemental/Arial.ttf /System/Library/Fonts/Supplemental/Courier\ New.ttf 2>/dev/null | head -1)"
if [ -z "$SYS" ]; then echo "SKIP không có font hệ thống để thử"; exit 0; fi
mkdir -p "$T/fx" "$T/empty"; cp "$SYS" "$T/fx/"
out="$(swift $Q/qa_preflight_fonts.swift "$T/fx" 2>&1)"; rc=$?
check "font trùng với font hệ thống đã đăng ký → exit 2" "$rc" 2
check "in đường dẫn font đã đăng ký" "$(grep -c "^FONT-REGISTERED .* $SYS\$" <<<"$out")" 1
swift $Q/qa_preflight_fonts.swift "$T/empty" >/dev/null 2>&1; check "thư mục fixture rỗng → exit 0" "$?" 0
# sweep: font hệ thống không bao giờ bị gỡ dù thư mục thử trỏ nhầm tới nó? chỉ kiểm không lỗi và chỉ in font dưới thư mục thử.
out="$(swift $Q/unregister_qa_fonts.swift "$T/fx" 2>&1)"; check "sweep không đụng font hệ thống (không in đường dẫn hệ thống)" "$(grep -c '/System/' <<<"$out")" 0
exit $fail
