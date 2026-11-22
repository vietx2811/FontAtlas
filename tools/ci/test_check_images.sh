#!/usr/bin/env bash
# Thử check_images.sh trên repo tạm. bash tools/ci/test_check_images.sh
set -uo pipefail
S="$(cd "$(dirname "$0")" && pwd)/check_images.sh"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT; cd "$T" && git init -q && git config user.email t@t && git config user.name t
fail=0; check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: muốn '$3', được '$2'"; fail=1; fi; }
echo a > a.txt; git add -A; git commit -qm base; base=$(git rev-parse HEAD)
mkdir -p UX/src-tauri/icons x/tests/data data/renders docs UX/public; for p in UX/src-tauri/icons/i.png x/tests/data/t.PNG data/renders/r.jpg UX/public/p.webp; do echo x > $p; done
git add -A; git commit -qm ok; bash "$S" $base HEAD >/dev/null; check "ảnh trong danh sách cho phép" "$?" 0
echo x > docs/shot.png; git add -A; git commit -qm bad; h=$(git rev-parse HEAD); bash "$S" $base $h >/dev/null; check "docs/shot.png bị chặn" "$?" 1
git mv docs/shot.png docs/shot2.png; git commit -qm rn; bash "$S" $h HEAD >/dev/null; check "đổi tên ảnh ngoài danh sách bị chặn" "$?" 1
echo x > Shot.JPEG; git add -A; git commit -qm up; bash "$S" HEAD~1 HEAD >/dev/null; check "JPEG viết hoa bị chặn" "$?" 1
git rm -q docs/shot2.png Shot.JPEG; git commit -qm rm; bash "$S" HEAD~1 HEAD >/dev/null; check "xoá ảnh không bị chặn" "$?" 0
echo x > doc.md; git add -A; git commit -qm md; bash "$S" HEAD~1 HEAD >/dev/null; check "file không phải ảnh bỏ qua" "$?" 0
mkdir -p docs/readme/sub; echo x > docs/readme/lib.png; git add -A; git commit -qm rd; bash "$S" HEAD~1 HEAD >/dev/null; check "docs/readme/*.png được phép" "$?" 0
echo x > docs/readme/sub/x.png; git add -A; git commit -qm rs; bash "$S" HEAD~1 HEAD >/dev/null; check "docs/readme/sub/*.png bị chặn" "$?" 1
echo x > docs/readme/x.jpg; git add -A; git commit -qm rj; bash "$S" HEAD~1 HEAD >/dev/null; check "docs/readme/*.jpg bị chặn" "$?" 1
exit $fail
