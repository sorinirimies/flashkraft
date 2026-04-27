#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# FlashKraft — Publish a single crate to crates.io (idempotent)
# ──────────────────────────────────────────────────────────────────────────────
# Publishes a crate only if the exact version is not already on crates.io.
# Copies the root README.md into the crate directory first so it ships with
# the package (crates.io requires a README per-crate).
#
# Usage:
#   nu scripts/ci/publish_crate.nu <crate> <version> [--readme-dir <dir>] [--wait <seconds>]
#
# Examples:
#   nu scripts/ci/publish_crate.nu flashkraft-core 1.0.3
#   nu scripts/ci/publish_crate.nu flashkraft      1.0.3 --readme-dir crates/flashkraft-gui --wait 20
#   nu scripts/ci/publish_crate.nu flashkraft-tui  1.0.3 --readme-dir crates/flashkraft-tui --wait 0
#
# Environment:
#   CARGO_REGISTRY_TOKEN — must be set for `cargo publish` to authenticate.
#
# Exit codes:
#   0  — published successfully or already published (idempotent)
#   1  — publish failed for an unexpected reason
# ──────────────────────────────────────────────────────────────────────────────

# ── Helpers ───────────────────────────────────────────────────────────────────

# Check whether a crate@version is already on crates.io.
export def is_already_published [crate: string, version: string]: nothing -> bool {
    let result = (do { cargo info $"($crate)@($version)" } | complete)
    if $result.exit_code != 0 {
        return false
    }
    # `cargo info` prints "crate-name@version …" when found
    $result.stdout | str contains $crate
}

# Copy README.md into the crate's directory so cargo publish ships it.
export def copy_readme [target_dir: string] {
    let readme_src = "README.md"
    if not ($readme_src | path exists) {
        print $"  (ansi yellow)⚠(ansi reset) README.md not found in workspace root — skipping copy."
        return
    }
    let readme_dst = ($target_dir | path join "README.md")
    cp $readme_src $readme_dst
    print $"  (ansi green)✓(ansi reset) Copied README.md → ($readme_dst)"
}

# Publish the crate, handling the "already exists" race gracefully.
export def do_publish [crate: string]: nothing -> bool {
    print $"  📦 Publishing ($crate)…"
    let result = (do { cargo publish -p $crate --allow-dirty } | complete)

    if $result.exit_code == 0 {
        print $"  (ansi green)✓(ansi reset) ($crate) published successfully."
        return true
    }

    # Check if the failure is just "already exists" (race with another run)
    let combined = $"($result.stdout)\n($result.stderr)"
    if ($combined | str contains "already exists") {
        print $"  (ansi yellow)⏭️(ansi reset)  ($crate) already exists on crates.io — skipping."
        return true
    }

    # Real failure
    print $"  (ansi red)✖(ansi reset) Failed to publish ($crate):"
    if ($result.stdout | str trim | is-not-empty) {
        print $result.stdout
    }
    if ($result.stderr | str trim | is-not-empty) {
        print $result.stderr
    }
    return false
}

# ── Main ──────────────────────────────────────────────────────────────────────

def main [
    crate: string,            # Crate name (e.g. flashkraft-core)
    version: string,          # Version to publish (e.g. 1.0.3)
    --readme-dir: string = "" # Directory to copy README.md into (default: crates/<crate>)
    --wait: int = 30          # Seconds to wait after publish for index propagation (0 to skip)
] {
    print ""
    print $"(ansi cyan)── publish ($crate)@($version) ──(ansi reset)"

    # 1. Check if already published
    if (is_already_published $crate $version) {
        print $"  (ansi yellow)⏭️(ansi reset)  ($crate)@($version) already on crates.io — skipping."
        return
    }

    # 2. Resolve the readme target directory
    let target_dir = if ($readme_dir | is-empty) {
        $"crates/($crate)"
    } else {
        $readme_dir
    }

    # 3. Copy README into crate directory
    copy_readme $target_dir

    # 4. Publish
    let ok = (do_publish $crate)
    if not $ok {
        print $"(ansi red)✖ Aborting: ($crate) publish failed.(ansi reset)"
        exit 1
    }

    # 5. Wait for crates.io index propagation
    if $wait > 0 {
        print $"  ⏳ Waiting ($wait)s for crates.io index propagation…"
        sleep ($"($wait)sec" | into duration)
    }

    print $"  (ansi green)✓(ansi reset) ($crate)@($version) done."
}
