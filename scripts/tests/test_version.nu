#!/usr/bin/env nu
# Tests for scripts/version.nu
#
# Run with: nu scripts/tests/test_version.nu

use std/assert
use runner.nu *

# ── Helpers ───────────────────────────────────────────────────────────────────

# Extract the [workspace.package] version from a Cargo.toml string.
# Mirrors the exact logic used in bump_version.nu / release_prepare.nu.
def parse_workspace_version [cargo_toml: string] {
    $cargo_toml
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

# Write a minimal workspace Cargo.toml at the given version into a temp dir.
# Returns the dir path.
def make_workspace_cargo [version: string] {
    let tmp = (mktemp -d)
    let content = $'[workspace]
members = ["crates/flashkraft-core"]
resolver = "2"

[workspace.package]
version      = "($version)"
edition      = "2021"
authors      = ["Test User <test@example.com>"]
license      = "MIT"

[workspace.dependencies]
serde = "1.0"

flashkraft-core = { path = "crates/flashkraft-core", version = "($version)" }
'
    $content | save --force ($tmp | path join "Cargo.toml")
    $tmp
}

# ── Version parsing tests ─────────────────────────────────────────────────────

def "test version reads simple semver" [] {
    let tmp = make_workspace_cargo "1.2.3"
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "1.2.3"
}

def "test version reads zero patch" [] {
    let tmp = make_workspace_cargo "0.1.0"
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "0.1.0"
}

def "test version reads triple zero" [] {
    let tmp = make_workspace_cargo "0.0.0"
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "0.0.0"
}

def "test version reads pre-release suffix" [] {
    let tmp = make_workspace_cargo "0.5.0-beta.1"
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "0.5.0-beta.1"
}

def "test version reads padded assignment" [] {
    # After a bump the line becomes `version      = "x.y.z"` with extra spaces.
    let tmp = (mktemp -d)
    let content = '[workspace]
members = []
resolver = "2"

[workspace.package]
version      = "2.0.0"
edition      = "2021"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "2.0.0"
}

def "test version ignores dependency version lines" [] {
    # Lines like `serde = "1.0"` and inline `version = "..."` inside
    # [workspace.dependencies] must not be picked up.
    let tmp = (mktemp -d)
    let content = '[workspace]
members = []
resolver = "2"

[workspace.package]
version = "3.1.4"
edition = "2021"

[workspace.dependencies]
serde      = { version = "1.0" }
ratatui    = { version = "0.29" }
flashkraft-core = { path = "crates/flashkraft-core", version = "3.1.4" }
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "3.1.4"
}

def "test version does not bleed from other sections" [] {
    # A [package] block in a different crate section must not interfere.
    let tmp = (mktemp -d)
    let content = '[workspace]
members = ["crates/other"]
resolver = "2"

[workspace.package]
version = "0.8.5"
edition = "2021"

[some.other.section]
version = "99.0.0"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "0.8.5"
}

def "test version output contains no whitespace" [] {
    let tmp = make_workspace_cargo "0.9.1"
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert ($got !~ '\s') $"expected no whitespace, got: ($got)"
}

def "test version output contains no quotes" [] {
    let tmp = make_workspace_cargo "1.0.0"
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert ($got !~ '"') $"expected no quotes, got: ($got)"
}

def "test version returns empty for missing workspace.package" [] {
    let tmp = (mktemp -d)
    # No [workspace.package] block at all
    let content = '[workspace]
members = []
resolver = "2"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = parse_workspace_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert ($got | is-empty) "should return empty string when section is absent"
}

def "test version native toml read matches text parse" [] {
    # Verify that `open Cargo.toml | get workspace.package.version` and the
    # text-based parse yield the same result.
    let tmp = make_workspace_cargo "1.7.3"
    let path = ($tmp | path join "Cargo.toml")
    let via_toml = (open $path | get workspace.package.version)
    let via_text = (parse_workspace_version (open --raw $path))
    rm -rf $tmp
    assert equal $via_toml $via_text
}

# ── Runner ────────────────────────────────────────────────────────────────────

def main [] {
    print $"(ansi cyan)═══ test_version.nu ═══(ansi reset)"
    run-tests
}
