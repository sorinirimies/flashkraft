#!/usr/bin/env nu
# Tests for scripts/release_prepare.nu
#
# Run with: nu scripts/tests/test_release_prepare.nu

use std/assert
use runner.nu *

# ── Helpers ───────────────────────────────────────────────────────────────────

# Write a minimal workspace Cargo.toml at the given version into a temp dir.
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

# Read back the [workspace.package] version from a Cargo.toml file path.
def read_workspace_version [cargo_path: string] {
    open --raw $cargo_path
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

# Apply the same Cargo.toml update logic that release_prepare.nu uses.
def apply_version_update [dir: string, new_version: string] {
    let cargo_path = ($dir | path join "Cargo.toml")
    let lines = open --raw $cargo_path | lines

    # Step 1: update [workspace.package] version
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

    # Step 2: update flashkraft-core dep version
    let final_lines = $after_wp.lines | each { |line|
        if ($line =~ '^flashkraft-core\s*=') {
            $line | str replace --regex 'version\s*=\s*"[^"]+"' $"version = \"($new_version)\""
        } else {
            $line
        }
    }

    $final_lines | str join "\n" | save --force $cargo_path
}

# Read the flashkraft-core dep version from a Cargo.toml file.
def read_core_dep_version [cargo_path: string] {
    open --raw $cargo_path
    | lines
    | where { |l| $l =~ '^flashkraft-core\s*=' }
    | first
    | parse --regex 'version\s*=\s*"(?P<v>[^"]+)"'
    | get v
    | first
}

# Build the release notes string the same way release_prepare.nu does.
def build_release_notes [version: string, cliff_changes: string, last_tag: string] {
    let changes_header = if ($last_tag | is-empty) {
        "### Initial Release"
    } else {
        $"### Changes since ($last_tag):"
    }

    [
        $"# FlashKraft ($version)"
        ""
        "## 🚀 What's New"
        ""
        $changes_header
        ""
        $cliff_changes
        ""
        "## 📦 Installation"
        ""
        "### Desktop GUI (Iced)"
        ""
        "```bash"
        "cargo install flashkraft"
        "```"
        ""
        "### Terminal UI (Ratatui)"
        ""
        "```bash"
        "cargo install flashkraft-tui"
        "```"
        ""
        "Or download a pre-built binary below."
        ""
        "## 🦀 Library Usage"
        ""
        "Add `flashkraft-core` to your `Cargo.toml`:"
        ""
        "```toml"
        "[dependencies]"
        $"flashkraft-core = \"($version)\""
        "```"
    ] | str join "\n"
}

# ── Tag stripping tests ────────────────────────────────────────────────────────

def "test tag v prefix is stripped" [] {
    let tag = "v0.9.0"
    let version = $tag | str replace --regex '^v' ''
    assert equal $version "0.9.0"
}

def "test tag without v prefix is unchanged" [] {
    let tag = "1.2.3"
    let version = $tag | str replace --regex '^v' ''
    assert equal $version "1.2.3"
}

def "test tag with pre-release suffix strips v only" [] {
    let tag = "v0.9.0-beta.1"
    let version = $tag | str replace --regex '^v' ''
    assert equal $version "0.9.0-beta.1"
}

def "test tag name round trips from version" [] {
    let version = "2.0.0"
    let tag = $"v($version)"
    let back = $tag | str replace --regex '^v' ''
    assert equal $back $version
}

def "test tag with rc suffix is handled" [] {
    let tag = "v1.0.0-rc.1"
    let version = $tag | str replace --regex '^v' ''
    assert equal $version "1.0.0-rc.1"
}

# ── Cargo.toml update tests ───────────────────────────────────────────────────

def "test cargo workspace version is updated" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_version_update $tmp "0.9.0"
    let got = read_workspace_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "0.9.0"
}

def "test cargo workspace version update is verified" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_version_update $tmp "1.0.0"
    let got = read_workspace_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "1.0.0" "verification should pass when update succeeded"
}

def "test cargo core dep version is updated" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_version_update $tmp "0.9.0"
    let got = read_core_dep_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "0.9.0"
}

def "test cargo workspace and core dep versions are in sync after update" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_version_update $tmp "0.9.0"
    let wp_ver   = read_workspace_version ($tmp | path join "Cargo.toml")
    let core_ver = read_core_dep_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $wp_ver $core_ver
}

def "test cargo non-version lines survive update" [] {
    let tmp = make_workspace_cargo "0.8.5"
    apply_version_update $tmp "0.9.0"
    let content = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert str contains $content "[workspace]"
    assert str contains $content "edition"
    assert str contains $content 'serde = "1.0"'
}

def "test cargo dependency version lines for serde are untouched" [] {
    let tmp = (mktemp -d)
    let content = '[workspace]
members = ["crates/flashkraft-core"]
resolver = "2"

[workspace.package]
version      = "1.0.0"
edition      = "2021"

[workspace.dependencies]
serde   = { version = "1.0" }
ratatui = { version = "0.29" }

flashkraft-core = { path = "crates/flashkraft-core", version = "1.0.0" }
'
    $content | save --force ($tmp | path join "Cargo.toml")
    apply_version_update $tmp "1.1.0"
    let updated = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert str contains $updated 'version      = "1.1.0"'
    assert str contains $updated 'serde   = { version = "1.0" }'
    assert str contains $updated 'ratatui = { version = "0.29" }'
}

# ── Last-tag detection logic tests ────────────────────────────────────────────

def "test empty last tag triggers initial release header" [] {
    let last_tag = ""
    let header = if ($last_tag | is-empty) {
        "### Initial Release"
    } else {
        $"### Changes since ($last_tag):"
    }
    assert equal $header "### Initial Release"
}

def "test non-empty last tag triggers changes-since header" [] {
    let last_tag = "v0.8.5"
    let header = if ($last_tag | is-empty) {
        "### Initial Release"
    } else {
        $"### Changes since ($last_tag):"
    }
    assert equal $header "### Changes since v0.8.5:"
}

def "test last tag is trimmed" [] {
    # git describe may include a trailing newline
    let raw = "v0.8.5\n"
    let trimmed = $raw | str trim
    assert equal $trimmed "v0.8.5"
}

def "test last tag trimmed empty is detected correctly" [] {
    let raw = "\n"
    let trimmed = $raw | str trim
    assert ($trimmed | is-empty) "trimmed empty string should be detected as empty"
}

# ── Release notes content tests ───────────────────────────────────────────────

def "test release notes contains version header" [] {
    let notes = build_release_notes "0.9.0" "- fix something" ""
    assert str contains $notes "# FlashKraft 0.9.0"
}

def "test release notes contains whats new section" [] {
    let notes = build_release_notes "0.9.0" "- fix something" ""
    assert str contains $notes "## 🚀 What's New"
}

def "test release notes initial release has correct header" [] {
    let notes = build_release_notes "0.1.0" "- initial" ""
    assert str contains $notes "### Initial Release"
    assert not ($notes | str contains "### Changes since")
}

def "test release notes with previous tag has changes-since header" [] {
    let notes = build_release_notes "0.9.0" "- add feature" "v0.8.5"
    assert str contains $notes "### Changes since v0.8.5:"
    assert not ($notes | str contains "### Initial Release")
}

def "test release notes contains cliff changes" [] {
    let cliff = "- feat: add cool feature\n- fix: patch a bug"
    let notes = build_release_notes "0.9.0" $cliff ""
    assert str contains $notes "feat: add cool feature"
    assert str contains $notes "fix: patch a bug"
}

def "test release notes contains installation section" [] {
    let notes = build_release_notes "0.9.0" "- changes" ""
    assert str contains $notes "## 📦 Installation"
}

def "test release notes contains gui install command" [] {
    let notes = build_release_notes "0.9.0" "- changes" ""
    assert str contains $notes "cargo install flashkraft"
}

def "test release notes contains tui install command" [] {
    let notes = build_release_notes "0.9.0" "- changes" ""
    assert str contains $notes "cargo install flashkraft-tui"
}

def "test release notes contains library usage section" [] {
    let notes = build_release_notes "0.9.0" "- changes" ""
    assert str contains $notes "## 🦀 Library Usage"
}

def "test release notes version appears in core cargo toml block" [] {
    let notes = build_release_notes "0.9.0" "- changes" ""
    assert str contains $notes "flashkraft-core = \"0.9.0\""
}

def "test release notes gui and tui sections both present" [] {
    let notes = build_release_notes "1.0.0" "- big release" ""
    assert str contains $notes "### Desktop GUI (Iced)"
    assert str contains $notes "### Terminal UI (Ratatui)"
}

def "test release notes is a single string" [] {
    let notes = build_release_notes "0.9.0" "- changes" ""
    assert ($notes | describe | str starts-with "string")
}

def "test release notes pre-release version is rendered correctly" [] {
    let notes = build_release_notes "1.0.0-rc.1" "- rc changes" "v0.9.9"
    assert str contains $notes "# FlashKraft 1.0.0-rc.1"
    assert str contains $notes "flashkraft-core = \"1.0.0-rc.1\""
}

# ── RELEASE_NOTES.md file write tests ─────────────────────────────────────────

def "test release notes is written to file" [] {
    let tmp = (mktemp -d)
    let notes = build_release_notes "0.9.0" "- initial release" ""
    $notes | save --force ($tmp | path join "RELEASE_NOTES.md")
    assert (($tmp | path join "RELEASE_NOTES.md") | path exists)
    rm -rf $tmp
}

def "test release notes file content matches notes" [] {
    let tmp = (mktemp -d)
    let notes = build_release_notes "0.9.0" "- big release" "v0.8.5"
    $notes | save --force ($tmp | path join "RELEASE_NOTES.md")
    let content = open --raw ($tmp | path join "RELEASE_NOTES.md")
    rm -rf $tmp
    assert str contains $content "# FlashKraft 0.9.0"
    assert str contains $content "### Changes since v0.8.5:"
}

def "test changelog is written to file" [] {
    let tmp = (mktemp -d)
    let changelog = "# Changelog\n\n## v0.9.0\n\n- feat: new stuff"
    $changelog | save --force ($tmp | path join "CHANGELOG.md")
    assert (($tmp | path join "CHANGELOG.md") | path exists)
    let content = open --raw ($tmp | path join "CHANGELOG.md")
    rm -rf $tmp
    assert str contains $content "v0.9.0"
}

# ── Runner ────────────────────────────────────────────────────────────────────

def main [] {
    print $"(ansi cyan)═══ test_release_prepare.nu ═══(ansi reset)"
    run-tests
}
