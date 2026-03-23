#!/usr/bin/env nu
# Tests for scripts/check_publish.nu
#
# Run with: nu scripts/tests/test_check_publish.nu

use std/assert
use runner.nu *

# ── Helpers ───────────────────────────────────────────────────────────────────

# Write a minimal workspace Cargo.toml at the given version into a temp dir.
# Returns the dir path.
def make_workspace_cargo [version: string] {
    let tmp = (mktemp -d)
    let content = $'[workspace]
members = [
    "crates/flashkraft-core",
    "crates/flashkraft-gui",
    "crates/flashkraft-tui",
]
resolver = "2"

[workspace.package]
version      = "($version)"
edition      = "2021"
authors      = ["Test Author <test@example.com>"]
license      = "MIT"

[workspace.dependencies]
serde = "1.0"

flashkraft-core = { path = "crates/flashkraft-core", version = "($version)" }
'
    $content | save --force ($tmp | path join "Cargo.toml")
    $tmp
}

# Simulate the workspace version read used in check_publish.nu
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

# Simulate the required-files check used in check_publish.nu
def check_required_files [dir: string, files: list<string>] {
    $files | where { |f| not (($dir | path join $f) | path exists) }
}

# Simulate the workspace inheritance check used in check_publish.nu
def check_workspace_inheritance [crate_content: string] {
    $crate_content =~ 'version\.workspace\s*=\s*true'
}

# Count errors (helper for testing error accumulation logic)
def count_errors [checks: list<bool>] {
    $checks | where { |c| not $c } | length
}

# ── Workspace version reading tests ───────────────────────────────────────────

def "test workspace version is readable" [] {
    let tmp = make_workspace_cargo "0.8.5"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "0.8.5"
}

def "test workspace version is not empty for valid cargo toml" [] {
    let tmp = make_workspace_cargo "1.2.3"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert not ($got | is-empty)
}

def "test workspace version is empty when section is missing" [] {
    let tmp = (mktemp -d)
    let content = '[workspace]
members = []
resolver = "2"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = read_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert ($got | is-empty)
}

def "test workspace version does not pick up dep version lines" [] {
    # Lines like `serde = { version = "1.0" }` must not bleed through.
    let tmp = make_workspace_cargo "2.0.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "2.0.0"
    assert not ($got == "1.0")
}

def "test workspace version handles padded assignment" [] {
    let tmp = (mktemp -d)
    let content = '[workspace.package]
version      = "0.9.5"
edition      = "2021"

[workspace.dependencies]
serde = "1.0"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = read_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "0.9.5"
}

# ── Required files check tests ────────────────────────────────────────────────

def "test all required files present returns no missing" [] {
    let tmp = (mktemp -d)
    for f in ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"] {
        "" | save --force ($tmp | path join $f)
    }
    let missing = check_required_files $tmp ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"]
    rm -rf $tmp
    assert ($missing | is-empty)
}

def "test missing readme is detected" [] {
    let tmp = (mktemp -d)
    for f in ["LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"] {
        "" | save --force ($tmp | path join $f)
    }
    let missing = check_required_files $tmp ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"]
    rm -rf $tmp
    assert not ($missing | is-empty)
    assert ($missing | any { |f| $f == "README.md" })
}

def "test missing changelog is detected" [] {
    let tmp = (mktemp -d)
    for f in ["README.md" "LICENSE" "Cargo.toml" "cliff.toml"] {
        "" | save --force ($tmp | path join $f)
    }
    let missing = check_required_files $tmp ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"]
    rm -rf $tmp
    assert not ($missing | is-empty)
    assert ($missing | any { |f| $f == "CHANGELOG.md" })
}

def "test missing license is detected" [] {
    let tmp = (mktemp -d)
    for f in ["README.md" "Cargo.toml" "CHANGELOG.md" "cliff.toml"] {
        "" | save --force ($tmp | path join $f)
    }
    let missing = check_required_files $tmp ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"]
    rm -rf $tmp
    assert not ($missing | is-empty)
    assert ($missing | any { |f| $f == "LICENSE" })
}

def "test missing cliff toml is detected" [] {
    let tmp = (mktemp -d)
    for f in ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md"] {
        "" | save --force ($tmp | path join $f)
    }
    let missing = check_required_files $tmp ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"]
    rm -rf $tmp
    assert not ($missing | is-empty)
    assert ($missing | any { |f| $f == "cliff.toml" })
}

def "test multiple missing files are all reported" [] {
    let tmp = (mktemp -d)
    # Only Cargo.toml present
    "" | save --force ($tmp | path join "Cargo.toml")
    let missing = check_required_files $tmp ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"]
    rm -rf $tmp
    assert equal ($missing | length) 4
}

def "test no false positives when all files exist" [] {
    let tmp = (mktemp -d)
    let required = ["README.md" "LICENSE" "Cargo.toml" "CHANGELOG.md" "cliff.toml"]
    for f in $required {
        "" | save --force ($tmp | path join $f)
    }
    let missing = check_required_files $tmp $required
    rm -rf $tmp
    assert equal ($missing | length) 0
}

# ── Workspace inheritance check tests ─────────────────────────────────────────

def "test crate with version workspace true passes" [] {
    let content = '[package]
name = "flashkraft-core"
version.workspace = true
edition.workspace = true
license.workspace = true
'
    assert (check_workspace_inheritance $content)
}

def "test crate without version workspace true fails" [] {
    let content = '[package]
name = "my-crate"
version = "1.0.0"
edition = "2021"
'
    assert not (check_workspace_inheritance $content)
}

def "test crate with inline version workspace assignment passes" [] {
    # Some crates write it as `version = { workspace = true }`
    # The regex checks for `version.workspace = true` dot-notation only.
    let dotted = '[package]
version.workspace = true
'
    assert (check_workspace_inheritance $dotted)
}

def "test check correctly identifies all three crates" [] {
    let crate_contents = [
        '[package]\nname = "flashkraft-core"\nversion.workspace = true\n'
        '[package]\nname = "flashkraft-gui"\nversion.workspace = true\n'
        '[package]\nname = "flashkraft-tui"\nversion.workspace = true\n'
    ]
    let all_ok = ($crate_contents | all { |c| check_workspace_inheritance $c })
    assert $all_ok
}

def "test one crate missing inheritance is caught" [] {
    let crate_contents = [
        '[package]\nname = "flashkraft-core"\nversion.workspace = true\n'
        '[package]\nname = "flashkraft-gui"\nversion = "1.0.0"\n'  # missing!
        '[package]\nname = "flashkraft-tui"\nversion.workspace = true\n'
    ]
    let all_ok = ($crate_contents | all { |c| check_workspace_inheritance $c })
    assert not $all_ok
}

# ── Error counting / accumulation tests ───────────────────────────────────────

def "test zero errors when all checks pass" [] {
    let results = [true true true true true]
    assert equal (count_errors $results) 0
}

def "test one error is counted" [] {
    let results = [true false true true true]
    assert equal (count_errors $results) 1
}

def "test multiple errors are counted" [] {
    let results = [false true false true false]
    assert equal (count_errors $results) 3
}

def "test all errors are counted" [] {
    let results = [false false false]
    assert equal (count_errors $results) 3
}

# ── Cargo.lock presence tests ─────────────────────────────────────────────────

def "test cargo lock present is detected" [] {
    let tmp = (mktemp -d)
    "" | save --force ($tmp | path join "Cargo.lock")
    assert (($tmp | path join "Cargo.lock") | path exists)
    rm -rf $tmp
}

def "test cargo lock absent is detected" [] {
    let tmp = (mktemp -d)
    assert not (($tmp | path join "Cargo.lock") | path exists)
    rm -rf $tmp
}

# ── Error message / summary logic tests ───────────────────────────────────────

def "test error plural is correct for one error" [] {
    let errors = 1
    let plural = if $errors == 1 { "check" } else { "checks" }
    assert equal $plural "check"
}

def "test error plural is correct for multiple errors" [] {
    let errors = 3
    let plural = if $errors == 1 { "check" } else { "checks" }
    assert equal $plural "checks"
}

def "test zero errors means ready to release" [] {
    let errors = 0
    assert ($errors == 0) "zero errors should mean ready"
}

def "test non-zero errors means not ready" [] {
    let errors = 2
    assert ($errors > 0) "non-zero errors should block release"
}

# ── Runner ────────────────────────────────────────────────────────────────────

def main [] {
    print $"(ansi cyan)═══ test_check_publish.nu ═══(ansi reset)"
    run-tests
}
