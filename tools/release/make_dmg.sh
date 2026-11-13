#!/usr/bin/env bash
# Đóng DMG phát hành gồm: Font Atlas.app, lối tắt Applications và script "Cài Font Atlas.command".
#
#   bash tools/release/make_dmg.sh <đường dẫn Font Atlas.app> <file .dmg đầu ra>
#
# App lấy từ bản build release (cargo tauri build) trong worktree CI. Script này không mở app.
set -euo pipefail
APP="${1:?thiếu đường dẫn Font Atlas.app}"
OUT="${2:?thiếu file .dmg đầu ra}"
HERE="$(cd "$(dirname "$0")" && pwd)"
CMD="$HERE/Cài Font Atlas.command"

[ -d "$APP" ] || { echo "không thấy app: $APP"; exit 1; }
[ -s "$APP/Contents/Resources/THIRD-PARTY-NOTICES.txt" ] || { echo "app thiếu Contents/Resources/THIRD-PARTY-NOTICES.txt (chạy tools/release/third_party_notices.sh, commit, build lại)"; exit 1; }
VER="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"

# Bản phát hành PHẢI khai scheme fontatlas:// (build bằng `cargo tauri build --config src-tauri/tauri.release.conf.json`, xem `npm run tauri:build`).
# Bản debug/CI chỉ khai fontatlas-dev:// để không giành link của app đã cài.
SCHEMES="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleURLTypes:0:CFBundleURLSchemes' "$APP/Contents/Info.plist" 2>/dev/null | tr -d ' {}()\n' || true)"
case "$SCHEMES" in
  fontatlas) ;;
  *) echo "Info.plist khai scheme '$SCHEMES', cần đúng 'fontatlas': build lại bằng tauri.release.conf.json"; exit 1 ;;
esac

STAGE="$(mktemp -d)"; trap 'rm -rf "$STAGE"' EXIT
ditto "$APP" "$STAGE/Font Atlas.app"
ln -s /Applications "$STAGE/Applications"
cp "$CMD" "$STAGE/"
chmod 755 "$STAGE/Cài Font Atlas.command"

rm -f "$OUT"
hdiutil create -quiet -volname "Font Atlas $VER" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$OUT"
hdiutil verify -quiet "$OUT"
echo "đã tạo $OUT (Font Atlas $VER)"
