#!/usr/bin/env bash
# CI chạy trên máy Mac của dự án, thay cho GitHub Actions.
#
#   bash tools/ci/local_ci.sh [ref] [--no-qa]
#
# - ref mặc định là dev/mvp. Kiểm đúng commit đã commit (checkout sạch trong ~/.fontatlas-ci/worktree),
#   không bao giờ kiểm cây đang sửa dở.
# - Các bước: npm ci + vite build, tsc, vitest, cargo test --workspace (sinh lại bindings và bắt lệch),
#   clippy -D warnings, QA trên app native thật (bản debug, dữ liệu tạm).
# - Kết quả: ~/.fontatlas-ci/runs/<sha>/ (log từng bước + summary.json) và git note ở refs/notes/ci.
#   Hook pre-push (tools/ci/install_hook.sh) đọc note này để chặn push dev/mvp và main chưa xanh.
# - Không đụng thư viện thật: QA dùng FONTATLAS_DATA_DIR tạm, chỉ chạy bản debug.
set -uo pipefail

REF="dev/mvp"; QA=1
for a in "$@"; do
  case "$a" in
    --no-qa) QA=0 ;;
    -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
    *) REF="$a" ;;
  esac
done

REPO="$(git rev-parse --path-format=absolute --git-common-dir)" || exit 2
CI="${FONTATLAS_CI_HOME:-$HOME/.fontatlas-ci}"
WT="$CI/worktree"
mkdir -p "$CI/runs"

SHA="$(git --git-dir="$REPO" rev-parse --verify "$REF^{commit}")" || { echo "không tìm thấy ref: $REF"; exit 2; }
TREE="$(git --git-dir="$REPO" rev-parse "$SHA^{tree}")"
SHORT="${SHA:0:9}"

# Một lần chạy một lúc: target/ và node_modules dùng chung.
LOCK="$CI/lock"
if ! mkdir "$LOCK" 2>/dev/null; then
  echo "CI đang chạy (khoá $LOCK, pid $(cat "$LOCK/pid" 2>/dev/null || echo ?)). Nếu chắc không còn lần chạy nào: rmdir/rm -r $LOCK"
  exit 3
fi
echo $$ > "$LOCK/pid"
trap 'rm -rf "$LOCK"' EXIT

# Worktree riêng, giữ lại giữa các lần chạy để target/ còn nóng.
if [ ! -e "$WT/.git" ]; then
  git --git-dir="$REPO" worktree add --detach "$WT" "$SHA" >/dev/null || exit 2
fi
cd "$WT" || exit 2
git checkout -q --detach -f "$SHA" || exit 2

# target/ của Rust rất nặng (~30 GB). Đặt nó ở ổ dữ liệu, không ở ổ hệ thống:
# UX/src-tauri/target là symlink tới $FONTATLAS_CI_TARGET (mặc định ổ VXData nếu có).
CI_TARGET="${FONTATLAS_CI_TARGET:-}"
[ -z "$CI_TARGET" ] && [ -d /Volumes/VXData/Users/VietX ] && CI_TARGET="/Volumes/VXData/Users/VietX/.fontatlas-ci/target"
if [ -n "$CI_TARGET" ] && [ ! -L UX/src-tauri/target ]; then
  mkdir -p "$CI_TARGET" || exit 2
  if [ -e UX/src-tauri/target ]; then echo "UX/src-tauri/target vẫn là thư mục thật; xoá nó để chuyển sang $CI_TARGET"; exit 2; fi
  ln -s "$CI_TARGET" UX/src-tauri/target || exit 2
fi
git clean -ffdxq -e UX/src-tauri/target -e UX/node_modules

OUT="$CI/runs/$SHA"; rm -rf "$OUT"; mkdir -p "$OUT"
START=$(date +%s)
STEPS=(); FAILED=()
echo "local CI · $SHORT ($REF) · log: $OUT"

# step <tên> <thư mục> <lệnh...>: ghi log, in một dòng kết quả, không dừng cả lượt khi một bước đỏ.
step() {
  local name="$1" dir="$2"; shift 2
  local t0=$(date +%s) rc
  ( cd "$WT/$dir" && "$@" ) > "$OUT/$name.log" 2>&1; rc=$?
  local dt=$(( $(date +%s) - t0 ))
  if [ $rc -eq 0 ]; then printf 'OK   %5ss  %s\n' "$dt" "$name"; STEPS+=("$name:ok:$dt")
  else printf 'FAIL %5ss  %s  (tail %s)\n' "$dt" "$name" "$OUT/$name.log"; tail -n 15 "$OUT/$name.log" | sed 's/^/     │ /'; STEPS+=("$name:fail:$dt"); FAILED+=("$name"); fi
  return $rc
}
skip() { printf 'SKIP        %s  (%s)\n' "$1" "$2"; STEPS+=("$1:skip:0"); }

npm_install() {
  local h; h="$(shasum package-lock.json | cut -c1-40)"
  if [ -f node_modules/.ci-lock-hash ] && [ "$(cat node_modules/.ci-lock-hash)" = "$h" ]; then echo "node_modules khớp package-lock, bỏ qua npm ci"; return 0; fi
  npm ci && echo "$h" > node_modules/.ci-lock-hash
}

step npm-ci UX npm_install && FRONT=1 || FRONT=0
if [ $FRONT = 1 ]; then
  step vite-build UX npx vite build || FRONT=0
  step tsc UX npx tsc --noEmit
  step vitest UX npm test
else
  skip vite-build "npm ci đỏ"; skip tsc "npm ci đỏ"; skip vitest "npm ci đỏ"
fi

if [ $FRONT = 1 ]; then
  step cargo-test UX/src-tauri env UPDATE_BINDINGS=1 cargo test --workspace --locked
  step bindings-drift . git diff --exit-code -- UX/bindings
  step clippy UX/src-tauri cargo clippy --workspace --all-targets --locked -- -D warnings
else
  skip cargo-test "thiếu UX/dist"; skip bindings-drift "thiếu UX/dist"; skip clippy "thiếu UX/dist"
fi

if [ $QA = 0 ]; then
  skip native-qa "--no-qa"
elif [ $FRONT = 0 ]; then
  skip native-qa "frontend đỏ"
else
  if step tauri-debug-build UX cargo tauri build --debug --bundles app; then
    # caffeinate: màn hình ngủ thì WebKit treo trang và QA chạy hết giờ.
    step native-qa . caffeinate -dimsu bash UX/src-tauri/tests/qa/run.sh
  else
    skip native-qa "build debug đỏ"
  fi
fi

TOTAL=$(( $(date +%s) - START ))
if [ ${#FAILED[@]} -eq 0 ]; then RESULT=pass; else RESULT=fail; fi
STEP_STR="$(IFS=' '; echo "${STEPS[*]}")"
DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

python3 - "$OUT/summary.json" "$SHA" "$TREE" "$REF" "$RESULT" "$TOTAL" "$DATE" "$QA" $STEP_STR <<'PY'
import json, sys
out, sha, tree, ref, result, total, date, qa, *steps = sys.argv[1:]
json.dump({"sha": sha, "tree": tree, "ref": ref, "result": result, "seconds": int(total), "date": date,
           "qa": qa == "1", "steps": [dict(zip(("name", "status", "seconds"), s.split(":"))) for s in steps]},
          open(out, "w"), indent=2)
PY

# Dòng đầu note là thứ hook pre-push đọc: "<pass|fail> tree=<tree> qa=<0|1>".
git --git-dir="$REPO" notes --ref=ci add -f -m "$RESULT tree=$TREE qa=$QA
$DATE $(hostname -s) ${TOTAL}s
$STEP_STR" "$SHA" >/dev/null

echo "──"
if [ $RESULT = pass ]; then echo "PASS $SHORT trong ${TOTAL}s"; else echo "FAIL $SHORT trong ${TOTAL}s: ${FAILED[*]}"; fi
[ $RESULT = pass ]
