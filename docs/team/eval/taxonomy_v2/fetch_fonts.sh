#!/usr/bin/env bash
# Tải lại font của bộ eval taxonomy v2 từ google/fonts ở commit đã ghim, rồi kiểm checksum từng file.
# Dùng: fetch_fonts.sh <thư-mục-đích>
#   <thư-mục-đích> phải nằm NGOÀI repo FontAtlas (không commit font). Chạy lại trên thư mục cũ thì chỉ cập nhật.
# Kết quả: <thư-mục-đích>/ofl/<họ>/... — truyền thư mục này cho `classify_eval --gf`.
# fonts.sha256 là danh sách đóng: mọi file .ttf/.otf, METADATA.pb, OFL.txt của các họ trong taxonomy_v2_labels.csv.
set -euo pipefail

COMMIT=47d54ca1ebb14fd864695f1d39dc734f39017c33
HERE="$(cd "$(dirname "$0")" && pwd)"
MANIFEST="$HERE/fonts.sha256"
DEST="${1:?dùng: fetch_fonts.sh <thư-mục-đích-ngoài-repo>}"

mkdir -p "$DEST"
cd "$DEST"
[ -d .git ] || git init -q .
git remote get-url origin >/dev/null 2>&1 || git remote add origin https://github.com/google/fonts.git
awk '{print $2}' "$MANIFEST" | cut -d/ -f1-2 | sort -u | sed 's#^#/#; s#$#/#' \
  | git sparse-checkout set --no-cone --stdin
git fetch -q --depth 1 --filter=blob:none origin "$COMMIT"
git checkout -q --force FETCH_HEAD
shasum -a 256 -c "$MANIFEST" --quiet
echo "OK: $(wc -l < "$MANIFEST" | tr -d ' ') file khớp checksum (google/fonts@$COMMIT)"
