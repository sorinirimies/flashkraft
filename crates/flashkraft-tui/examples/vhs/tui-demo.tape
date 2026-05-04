# VHS tape for FlashKraft TUI — Full Workflow Demo
# Shows the complete SelectImage → SelectDrive → DriveInfo → ConfirmFlash flow.
# Run: vhs crates/flashkraft-tui/examples/vhs/tui-demo.tape

Output crates/flashkraft-tui/examples/vhs/generated/tui-demo.gif

Set Theme "Catppuccin Mocha"
Set Width 1400
Set Height 900
Set FontSize 14
Set PlaybackSpeed 1.5

# ── Launch silently — skip the shell prompt entirely ──────────────────────────
Hide
Type "dd if=/dev/zero of=/tmp/flashkraft-demo.iso bs=1M count=1 2>/dev/null"
Enter
Sleep 1s
Type "cargo run -p flashkraft-tui --example tui_demo"
Enter
Sleep 5s
Show

# ── SelectImage screen — already in Editing mode ──────────────────────────────
Sleep 800ms

# Type an image path
Type "/tmp/flashkraft-demo.iso"
Sleep 600ms

# Open the file browser with Tab
Tab
Sleep 1200ms

# Navigate the explorer a bit
Down
Sleep 300ms
Down
Sleep 300ms
Down
Sleep 300ms
Up
Sleep 300ms

# Page through the listing
PageDown
Sleep 500ms
PageUp
Sleep 500ms

# Ascend to parent directory
Backspace
Sleep 800ms

# Dismiss browser — back to SelectImage with path filled
Escape
Sleep 800ms

# Confirm the path → SelectDrive
Enter
Sleep 1200ms

# ── SelectDrive screen ────────────────────────────────────────────────────────
# Refresh drive list
Type "r"
Sleep 2s

Down
Sleep 400ms
Down
Sleep 400ms
Up
Sleep 400ms

# Attempt to confirm (no-op if list is empty in CI)
Enter
Sleep 1000ms

# ── DriveInfo → ConfirmFlash ──────────────────────────────────────────────────
Type "f"
Sleep 1000ms

# ── Back out safely — never write to real hardware ───────────────────────────
Type "b"
Sleep 400ms
Type "b"
Sleep 400ms
Type "b"
Sleep 400ms

# ── Quit ─────────────────────────────────────────────────────────────────────
Escape
Sleep 800ms
