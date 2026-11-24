#!/usr/bin/env bash
# Tạo/kiểm THIRD-PARTY-NOTICES cho bản phát hành. Chạy khi phát hành (không nằm trong local_ci thường).
#   bash tools/release/third_party_notices.sh           # kiểm giấy phép + ghi UX/src-tauri/resources/THIRD-PARTY-NOTICES.txt
#   bash tools/release/third_party_notices.sh --check   # kiểm giấy phép + file commit còn khớp; thoát 1 nếu không
# Cần: cargo (đã fetch/build), UX/node_modules (npm ci). Không tải gì. File được đóng vào .app/Contents/Resources.
set -euo pipefail
exec python3 "$(cd "$(dirname "$0")" && pwd)/third_party_notices.py" "$@"
