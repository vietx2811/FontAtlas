#!/usr/bin/env bash
# Dùng: fetch_google_fonts.sh <DEST> [SHA]. Sparse + shallow clone ofl/ và apache/ của google/fonts ở SHA ghim.
set -euo pipefail
DEST="$1"; SHA="${2:-$(cat "$(dirname "$0")/GOOGLE_FONTS_SHA")}"
mkdir -p "$DEST" && cd "$DEST"
[ -d .git ] || git init -q
git remote get-url origin >/dev/null 2>&1 || git remote add origin https://github.com/google/fonts.git
git sparse-checkout set ofl apache
git fetch -q --depth 1 --filter=blob:none origin "$SHA"
git checkout -q FETCH_HEAD
