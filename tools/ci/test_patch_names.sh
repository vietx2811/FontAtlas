#!/usr/bin/env bash
# Thử patch_font_names.py (đổi tên bản sao fixture QA): xác định, VF/static cùng PostScript, font vẫn đọc được, repo không đổi. Không mở app.
#   bash tools/ci/test_patch_names.sh
set -uo pipefail
cd "$(dirname "$0")/../.."
Q=UX/src-tauri/tests/qa; F=UX/src-tauri/crates/fontatlas-core/tests/fixtures; fail=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: muốn '$3', được '$2'"; fail=1; fi; }
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
mkdir -p "$T/a" "$T/b"; for n in roboto roboto_vf oswald_vf open_sans; do cp $F/$n.ttf $T/a/; cp $F/$n.ttf $T/b/; done
map="$(python3 $Q/patch_font_names.py $T/a)"; python3 $Q/patch_font_names.py $T/b >/dev/null
check "xác định: hai lần đổi cho cùng byte" "$(cd $T/a && shasum *.ttf | awk '{print $1}' | tr '\n' ' ')" "$(cd $T/b && shasum *.ttf | awk '{print $1}' | tr '\n' ' ')"
check "roboto.ttf: Roboto-Regular → RobotoFAQA-Regular" "$(grep '^roboto.ttf' <<<"$map" | cut -f3)" RobotoFAQA-Regular
check "VF và static cùng họ vẫn cùng PostScript" "$(grep '^roboto_vf.ttf' <<<"$map" | cut -f3)" RobotoFAQA-Regular
check "ban đầu hai font cùng PostScript gốc" "$(grep -c -P '\tRoboto-Regular\t' <<<"$map" 2>/dev/null || grep -c 'Roboto-Regular' <<<"$map")" 2
cat > $T/v.swift <<'S'
import CoreText
import Foundation
for f in CommandLine.arguments.dropFirst() {
  guard let ds = CTFontManagerCreateFontDescriptorsFromURL(URL(fileURLWithPath: f) as CFURL) as? [CTFontDescriptor], let d = ds.first else { print("LỖI", f); continue }
  let font = CTFontCreateWithFontDescriptor(d, 24, nil)
  print((f as NSString).lastPathComponent, CTFontCopyFamilyName(font), CTFontGetGlyphCount(font) > 100 ? "đọc được" : "ít glyph")
}
S
out="$(swift $T/v.swift $T/a/roboto.ttf $T/a/oswald_vf.ttf $T/a/open_sans.ttf 2>&1)"
check "CoreText đọc được font đã đổi tên (family có « FAQA»)" "$(grep -c 'FAQA đọc được' <<<"$out")" 3
swift $Q/qa_preflight_fonts.swift $T/a >/dev/null 2>&1; check "preflight sạch trên tên đã đổi (không va chạm font thật của máy)" "$?" 0
check "fixture trong repo không bị sửa" "$(git status --porcelain -- $F | wc -l | tr -d ' ')" 0
exit $fail
