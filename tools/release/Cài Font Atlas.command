#!/bin/bash
# Cài Font Atlas từ DMG (bản chưa notarize): chép app vào Applications, gỡ quarantine, ký ad-hoc lại, rồi mở app.
# Double-click trong Finder để chạy. Không tắt Gatekeeper của máy; chỉ áp dụng cho Font Atlas.
# Biến môi trường dùng khi kiểm thử: FA_INSTALL_DIR (thư mục đích), FA_NO_OPEN=1 (không mở app), FA_YES=1 (không hỏi).
set -euo pipefail

APP="Font Atlas.app"
BUNDLE_ID="com.vietx.fontatlas"
HERE="$(cd "$(dirname "$0")" && pwd)"
SRC="$HERE/$APP"

say()  { printf '%s\n' "$*"; }
fail() { say ""; say "✗ $*"; say ""; read -r -p "Nhấn Enter để đóng…" _ || true; exit 1; }

say "Cài Font Atlas"
say "──────────────"
[ -d "$SRC" ] || fail "Không thấy \"$APP\" cạnh script này. Hãy chạy script từ trong cửa sổ DMG."

# Thư mục đích: /Applications nếu ghi được, nếu không thì ~/Applications.
if [ -n "${FA_INSTALL_DIR:-}" ]; then
  DIR="$FA_INSTALL_DIR"
elif [ -w /Applications ]; then
  DIR="/Applications"
else
  DIR="$HOME/Applications"
  say "Không có quyền ghi /Applications, sẽ cài vào $DIR"
fi
mkdir -p "$DIR"
DST="$DIR/$APP"

# App đang chạy thì xin đóng nhẹ nhàng (như bấm Quit), không ép tắt.
if [ -z "${FA_INSTALL_DIR:-}" ] && osascript -e "application id \"$BUNDLE_ID\" is running" 2>/dev/null | grep -q true; then
  say "Font Atlas đang mở, đang đóng app…"
  osascript -e "tell application id \"$BUNDLE_ID\" to quit" >/dev/null 2>&1 || true
  for _ in $(seq 1 20); do
    osascript -e "application id \"$BUNDLE_ID\" is running" 2>/dev/null | grep -q true || break
    sleep 0.5
  done
  osascript -e "application id \"$BUNDLE_ID\" is running" 2>/dev/null | grep -q true \
    && fail "Font Atlas vẫn đang mở. Hãy thoát app (⌘Q) rồi chạy lại script."
fi

if [ -e "$DST" ]; then
  if [ "${FA_YES:-}" != 1 ]; then
    read -r -p "Đã có $DST. Thay bằng bản mới? Thư viện và cài đặt của bạn được giữ nguyên. [Y/n] " ans || ans=y
    case "${ans:-y}" in [nN]*) say "Đã huỷ, không thay đổi gì."; exit 0 ;; esac
  fi
  rm -rf "$DST"
fi

say "• Chép app vào $DIR"
ditto "$SRC" "$DST" || fail "Không chép được app vào $DIR."

say "• Gỡ quarantine"
xattr -dr com.apple.quarantine "$DST" 2>/dev/null || true

say "• Ký ad-hoc lại"
codesign --force --deep --sign - --preserve-metadata=entitlements,requirements,flags,runtime "$DST" >/dev/null 2>&1 \
  || fail "Ký ad-hoc không thành công."
codesign --verify --deep --strict "$DST" >/dev/null 2>&1 || fail "Chữ ký sau khi ký lại không hợp lệ."

say ""
say "✓ Đã cài: $DST"
if [ "${FA_NO_OPEN:-}" != 1 ]; then
  say "Đang mở Font Atlas…"
  open "$DST"
fi
say "Có thể đóng cửa sổ này và tháo (eject) DMG."
