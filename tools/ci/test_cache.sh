#!/usr/bin/env bash
# Thử khoá cache của local_ci.sh (--print-keys) trên các commit tổng hợp: đổi file QA / frontend / Rust phải ra đúng tổ hợp khoá.
#   bash tools/ci/test_cache.sh
set -uo pipefail
cd "$(dirname "$0")/../.."
fail=0; check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: muốn '$3', được '$2'"; fail=1; fi; }
IDX="$(mktemp)"; trap 'rm -f "$IDX"' EXIT
# mk <đường dẫn> : commit tổng hợp (cha HEAD) thêm một dòng vào file đó, không đụng cây làm việc.
mk() { rm -f "$IDX"; GIT_INDEX_FILE="$IDX" git read-tree HEAD
  local blob; blob="$( { git show "HEAD:$1"; echo "// ci-cache-test"; } | git hash-object -w --stdin)"
  GIT_INDEX_FILE="$IDX" git update-index --cacheinfo "100644,$blob,$1"
  git commit-tree "$(GIT_INDEX_FILE="$IDX" git write-tree)" -p HEAD -m t; }
key() { bash tools/ci/local_ci.sh "$1" --print-keys | grep "^$2=" | cut -d= -f2; }
base="$(git rev-parse HEAD)"
q="$(mk UX/src-tauri/tests/qa/main_flows.js)"
check "đổi main_flows.js: khoá Rust giữ nguyên" "$(key $q rust)" "$(key $base rust)"
check "đổi main_flows.js: khoá frontend giữ nguyên" "$(key $q front)" "$(key $base front)"
check "đổi main_flows.js: khoá bundle giữ nguyên" "$(key $q bundle)" "$(key $base bundle)"
f="$(git ls-files 'UX/components/*.tsx' | head -1)"
c="$(mk "$f")"
check "đổi .tsx: khoá Rust giữ nguyên" "$(key $c rust)" "$(key $base rust)"
check "đổi .tsx: khoá frontend đổi" "$([ "$(key $c front)" != "$(key $base front)" ] && echo diff)" diff
check "đổi .tsx: khoá bundle đổi" "$([ "$(key $c bundle)" != "$(key $base bundle)" ] && echo diff)" diff
r="$(mk UX/src-tauri/Cargo.lock)"
check "đổi Cargo.lock: khoá Rust đổi" "$([ "$(key $r rust)" != "$(key $base rust)" ] && echo diff)" diff
check "đổi Cargo.lock: khoá frontend giữ nguyên" "$(key $r front)" "$(key $base front)"

# doc_crates: crate nào có khối ``` trong doc comment thì được chạy cargo test --doc.
eval "$(sed -n '/^doc_crates() {/,/^}/p' tools/ci/local_ci.sh)"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
mkdir -p "$T/a/src" "$T/b/src"; printf '[package]\nname = "crate_a"\n' > "$T/a/Cargo.toml"; printf '[package]\nname = "crate_b"\n' > "$T/b/Cargo.toml"
printf '/// ```\n/// assert!(true);\n/// ```\npub fn f() {}\n' > "$T/a/src/lib.rs"; printf '/// chỉ văn bản\npub fn g() {}\n' > "$T/b/src/lib.rs"
check "doc_crates chỉ trả crate có ví dụ" "$(doc_crates "$T")" crate_a
mkdir -p "$T/c/src" "$T/d/src" "$T/e/src"
printf '[package]\nname = "crate_c"\n[lib]\n' > "$T/c/Cargo.toml"; printf '/**\n```\nlet _ = 1;\n```\n*/\npub fn h() {}\n' > "$T/c/src/lib.rs"
printf '[package]\nname = "bin_d"\n' > "$T/d/Cargo.toml"; printf '//! ```\n//! x\n//! ```\nfn main() {}\n' > "$T/d/src/main.rs"
printf '[package]\nversion = "1"\nname = "crate_e"\n[dependencies]\nname = "no"\n' > "$T/e/Cargo.toml"; printf '#![doc = include_str!("../README.md")]\n' > "$T/e/src/lib.rs"
check "doc_crates thấy khối /** */, bỏ crate chỉ có bin, lấy name từ [package], thấy #![doc = include_str!]" "$(doc_crates "$T" | tr '\n' ' ')" "crate_a crate_c crate_e "
check "cây hiện tại không có doc-test" "$(WT="$PWD"; doc_crates)" ""

# cstep: PASS ghi cache; lượt sau cùng khoá thì SKIP; FAIL không ghi cache; --no-cache (CACHE=0) ép chạy lại.
eval "$(sed -n '/^step() {/,/^}/p;/^cstep() {/,/^}/p' tools/ci/local_ci.sh)"
CI="$T/ci"; OUT="$T/out"; WT="$T"; REPO="$(git rev-parse --path-format=absolute --git-common-dir)"; SHA="$(git rev-parse HEAD)"; mkdir -p "$CI/cache" "$OUT"
STEPS=(); FAILED=(); CACHED=(); CACHE=1
cstep demo k1 . true >/dev/null; check "PASS ghi cache" "$([ -s "$CI/cache/demo-k1" ] && echo yes)" yes
STEPS=(); CACHED=(); out="$(cstep demo k1 . false)"; check "cùng khoá: SKIP (cache)" "$(grep -c 'SKIP.*cache' <<<"$out")" 1
cstep demo k1 . false >/dev/null; check "SKIP trả OK" "$?" 0
CACHE=0; cstep demo k1 . false >/dev/null; check "--no-cache ép chạy (bước đỏ → lỗi)" "$?" 1
CACHE=1; cstep demo k2 . false >/dev/null; check "khoá khác chạy thật" "$?" 1
check "FAIL không ghi cache" "$([ -e "$CI/cache/demo-k2" ] && echo yes || echo no)" no

# Chốt cảnh báo: đường dẫn trèo ra ngoài UX/src-tauri (>= 3 cấp ../ từ crate) chưa nằm trong khoá Rust của local_ci.sh. Xuất hiện đường mới → thêm vào RUST_KEY.
outside="$(grep -rnE '(\.\./){3,}' --include='*.rs' UX/src-tauri | grep -v '/target/' | grep -v 'docs/team/eval' | grep -v 'safe_name' | grep -v '#\[path' || true)"
check "không có đường dẫn ngoài src-tauri nào ngoài docs/team/eval" "$outside" ""
exit $fail
