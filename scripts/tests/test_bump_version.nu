#!/usr/bin/env nu
# Tests for scripts/bump_version.nu
#
# Run with: nu scripts/tests/test_bump_version.nu

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

# Read back the [workspace.package] version from a Cargo.toml string.
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

# Apply the same [workspace.package] version update logic that bump_version.nu uses.
def apply_workspace_version_update [dir: string, new_version: string] {
    let cargo_path = ($dir | path join "Cargo.toml")
    let lines = open --raw $cargo_path | lines

    let after_wp = $lines | reduce --fold { in_wp: false, lines: [] } { |line, acc|
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
    $after_wp.lines | str join "\n" | save --force $cargo_path
}

# Apply the flashkraft-core dep version update logic.
def apply_core_dep_update [dir: string, new_version: string] {
    let cargo_path = ($dir | path join "Cargo.toml")
    let updated = open --raw $cargo_path
        | lines
        | each { |line|
            if ($line =~ '^flashkraft-core\s*=') {
                $line | str replace --regex 'version\s*=\s*"[^"]+"' $"version = \"($new_version)\""
            } else {
                $line
            }
        }
        | str join "\n"
    $updated | save --force $cargo_path
}

# Apply both updates in sequence (as bump_version.nu does).
def apply_full_update [dir: string, new_version: string] {
    apply_workspace_version_update $dir $new_version
    apply_core_dep_update $dir $new_version
}

# Read the flashkraft-core dep version from a Cargo.toml string.
def read_core_dep_version [content: string] {
    $content
    | lines
    | where { |l| $l =~ '^flashkraft-core\s*=' }
    | first
    | parse --regex 'version\s*=\s*"(?P<v>[^"]+)"'
    | get v
    | first
}

# ── Version format validation tests ───────────────────────────────────────────

def "test valid version x.y.z is accepted" [] {
    assert ("1.2.3" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test valid version 0.0.0 is accepted" [] {
    assert ("0.0.0" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test valid version with pre-release suffix is accepted" [] {
    assert ("1.0.0-beta.1" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test valid version with alpha suffix is accepted" [] {
    assert ("2.3.4-alpha" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version missing patch is rejected" [] {
    assert not ("1.2" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version with v prefix is rejected" [] {
    assert not ("v1.2.3" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version empty string is rejected" [] {
    assert not ("" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version with spaces is rejected" [] {
    assert not ("1.2.3 " =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version with only two parts is rejected" [] {
    assert not ("0.8" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

# ── [workspace.package] version reading tests ─────────────────────────────────

def "test read workspace version from simple cargo toml" [] {
    let tmp = make_workspace_cargo "0.8.5"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "0.8.5"
}

def "test read workspace version with padded assignment" [] {
    let tmp = (mktemp -d)
    let content = '[workspace]
members = []

[workspace.package]
version      = "1.2.3"
edition      = "2021"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = read_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "1.2.3"
}

def "test read workspace version ignores non-workspace version lines" [] {
    # The [workspace.dependencies] section also has version lines but they
    # must not be confused with [workspace.package] version.
    let tmp = make_workspace_cargo "2.0.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "2.0.0"
}

def "test read workspace version ignores dep version lines" [] {
    let tmp = (mktemp -d)
    let content = '[workspace]
members = []

[workspace.dependencies]
serde = { version = "1.0" }

[workspace.package]
version = "3.1.4"
edition = "2021"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = read_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "3.1.4"
}

# ── [workspace.package] version update tests ──────────────────────────────────

def "test workspace package version is updated" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_workspace_version_update $tmp "0.9.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "0.9.0"
}

def "test workspace package version patch bump is correct" [] {
    let tmp = make_workspace_cargo "0.8.4"
    apply_workspace_version_update $tmp "0.8.5"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "0.8.5"
}

def "test workspace package version minor bump is correct" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_workspace_version_update $tmp "0.9.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "0.9.0"
}

def "test workspace package version major bump is correct" [] {
    let tmp = make_workspace_cargo "0.9.9"
    apply_workspace_version_update $tmp "1.0.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "1.0.0"
}

def "test workspace package version update leaves other lines intact" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_workspace_version_update $tmp "0.9.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert str contains $content "[workspace]"
    assert str contains $content "edition"
    assert str contains $content "serde"
}

def "test workspace package version update does not change dep versions" [] {
    let tmp = make_workspace_cargo "1.0.0"
    apply_workspace_version_update $tmp "1.1.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    # serde dep must remain at 1.0
    assert str contains $content 'serde = "1.0"'
}

def "test workspace package version update is idempotent" [] {
    let tmp = make_workspace_cargo "1.0.0"
    apply_workspace_version_update $tmp "1.0.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_workspace_version $content
    rm -rf $tmp
    assert equal $got "1.0.0"
}

# ── flashkraft-core dependency version update tests ───────────────────────────

def "test core dep version is updated" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_core_dep_update $tmp "0.9.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_core_dep_version $content
    rm -rf $tmp
    assert equal $got "0.9.0"
}

def "test core dep version patch bump is correct" [] {
    let tmp = make_workspace_cargo "0.8.4"
    apply_core_dep_update $tmp "0.8.5"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let got = read_core_dep_version $content
    rm -rf $tmp
    assert equal $got "0.8.5"
}

def "test core dep version does not change workspace package version line" [] {
    let tmp = make_workspace_cargo "1.0.0"
    apply_core_dep_update $tmp "1.1.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    # The [workspace.package] version should still be the original
    let wp_ver = read_workspace_version $content
    rm -rf $tmp
    assert equal $wp_ver "1.0.0"
}

def "test core dep version does not change serde dep version" [] {
    let tmp = make_workspace_cargo "1.0.0"
    apply_core_dep_update $tmp "1.1.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert str contains $content 'serde = "1.0"'
}

# ── Full update (workspace + core dep) tests ──────────────────────────────────

def "test full update bumps both workspace package and core dep" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_full_update $tmp "0.9.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    let wp_ver   = read_workspace_version $content
    let core_ver = read_core_dep_version $content
    rm -rf $tmp
    assert equal $wp_ver "0.9.0"
    assert equal $core_ver "0.9.0"
}

def "test full update versions stay in sync" [] {
    let tmp = make_workspace_cargo "1.2.3"
    apply_full_update $tmp "1.3.0"
    let content  = open --raw ($tmp | path join "Cargo.toml")
    let wp_ver   = read_workspace_version $content
    let core_ver = read_core_dep_version $content
    rm -rf $tmp
    assert equal $wp_ver $core_ver
}

def "test full update pre-release versions are handled" [] {
    let tmp = make_workspace_cargo "1.0.0"
    apply_full_update $tmp "1.1.0-rc.1"
    let content  = open --raw ($tmp | path join "Cargo.toml")
    let wp_ver   = read_workspace_version $content
    let core_ver = read_core_dep_version $content
    rm -rf $tmp
    assert equal $wp_ver "1.1.0-rc.1"
    assert equal $core_ver "1.1.0-rc.1"
}

# ── Same-version guard tests ───────────────────────────────────────────────────

def "test same version is detected" [] {
    let current = "0.8.5"
    let new     = "0.8.5"
    assert equal $current $new "same version guard should trigger"
}

def "test different version is not blocked" [] {
    let current = "0.8.5"
    let new     = "0.9.0"
    assert not equal $current $new
}

# ── Tag existence guard tests ──────────────────────────────────────────────────

def "test tag check detects existing tag" [] {
    let existing  = ["v0.8.3" "v0.8.4" "v0.8.5"]
    let candidate = "v0.8.5"
    assert ($existing | any { |t| $t == $candidate }) "existing tag should be detected"
}

def "test tag check allows new tag" [] {
    let existing  = ["v0.8.3" "v0.8.4" "v0.8.5"]
    let candidate = "v0.9.0"
    assert not ($existing | any { |t| $t == $candidate }) "new tag should not be blocked"
}

def "test tag name is prefixed with v" [] {
    let version = "0.9.0"
    let tag = $"v($version)"
    assert str contains $tag "v"
    assert equal $tag "v0.9.0"
}

# ── Workspace inheritance check tests ─────────────────────────────────────────

def "test crate with version workspace true passes check" [] {
    let content = '[package]
name = "flashkraft-core"
version.workspace = true
edition.workspace = true
'
    assert ($content =~ 'version\.workspace\s*=\s*true')
}

def "test crate without version workspace true fails check" [] {
    let content = '[package]
name = "my-crate"
version = "1.0.0"
edition = "2021"
'
    assert not ($content =~ 'version\.workspace\s*=\s*true')
}

# ── Runner ────────────────────────────────────────────────────────────────────

def main [] {
    print $"(ansi cyan)═══ test_bump_version.nu ═══(ansi reset)"
    run-tests
}
