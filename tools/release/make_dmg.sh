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
VER="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"

STAGE="$(mktemp -d)"; trap 'rm -rf "$STAGE"' EXIT
ditto "$APP" "$STAGE/Font Atlas.app"
ln -s /Applications "$STAGE/Applications"
cp "$CMD" "$STAGE/"
chmod 755 "$STAGE/Cài Font Atlas.command"

rm -f "$OUT"
hdiutil create -quiet -volname "Font Atlas $VER" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$OUT"
hdiutil verify -quiet "$OUT"
echo "đã tạo $OUT (Font Atlas $VER)"
