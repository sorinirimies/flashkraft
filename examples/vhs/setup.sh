#!/usr/bin/env bash
# Demo environment for the VHS tapes and screenshots: fake drives, a throw-away
# HOME with a chosen theme, and a 700 MB sparse image. Never touches real drives.
# Usage: source examples/vhs/setup.sh [theme]
THEME="${1:-Tokyo Night}"
export FLASHKRAFT_DEMO=1
export HOME=/tmp/fk/home
export XDG_CONFIG_HOME=$HOME/.config
rm -rf "$HOME"
mkdir -p "$HOME/.config/flashkraft" "$HOME/Library/Application Support/flashkraft"
for SETTINGS in tui-settings.json gui-settings.json; do
for d in "$HOME/.config/flashkraft" "$HOME/Library/Application Support/flashkraft"; do
    printf '{"theme":"%s"}\n' "$THEME" > "$d/$SETTINGS"
done
done
truncate -s 700m /tmp/fk/debian-12.iso
