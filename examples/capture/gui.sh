#!/usr/bin/env bash
# Screenshots of the desktop GUI (macOS only — osascript + screencapture).
#
#   examples/capture/gui.sh <out-dir> <theme> <scene> [scene ...]
#
# Runs target/debug/flashkraft in demo mode (fake drives, simulated writes) and
# saves <out-dir>/gui-<scene>.png for each scene
# (home | ready | drives | batch | flashing | complete).
set -euo pipefail
[ "$(uname)" = Darwin ] || { echo "gui.sh needs macOS (screencapture)"; exit 1; }
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="${1:?usage: gui.sh <out-dir> <theme> <scene>...}"; theme="$2"; shift 2
mkdir -p "$out"
# shellcheck disable=SC1091
source "$root/examples/vhs/setup.sh" "$theme"

for scene in "$@"; do
    FLASHKRAFT_DEMO_STEP_MS=250 FLASHKRAFT_SCENE="$scene" "$root/target/debug/flashkraft" >/dev/null 2>&1 &
    pid=$!
    sleep 4
    # The window must really be on top — screencapture grabs whatever is on screen
    # in that rectangle, so a covered window would capture someone else's app.
    front=false
    for _ in 1 2 3 4 5 6; do
        osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null 2>&1 || true
        sleep 1
        if [ "$(osascript -e "tell application \"System Events\" to get frontmost of (first process whose unix id is $pid)" 2>/dev/null)" = true ]; then front=true; break; fi
    done
    if [ "$front" != true ]; then
        echo "skipping $scene: could not bring the window to the front" >&2
        kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
        continue
    fi
    # Let the simulated flash get part-way.
    [ "$scene" = flashing ] && sleep 3
    sleep 1
    read -r x y w h < <(osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid) to get {position, size} of window 1" | tr -d ',' | awk '{print $1,$2,$3,$4}')
    screencapture -x -R "$x,$y,$w,$h" "$out/gui-$scene.png"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    echo "wrote $out/gui-$scene.png"
done
