#!/usr/bin/env bash
# Thử hàng đợi của local_ci.sh bằng tiến trình giả (FONTATLAS_CI_TEST_HOLD: giữ khoá vài giây rồi thoát, không chạy bước CI).
#   bash tools/ci/test_queue.sh
set -uo pipefail
cd "$(dirname "$0")/../.."
export FONTATLAS_CI_HOME="$(mktemp -d)"; trap 'rm -rf "$FONTATLAS_CI_HOME"' EXIT
CI=tools/ci/local_ci.sh; fail=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: muốn '$3', được '$2'"; fail=1; fi; }
run() { FONTATLAS_CI_TEST_HOLD=$2 bash $CI HEAD ${3:-} > "$FONTATLAS_CI_HOME/$1.out" 2>&1; echo "$(date +%s)" > "$FONTATLAS_CI_HOME/$1.end"; }

# 1) FIFO: A giữ khoá, B rồi C xếp hàng; thứ tự xong phải A < B < C.
run A 3 & sleep 1; run B 2 & sleep 1; run C 1 & sleep 1
out="$(bash $CI --status)"; check "status liệt kê giữ khoá + 2 vé" "$(grep -c "pid" <<<"$out")" 3; check "status có ước lượng chờ" "$(grep -c "ước lượng bắt đầu" <<<"$out")" 2
wait
a=$(cat "$FONTATLAS_CI_HOME/A.end"); b=$(cat "$FONTATLAS_CI_HOME/B.end"); c=$(cat "$FONTATLAS_CI_HOME/C.end")
check "FIFO A<B<C" "$([ "$a" -le "$b" ] && [ "$b" -le "$c" ] && echo yes)" yes
check "B thật sự chờ A" "$([ $((b-a)) -ge 2 ] && echo yes)" yes

# 2) Vé của tiến trình đã chết bị dọn: tạo vé giả với pid không tồn tại đứng đầu.
touch "$FONTATLAS_CI_HOME/queue/Q00000000000000000001-999999-ghost"
start=$(date +%s); run D 0; end=$(cat "$FONTATLAS_CI_HOME/D.end")
check "vé ma không chặn hàng" "$([ $((end-start)) -le 8 ] && echo yes)" yes
check "vé ma đã bị dọn" "$(ls "$FONTATLAS_CI_HOME/queue" | wc -l | tr -d ' ')" 0

# 3) --priority bị từ chối nếu không phải Lead; được nhận và chen lên đầu nếu là Lead.
bash $CI HEAD --priority >/dev/null 2>&1; check "priority bị từ chối" "$?" 2
run E 3 & sleep 1; run F 1 & sleep 1
FONTATLAS_CI_PRIORITY=1 run G 1 --priority & sleep 1; wait
e=$(cat "$FONTATLAS_CI_HOME/E.end"); f=$(cat "$FONTATLAS_CI_HOME/F.end"); g=$(cat "$FONTATLAS_CI_HOME/G.end")
check "G (priority) chạy trước F" "$([ "$g" -le "$f" ] && [ "$e" -le "$g" ] && echo yes)" yes

# 4) --no-wait gặp khoá bận thì thoát 3; khoá mồ côi (pid chết) bị dọn.
run H 3 & sleep 1; bash $CI HEAD --no-wait >/dev/null 2>&1; check "--no-wait thoát 3" "$?" 3; wait
mkdir "$FONTATLAS_CI_HOME/lock"; echo 999999 > "$FONTATLAS_CI_HOME/lock/pid"
run I 0 --no-wait; check "khoá mồ côi bị dọn" "$(grep -c 'đã có khoá' "$FONTATLAS_CI_HOME/I.out")" 1
exit $fail
