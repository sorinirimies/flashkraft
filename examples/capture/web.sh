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

shot() { # name width height query
    "$BROWSER" --headless=new --disable-gpu --hide-scrollbars --window-size="$2,$3" \
        --virtual-time-budget=5000 --screenshot="$out/$1.png" \
        "http://127.0.0.1:$port/$4" >/dev/null 2>&1
    echo "wrote $out/$1.png"
}
q="image=/tmp/fk/debian-12.iso"
shot web-wide   1400 760 "?$q"
shot web-medium 860  900 "?$q"
shot web-narrow 420  1100 "?$q"
shot web-history 1400 520 "history"
