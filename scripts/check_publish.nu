#!/usr/bin/env nu
# Pre-publish readiness check for the FlashKraft workspace.
# Usage: nu scripts/check_publish.nu
# Run this before pushing a release tag to catch problems early.

def main [] {
    let green  = (ansi green)
    let red    = (ansi red)
    let yellow = (ansi yellow)
    let cyan   = (ansi cyan)
    let reset  = (ansi reset)

    print ""
    print $"($cyan)════════════════════════════════════════($reset)"
    print $"($cyan)  FlashKraft — Publish Readiness Check($reset)"
    print $"($cyan)════════════════════════════════════════($reset)"
    print ""

    mut errors = 0

    # ── 1. Formatting ─────────────────────────────────────────────────────────
    print $"($cyan)── Formatting ──($reset)"
    print -n "  cargo fmt --all -- --check ... "
    let fmt = (do { run-external "cargo" "fmt" "--all" "--" "--check" } | complete)
    if $fmt.exit_code == 0 {
        print $"($green)✓ code is formatted($reset)"
    } else {
        print $"($red)✗ formatting issues found  \(run: cargo fmt --all\)($reset)"
        $errors = $errors + 1
    }

    # ── 2. Clippy ─────────────────────────────────────────────────────────────
    print ""
    print $"($cyan)── Clippy ──($reset)"
    print -n "  cargo clippy --workspace ... "
    let clippy = (do {
        run-external "cargo" "clippy" "--workspace" "--all-targets" "--all-features"
            "--" "-D" "warnings" "-A" "deprecated"
    } | complete)
    if $clippy.exit_code == 0 {
        print $"($green)✓ no clippy warnings($reset)"
    } else {
        print $"($red)✗ clippy found issues  \(run: cargo clippy --workspace --all-targets --all-features -- -D warnings\)($reset)"
        $errors = $errors + 1
    }

    # ── 3. Tests ──────────────────────────────────────────────────────────────
    print ""
    print $"($cyan)── Tests ──($reset)"
    print -n "  cargo test --workspace ... "
    let tests = (do {
        run-external "cargo" "test" "--workspace" "--all-features" "--all-targets"
    } | complete)
    if $tests.exit_code == 0 {
        print $"($green)✓ all tests pass($reset)"
    } else {
        print $"($red)✗ test failures found  \(run: cargo test --workspace --all-features\)($reset)"
        $errors = $errors + 1
    }

    # ── 4. Documentation ──────────────────────────────────────────────────────
    print ""
    print $"($cyan)── Documentation ──($reset)"
    # core is internal (publish = false); build docs for the two public crates.
    for crate in ["flashkraft" "flashkraft-tui"] {
        print -n $"  cargo doc -p ($crate) ... "
        let doc = (do {
            run-external "cargo" "doc" "--no-deps" "-p" $crate "--all-features"
        } | complete)
        if $doc.exit_code == 0 {
            print $"($green)✓ ($crate)($reset)"
        } else {
            print $"($red)✗ ($crate)  \(run: cargo doc --no-deps -p ($crate)\)($reset)"
            $errors = $errors + 1
        }
    }

    # ── 5. Required files ─────────────────────────────────────────────────────
    print ""
    print $"($cyan)── Required files ──($reset)"
    let required = ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"]
    for f in $required {
        print -n $"  ($f) ... "
        if ($f | path exists) {
            print $"($green)✓ present($reset)"
        } else {
            print $"($red)✗ missing($reset)"
            $errors = $errors + 1
        }
    }

    # ── 6. Workspace version consistency ─────────────────────────────────────
    print ""
    print $"($cyan)── Workspace version consistency ──($reset)"

    let workspace_version = (open Cargo.toml | get workspace.package.version)

    if ($workspace_version | is-empty) {
        print $"($red)✗ could not read \[workspace.package\] version from Cargo.toml($reset)"
        $errors = $errors + 1
    } else {
        print $"  workspace version: ($yellow)($workspace_version)($reset) ... ($green)✓ found($reset)"
    }

    # Verify each crate uses version.workspace = true
    let crate_tomls = (glob "crates/*/Cargo.toml")
    for crate_toml in $crate_tomls {
        let crate_name = ($crate_toml | path dirname | path basename)
        print -n $"  ($crate_name) uses version.workspace ... "
        let content = (open --raw $crate_toml)
        if ($content =~ 'version\.workspace\s*=\s*true') {
            print $"($green)✓ yes($reset)"
        } else {
            print $"($yellow)⚠ ($crate_name) does not set version.workspace = true($reset)"
        }
    }

    # ── 7. Cargo.lock ─────────────────────────────────────────────────────────
    print ""
    print $"($cyan)── Cargo.lock ──($reset)"
    print -n "  Cargo.lock present ... "
    if ("Cargo.lock" | path exists) {
        print $"($green)✓ present($reset)"
    } else {
        print $"($red)✗ missing — run: cargo generate-lockfile($reset)"
        $errors = $errors + 1
    }

    # ── 8. Publish readiness ──────────────────────────────────────────────────
    # flashkraft-core: full publish dry-run — it has no workspace path deps, so
    #   cargo can resolve everything against the crates.io index.
    #
    # flashkraft-gui / flashkraft-tui: these depend on flashkraft-core via a path
    #   dep.  Both `cargo publish --dry-run` and `cargo package` require all
    #   transitive deps to be resolvable on crates.io — which is only true *after*
    #   core is published.  We verify them with `cargo check` instead; the full
    #   packaging is validated by the CI release workflow.
    print ""
    print $"($cyan)── Publish readiness ──($reset)"

    print -n "  cargo publish --dry-run --allow-dirty -p flashkraft-core ... "
    let dry_core = (do {
        run-external "cargo" "publish" "--dry-run" "--allow-dirty" "-p" "flashkraft-core"
    } | complete)
    if $dry_core.exit_code == 0 {
        print $"($green)✓ flashkraft-core($reset)"
    } else {
        print $"($red)✗ flashkraft-core  \(run: cargo publish --dry-run --allow-dirty -p flashkraft-core\)($reset)"
        $errors = $errors + 1
    }

    for crate in ["flashkraft" "flashkraft-tui"] {
        print -n $"  cargo check -p ($crate) \(packaging verified by CI\) ... "
        let chk = (do { run-external "cargo" "check" "-p" $crate } | complete)
        if $chk.exit_code == 0 {
            print $"($green)✓ ($crate)($reset)"
        } else {
            print $"($red)✗ ($crate)  \(run: cargo check -p ($crate)\)($reset)"
            $errors = $errors + 1
        }
    }

    # ── Summary ───────────────────────────────────────────────────────────────
    print ""
    print $"($cyan)════════════════════════════════════════($reset)"
    if $errors == 0 {
        print $"($green)✓ All checks passed — ready to release! 🚀($reset)"
        print ""
        print $"($cyan)Next step:($reset)"
        print "  just bump <version>   # e.g. just bump 0.5.0"
    } else {
        let plural = if $errors == 1 { "check" } else { "checks" }
        print $"($red)✗ ($errors) ($plural) failed — please fix before releasing.($reset)"
        exit 1
    }
}
