#!/usr/bin/env bash
# Cài hook pre-push của CI máy cho mọi worktree (hook nằm trong thư mục .git chung).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
HOOKS="$(git rev-parse --path-format=absolute --git-common-dir)/hooks"
DST="$HOOKS/pre-push"
if [ -e "$DST" ] && ! grep -q 'local CI' "$DST"; then
  echo "đã có $DST không phải của CI máy; không ghi đè"; exit 1
fi
# Gọi bản trong commit đang checkout ở worktree push, nên hook luôn theo phiên bản của nhánh.
cat > "$DST" <<'SH'
#!/usr/bin/env bash
# local CI (tools/ci/install_hook.sh)
H="$(git rev-parse --show-toplevel)/tools/ci/pre-push"
[ -f "$H" ] || exit 0   # nhánh cũ chưa có CI máy (vd App): không chặn
exec bash "$H" "$@"
SH
chmod +x "$DST"
echo "đã cài $DST"
