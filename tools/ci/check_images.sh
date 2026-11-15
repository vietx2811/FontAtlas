#!/usr/bin/env bash
# Chặn ảnh MỚI nằm ngoài danh sách cho phép (repo công khai: không đưa ảnh chụp màn hình / ảnh cá nhân lên).
#   bash tools/ci/check_images.sh <base> <head>     # kiểm các file ảnh được THÊM/đổi tên trong base..head
# Cho phép: UX/src-tauri/icons/**, **/tests/data/**, data/renders/**, UX/public/**. Không có cách bỏ qua bằng biến môi trường:
# cần ngoại lệ thật thì sửa danh sách ở đây và giải thích trong commit.
set -uo pipefail
base="${1:-}"; head="${2:-HEAD}"
ZERO=0000000000000000000000000000000000000000
if [ -z "$base" ] || [ "$base" = "$ZERO" ]; then
  base="$(git merge-base "$head" origin/dev/mvp 2>/dev/null || git merge-base "$head" origin/main 2>/dev/null || true)"
  [ -n "$base" ] || { echo "check_images: không tìm được điểm gốc để so sánh; bỏ qua (cần origin/dev/mvp hoặc origin/main)"; exit 0; }
fi
bad=0
while IFS= read -r f; do
  case "$(printf '%s' "$f" | tr 'A-Z' 'a-z')" in
    *.png|*.jpg|*.jpeg|*.gif|*.webp|*.heic|*.heif|*.bmp|*.tif|*.tiff|*.psd) ;;
    *) continue ;;
  esac
  case "$f" in
    UX/src-tauri/icons/*|*/tests/data/*|data/renders/*|UX/public/*) ;;
    *) echo "ẢNH KHÔNG ĐƯỢC PHÉP: $f"; bad=1 ;;
  esac
done < <(git diff --diff-filter=AR --name-only "$base" "$head" --)
if [ $bad = 1 ]; then
  echo "Repo công khai: ảnh chụp màn hình và ảnh cá nhân không được commit. Chỉ cho phép: UX/src-tauri/icons/**, **/tests/data/**, data/renders/**, UX/public/**."
  exit 1
fi
echo "check_images: không có ảnh mới ngoài danh sách cho phép ($(git rev-parse --short "$base")..$(git rev-parse --short "$head"))"
