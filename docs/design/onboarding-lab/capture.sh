#!/usr/bin/env bash
# Chụp từng bước onboarding (sáng/tối) bằng Chrome headless từ dev server (mock, #first=1). Khung CUỐI của animation (reduced-motion) để ảnh ổn định.
# Dùng:  (cd UX && npx vite --port 5397 --strictPort) &  rồi  bash docs/design/onboarding-lab/capture.sh
set -euo pipefail
OUT="$(cd "$(dirname "$0")" && pwd)"; PORT="${PORT:-5397}"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"; PROFILE="$(mktemp -d)"
shot() { # tên, hash, sáng=1/tối=0, w, h
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --user-data-dir="$PROFILE" --window-size="$4,$5" --virtual-time-budget=6000 \
    --disable-breakpad --force-prefers-reduced-motion --blink-settings=preferredColorScheme="$3" --screenshot="$OUT/$1.png" "http://localhost:$PORT/#first=1&$2" >/dev/null 2>&1 || true
}
for theme in dark light; do
  s=0; [ "$theme" = light ] && s=1
  for step in ${STEPS:-o1 o2 o3 o4 o5 o6 o7 end}; do shot "$theme-$step" "ob=$step" $s 1280 800; done
  shot "$theme-intro" "ob=intro&hold=1" $s 1280 800
  shot "$theme-narrow-o1" "ob=o1" $s 720 900
done
rm -rf "$PROFILE"
