#!/usr/bin/env bash
# l2d-compare: deterministic pixel-parity harness, V1 (deployed reama-cubism-runtime.js bundle) vs V2
# (amadeus-live2d through tools/live2d-viewer), same Core, model and texture, headless Chrome + SwiftShader.
#
#   tools/l2d-compare/run.sh [OUT_DIR]                      (default OUT_DIR: <repo>/target/l2d-compare)
#
# Env (all optional):
#   V1_PUBLIC   V1's public/ directory, served as /resources for BOTH pages (default <repo>/../public)
#   VIEWPORTS   default "1280x720,390x844"        EMOTIONS  default "NORMAL,SMILE,SLEEPING"
#   FRAMES      default "1,60,300"                DPR       default 1
#   SEED        Math.random seed (default 20261118)
#   REPEAT      extra captures of the first scenario per version, noise-floor check (default 1)
#   CONTROLS    1 (default): V1 one frame later per frame count of the first viewport/emotion, the
#               sensitivity control (must differ); 0: off
#   QUERY       extra query string for both pages, e.g. 'lip=0.8&char=あ&do=120:setEmotion:"NORMAL"'
#               (emotion/lip/char/eyeX/eyeY + the shim's frames/seed/timeout/readback/stepms/do, see shim.js)
#   TAG         suffix of the scenario directory names (-TAG); SHOTS  capture sub-directory (default shots)
#   THRESHOLD   "differs" threshold for the statistics, 0..255 (default 8)
#   CAPTURE_TIMEOUT  real-time limit per page in seconds (default 180; raise it for thousands of frames)
#   SKIP_BUILD=1  reuse OUT_DIR/dist-v2 if present
#   CARGO_TARGET_DIR  passed through to Trunk
# Needs: trunk, node >= 22 (global WebSocket), google-chrome, python3 with PIL + numpy.
#
# Pages (both load /shim.js first: virtual clock frozen while loading, frame 1 = first frame with WebGL draw
# calls, then exactly 1/60 s per frame (or ?stepms), mulberry32 Math.random, stop after ?frames=N, title
# "DONE"). Extra scenarios (all emotions, DPR 3/2.625/1.25, scripted actions, irregular frame times, long
# runs, the sneeze easter egg): tools/l2d-compare/extended.sh.
#   /v1.html?emotion=E&frames=N   tools/l2d-compare/v1.html (V1 Live2DCanvas mount, Core ready first)
#   /v2/?emotion=E&frames=N       tools/l2d-compare/v2/index.html built by Trunk (the viewer + shim)
# Both are also usable by hand: OUT_DIR/serve is a static root (python3 -m http.server -d OUT_DIR/serve).
#
# Output: OUT_DIR/shots/<W>x<H>[@DPR]-<EMOTION>-f<NNN>[-TAG]/{v1,v2}.png (screenshots), {v1,v2}.canvas.png (RGBA canvas
# readback of frame N), {v1,v2}.json/.log (shim report + console), diff.png (heatmap), compare.png
# (V1 | V2 | heatmap), canvas-diff.png; OUT_DIR/shots/results.{txt,json} (table). Exit status 0 when every
# page reached frame N (differences are reported, not failed: this is a measuring tool).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:-$ROOT/target/l2d-compare}"
V1_PUBLIC="${V1_PUBLIC:-$(cd "$ROOT/.." && pwd)/public}"
VIEWPORTS="${VIEWPORTS:-1280x720,390x844}"
EMOTIONS="${EMOTIONS:-NORMAL,SMILE,SLEEPING}"
FRAMES="${FRAMES:-1,60,300}"
DPR="${DPR:-1}"
SEED="${SEED:-20261118}"
REPEAT="${REPEAT:-1}"
CONTROLS="${CONTROLS:-1}"
QUERY="${QUERY:-}"
TAG="${TAG:-}"
SHOTS="${SHOTS:-shots}"
THRESHOLD="${THRESHOLD:-8}"
CAPTURE_TIMEOUT="${CAPTURE_TIMEOUT:-180}"

for f in live2d/reama-cubism-runtime.js live2d/live2dcubismcore.min.js; do
  [ -f "$V1_PUBLIC/resources/$f" ] || { echo "missing $V1_PUBLIC/resources/$f (set V1_PUBLIC)"; exit 2; }
done
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
echo "output: $OUT"

# --- 1. V2 page: the live2d-viewer built from the shim-first index variant ----------------------------
if [ "${SKIP_BUILD:-0}" = 1 ] && [ -f "$OUT/dist-v2/index.html" ]; then
  echo "build: reusing $OUT/dist-v2"
else
  echo "build: trunk (live2d-viewer, release) ..."
  (cd "$HERE/v2" && trunk build --release --public-url /v2/ --dist "$OUT/dist-v2" index.html) \
    >"$OUT/trunk.log" 2>&1 || { tail -40 "$OUT/trunk.log"; exit 1; }
fi
# The shim must come before the wasm bootstrap (module scripts run after all classic scripts anyway).
python3 - "$OUT/dist-v2/index.html" <<'EOF' || exit 1
import re, sys
html = open(sys.argv[1], encoding="utf-8").read()
shim = html.find('<script src="/shim.js"></script>')
boot = min([i for i in (html.find("<script type=\"module\""), html.find("modulepreload")) if i >= 0] or [len(html)])
if shim < 0 or shim > boot:
    sys.exit("V2 page: /shim.js missing or not before the wasm bootstrap")
EOF

# --- 2. static server: /resources (V1 public, both pages), /shim.js, /v1.html, /v2/ ---------------------
SERVE="$OUT/serve"
rm -rf "$SERVE"
mkdir -p "$SERVE"
ln -s "$V1_PUBLIC/resources" "$SERVE/resources"
ln -s "$OUT/dist-v2" "$SERVE/v2"
cp "$HERE/shim.js" "$HERE/v1.html" "$SERVE/"
PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')"
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$SERVE" >"$OUT/http.log" 2>&1 &
SERVER=$!
trap 'kill "$SERVER" 2>/dev/null || true' EXIT
for _ in $(seq 100); do curl -sf -o /dev/null "http://127.0.0.1:$PORT/v1.html" && break; sleep 0.1; done
curl -sf -o /dev/null "http://127.0.0.1:$PORT/v2/" || { echo "server not reachable"; exit 1; }

# --- 3. captures ----------------------------------------------------------------------------------------
rm -rf "${OUT:?}/$SHOTS"
CAPTURE_STATUS=0
node "$HERE/capture.mjs" --base "http://127.0.0.1:$PORT" --out "$OUT/$SHOTS" \
  --viewports "$VIEWPORTS" --emotions "$EMOTIONS" --frames "$FRAMES" --dpr "$DPR" --seed "$SEED" \
  --repeat "$REPEAT" --controls "$CONTROLS" --query "$QUERY" --tag "$TAG" --timeout "$CAPTURE_TIMEOUT" \
  || CAPTURE_STATUS=$?

# --- 4. statistics, heatmaps ------------------------------------------------------------------------------
python3 -I "$HERE/diff.py" "$OUT/$SHOTS" --threshold "$THRESHOLD"
echo "results: $OUT/$SHOTS/results.txt"
exit "$CAPTURE_STATUS"
