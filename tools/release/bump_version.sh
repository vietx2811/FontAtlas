#!/usr/bin/env bash
# Đồng bộ số phiên bản ở mọi nơi.
#
#   bash tools/release/bump_version.sh <x.y.z>      # ghi phiên bản mới
#   bash tools/release/bump_version.sh --check      # chỉ kiểm: mọi nơi phải cùng một phiên bản
#
# Các nơi được cập nhật:
#   UX/package.json, UX/package-lock.json (2 chỗ),
#   UX/src-tauri/tauri.conf.json, UX/src-tauri/Cargo.toml, UX/src-tauri/Cargo.lock (gói "app").
# Chỉ sửa đúng dòng version; không chạy cargo/npm. Dừng nếu sau khi ghi còn nơi nào lệch.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

PKG=UX/package.json
LOCK=UX/package-lock.json
TAURI=UX/src-tauri/tauri.conf.json
CARGO=UX/src-tauri/Cargo.toml
CLOCK=UX/src-tauri/Cargo.lock

# In ra phiên bản hiện có ở từng nơi: "<nhãn>\t<phiên bản>"
read_versions() {
  perl -ne 'print "package.json\t$1\n" if !$d && /^\s*"version":\s*"([^"]+)"/ && ($d=1)' "$PKG"
  perl -ne 'if ($.<=12 && /^\s*"version":\s*"([^"]+)"/) { print "package-lock.json#", ++$n, "\t$1\n" }' "$LOCK"
  perl -ne 'print "tauri.conf.json\t$1\n" if !$d && /^\s*"version":\s*"([^"]+)"/ && ($d=1)' "$TAURI"
  perl -ne 'print "Cargo.toml\t$1\n" if !$d && $s && /^version\s*=\s*"([^"]+)"/ && ($d=1); $s=1 if /^\[package\]/' "$CARGO"
  perl -ne 'if (/^name = "app"$/) { $a=1; next } if ($a && /^version = "([^"]+)"/) { print "Cargo.lock(app)\t$1\n"; $a=0 }' "$CLOCK"
}

check() {
  local out uniq
  out="$(read_versions)"
  echo "$out" | awk -F'\t' '{ printf "  %-28s %s\n", $1, $2 }'
  [ "$(echo "$out" | wc -l | tr -d ' ')" -eq 6 ] || { echo "LỖI: kỳ vọng 6 vị trí version, đọc được $(echo "$out" | wc -l | tr -d ' ')"; return 2; }
  uniq="$(echo "$out" | cut -f2 | sort -u)"
  if [ "$(echo "$uniq" | wc -l | tr -d ' ')" -ne 1 ]; then echo "LỆCH phiên bản"; return 1; fi
  echo "OK: mọi nơi đều $uniq"
}

case "${1:-}" in
  --check) check; exit $? ;;
  "") echo "dùng: $0 <x.y.z> | --check" >&2; exit 64 ;;
esac

VER="$1"
[[ "$VER" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] || { echo "phiên bản không hợp lệ: $VER" >&2; exit 64; }

perl -0pi -e 's/("version":\s*")[^"]+(")/${1}'"$VER"'${2}/' "$PKG"
perl -pi -e 'if ($.<=12 && /^\s*"version":/) { s/("version":\s*")[^"]+(")/${1}'"$VER"'${2}/ }' "$LOCK"
perl -0pi -e 's/("version":\s*")[^"]+(")/${1}'"$VER"'${2}/' "$TAURI"
perl -0pi -e 's/(\[package\][^\[]*?\nversion\s*=\s*")[^"]+(")/${1}'"$VER"'${2}/' "$CARGO"
perl -0pi -e 's/(name = "app"\nversion = ")[^"]+(")/${1}'"$VER"'${2}/' "$CLOCK"

echo "đã ghi $VER:"
check
