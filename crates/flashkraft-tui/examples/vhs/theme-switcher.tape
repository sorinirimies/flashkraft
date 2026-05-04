# VHS tape — FlashKraft TUI · Theme Switcher Demo
# Showcases 43-theme catalogue, live-preview panel (Shift+T), and t cycling.
# Run: vhs crates/flashkraft-tui/examples/vhs/theme-switcher.tape

Output crates/flashkraft-tui/examples/vhs/generated/theme-switcher.gif

Set Theme "Catppuccin Mocha"
Set Width 1400
Set Height 900
Set FontSize 14
Set PlaybackSpeed 1.0

# ── Launch silently ───────────────────────────────────────────────────────────
Hide
Type "cargo run -p flashkraft-tui --example theme_demo"
Enter
Sleep 6s
Show

# ── Theme panel is open at startup — navigate it with live preview ────────────
Sleep 800ms

# Scroll down through the catalogue — each move previews immediately
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 600ms

# Scroll back up a few entries
Up
Sleep 500ms
Up
Sleep 500ms
Up
Sleep 600ms

# Continue down into the Ghostty-sourced themes
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 600ms

# ── Confirm the current selection (Enter) ─────────────────────────────────────
Enter
Sleep 1000ms

# ── Close panel and cycle with t ─────────────────────────────────────────────
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms
Type "t"
Sleep 800ms

# ── Re-open the panel with Shift+T to show scrolling ─────────────────────────
Type "T"
Sleep 1000ms

Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms

# ── Esc to revert live preview back to previously confirmed theme ─────────────
Escape
Sleep 1500ms

# ── Hold on final state ───────────────────────────────────────────────────────
Sleep 2s

# ── Quit ─────────────────────────────────────────────────────────────────────
Ctrl+Q
Sleep 500ms
