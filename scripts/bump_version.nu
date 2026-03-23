#!/usr/bin/env nu
# Automated version bump script for FlashKraft workspace
# Usage: nu scripts/bump_version.nu [--yes] <new_version>
# Example: nu scripts/bump_version.nu 0.9.0
#          nu scripts/bump_version.nu --yes 0.9.0   # skip confirmation
#
# Updates:
#   • [workspace.package] version in Cargo.toml
#   • flashkraft-core workspace dependency version in Cargo.toml
#   • Cargo.lock (cargo update --workspace)
#   • CHANGELOG.md (via git-cliff)
# Then commits and tags locally.  Use `just push-release-all` to push.

# ── Helpers ───────────────────────────────────────────────────────────────────

# Read the current [workspace.package] version from a Cargo.toml string.
def read_workspace_version [content: string] {
    $content
    | lines
    | reduce --fold { in_wp: false, version: "" } { |line, acc|
        let new_in_wp = if ($line =~ '^\[workspace\.package\]') {
            true
        } else if ($acc.in_wp and ($line =~ '^\[')) {
            false
        } else {
            $acc.in_wp
        }

        let new_version = if ($acc.in_wp and ($line =~ '^version\s*=\s*"[^"]*"')) {
            $line
            | parse --regex 'version\s*=\s*"(?P<v>[^"]+)"'
            | get v
            | first
        } else {
            $acc.version
        }

        { in_wp: $new_in_wp, version: $new_version }
    }
    | get version
}

# Replace the version line inside [workspace.package] only.
def update_workspace_package_version [lines: list<string>, new_version: string] {
    let result = $lines | reduce --fold { in_wp: false, lines: [] } { |line, acc|
        let new_in_wp = if ($line =~ '^\[workspace\.package\]') {
            true
        } else if ($acc.in_wp and ($line =~ '^\[')) {
            false
        } else {
            $acc.in_wp
        }

        let new_line = if ($acc.in_wp and ($line =~ '^version\s*=\s*"[^"]*"')) {
            $'version      = "($new_version)"'
        } else {
            $line
        }

        { in_wp: $new_in_wp, lines: ($acc.lines | append $new_line) }
    }
    $result.lines
}

# Update the version = "…" field on the flashkraft-core workspace dep line.
def update_core_dep_version [lines: list<string>, new_version: string] {
    $lines | each { |line|
        if ($line =~ '^flashkraft-core\s*=') {
            $line | str replace --regex 'version\s*=\s*"[^"]+"' $"version = \"($new_version)\""
        } else {
            $line
        }
    }
}

# ── Main ──────────────────────────────────────────────────────────────────────

def main [
    new_version: string,  # New version in X.Y.Z format
    --yes (-y),           # Skip confirmation prompt (non-interactive)
] {
    let red    = (ansi red)
    let green  = (ansi green)
    let yellow = (ansi yellow)
    let cyan   = (ansi cyan)
    let reset  = (ansi reset)

    # ── Validate version format ───────────────────────────────────────────────
    if not ($new_version =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$') {
        error make { msg: $"($red)Error: Invalid version format($reset)
Version must be in format: X.Y.Z or X.Y.Z-suffix \(e.g., 0.9.0 or 0.9.0-beta.1\)" }
    }

    print $"($cyan)════════════════════════════════════════($reset)"
    print $"($cyan)  FlashKraft Workspace Version Bump($reset)"
    print $"($cyan)════════════════════════════════════════($reset)"
    print ""

    # ── Read current version from [workspace.package] ─────────────────────────
    let cargo_content = (open Cargo.toml --raw)
    let cargo_lines   = ($cargo_content | lines)

    let current_version = (read_workspace_version $cargo_content)

    if ($current_version | is-empty) {
        error make { msg: $"($red)Error: Could not read [workspace.package] version from Cargo.toml($reset)" }
    }

    print $"Current version: ($yellow)($current_version)($reset)"
    print $"New version:     ($green)($new_version)($reset)"
    print ""

    # ── Guard: already at requested version ──────────────────────────────────
    if $current_version == $new_version {
        error make { msg: $"($red)Error: Cargo.toml is already at version ($new_version).($reset)
($yellow)  Bump to the next version, or delete the tag if you need to re-release:($reset)
      git tag -d v($new_version) && git push origin :refs/tags/v($new_version)" }
    }

    # ── Guard: tag already exists locally ────────────────────────────────────
    let tag_name = $"v($new_version)"
    let existing_tags = (git tag | lines)
    if ($existing_tags | any { |t| $t == $tag_name }) {
        error make { msg: $"($red)Error: Tag ($tag_name) already exists locally.($reset)
($yellow)  Delete it first if you really want to recreate it:($reset)
      git tag -d ($tag_name)" }
    }

    # ── Confirmation ─────────────────────────────────────────────────────────
    if $yes {
        print $"($cyan)Running non-interactively \(--yes passed\).($reset)"
    } else {
        let reply = (input "Continue with version bump? (y/n) ")
        if not ($reply =~ '^[Yy]') {
            print $"($yellow)Aborted($reset)"
            return
        }
    }

    print ""

    # ── Step 1: Update [workspace.package] version ────────────────────────────
    print $"($cyan)Step 1/8: Updating [workspace.package] version in Cargo.toml...($reset)"

    let after_wp = (update_workspace_package_version $cargo_lines $new_version)

    # ── Step 2: Update flashkraft-core workspace dep version ──────────────────
    print $"($cyan)Step 2/8: Updating flashkraft-core dependency version in Cargo.toml...($reset)"

    let final_lines = (update_core_dep_version $after_wp $new_version)
    $final_lines | str join "\n" | save --force Cargo.toml

    # Verify workspace.package version took effect
    let verify_version = (read_workspace_version (open Cargo.toml --raw))
    if $verify_version != $new_version {
        error make { msg: $"($red)Failed to update [workspace.package] version \(got ($verify_version)\).($reset)
($yellow)  Check the version line format in Cargo.toml and update manually.($reset)" }
    }
    print $"($green)✓ Cargo.toml updated \(($current_version) → ($new_version)\)($reset)"

    # Verify workspace crates inherit version via version.workspace = true
    let crate_tomls = (
        ls crates/*/Cargo.toml
        | get name
    )
    let missing_workspace = (
        $crate_tomls
        | where { |f|
            not (open --raw $f | str contains "version.workspace = true")
        }
    )
    if not ($missing_workspace | is-empty) {
        print ""
        for f in $missing_workspace {
            print $"($yellow)⚠ ($f) does not use version.workspace = true($reset)"
        }
        print $"($yellow)  These crates may not pick up the new version automatically.($reset)"
    }

    # ── Step 3: Update README.md badges ──────────────────────────────────────
    print ""
    print $"($cyan)Step 3/8: Updating README.md badges...($reset)"

    if ("README.md" | path exists) {
        let readme = (open README.md --raw)
        if ($readme =~ 'version-[0-9]+\.[0-9]+\.[0-9]+-blue') {
            let updated_readme = (
                $readme
                | str replace --all --regex 'version-[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?-blue' $"version-($new_version)-blue"
            )
            $updated_readme | save --force README.md
            print $"($green)✓ README.md badges updated($reset)"
        } else {
            print $"($yellow)⚠ No version badge found in README.md — skipping($reset)"
        }
    } else {
        print $"($yellow)⚠ README.md not found — skipping($reset)"
    }

    # ── Step 4: Update Cargo.lock ─────────────────────────────────────────────
    print ""
    print $"($cyan)Step 4/8: Updating Cargo.lock...($reset)"
    let lock_result = (do { run-external "cargo" "update" "--workspace" } | complete)
    if $lock_result.exit_code != 0 {
        # Fallback to generate-lockfile if update --workspace is not available
        run-external "cargo" "generate-lockfile"
    }
    print $"($green)✓ Cargo.lock updated($reset)"

    # ── Step 5: cargo fmt ─────────────────────────────────────────────────────
    print ""
    print $"($cyan)Step 5/8: Running cargo fmt...($reset)"
    run-external "cargo" "fmt" "--all"
    print $"($green)✓ Code formatted($reset)"

    # ── Step 6: cargo clippy ──────────────────────────────────────────────────
    print ""
    print $"($cyan)Step 6/8: Running cargo clippy...($reset)"
    let clippy = (do {
        run-external "cargo" "clippy" "--workspace" "--all-targets" "--all-features" "--" "-D" "warnings" "-A" "deprecated"
    } | complete)
    if $clippy.exit_code != 0 {
        error make { msg: $"($red)✗ Clippy found issues. Please fix them before continuing.($reset)" }
    }
    print $"($green)✓ Clippy passed($reset)"

    # ── Step 7: cargo test ────────────────────────────────────────────────────
    print ""
    print $"($cyan)Step 7/8: Running tests...($reset)"
    let tests = (do {
        run-external "cargo" "test" "--workspace" "--all-features" "--all-targets"
    } | complete)
    if $tests.exit_code != 0 {
        error make { msg: $"($red)✗ Tests failed. Please fix them before continuing.($reset)" }
    }
    print $"($green)✓ All tests passed($reset)"

    # ── Step 8: Generate CHANGELOG.md + commit + tag ──────────────────────────
    print ""
    print $"($cyan)Step 8/8: Generating CHANGELOG.md and creating git commit + tag...($reset)"

    if (which git-cliff | length) > 0 {
        run-external "git-cliff" "--tag" $tag_name "-o" "CHANGELOG.md"
        print $"($green)✓ CHANGELOG.md generated($reset)"
    } else {
        print $"($yellow)⚠ git-cliff not found — skipping changelog generation($reset)"
        print $"($yellow)  Install it with: cargo install git-cliff($reset)"
    }

    # Stage changed files
    let diff = (do {
        run-external "git" "diff" "--quiet" "Cargo.toml" "Cargo.lock" "README.md" "CHANGELOG.md"
    } | complete)

    if $diff.exit_code == 0 {
        print $"($yellow)⚠ No changes to commit($reset)"
    } else {
        run-external "git" "add" "Cargo.toml" "Cargo.lock" "README.md" "CHANGELOG.md"
        let commit_msg = $"chore: bump version to ($new_version)

- Update [workspace.package] version in Cargo.toml to ($new_version)
- Update flashkraft-core workspace dependency version
- All crates inherit version via version.workspace = true
- Regenerate Cargo.lock
- Generate updated CHANGELOG.md"
        run-external "git" "commit" "-m" $commit_msg
        print $"($green)✓ Changes committed($reset)"
    }

    let tag_msg = $"Release ($tag_name)

Includes all changes documented in CHANGELOG.md for version ($new_version)."
    run-external "git" "tag" "-a" $tag_name "-m" $tag_msg
    print $"($green)✓ Tag ($tag_name) created($reset)"

    # ── Summary ───────────────────────────────────────────────────────────────
    print ""
    print $"($cyan)════════════════════════════════════════($reset)"
    print $"($green)✓ Version bump complete! 🚀($reset)"
    print $"($cyan)════════════════════════════════════════($reset)"
    print ""
    print $"($yellow)Next steps:($reset)"
    print  "  1. Review the changes:"
    print $"     ($cyan)git show($reset)"
    print  ""
    print  "  2. Push to GitHub (triggers the release workflow):"
    print $"     ($cyan)git push --follow-tags origin main($reset)"
    print  ""
    print  "  3. Push to Gitea as well:"
    print $"     ($cyan)git push --follow-tags gitea main($reset)"
    print  ""
    print  "  4. Or use the just shortcuts:"
    print $"     ($cyan)just push-release-all($reset)   # push branch + tags to all remotes"
    print  ""
    print  "  5. The Release workflow publishes to crates.io automatically"
    print  "     once CRATES_IO_TOKEN is set in repository secrets."
    print ""
}
