#!/usr/bin/env bash
# CI chạy trên máy Mac của dự án, thay cho GitHub Actions.
#
#   bash tools/ci/local_ci.sh [ref] [--no-qa] [--qa-only] [--no-cache] [--print-keys] [--no-wait] [--priority]
#   bash tools/ci/local_ci.sh --status
#
# - ref mặc định là dev/mvp. Kiểm đúng commit đã commit (checkout sạch trong ~/.fontatlas-ci/worktree),
#   không bao giờ kiểm cây đang sửa dở.
# - Các bước: npm ci + vite build, tsc, vitest, cargo test --workspace (sinh lại bindings và bắt lệch),
#   clippy -D warnings, QA trên app native thật (bản debug, dữ liệu tạm).
# - Kết quả: ~/.fontatlas-ci/runs/<sha>/ (log từng bước + summary.json) và git note ở refs/notes/ci.
#   Hook pre-push (tools/ci/install_hook.sh) đọc note này để chặn push dev/mvp và main chưa xanh.
# - Hàng đợi FIFO (mặc định --wait): mỗi lần chạy lấy một vé ~/.fontatlas-ci/queue/<Q|P><thời gian>-<pid>-<ref>, chỉ chạy khi vé đứng đầu
#   và khoá rảnh. Vé của tiến trình đã chết tự bị dọn. --no-wait: gặp khoá bận thì thoát mã 3 (hành vi cũ). --status: in hàng đợi.
#   --qa-only: chỉ chạy lại native QA trên bundle debug ĐÃ build của đúng SHA (từ chối nếu bundle không khớp); cũng đi qua hàng đợi/khoá,
#   không ghi git note. Native QA (run.sh) từ chối chạy ngoài local_ci hoặc khi bundle không khớp SHA.
#   Cache kết quả PASS theo hash NỘI DUNG đầu vào (Rust: UX/src-tauri trừ tests/qa/ + UX/bindings + rustc; frontend: UX trừ src-tauri + node):
#   trùng hash với một lượt đã PASS thì bước đó in "SKIP (cache <sha>)", ghi vào git note. Native QA và vite-build luôn chạy. --no-cache ép
#   chạy đủ (dùng khi phát hành). --print-keys in các khoá hash rồi thoát.
#   --priority (chỉ Lead, cần FONTATLAS_CI_PRIORITY=1) xếp vé đứng trước mọi vé thường.
# - Không đụng thư viện thật: QA dùng FONTATLAS_DATA_DIR tạm, chỉ chạy bản debug.
set -uo pipefail

REF="dev/mvp"; QA=1; WAIT=1; PRIO=0; STATUS=0; QAONLY=0; CACHE=1; PRINTKEYS=0
for a in "$@"; do
  case "$a" in
    --no-qa) QA=0 ;;
    --qa-only) QAONLY=1 ;;
    --no-cache) CACHE=0 ;;
    --print-keys) PRINTKEYS=1 ;;
    --wait) WAIT=1 ;;
    --no-wait) WAIT=0 ;;
    --priority) PRIO=1 ;;
    --status) STATUS=1 ;;
    -h|--help) sed -n '2,24p' "$0"; exit 0 ;;
    *) REF="$a" ;;
  esac
done

CI="${FONTATLAS_CI_HOME:-$HOME/.fontatlas-ci}"
LOCK="$CI/lock"; QUEUE="$CI/queue"
mkdir -p "$CI/runs" "$QUEUE"

alive() { [ -n "$1" ] && kill -0 "$1" 2>/dev/null; }
ticket_pid() { local b="${1##*/}"; b="${b#*-}"; echo "${b%%-*}"; }   # <Q|P><thời gian>-<pid>-<ref>
# Dọn vé của tiến trình đã chết và khoá mồ côi (pid trong khoá không còn sống).
reap() {
  local t
  for t in "$QUEUE"/*; do [ -e "$t" ] || continue; alive "$(ticket_pid "$t")" || rm -f "$t"; done
  if [ -d "$LOCK" ]; then
    local lp; lp="$(cat "$LOCK/pid" 2>/dev/null)"
    [ -n "$lp" ] && ! alive "$lp" && rm -rf "$LOCK"
  fi
}
queue_list() { ls -1 "$QUEUE" 2>/dev/null | LC_ALL=C sort; }

if [ $STATUS = 1 ]; then
  reap
  python3 - "$CI" "$QUEUE" "$LOCK" <<'PY'
import glob, json, os, statistics, sys, time
ci, queue, lock = sys.argv[1:]
fmt = lambda s: "%dh%02dm" % (s // 3600, s % 3600 // 60) if s >= 3600 else "%dm%02ds" % (s // 60, s % 60)
# Ước lượng: trung vị thời gian của các lượt đủ (có QA) gần đây nhất; chỉ là gợi ý, máy tải nặng có thể gấp đôi.
runs = sorted(glob.glob(ci + "/runs/*/summary.json"), key=os.path.getmtime, reverse=True)
secs = []
for f in runs[:12]:
    try:
        d = json.load(open(f))
        if d.get("qa") and d.get("seconds"): secs.append(d["seconds"])
    except Exception: pass
est = int(statistics.median(secs)) if secs else None
now = time.time(); wait = 0
if os.path.isdir(lock):
    pid = open(lock + "/pid").read().strip() if os.path.exists(lock + "/pid") else "?"
    info = open(lock + "/info").read().split() if os.path.exists(lock + "/info") else []
    if len(info) >= 3:
        el = int(now - int(info[0])); left = max(est - el, 0) if est else None
        print("đang chạy: pid %s · %s (%s) · đã %s%s" % (pid, info[1], info[2][:9], fmt(el), " · còn ~" + fmt(left) if left is not None else ""))
        wait = left or 0
    else: print("đang chạy: pid %s" % pid)
else: print("khoá rảnh")
names = sorted(os.listdir(queue))
for i, t in enumerate(names, 1):
    ns = int(t[1:21]); pid = t.split("-")[1]; ref = t.split("-", 2)[2]
    print("%2d. %s%s · %s · pid %s · chờ %s · ước lượng bắt đầu sau ~%s" % (i, ref, " [PRIORITY]" if t[0] == "P" else "", time.strftime("%H:%M", time.localtime(ns / 1e9)), pid, fmt(int(now - ns / 1e9)), fmt(int(wait)) if est or wait else "?"))
    if est: wait += est
if not names: print("hàng đợi trống")
if est: print("(ước lượng một lượt đủ ~%s, trung vị %d lượt gần nhất)" % (fmt(est), len(secs)))
PY
  exit 0
fi

REPO="$(git rev-parse --path-format=absolute --git-common-dir)" || exit 2
WT="$CI/worktree"

SHA="$(git --git-dir="$REPO" rev-parse --verify "$REF^{commit}")" || { echo "không tìm thấy ref: $REF"; exit 2; }
TREE="$(git --git-dir="$REPO" rev-parse "$SHA^{tree}")"
SHORT="${SHA:0:9}"

# Khoá cache theo nội dung: hash (đường dẫn + blob) của đúng phần đầu vào, không phụ thuộc commit.
tree_hash() { local ex="$1"; shift; git --git-dir="$REPO" ls-tree -r "$SHA" -- "$@" | { if [ -n "$ex" ]; then grep -Ev "$ex"; else cat; fi; } | shasum | cut -c1-16; }
# Đầu vào NGOÀI UX/src-tauri mà test Rust đọc: docs/team/eval (include_str!/đọc lúc chạy trong fontatlas-eval), UX/bindings (so với bindings sinh ra),
# UX/python-sidecar và UX/binaries (tests/config.rs khẳng định chúng không tồn tại). test_cache.sh có chốt cảnh báo khi xuất hiện đường dẫn ngoài mới.
# Phiên bản công cụ lấy trong UX/src-tauri (rust-toolchain.toml ghim bản rustc ở đó).
compute_keys() {
  local tc fe; tc="$(cd "${1:-$PWD}/UX/src-tauri" 2>/dev/null && { rustc -V; cargo -V; cargo clippy -V; } 2>&1 | shasum | cut -c1-8)"
  fe="$( { node -v; npm -v; } 2>&1 | shasum | cut -c1-8)"
  local tauri; tauri="$(cargo tauri -V 2>&1 | shasum | cut -c1-8)"
  RUST_KEY="$(tree_hash $'\tUX/src-tauri/tests/qa/' UX/src-tauri UX/bindings docs/team/eval UX/python-sidecar UX/binaries)-$tc"
  # Frontend còn đọc 3 file trong src-tauri (fixture face_row.json, Cargo.toml, tauri.conf.json): đưa vào khoá frontend.
  FRONT_KEY="$(tree_hash $'\tUX/src-tauri/' UX)$(tree_hash '' UX/src-tauri/Cargo.toml UX/src-tauri/tauri.conf.json UX/src-tauri/crates/fontatlas-store/tests/fixtures/face_row.json)-$fe"
  BUNDLE_KEY="$(printf %s "$RUST_KEY$FRONT_KEY$tauri" | shasum | cut -c1-16)"
}
compute_keys "$(git rev-parse --show-toplevel 2>/dev/null)"
if [ $PRINTKEYS = 1 ]; then echo "rust=$RUST_KEY"; echo "front=$FRONT_KEY"; echo "bundle=$BUNDLE_KEY"; exit 0; fi

# Một lần chạy một lúc: target/ và node_modules dùng chung. Hàng đợi FIFO giữ thứ tự giữa các lần chạy.
if [ $PRIO = 1 ] && [ "${FONTATLAS_CI_PRIORITY:-}" != 1 ]; then echo "--priority chỉ dành cho Lead (đặt FONTATLAS_CI_PRIORITY=1)"; exit 2; fi
TICKET=""
cleanup() { [ -n "$TICKET" ] && rm -f "$TICKET"; if [ "$(cat "$LOCK/pid" 2>/dev/null)" = "$$" ]; then rm -rf "$LOCK"; fi; }
trap cleanup EXIT
trap 'exit 130' INT TERM
if [ $WAIT = 1 ]; then
  KIND=Q; [ $PRIO = 1 ] && KIND=P
  NOW="$(python3 -c 'import time; print("%020d" % time.time_ns())')"
  TICKET="$QUEUE/$KIND$NOW-$$-$(printf %s "$REF" | tr -c 'A-Za-z0-9._' '_')"
  : > "$TICKET"
  shown=""
  while :; do
    reap
    head="$(queue_list | head -n1)"
    if [ "$QUEUE/$head" = "$TICKET" ] && mkdir "$LOCK" 2>/dev/null; then break; fi
    pos="$(queue_list | grep -n -F -x "${TICKET##*/}" | cut -d: -f1)"
    [ "$pos" != "$shown" ] && { echo "chờ CI: vị trí ${pos:-?} trong hàng đợi (xem: bash tools/ci/local_ci.sh --status)"; shown="$pos"; }
    sleep 3
  done
  rm -f "$TICKET"; TICKET=""
else
  reap
  if ! mkdir "$LOCK" 2>/dev/null; then
    echo "CI đang chạy (khoá $LOCK, pid $(cat "$LOCK/pid" 2>/dev/null || echo ?)). Dùng --wait để xếp hàng."
    exit 3
  fi
fi
echo $$ > "$LOCK/pid"
echo "$(date +%s) $REF $SHA" > "$LOCK/info"
[ -n "${FONTATLAS_CI_TEST_HOLD:-}" ] && { echo "đã có khoá"; sleep "$FONTATLAS_CI_TEST_HOLD"; exit 0; }   # chỉ cho tools/ci/test_queue.sh

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
touch "$CI/.metadata_never_index" ${CI_TARGET:+"$CI_TARGET/.metadata_never_index"}   # Spotlight không index target/ và worktree (không đổi cấu hình hệ thống)
JOBS=$(( $(sysctl -n hw.ncpu 2>/dev/null || echo 8) * 2 / 3 )); [ $JOBS -ge 1 ] || JOBS=1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$JOBS}"   # chừa CPU cho máy/worker khác; ghi đè bằng CARGO_BUILD_JOBS
git clean -ffdxq -e UX/src-tauri/target -e UX/node_modules

compute_keys "$WT"   # công cụ đúng như trong checkout của SHA (rust-toolchain.toml)
export FONTATLAS_CI_RUN=1   # run.sh từ chối chạy khi không có biến này (tức là ngoài khoá CI)
STAMP="UX/src-tauri/target/debug/bundle/.built-sha"   # SHA của bundle debug đã build; run.sh so với HEAD của checkout
loadavg() { uptime | sed 's/.*load averages*: //'; }

if [ $QAONLY = 1 ]; then
  [ "$(cat "$WT/$STAMP" 2>/dev/null)" = "$SHA" ] || { echo "bundle debug không phải của $SHORT (stamp: $(cat "$WT/$STAMP" 2>/dev/null || echo không có)) → chạy đủ: bash tools/ci/local_ci.sh $REF"; exit 2; }
  OUT="$CI/runs/$SHA"; mkdir -p "$OUT"; LOG="$OUT/native-qa.rerun-$(date +%Y%m%d-%H%M%S).log"
  echo "native QA lại · $SHORT ($REF) · tải $(loadavg) · log: $LOG"
  caffeinate -dimsu bash UX/src-tauri/tests/qa/run.sh > "$LOG" 2>&1; rc=$?
  tail -n 40 "$LOG"; echo "──"; echo "tải khi xong: $(loadavg)"
  [ $rc = 0 ] && echo "PASS native-qa $SHORT (không ghi git note)" || echo "FAIL native-qa $SHORT"
  exit $rc
fi

OUT="$CI/runs/$SHA"; rm -rf "$OUT"; mkdir -p "$OUT"
LOAD_START="$(loadavg)"
START=$(date +%s)
STEPS=(); FAILED=(); CACHED=()
mkdir -p "$CI/cache"
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
# cstep <tên> <khoá> <thư mục> <lệnh...>: như step, nhưng nếu một lượt trước đã PASS với cùng khoá nội dung thì bỏ qua.
cstep() {
  local name="$1" key="$2"; shift 2
  local f="$CI/cache/$name-$key" from
  if [ $CACHE = 1 ] && [ -s "$f" ]; then
    from="$(cut -c1-9 "$f")"
    printf 'SKIP        %s  (cache %s)\n' "$name" "$from"; STEPS+=("$name:ok:0"); CACHED+=("$name=$from"); return 0
  fi
  step "$name" "$@" && git --git-dir="$REPO" rev-parse "$SHA" > "$f"
}
skip() { printf 'SKIP        %s  (%s)\n' "$1" "$2"; STEPS+=("$1:skip:0"); }

npm_install() {
  local h; h="$(shasum package-lock.json | cut -c1-40)"
  if [ -f node_modules/.ci-lock-hash ] && [ "$(cat node_modules/.ci-lock-hash)" = "$h" ]; then echo "node_modules khớp package-lock, bỏ qua npm ci"; return 0; fi
  npm ci && echo "$h" > node_modules/.ci-lock-hash
}

step npm-ci UX npm_install && FRONT=1 || FRONT=0
if [ $FRONT = 1 ]; then
  step vite-build UX npx vite build || FRONT=0   # luôn chạy: tạo UX/dist cho cargo và bản build debug (2 s)
  cstep tsc "$FRONT_KEY" UX npx tsc --noEmit
  cstep vitest "$FRONT_KEY" UX npm test
else
  skip vite-build "npm ci đỏ"; skip tsc "npm ci đỏ"; skip vitest "npm ci đỏ"
fi

# Doc-tests: cargo test --workspace chạy rustdoc cho TỪNG crate dù có 0 ví dụ (~9 phút/crate khi máy tải, ~2 giờ cho 13 crate).
# Chỉ chạy lib/bin/tests; riêng crate nào CÓ khối ``` trong doc comment thì vẫn chạy `cargo test --doc` cho crate đó.
doc_crates() {
  local root="${1:-$WT/UX/src-tauri}" f d
  # Có thể thừa (chạy --doc cho crate không cần) nhưng không sót: ``` trong ///, //!, /** */, /*! */, #[doc = "..."], #![doc = include_str!(...)].
  grep -rlE '^[[:space:]]*(///|//!)[[:space:]]*```|#!?\[doc[[:space:]]*=|/\*[*!]' --include='*.rs' "$root" 2>/dev/null | grep -v '/target/' | while IFS= read -r f; do
    grep -qE '```|include_str!' "$f" || continue
    d="$(dirname "$f")"; while [ "$d" != "/" ] && [ ! -f "$d/Cargo.toml" ]; do d="$(dirname "$d")"; done
    [ -f "$d/src/lib.rs" ] || grep -q '^\[lib\]' "$d/Cargo.toml" || continue   # crate chỉ có bin: không có doc-test
    awk '/^\[package\]/{p=1;next} /^\[/{p=0} p && /^name[[:space:]]*=/{gsub(/^name[[:space:]]*=[[:space:]]*"|"[[:space:]]*$/,""); print; exit}' "$d/Cargo.toml"
  done | sort -u
}
rust_tests() {
  env UPDATE_BINDINGS=1 cargo test --workspace --locked --lib --bins --tests 2>&1 | python3 "$WT/tools/ci/ts_filter.py" || return 1
  # Kiểm lệch bindings NGAY trong đơn vị cache của cargo-test: bindings chỉ được sinh lại ở bước này, nên "PASS" của cargo-test chỉ được cache khi bindings cũng khớp.
  git -C "$WT" diff --exit-code -- UX/bindings || { echo "bindings-drift: UX/bindings lệch so với bản sinh ra"; return 1; }
  local c; for c in $(doc_crates); do echo "doc-tests (có ví dụ trong doc comment): $c"; cargo test --doc --locked -p "$c" || return 1; done
}

if [ $FRONT = 1 ]; then
  cstep cargo-test "$RUST_KEY" UX/src-tauri rust_tests
  # drift chỉ được SKIP khi cargo-test cũng được lấy từ cache (cùng một đơn vị, xem rust_tests); nếu cargo-test chạy thật thì drift chạy thật.
  if [ "${#CACHED[@]}" -gt 0 ] && printf '%s\n' "${CACHED[@]}" | grep -q '^cargo-test='; then
    printf 'SKIP        %s  (cache, cùng đơn vị với cargo-test)\n' bindings-drift; STEPS+=("bindings-drift:ok:0"); CACHED+=("bindings-drift=unit")
  else
    step bindings-drift . git diff --exit-code -- UX/bindings
  fi
  cstep clippy "$RUST_KEY" UX/src-tauri cargo clippy --workspace --all-targets --locked -- -D warnings
else
  skip cargo-test "thiếu UX/dist"; skip bindings-drift "thiếu UX/dist"; skip clippy "thiếu UX/dist"
fi

if [ $QA = 0 ]; then
  skip native-qa "--no-qa"
elif [ $FRONT = 0 ]; then
  skip native-qa "frontend đỏ"
else
  BKEY="${STAMP%.built-sha}.built-key"   # khoá nội dung của bundle (Rust + frontend, không tính tests/qa)
  APPBIN="$WT/UX/src-tauri/target/debug/bundle/macos/Font Atlas.app/Contents/MacOS/app"
  build_debug() { cargo tauri build --debug --bundles app && echo "$BUNDLE_KEY" > "$WT/$BKEY" && echo "$SHA" > "$WT/$STAMP"; }
  BUILD_OK=0
  if [ $CACHE = 1 ] && [ -x "$APPBIN" ] && [ "$(cat "$WT/$BKEY" 2>/dev/null)" = "$BUNDLE_KEY" ]; then
    # Cùng nội dung đầu vào ⇒ bundle y hệt: chỉ cập nhật stamp sang SHA này để run.sh chấp nhận.
    echo "$SHA" > "$WT/$STAMP"; printf 'SKIP        tauri-debug-build  (bundle cùng nội dung, key %s)\n' "$BUNDLE_KEY"
    STEPS+=("tauri-debug-build:ok:0"); CACHED+=("tauri-debug-build=bundle"); BUILD_OK=1
  else
    rm -f "$WT/$STAMP" "$WT/$BKEY"   # build hỏng/bị ngắt thì không để lại stamp của bundle cũ
    step tauri-debug-build UX build_debug && BUILD_OK=1
  fi
  if [ $BUILD_OK = 1 ]; then
    # caffeinate: màn hình ngủ thì WebKit treo trang và QA chạy hết giờ.
    echo "tải máy: $(loadavg)"
    step native-qa . caffeinate -dimsu bash UX/src-tauri/tests/qa/run.sh
  else
    skip native-qa "build debug đỏ"
  fi
fi

TOTAL=$(( $(date +%s) - START ))
if [ ${#FAILED[@]} -eq 0 ]; then RESULT=pass; else RESULT=fail; fi
STEP_STR="$(IFS=' '; echo "${STEPS[*]}")"
CACHE_STR="${CACHED[*]:-}"
DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

LOAD_END="$(loadavg)"
export LOAD_START LOAD_END CACHE_STR
python3 - "$OUT/summary.json" "$SHA" "$TREE" "$REF" "$RESULT" "$TOTAL" "$DATE" "$QA" $STEP_STR <<'PY'
import json, os, sys
out, sha, tree, ref, result, total, date, qa, *steps = sys.argv[1:]
json.dump({"sha": sha, "tree": tree, "ref": ref, "result": result, "seconds": int(total), "date": date,
           "qa": qa == "1", "load_start": os.environ.get("LOAD_START"), "load_end": os.environ.get("LOAD_END"), "cached": os.environ.get("CACHE_STR", "").split(), "steps": [dict(zip(("name", "status", "seconds"), s.split(":"))) for s in steps]},
          open(out, "w"), indent=2)
PY

# Dòng đầu note là thứ hook pre-push đọc: "<pass|fail> tree=<tree> qa=<0|1>".
git --git-dir="$REPO" notes --ref=ci add -f -m "$RESULT tree=$TREE qa=$QA
$DATE $(hostname -s) ${TOTAL}s
$STEP_STR
cache: ${CACHE_STR:-none}" "$SHA" >/dev/null

echo "──"
if [ $RESULT = pass ]; then echo "PASS $SHORT trong ${TOTAL}s"; else echo "FAIL $SHORT trong ${TOTAL}s: ${FAILED[*]}"; fi
[ $RESULT = pass ]
