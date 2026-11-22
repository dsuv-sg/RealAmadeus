#!/usr/bin/env bash
# l2d-compare extended suite: the scenarios beyond run.sh's default matrix, each a run.sh invocation with its
# own capture sub-directory (OUT_DIR/<suite>/), sharing one V2 build.
#
#   tools/l2d-compare/extended.sh [OUT_DIR] [SUITE ...]     (default OUT_DIR: <repo>/target/l2d-compare)
#
# Suites (default: all, in this order):
#   emotions  1280x720, every V1 emotion tag + an unknown one (CONFUSED: NORMAL target, generic idle branch),
#             frames 30 (bursts in flight) and 120
#   dpr3      390x844 @ DPR 3 (1170x2532 backing store, a phone), NORMAL f60
#   dpr2625   412x915 @ DPR 2.625 (fractional backing store: 1081.5 x 2401.875 truncated), SMILE f60
#   dpr125    1920x1045 @ DPR 1.25 (V1's desktop stage on a 125 % display: 2400x1306), NORMAL f60
#   wake      SLEEPING, setEmotion("NORMAL") at frame 120 (startled wake-up + look-around), f200
#   speak     SMILE + lip 0.8 + "あ" + gaze, then "い" @40, ANGRY @60, lip 0.3 + "ん" @80; f100, both viewports
#   long      NORMAL and SLEEPING f1900 (≈31.6 s: several blinks, the periodic sleep nod at 30 s)
#   sneeze    NORMAL f2780 with seed 20261120, whose Math.random fires the 45 s sneeze easter egg at frame
#             2701 (verified by simulating V1's timers); frame 2780 is mid-sneeze
#   jitter    NORMAL and SMILE f240 with irregular frame durations (stepms 16.667,33.333,8.333,16.667,120,
#             16.667,50,4 ms): the player's 0.1 s delta cap, multi-step physics, uneven Date.now deltas
#   gap       SLEEPING f150 with a 6 s frame at frame 91 (a tab resumed from the background): physics'
#             remain-time reset (> 5 s), the player's cap in the middle of the sleep nod
# Env: as run.sh (V1_PUBLIC, CARGO_TARGET_DIR, THRESHOLD); SKIP_BUILD=1 reuses OUT_DIR/dist-v2 for the first
# suite too. Exit status: non-zero when a page did not finish (differences are reported, not failed).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:-$ROOT/target/l2d-compare}"
shift || true
SUITES=("$@")
[ ${#SUITES[@]} -gt 0 ] || SUITES=(emotions dpr3 dpr2625 dpr125 wake speak jitter gap long sneeze)
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
STATUS=0
FIRST=1

suite() { # name, then VAR=value ... for run.sh
  local name="$1"
  shift
  local skip=1
  if [ "$FIRST" = 1 ] && [ "${SKIP_BUILD:-0}" != 1 ]; then skip=0; fi
  FIRST=0
  echo "=== suite $name"
  env SKIP_BUILD="$skip" SHOTS="$name" REPEAT=0 CONTROLS=0 "$@" "$HERE/run.sh" "$OUT" >"$OUT/$name.log" 2>&1 || STATUS=1
  grep -E "FAILED|TIMEOUT" "$OUT/$name.log" || true
  tail -n 1 "$OUT/$name/results.txt" 2>/dev/null || { echo "  no results (see $OUT/$name.log)"; STATUS=1; }
}

for s in "${SUITES[@]}"; do
  case "$s" in
    emotions) suite emotions VIEWPORTS=1280x720 FRAMES=30,120 \
      EMOTIONS=NORMAL,SMILE,ANGRY,SAD,SURPRISED,BLUSH,WINK,DISGUST,SMUG,THINKING,PANIC,SLEEPING,CONFUSED ;;
    dpr3) suite dpr3 VIEWPORTS=390x844 DPR=3 EMOTIONS=NORMAL FRAMES=60 ;;
    dpr2625) suite dpr2625 VIEWPORTS=412x915 DPR=2.625 EMOTIONS=SMILE FRAMES=60 ;;
    dpr125) suite dpr125 VIEWPORTS=1920x1045 DPR=1.25 EMOTIONS=NORMAL FRAMES=60 ;;
    wake) suite wake VIEWPORTS=1280x720 EMOTIONS=SLEEPING FRAMES=200 TAG=wake \
      'QUERY=do=120:setEmotion:"NORMAL"' ;;
    speak) suite speak VIEWPORTS=1280x720,390x844 EMOTIONS=SMILE FRAMES=100 TAG=speak \
      'QUERY=lip=0.8&char=あ&eyeX=0.5&eyeY=-0.3&do=40:setSpokenChar:"い";60:setEmotion:"ANGRY";80:setLipSync:0.3;80:setSpokenChar:"ん"' ;;
    jitter) suite jitter VIEWPORTS=1280x720 EMOTIONS=NORMAL,SMILE FRAMES=240 TAG=jitter \
      'QUERY=stepms=16.667,33.333,8.333,16.667,120,16.667,50,4' ;;
    gap) suite gap VIEWPORTS=1280x720 EMOTIONS=SLEEPING FRAMES=150 TAG=gap 'QUERY=stepms=16.667*89,6000,16.667' ;;
    long) suite long VIEWPORTS=1280x720 EMOTIONS=NORMAL,SLEEPING FRAMES=1900 CAPTURE_TIMEOUT=900 ;;
    sneeze) suite sneeze VIEWPORTS=1280x720 EMOTIONS=NORMAL FRAMES=2780 SEED=20261120 TAG=sneeze \
      CAPTURE_TIMEOUT=900 ;;
    *) echo "unknown suite $s"; STATUS=2 ;;
  esac
done
exit "$STATUS"
