#!/usr/bin/env bash
# Headless-Chrome checks of amadeus-live2d through the live2d-viewer.
#
#   tools/live2d-viewer/browser-tests/run.sh [WORK_DIR]
#
# Needs: trunk, node >= 22 (global WebSocket/fetch), google-chrome, python3 with PIL + numpy.
# Env: V1_PUBLIC (default: <repo>/../public, V1's public/ with reama-cubism-runtime.js — the parity
#      reference; parity is skipped when it is missing), CARGO_TARGET_DIR (optional).
#
# 1. smoke    — `--screenshot` after `--virtual-time-budget=8000` (1280×720 and 500×900): the model must
#               be visible (non-black pixels).
# 2. parity   — V1's deployed bundle vs V2 on the same Core/model/texture with deterministic clocks
#               (frozen while loading, then exactly N frames of 1000/60 ms, mulberry32 Math.random):
#               every scenario must be pixel-identical (max diff 0).
# 3. lifecycle — context loss + restore (D6), resize, destroy (no throw, host emptied), remount,
#               destroy while loading, 20 UI-style remounts: expected states, no console errors.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
WORK="${1:-$(mktemp -d "${TMPDIR:-/tmp}/l2d-browser-tests.XXXXXX")}"
V1_PUBLIC="${V1_PUBLIC:-$(cd "$ROOT/.." && pwd)/public}"
mkdir -p "$WORK"
echo "work dir: $WORK"

# --- build the viewer -------------------------------------------------------------------------
(cd "$ROOT/tools/live2d-viewer" && trunk build --release --public-url /v2/ --dist "$WORK/dist-v2") \
  >"$WORK/trunk.log" 2>&1 || { tail -30 "$WORK/trunk.log"; exit 1; }

# --- serve /resources + /v2 (+ /v1) -------------------------------------------------------------
mkdir -p "$WORK/serve"
if [ -f "$V1_PUBLIC/resources/live2d/reama-cubism-runtime.js" ]; then
  ln -sfn "$V1_PUBLIC/resources" "$WORK/serve/resources"
  mkdir -p "$WORK/serve/v1" && cp "$HERE/v1/index.html" "$WORK/serve/v1/index.html"
  HAVE_V1=1
else
  echo "V1 runtime not found under $V1_PUBLIC — parity checks skipped"
  ln -sfn "$ROOT/assets/public/resources" "$WORK/serve/resources"
  HAVE_V1=0
fi
ln -sfn "$WORK/dist-v2" "$WORK/serve/v2"
PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')"
(cd "$WORK/serve" && exec python3 -m http.server "$PORT" --bind 127.0.0.1) >"$WORK/http.log" 2>&1 &
SERVER=$!
trap 'kill "$SERVER" 2>/dev/null || true' EXIT
for _ in $(seq 50); do curl -sf -o /dev/null "http://127.0.0.1:$PORT/v2/" && break; sleep 0.1; done
V2="http://127.0.0.1:$PORT/v2/?resources=/resources/"
FAIL=0

# --- 1. smoke ------------------------------------------------------------------------------------
for size in 1280,720 500,900; do
  shot="$WORK/smoke-${size/,/x}.png"
  rm -rf "$WORK/prof-smoke"
  env -u DISPLAY -u XAUTHORITY timeout 170 google-chrome --headless=new --no-sandbox \
    --use-angle=swiftshader --enable-unsafe-swiftshader --user-data-dir="$WORK/prof-smoke" \
    --no-first-run --window-size="$size" --virtual-time-budget=8000 --screenshot="$shot" "$V2" \
    >"$WORK/smoke-${size/,/x}.log" 2>&1 || true
  if python3 -I "$HERE/imgdiff.py" "$shot" "$shot" >/dev/null; then
    echo "smoke $size: OK ($shot)"
  else
    echo "smoke $size: FAILED (no model visible in $shot)"; FAIL=1
  fi
done

# --- 2. parity with V1 ---------------------------------------------------------------------------
parity() { # name w h frames setup (P. = the player object)
  local name="$1" w="$2" h="$3" n="$4" setup="$5"
  timeout 250 node "$HERE/parity.mjs" "http://127.0.0.1:$PORT/v1/" "$V2" "$WORK/$name" "$w" "$h" "$n" \
    "${setup//P./v1Player.}" "${setup//P./l2dViewer.}" >"$WORK/$name.json" 2>&1 || true
  if python3 -I "$HERE/imgdiff.py" "$WORK/$name/A.png" "$WORK/$name/B.png" "$WORK/$name/diff.png" >"$WORK/$name.diff" 2>&1; then
    echo "parity $name: identical ($(tail -1 "$WORK/$name.diff"))"
  else
    echo "parity $name: DIFFERENT"; cat "$WORK/$name.json" "$WORK/$name.diff"; FAIL=1
  fi
}
if [ "$HAVE_V1" = 1 ]; then
  parity idle 1280 720 1 ""
  parity smile-speak 1280 720 90 "P.setEmotion('smile'); P.setLipSync(1); P.setSpokenChar('あ'); P.setEyeTracking(0.5, 0.3)"
  parity portrait-wink 430 932 120 "P.setEmotion('wink'); P.setEyeTracking(-0.6, 0.4)"
  parity sleeping 1280 720 200 "P.setEmotion('SLEEPING')"
  DSF=1.25 parity dpr125-angry 1280 720 60 "P.setEmotion('angry'); P.setLipSync(0.8)"
fi

# --- 3. lifecycle --------------------------------------------------------------------------------
if timeout 280 node "$HERE/lifecycle.mjs" "$V2" "$WORK/lifecycle" >"$WORK/lifecycle.json" 2>&1; then
  echo "lifecycle: OK"
else
  echo "lifecycle: FAILED"; cat "$WORK/lifecycle.json"; FAIL=1
fi

exit "$FAIL"
