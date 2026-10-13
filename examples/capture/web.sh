#!/usr/bin/env bash
# Screenshots of the web dashboard through headless Chromium/Brave.
#
#   examples/capture/web.sh <out-dir> [theme]
#
# Needs target/debug/flashkraft-web and the Topcoat bundle
# (`just bundle-web`). Runs in demo mode: fake drives, nothing is written.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="${1:?usage: web.sh <out-dir> [theme]}"; theme="${2:-Tokyo Night}"
mkdir -p "$out"
# shellcheck disable=SC1091
source "$root/examples/vhs/setup.sh" "$theme"

case "$(uname)" in
    Darwin) default_browser="/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" ;;
    *) default_browser="$(command -v chromium || command -v google-chrome || command -v brave-browser || true)" ;;
esac
BROWSER="${BROWSER:-$default_browser}"
[ -x "$BROWSER" ] || { echo "no Chromium-based browser found; set BROWSER=/path/to/it"; exit 1; }

port=3978
FLASHKRAFT_WEB_ASSETS="$root/target/debug/assets" "$root/target/debug/flashkraft-web" --port "$port" --no-open >/dev/null 2>&1 &
web=$!; trap 'kill $web 2>/dev/null || true' EXIT
sleep 2

shot() { # name width height path
    rm -f "$out/$1.png"
    "$BROWSER" --headless=new --disable-gpu --hide-scrollbars --window-size="$2,$3" \
        --virtual-time-budget=4000 --screenshot="$out/$1.png" \
        "http://127.0.0.1:$port/$4" >/dev/null 2>&1 &
    local b=$!
    # A stuck headless browser must not hang the script.
    for _ in $(seq 1 30); do [ -s "$out/$1.png" ] && break; sleep 1; done
    sleep 1; kill "$b" 2>/dev/null || true; wait "$b" 2>/dev/null || true
    [ -s "$out/$1.png" ] && echo "wrote $out/$1.png" || echo "FAILED $1" >&2
}
q="image=/tmp/fk/debian-12.iso"
shot web-wide   1400 760 "?$q"
shot web-medium 860  900 "?$q"
shot web-narrow 420  1100 "?$q"
shot web-history 1400 520 "history"

# A flash in progress: three simulated drives, slowed down so it can be captured.
kill $web 2>/dev/null || true; wait $web 2>/dev/null || true
FLASHKRAFT_DEMO_STEP_MS=200 FLASHKRAFT_WEB_ASSETS="$root/target/debug/assets" "$root/target/debug/flashkraft-web" --port "$port" --no-open >/dev/null 2>&1 &
web=$!; sleep 2
token=$(curl -s "http://127.0.0.1:$port/" | grep -Eo '[0-9a-f]{32}' | head -1)
curl -s -XPOST "http://127.0.0.1:$port/api/flash" -H 'content-type: application/json' -H "x-flashkraft-token: $token" \
    -d '{"image":"/tmp/fk/debian-12.iso","devices":["/dev/sdb","/dev/sdc","/dev/sdd"],"confirm":3}' >/dev/null
sleep 4
shot web-flashing 1400 1150 "?$q"
