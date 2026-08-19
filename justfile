# flashkraft workspace — task runner
# Install just:      cargo install just
# Install git-cliff: cargo install git-cliff
# Install vhs:       brew install vhs  OR  go install github.com/charmbracelet/vhs@latest
# Usage: just <task>
# ── Default ───────────────────────────────────────────────────────────────────

default:
    @just --list

# ── Tool checks ───────────────────────────────────────────────────────────────

_check-git-cliff:
    @command -v git-cliff >/dev/null 2>&1 || { \
        echo "❌ git-cliff not found. Install with: cargo install git-cliff"; exit 1; \
    }

# Check nu (nushell) is available
_check-nu:
    @command -v nu >/dev/null 2>&1 || { \
        echo "❌ nu (nushell) not found. Install: https://www.nushell.sh"; exit 1; \
    }

_check-vhs:
    @command -v vhs >/dev/null 2>&1 || { \
        echo "❌ vhs not found."; \
        echo "   macOS:      brew install vhs"; \
        echo "   Any:        go install github.com/charmbracelet/vhs@latest"; \
        exit 1; \
    }

# Install all recommended development tools
install-tools:
    @echo "Installing development tools…"
    @command -v git-cliff >/dev/null 2>&1 || cargo install git-cliff
    @command -v nu >/dev/null 2>&1 && echo "✅ nu found" || echo "⚠ nu (nushell) not found. Install: https://www.nushell.sh"
    @echo "✅ All tools installed!"

# ── System install / uninstall ────────────────────────────────────────────────

INSTALL_BIN := "/usr/bin/flashkraft"
INSTALL_BIN_TUI := "/usr/bin/flashkraft-tui"

# Build a release binary and install it with the setuid-root bit.
#
# The setuid bit lets the flash pipeline call seteuid(0) for the single
# instant needed to open a raw block device, then immediately drops back
# to the real user — no pkexec, no polkit policy file required.
#
# Usage:  just install          (installs GUI binary)

# just install tui      (installs TUI binary)
install target="gui":
    #!/usr/bin/env sh
    set -e

    if [ "{{ target }}" = "tui" ]; then
        CRATE="flashkraft-tui"
        BIN_SRC="target/release/flashkraft-tui"
        BIN_DEST="{{ INSTALL_BIN_TUI }}"
    else
        CRATE="flashkraft"
        BIN_SRC="target/release/flashkraft"
        BIN_DEST="{{ INSTALL_BIN }}"
    fi

    echo "Building release binary for $CRATE…"
    cargo build --release -p "$CRATE"

    echo "Installing $BIN_SRC → $BIN_DEST (requires sudo)…"
    sudo install -m 755 "$BIN_SRC" "$BIN_DEST"

    # Set the setuid-root bit so the flash pipeline can open block devices.
    echo "Setting setuid-root bit on $BIN_DEST…"
    sudo chown root:root "$BIN_DEST"
    sudo chmod u+s       "$BIN_DEST"

    echo "✅ Installed $BIN_DEST (setuid-root)"

# Remove the installed binary (GUI and/or TUI).
uninstall:
    #!/usr/bin/env sh
    set -e
    echo "Removing {{ INSTALL_BIN }} …"
    sudo rm -f "{{ INSTALL_BIN }}"
    echo "Removing {{ INSTALL_BIN_TUI }} …"
    sudo rm -f "{{ INSTALL_BIN_TUI }}"
    echo "✅ Uninstalled."

# ── Build ─────────────────────────────────────────────────────────────────────

# Build the entire workspace (dev)
build:
    cargo build --workspace

# Build only the core library (dev)
build-core:
    cargo build -p flashkraft-core

# Build only the GUI crate (dev)
build-gui:
    cargo build -p flashkraft

# Build only the TUI crate (dev)
build-tui:
    cargo build -p flashkraft-tui

# Build release binaries for GUI and TUI
build-release:
    cargo build --release -p flashkraft
    cargo build --release -p flashkraft-tui

# Build a static (musl) TUI binary — great for portable distribution
build-tui-musl:
    @rustup target add x86_64-unknown-linux-musl 2>/dev/null || true
    cargo build --release -p flashkraft-tui --target x86_64-unknown-linux-musl
    @echo "✅ Static TUI binary: target/x86_64-unknown-linux-musl/release/flashkraft-tui"

# ── Run ───────────────────────────────────────────────────────────────────────

# Launch the Iced desktop GUI
run-gui:
    cargo run -p flashkraft

# Launch the Ratatui terminal UI
run-tui:
    cargo run -p flashkraft-tui

# Alias: default run launches the TUI (headless-friendly)
run: run-tui

# ── Test ──────────────────────────────────────────────────────────────────────

# Run the full workspace test suite
test:
    cargo test --workspace --locked --all-features --all-targets

# Test only the core library
test-core:
    cargo test -p flashkraft-core --all-features

# Test only the GUI crate
test-gui:
    cargo test -p flashkraft --all-features

# Test only the TUI crate
test-tui:
    cargo test -p flashkraft-tui --all-features

# Run Nu script tests
test-nu: _check-nu
    nu scripts/tests/run_all.nu

# Run both Rust and Nu tests
test-all-nu: test test-nu
    @echo "✅ All Rust and Nu tests passed!"

# ── Code quality ──────────────────────────────────────────────────────────────

# Check without building
check:
    cargo check --workspace

# Format all code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run clippy across the workspace
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated

# Run all quality checks (fmt, clippy, test) — must pass before a release
check-all: fmt-check clippy test
    @echo "✅ All checks passed!"

# ── Examples ──────────────────────────────────────────────────────────────────

# Run the basic_usage GUI example
example-basic:
    cargo run -p flashkraft --example basic_usage

# Run the custom_theme GUI example
example-theme:
    cargo run -p flashkraft --example custom_theme

# Run the fully functional TUI application example
example-tui:
    cargo run -p flashkraft-tui --example tui_demo

# Run the headless (no TTY) TUI state-machine demo
example-tui-headless:
    cargo run -p flashkraft-tui --example headless_demo

# Run the flash progress demo (animated tui-slider without real hardware)
example-flash-progress:
    cargo run -p flashkraft-tui --example flash_progress_demo

# Run the file-explorer theme switcher demo
example-theme-demo:
    cargo run -p flashkraft-tui --example theme_demo

# Run the detect_drives core example
example-drives:
    cargo run -p flashkraft-core --example detect_drives

# Run the constraints_demo core example
example-constraints:
    cargo run -p flashkraft-core --example constraints_demo

# Run the flash_writer_demo core example
example-flash-writer:
    cargo run -p flashkraft-core --example flash_writer_demo

# ── VHS Demo GIFs ─────────────────────────────────────────────────────────────

GUI_VHS := "crates/flashkraft-gui/examples/vhs"
TUI_VHS := "crates/flashkraft-tui/examples/vhs"
GUI_VHS_GENERATED := "crates/flashkraft-gui/examples/vhs/generated"
TUI_VHS_GENERATED := "crates/flashkraft-tui/examples/vhs/generated"

# Generate all VHS demo GIFs (GUI + TUI)
vhs-all: vhs-gui vhs-tui

# Generate only the GUI demo GIFs (crates/flashkraft-gui/examples/vhs/generated/)
vhs-gui: _check-vhs
    @mkdir -p {{ GUI_VHS_GENERATED }}
    @echo "╔════════════════════════════════════════════╗"
    @echo "║   GUI Tapes (Iced desktop)                ║"
    @echo "╚════════════════════════════════════════════╝"
    @for tape in {{ GUI_VHS }}/*.tape; do \
        echo "▶  $tape"; \
        vhs "$tape" || echo "❌ Failed: $tape"; \
    done
    @echo "✅ GUI demos done → {{ GUI_VHS_GENERATED }}/"

# Generate only the TUI demo GIFs (crates/flashkraft-tui/examples/vhs/generated/)
vhs-tui: _check-vhs
    @mkdir -p {{ TUI_VHS_GENERATED }}
    @echo "╔════════════════════════════════════════════╗"
    @echo "║   TUI Tapes (Ratatui terminal)            ║"
    @echo "╚════════════════════════════════════════════╝"
    @for tape in {{ TUI_VHS }}/*.tape; do \
        echo "▶  $tape"; \
        vhs "$tape" || echo "❌ Failed: $tape"; \
    done
    @echo "✅ TUI demos done → {{ TUI_VHS_GENERATED }}/"

# Render a single tape by name, e.g.: just vhs-tape tui-demo-workflow
vhs-tape name: _check-vhs
    @if [ -f "{{ GUI_VHS }}/{{ name }}.tape" ]; then \
        echo "▶  {{ GUI_VHS }}/{{ name }}.tape"; \
        vhs "{{ GUI_VHS }}/{{ name }}.tape" && echo "✅ Done."; \
    elif [ -f "{{ TUI_VHS }}/{{ name }}.tape" ]; then \
        echo "▶  {{ TUI_VHS }}/{{ name }}.tape"; \
        vhs "{{ TUI_VHS }}/{{ name }}.tape" && echo "✅ Done."; \
    else \
        echo "❌ Tape not found: {{ name }}.tape"; \
        echo ""; \
        just vhs-list; \
        exit 1; \
    fi

# List all available VHS tapes and any already-generated GIFs
vhs-list:
    @echo "GUI tapes  →  {{ GUI_VHS }}/"
    @ls {{ GUI_VHS }}/*.tape | sed 's|.*/||; s|\.tape||' | sed 's/^/  /'
    @echo "GUI generated  →  {{ GUI_VHS_GENERATED }}/"
    @ls {{ GUI_VHS_GENERATED }}/*.gif 2>/dev/null | sed 's|.*/||' | sed 's/^/  /' || echo "  (none yet)"
    @echo ""
    @echo "TUI tapes  →  {{ TUI_VHS }}/"
    @ls {{ TUI_VHS }}/*.tape | sed 's|.*/||; s|\.tape||' | sed 's/^/  /'
    @echo "TUI generated  →  {{ TUI_VHS_GENERATED }}/"
    @ls {{ TUI_VHS_GENERATED }}/*.gif 2>/dev/null | sed 's|.*/||' | sed 's/^/  /' || echo "  (none yet)"

# Pull GIF files from Git LFS (run once after a fresh clone)
lfs-pull:
    @command -v git-lfs >/dev/null 2>&1 || { \
        echo "❌ git-lfs not found. Install with: brew install git-lfs"; exit 1; \
    }
    git lfs pull
    @echo "✅ LFS objects pulled."

# ── Documentation ─────────────────────────────────────────────────────────────

# Generate and open docs for the GUI crate
doc-gui:
    cargo doc --no-deps -p flashkraft --open

# Generate and open docs for the TUI crate
doc-tui:
    cargo doc --no-deps -p flashkraft-tui --open

# Generate docs for the full workspace (no browser)
doc:
    cargo doc --no-deps --workspace

# ── Changelog ─────────────────────────────────────────────────────────────────

# Regenerate the full CHANGELOG.md from all tags
changelog: _check-git-cliff
    @echo "Generating full changelog…"
    git-cliff --output CHANGELOG.md
    @echo "✅ CHANGELOG.md updated."

# Prepend only unreleased commits to CHANGELOG.md
changelog-unreleased: _check-git-cliff
    git-cliff --unreleased --prepend CHANGELOG.md
    @echo "✅ Unreleased changes prepended."

# Preview changelog for the next release without writing the file
changelog-preview: _check-git-cliff
    @git-cliff --unreleased

# ── Version bump ─────────────────────────────────────────────────────────────
# Usage: just bump 0.5.0
#
# Runs fmt → clippy → test → changelog → commit → tag, then shows push hints.
# Bump the workspace version, regenerate Cargo.lock + CHANGELOG.md, commit and tag.
# All three crates (core / gui / tui) share the version via version.workspace = true
# in their Cargo.toml files — a single source of truth in [workspace.package].
#
# Flow:
#   1. check-all  — fmt-check → clippy → tests (quality gate)
#   2. bump_version.nu — updates Cargo.toml, Cargo.lock, CHANGELOG.md, commits, tags
#
# After this completes, push with one of:
#   just push-release-all   (both remotes)

# git push origin main && git push origin v<version>
bump version: check-all _check-git-cliff _check-nu
    nu scripts/bump_version.nu --yes {{ version }}

# ── Publish (crates.io) ───────────────────────────────────────────────────────
# Publish order must be: core → gui → tui (dependency order).
# GUI and TUI are the only crates intended for public consumption; core is
# published as a prerequisite because cargo requires resolved version deps.

# Run the full pre-publish readiness check (fmt, clippy, tests, docs, dry-run)
check-publish: _check-nu
    nu scripts/check_publish.nu

# Dry-run publish for all three crates (in dependency order)
publish-dry: check-all
    @echo "Dry-run: flashkraft-core"
    cargo publish --dry-run -p flashkraft-core
    @echo "Dry-run: flashkraft (GUI)"
    cargo publish --dry-run -p flashkraft
    @echo "Dry-run: flashkraft-tui"
    cargo publish --dry-run -p flashkraft-tui

# Publish all three in dependency order: core → gui → tui.

# core must hit the crates.io index before gui and tui can resolve it.
publish: check-all publish-core publish-gui publish-tui
    @echo "✅ flashkraft-core, flashkraft, and flashkraft-tui published to crates.io!"

# Publish flashkraft-core (required by gui and tui)
publish-core:
    @echo "📦 Publishing flashkraft-core…"
    cargo publish -p flashkraft-core
    @echo "⏳ Waiting 30 s for the index to propagate…"
    sleep 30

# Publish flashkraft-gui (released as `flashkraft` on crates.io)
publish-gui:
    @echo "📦 Publishing flashkraft (GUI)…"
    cargo publish -p flashkraft

# Publish flashkraft-tui
publish-tui:
    @echo "📦 Publishing flashkraft-tui…"
    cargo publish -p flashkraft-tui

# Show what would be released without making any changes
release-preview: _check-git-cliff
    @echo "Current version: $(just version)"
    @echo ""
    @echo "Unreleased commits:"
    @git-cliff --unreleased
    @echo ""
    @echo "Workspace version:"
    @grep -A5 '^\[workspace\.package\]' Cargo.toml | grep '^version'
    @echo ""
    @echo "Published crates:  flashkraft-core (lib) • flashkraft (GUI) • flashkraft-tui (TUI)"

# ── Housekeeping ──────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# Update all dependencies (Cargo.lock only)
update:
    cargo update

# Update dependencies, run the full quality gate, then commit and push if all green.

# Aborts without committing if fmt, clippy, or tests fail.
update-deps:
    #!/usr/bin/env sh
    set -e
    echo "⬆️  Updating dependencies…"
    cargo update
    echo "🔍 Running quality gate…"
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated
    cargo test --workspace --locked --all-features --all-targets
    echo "✅ All checks passed — committing dependency updates…"
    git add Cargo.lock
    git diff --cached --quiet || git commit -m "chore: update dependencies"
    set +e
    fail=0
    git push origin main            || { echo "⚠️  origin failed";            fail=1; }
    git push gitea-microlab main             || { echo "⚠️  gitea-microlab failed";             fail=1; }
    git push gitea-starscream main  || { echo "⚠️  gitea-starscream failed";  fail=1; }
    if [ "$fail" -eq 0 ]; then echo "✅ Dependency updates pushed to all remotes.";
    else echo "⚠️  Some remotes failed — check output above."; fi

# Show outdated dependencies (requires cargo-outdated)
outdated:
    cargo outdated

# Show the current workspace version
version: _check-nu
    @nu scripts/version.nu

# Show project info
info:
    @echo "Project:   flashkraft"
    @echo "Version:   $(just version)"
    @echo "Author:    Sorin Irimies"
    @echo "License:   MIT"
    @echo ""
    @echo "Crates:"
    @echo "  flashkraft-core  — shared domain + flash engine (internal)"
    @echo "  flashkraft-gui   — Iced desktop GUI"
    @echo "  flashkraft-tui   — Ratatui terminal UI"

# ── Git helpers ───────────────────────────────────────────────────────────────

# Show configured remotes
remotes:
    @git remote -v

# Stage all changes and commit
commit message:
    git add -A
    git commit -m "{{ message }}"

# Push the current branch to GitHub (origin)
push:
    git push origin main

# Push the current branch to Gitea Microlab
push-gitea-microlab:
    git push gitea-microlab main

# Push the current branch to Gitea Starscream
push-gitea-starscream:
    git push gitea-starscream main

# Push the current branch to Gitea (nexus-lab instance)
push-gitea-nexus-lab:
    git push gitea-nexus-lab main

# Push the current branch to all remotes (continues on failure)
push-all:
    #!/usr/bin/env sh
    fail=0
    git push origin main            || { echo "⚠️  origin failed";            fail=1; }
    git push gitea-microlab main             || { echo "⚠️  gitea-microlab failed";             fail=1; }
    git push gitea-starscream main  || { echo "⚠️  gitea-starscream failed";  fail=1; }
    git push gitea-nexus-lab main   || { echo "⚠️  gitea-nexus-lab failed";   fail=1; }
    if [ "$fail" -eq 0 ]; then echo "✅ Pushed to GitHub, Gitea, Gitea Starscream, and Gitea (nexus-lab)!"; \
    else echo "⚠️  Some remotes failed — check output above."; fi

# Force-push the current branch to all remotes (continues on failure)
push-all-force:
    #!/usr/bin/env sh
    fail=0
    git push --force origin main            || { echo "⚠️  origin failed";            fail=1; }
    git push --force gitea-microlab main             || { echo "⚠️  gitea-microlab failed";             fail=1; }
    git push --force gitea-starscream main  || { echo "⚠️  gitea-starscream failed";  fail=1; }
    git push --force gitea-nexus-lab main   || { echo "⚠️  gitea-nexus-lab failed";   fail=1; }
    if [ "$fail" -eq 0 ]; then echo "✅ Force-pushed to GitHub, Gitea, Gitea Starscream, and Gitea (nexus-lab)!"; \
    else echo "⚠️  Some remotes failed — check output above."; fi

# Pull the current branch from GitHub (origin)
pull:
    git pull origin main

# Pull the current branch from Gitea Microlab
pull-gitea-microlab:
    git pull gitea-microlab main

# Pull the current branch from Gitea Starscream
pull-gitea-starscream:
    git pull gitea-starscream main

# Pull the current branch from Gitea (nexus-lab instance)
pull-gitea-nexus-lab:
    git pull gitea-nexus-lab main

# Pull the current branch from all remotes (continues on failure)
pull-all:
    #!/usr/bin/env sh
    fail=0
    git pull origin main            || { echo "⚠️  origin failed";            fail=1; }
    git pull gitea-microlab main             || { echo "⚠️  gitea-microlab failed";             fail=1; }
    git pull gitea-starscream main  || { echo "⚠️  gitea-starscream failed";  fail=1; }
    git pull gitea-nexus-lab main   || { echo "⚠️  gitea-nexus-lab failed";   fail=1; }
    if [ "$fail" -eq 0 ]; then echo "✅ Pulled from GitHub, Gitea, Gitea Starscream, and Gitea (nexus-lab)!"; \
    else echo "⚠️  Some remotes failed — check output above."; fi

# Push all tags to GitHub
push-tags:
    git push origin --tags

# Push all tags to all remotes (continues on failure)
push-tags-all:
    #!/usr/bin/env sh
    fail=0
    git push origin --tags            || { echo "⚠️  origin failed";            fail=1; }
    git push gitea-microlab --tags             || { echo "⚠️  gitea-microlab failed";             fail=1; }
    git push gitea-starscream --tags  || { echo "⚠️  gitea-starscream failed";  fail=1; }
    git push gitea-nexus-lab --tags   || { echo "⚠️  gitea-nexus-lab failed";   fail=1; }
    if [ "$fail" -eq 0 ]; then echo "✅ Tags pushed to all remotes!"; \
    else echo "⚠️  Some remotes failed — check output above."; fi

# ── Release workflows ─────────────────────────────────────────────────────────
# Full release flow (quality-gate → bump → push → CI triggers build & publish):
#
#   just release-preview          # see unreleased commits and current version
#   just release 0.5.0            # bump + push to GitHub + dispatch workflow (requires gh CLI)
#   just release-all 0.5.0        # bump + push to GitHub + Gitea → Release workflow fires
#
# Version is shared across all three crates via version.workspace = true —
# bumping [workspace.package] in Cargo.toml is the single change needed.
#
# If you want to bump locally first and push later:
#   just bump 0.5.0               # runs quality-gate, commits, tags locally
#   just push-release-all         # push branch + tags to all remotes
# Bump, commit, tag, then push to GitHub — the tag push automatically triggers
# the Release workflow via `on: push: tags: v*`. No manual dispatch needed.
# --follow-tags pushes the branch and tag in a single operation to prevent

# GitHub from firing the release workflow twice.
release version: (bump version)
    @echo "Pushing release v{{ version }} to GitHub…"
    git push --follow-tags origin main
    @echo "✅ Release v{{ version }} pushed — Release workflow will trigger automatically."
    @echo "   https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]//' | sed 's/\.git//')/actions"

# Bump, commit, tag, then push to Gitea Microlab only.

# Note: Gitea Actions must be enabled and the release.yml workflow must exist there.
release-gitea-microlab version: (bump version)
    @echo "Pushing release v{{ version }} to Gitea Microlab…"
    git push --follow-tags gitea-microlab main
    @echo "✅ Release v{{ version }} live on Gitea Microlab."

# Bump, commit, tag, then push to Gitea Starscream only.
release-gitea-starscream version: (bump version)
    @echo "Pushing release v{{ version }} to Gitea Starscream…"
    git push --follow-tags gitea-starscream main
    @echo "✅ Release v{{ version }} live on Gitea Starscream."

# Bump, commit, tag, then push to Gitea (nexus-lab instance) only.
release-gitea-nexus-lab version: (bump version)
    @echo "Pushing release v{{ version }} to Gitea (nexus-lab)…"
    git push --follow-tags gitea-nexus-lab main
    @echo "✅ Release v{{ version }} live on Gitea (nexus-lab)."

# Bump, commit, tag, then push to all remotes (continues on failure).
release-all version: (bump version)
    #!/usr/bin/env sh
    echo "Pushing release v{{ version }} to all remotes…"
    fail=0
    git push --follow-tags origin main            || { echo "⚠️  origin failed";            fail=1; }
    git push --follow-tags gitea-microlab main             || { echo "⚠️  gitea-microlab failed";             fail=1; }
    git push --follow-tags gitea-starscream main  || { echo "⚠️  gitea-starscream failed";  fail=1; }
    git push --follow-tags gitea-nexus-lab main   || { echo "⚠️  gitea-nexus-lab failed";   fail=1; }
    if [ "$fail" -eq 0 ]; then echo "✅ Release v{{ version }} pushed to GitHub, Gitea, Gitea Starscream, and Gitea (nexus-lab)!"; \
    else echo "⚠️  Some remotes failed — check output above."; fi

# Push the latest commit and all tags to every remote (no bump).
# Use this after `just bump <version>` when you want to push manually.
# --follow-tags sends the branch ref and its reachable tags in a single push
# event, which is what GitHub/Gitea need to fire the `on: push: tags: v*`

# release workflow trigger reliably.
push-release-all: check-all
    #!/usr/bin/env sh
    fail=0
    git push --follow-tags origin main            || { echo "⚠️  origin failed";            fail=1; }
    git push --follow-tags gitea-microlab main             || { echo "⚠️  gitea-microlab failed";             fail=1; }
    git push --follow-tags gitea-starscream main  || { echo "⚠️  gitea-starscream failed";  fail=1; }
    git push --follow-tags gitea-nexus-lab main   || { echo "⚠️  gitea-nexus-lab failed";   fail=1; }
    if [ "$fail" -eq 0 ]; then echo "✅ Latest commit + tags pushed to all remotes."; \
    else echo "⚠️  Some remotes failed — check output above."; fi

# Manually re-trigger the Release workflow for an existing tag via the gh CLI.
# Use this ONLY if the tag push was received but the workflow did not fire.

# Requires: gh auth login  (GitHub CLI authenticated)
release-retrigger version:
    @command -v gh >/dev/null 2>&1 || { \
        echo "❌ GitHub CLI (gh) not found. Install from https://cli.github.com"; exit 1; \
    }
    @echo "Manually dispatching Release workflow for tag v{{ version }}…"
    gh workflow run release.yml --field tag=v{{ version }}
    @echo "✅ Dispatched — check progress at: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/actions"

# Force-sync Gitea Microlab with GitHub
sync-gitea-microlab:
    git push gitea-microlab main --force
    git push gitea-microlab --tags --force
    @echo "✅ Gitea Microlab force-synced with GitHub."

# Force-sync Gitea Starscream with GitHub
sync-gitea-starscream:
    git push gitea-starscream main --force
    git push gitea-starscream --tags --force
    @echo "✅ Gitea Starscream force-synced with GitHub."

# Force-sync Gitea (nexus-lab instance) with GitHub
sync-gitea-nexus-lab:
    git push gitea-nexus-lab main --force
    git push gitea-nexus-lab --tags --force
    @echo "✅ Gitea (nexus-lab) force-synced with GitHub."

# Force-sync all Gitea instances with GitHub (continues on failure)
sync-all:
    #!/usr/bin/env sh
    fail=0
    git push gitea-microlab main --force                  || { echo "⚠️  gitea-microlab main failed";              fail=1; }
    git push gitea-microlab --tags --force                || { echo "⚠️  gitea-microlab tags failed";              fail=1; }
    git push gitea-starscream main --force       || { echo "⚠️  gitea-starscream main failed";   fail=1; }
    git push gitea-starscream --tags --force     || { echo "⚠️  gitea-starscream tags failed";   fail=1; }
    git push gitea-nexus-lab main --force        || { echo "⚠️  gitea-nexus-lab main failed";    fail=1; }
    git push gitea-nexus-lab --tags --force      || { echo "⚠️  gitea-nexus-lab tags failed";    fail=1; }
    if [ "$fail" -eq 0 ]; then echo "✅ All Gitea instances force-synced with GitHub."; \
    else echo "⚠️  Some remotes failed — check output above."; fi

# Add a Gitea remote and optionally push — interactive (nu script)
setup-gitea url: _check-nu
    nu scripts/setup_gitea.nu {{ url }}

# Migrate this project to dual GitHub + Gitea hosting (interactive)
migrate-gitea: _check-nu
    nu scripts/migrate_to_gitea.nu
