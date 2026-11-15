#!/usr/bin/env bash
# Build bản phát hành (.app) đúng cách: dùng lớp phủ tauri.release.conf.json để đổi scheme fontatlas-dev → fontatlas.
#   bash tools/release/build_release.sh            # kiểm giấy phép, build, kiểm kết quả; in đường dẫn .app
# Không có lớp phủ thì bản release không có scheme nào. Script này không mở app (dữ liệu thật của người dùng; xem make_dmg.sh để đóng DMG).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
bash tools/release/third_party_notices.sh --check
bash tools/release/bump_version.sh --check
( cd UX && npm ci && cargo tauri build --bundles app --config src-tauri/tauri.release.conf.json )
APP="$ROOT/UX/src-tauri/target/release/bundle/macos/Font Atlas.app"
[ -d "$APP" ] || { echo "không thấy $APP sau khi build"; exit 1; }
# PlistBuddy in mảng dạng "Array { … }": đọc phần tử đầu và kiểm không có phần tử thứ hai (như make_dmg.sh).
SCHEMES="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleURLTypes:0:CFBundleURLSchemes:0' "$APP/Contents/Info.plist" 2>/dev/null || true)"
/usr/libexec/PlistBuddy -c 'Print :CFBundleURLTypes:0:CFBundleURLSchemes:1' "$APP/Contents/Info.plist" >/dev/null 2>&1 && SCHEMES="$SCHEMES+more"
[ "$SCHEMES" = "fontatlas" ] || { echo "Info.plist khai scheme '$SCHEMES', cần đúng 'fontatlas'"; exit 1; }
[ -s "$APP/Contents/Resources/THIRD-PARTY-NOTICES.txt" ] || { echo "app thiếu Contents/Resources/THIRD-PARTY-NOTICES.txt"; exit 1; }
echo "OK: $APP (scheme fontatlas, có THIRD-PARTY-NOTICES.txt)"
