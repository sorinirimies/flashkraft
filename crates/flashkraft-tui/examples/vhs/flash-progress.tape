# VHS tape — FlashKraft TUI · Flash Progress Demo
# Showcases the tui-slider animated progress bar during a simulated image write.
# Run: vhs crates/flashkraft-tui/examples/vhs/flash-progress.tape

Output crates/flashkraft-tui/examples/vhs/generated/flash-progress.gif

Set Theme "Catppuccin Mocha"
Set Width 1400
Set Height 900
Set FontSize 14
Set PlaybackSpeed 1.0

# ── Launch silently ───────────────────────────────────────────────────────────
Hide
Type "cargo run -p flashkraft-tui --example flash_progress_demo"
Enter
Sleep 2s
Show

# ── Preparing stage (~1 s) ────────────────────────────────────────────────────
Sleep 1500ms

# ── Writing stage — let the slider animate across the full range (~8 s) ───────
Sleep 8500ms

# ── Verifying stage ───────────────────────────────────────────────────────────
Sleep 1500ms

# ── Complete screen — hold so viewers can read it ─────────────────────────────
Sleep 3s
