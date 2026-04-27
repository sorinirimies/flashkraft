#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# FlashKraft — Stage release artifacts
# ──────────────────────────────────────────────────────────────────────────────
# Copies the compiled binaries from Cargo's target directory into a flat
# `dist/` folder with target-triple-based names suitable for upload.
#
# Usage (from the workspace root):
#   nu scripts/ci/stage_artifacts.nu <target> [--suffix <ext>] [--gui-binary <name>]
#
# Examples:
#   nu scripts/ci/stage_artifacts.nu x86_64-unknown-linux-gnu
#   nu scripts/ci/stage_artifacts.nu x86_64-pc-windows-msvc --suffix .exe
#   nu scripts/ci/stage_artifacts.nu x86_64-unknown-linux-gnu --gui-binary flashkraft
#   nu scripts/ci/stage_artifacts.nu x86_64-unknown-linux-musl --gui-binary ""
#
# What it does:
#   1. Creates the `dist/` directory if it doesn't exist.
#   2. Copies the TUI binary:
#        target/<target>/release/flashkraft-tui<suffix>
#        → dist/flashkraft-tui-<target><suffix>
#   3. If --gui-binary is non-empty, copies the GUI binary:
#        target/<target>/release/flashkraft<suffix>
#        → dist/flashkraft-gui-<target><suffix>
#   4. Lists all staged files.
# ──────────────────────────────────────────────────────────────────────────────

def main [
    target: string          # Rust target triple (e.g. x86_64-unknown-linux-gnu)
    --suffix: string = ""   # Binary file extension (e.g. .exe for Windows)
    --gui-binary: string = ""  # GUI binary name; empty string skips GUI staging
] {
    let dist_dir = "dist"

    # ── 1. Ensure dist/ exists ────────────────────────────────────────────────
    if not ($dist_dir | path exists) {
        mkdir $dist_dir
    }

    # ── 2. Stage the TUI binary ──────────────────────────────────────────────
    let tui_src = $"target/($target)/release/flashkraft-tui($suffix)"
    let tui_dst = $"($dist_dir)/flashkraft-tui-($target)($suffix)"

    if not ($tui_src | path exists) {
        print $"(ansi red)Error:(ansi reset) TUI binary not found at ($tui_src)"
        exit 1
    }

    cp $tui_src $tui_dst
    print $"(ansi green)✓(ansi reset) Staged TUI: ($tui_dst)"

    # ── 3. Stage the GUI binary (if requested) ───────────────────────────────
    if ($gui_binary | is-not-empty) {
        let gui_src = $"target/($target)/release/($gui_binary)($suffix)"
        let gui_dst = $"($dist_dir)/flashkraft-gui-($target)($suffix)"

        if not ($gui_src | path exists) {
            print $"(ansi red)Error:(ansi reset) GUI binary not found at ($gui_src)"
            exit 1
        }

        cp $gui_src $gui_dst
        print $"(ansi green)✓(ansi reset) Staged GUI: ($gui_dst)"
    }

    # ── 4. List staged artifacts ─────────────────────────────────────────────
    print ""
    print $"(ansi cyan)Staged artifacts in ($dist_dir)/:(ansi reset)"
    ls $dist_dir | select name size | print
}
