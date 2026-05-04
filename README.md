# FlashKraft ⚡

[![Crates.io](https://img.shields.io/crates/v/flashkraft)](https://crates.io/crates/flashkraft)
[![Documentation](https://docs.rs/flashkraft/badge.svg)](https://docs.rs/flashkraft)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Release](https://github.com/sorinirimies/flashkraft/actions/workflows/release.yml/badge.svg)](https://github.com/sorinirimies/flashkraft/actions/workflows/release.yml)
[![CI](https://github.com/sorinirimies/flashkraft/actions/workflows/ci.yml/badge.svg)](https://github.com/sorinirimies/flashkraft/actions/workflows/ci.yml)
[![GUI Downloads](https://img.shields.io/crates/d/flashkraft?label=GUI%20downloads)](https://crates.io/crates/flashkraft)
[![TUI Downloads](https://img.shields.io/crates/d/flashkraft-tui?label=TUI%20downloads)](https://crates.io/crates/flashkraft-tui)

FlashKraft is a lightning-fast OS image writer built entirely in Rust.
No Electron, no shell scripts, no external tooling — pure Rust from UI to block device.

FlashKraft ships two front-ends from a single Rust workspace:

| Binary | Use case |
|--------|----------|
| `flashkraft` | Desktop GUI — mouse-driven with animated progress, native file dialogs, 43 themes |
| `flashkraft-tui` | Terminal UI — keyboard-driven, works over SSH, built-in file explorer & themes |

| | `flashkraft` (GUI) | `flashkraft-tui` (TUI) |
|---|---|---|
| Framework | [Iced](https://github.com/iced-rs/iced) 0.14 | [Ratatui](https://github.com/ratatui-org/ratatui) 0.30 |
| Input | Mouse + keyboard | Keyboard only |
| Themes | 43 themes from `flashkraft-core` | 43 themes from `flashkraft-core` |
| Best for | Desktop users | SSH / headless / minimal setups |

## Preview

### GUI (Iced desktop)

![flashkraft_demo](https://github.com/user-attachments/assets/76549cb3-a65e-4a99-b638-1aac6d50c553)

### TUI (Ratatui terminal)

#### Full workflow

![tui-demo](crates/flashkraft-tui/examples/vhs/generated/tui-demo.gif)

#### Flash progress (tui-slider)

![flash-progress](crates/flashkraft-tui/examples/vhs/generated/flash-progress.gif)

#### File explorer & theme switcher

![theme-switcher](crates/flashkraft-tui/examples/vhs/generated/theme-switcher.gif)

> **Note:** Demo GIFs are stored with [Git LFS](https://git-lfs.github.com/).
> Run `git lfs install && git lfs pull` after cloning if the images appear broken.

## Features

### Shared (both interfaces)

- ⚡ **Pure-Rust flash engine** — no `dd`, no bash scripts; writes directly to the block device using `std::fs`, `nix` ioctls, and `sha2` verification
- 🔒 **Write verification** — SHA-256 of the source image is compared against a read-back of the device after every flash
- 🔄 **Partition table refresh** — `BLKRRPART` ioctl ensures the kernel picks up the new partition layout immediately, so the USB boots first time
- 🧲 **Lazy unmount** — all partitions are cleanly detached via `umount2(MNT_DETACH)` before writing
- 📁 **Multiple image formats** — ISO, IMG, DMG, ZIP, and more
- 💾 **Automatic drive detection** — removable drives refreshed on demand
- 🛡️ **Safe drive selection** — system drives flagged, oversized drives warned, read-only drives blocked
- 🎯 **Real-time progress** — stage-aware progress bar with live MB/s speed display
- 🪶 **Tiny footprint** — Rust-compiled binary, C-like memory usage, no Electron runtime

### GUI extras

- 🎨 **43 themes** shared with the TUI — all from `flashkraft-core`, persisted across sessions in `gui-settings.json`
- 🖱️ **Native file picker** — powered by `rfd` for OS-native open dialogs

### TUI extras

- ⌨️ **Fully keyboard-driven** — vim-style `j/k` navigation, `b`/`Esc` to go back
- 📂 **Built-in file explorer** — browse and pick ISO files without leaving the terminal (`Tab` / `Ctrl+F`)
- 📊 **Pie-chart drive overview** — storage breakdown rendered inline using `tui-piechart`
- ✅ **Checkbox confirmation screen** — safety checklist before every flash via `tui-checkbox`
- 🎚️ **Slider progress bar** — smooth flash-progress widget via [`tui-slider`](https://crates.io/crates/tui-slider)
- 🎨 **43 themes shared with the GUI** — all from `flashkraft-core`, switchable live with `t`, instant live-preview panel with `Shift+T`, persisted in `tui-settings.json`
- 🌀 **Animated spinners** — braille flux spinners and bouncing bars via [`tui-spinner`](https://crates.io/crates/tui-spinner)
- 🖥️ **Works over SSH** — no display server required

## How flashing works

FlashKraft uses a **self-elevating pure-Rust helper** pattern. When you click/confirm Flash:

```
Main process (GUI or TUI)
  └─ pkexec /path/to/flashkraft[−tui] --flash-helper <image> <device>
       └─ Runs as root, pure Rust, no shell
            1. UNMOUNTING  — reads /proc/mounts, calls umount2(MNT_DETACH) per partition
            2. WRITING     — streams image → block device in 4 MiB chunks, emits PROGRESS lines
            3. SYNCING     — fsync(fd) + sync() to flush all kernel write-back caches
            4. REREADING   — BLKRRPART ioctl so the kernel sees the new partition table
            5. VERIFYING   — SHA-256(image) == SHA-256(device[0..image_size])
            6. DONE        — UI shows success
```

The same binary is re-executed with elevated privileges via `pkexec` — no separate helper binary needs to be installed. All output (progress, logs, errors) is written to stdout as structured lines that the UI reads in real time.

### Why not `dd`?

| | `dd` approach | FlashKraft |
|---|---|---|
| Shell dependency | ✗ requires bash, coreutils | ✓ pure Rust |
| Progress format | `\r`-terminated, locale-dependent | ✓ structured `PROGRESS:bytes:speed` |
| Speed unit handling | ✗ kB/s / MB/s / GB/s mixed | ✓ always normalised to MB/s |
| Write verification | ✗ none | ✓ SHA-256 read-back |
| Partition table refresh | ✗ not done | ✓ `BLKRRPART` ioctl |
| Error reporting | ✗ exit code only | ✓ `ERROR:<message>` on every failure path |

## The Elm Architecture (GUI)

The GUI crate is built using **The Elm Architecture (TEA)**, which Iced embraces as its natural pattern for interactive applications.

### Core Concepts

#### 1. Model (State)

```rust
struct FlashKraft {
    selected_image: Option<ImageInfo>,   // Currently selected image file
    selected_target: Option<DriveInfo>,  // Currently selected target drive
    available_drives: Vec<DriveInfo>,    // Detected drives
    flash_progress: Option<f32>,         // Progress 0.0–1.0
    flash_bytes_written: u64,            // Bytes written so far
    flash_speed_mb_s: f32,              // Current transfer speed
    error_message: Option<String>,       // Error message if any
    flashing_active: bool,              // Subscription guard
    flash_cancel_token: Arc<AtomicBool>, // Cancellation signal
}
```

#### 2. Messages

```rust
enum Message {
    SelectImageClicked,
    TargetDriveClicked(DriveInfo),
    FlashClicked,
    CancelFlash,
    ResetClicked,
    ImageSelected(Option<PathBuf>),
    DrivesRefreshed(Vec<DriveInfo>),
    FlashProgressUpdate(f32, u64, f32),
    FlashCompleted(Result<(), String>),
    ThemeChanged(Theme),
}
```

#### 3. Data Flow

```
User Action → Message → Update → State
                            ↓
                         Task/Subscription
                            ↓
                    Async Result → Message → Update → State
                                                ↓
                                             View → UI
```

## TUI Screen Flow

The TUI is a multi-screen application driven entirely by keyboard input:

```
SelectImage ──(Enter/confirm)──► SelectDrive ──(Enter)──► DriveInfo
     ▲                                ▲                        │
     │  (Esc/b)                       │  (Esc/b)           (f/Enter)
     │                                │                        ▼
     │                           SelectDrive            ConfirmFlash
     │                                                       │
     │                                                   (y — flash)
     │                                                       ▼
     │                                                   Flashing
     │                                                  (c — cancel)
     │                                                       │
     └──────────────────(r — reset)────────── Complete / Error
```

### TUI Key Bindings

| Screen | Key | Action |
|--------|-----|--------|
| SelectImage | `i` / `Enter` | Enter editing mode |
| SelectImage | `Tab` / `Ctrl+F` | Open built-in file browser |
| SelectImage | `Esc` / `q` | Quit |
| BrowseImage | `j` / `↓` | Move cursor down |
| BrowseImage | `k` / `↑` | Move cursor up |
| BrowseImage | `Enter` | Descend into directory / select file |
| BrowseImage | `Backspace` | Ascend to parent directory |
| BrowseImage | `Esc` / `q` | Dismiss without selecting |
| SelectDrive | `j` / `↓` | Scroll drive list down |
| SelectDrive | `k` / `↑` | Scroll drive list up |
| SelectDrive | `Enter` / `Space` | Confirm selected drive |
| SelectDrive | `r` / `F5` | Refresh drive list |
| SelectDrive | `Esc` / `b` | Go back |
| DriveInfo | `f` / `Enter` | Advance to ConfirmFlash |
| DriveInfo | `Esc` / `b` | Go back |
| ConfirmFlash | `y` / `Y` | Begin flashing |
| ConfirmFlash | `n` / `Esc` / `b` | Go back |
| Flashing | `c` / `Esc` | Cancel flash |
| Complete | `r` / `R` | Reset to start |
| Complete | `q` / `Esc` | Quit |
| Error | `r` / `Enter` | Reset to start |
| Error | `q` / `Esc` | Quit |
| Any | `Ctrl+C` / `Ctrl+Q` | Force quit |

## Building and Running

### Prerequisites

- Rust 1.70 or later
- `pkexec` (part of `polkit`, available on all major Linux distributions)
- For GUI: a running display server (X11 or Wayland)
- For TUI: any terminal emulator (works over SSH)

### Build

```bash
git clone https://github.com/sorinirimies/flashkraft.git
cd flashkraft

# Build everything
cargo build --release

# Build only the GUI
cargo build --release --bin flashkraft

# Build only the TUI
cargo build --release --bin flashkraft-tui
```

### Run

```bash
# Launch the GUI
cargo run --bin flashkraft

# Launch the TUI
cargo run --bin flashkraft-tui
```

### Development

```bash
# Debug builds (faster compilation)
cargo run --bin flashkraft
cargo run --bin flashkraft-tui

# Run all tests across the workspace
cargo test

# Check without building
cargo check --workspace

# Lint
cargo clippy --workspace

# With backtraces
RUST_BACKTRACE=1 cargo run --bin flashkraft-tui
```

## Usage

### GUI

1. **Select Image** — click the `+` button and choose an ISO, IMG, or DMG file
2. **Select Drive** — pick the target USB or SD card from the detected drives list
3. **Flash** — click **Flash!**; authenticate with `pkexec` when prompted
4. **Wait** — the progress bar shows live stage, bytes written, and MB/s
5. **Done** — verification passes automatically; safely remove the drive

### TUI

1. **Select Image** — press `i` to start typing a path, or `Tab`/`Ctrl+F` to open the file browser
2. **Select Drive** — use `j`/`k` to scroll, `r` to refresh, `Enter` to confirm
3. **Review Drive Info** — inspect the storage pie-chart, then press `f` to proceed
4. **Confirm** — read the safety checklist and press `y` to flash, or `b` to go back
5. **Wait** — the slider progress bar shows live stage, bytes written, and MB/s
6. **Done** — press `r` to reset or `q` to quit

## Project Structure

This is a [Cargo workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html) with three crates:

```
flashkraft/                              ← workspace root
├── Cargo.toml                           ← workspace manifest (shared dep versions)
│
├── crates/
│   │
│   ├── flashkraft-core/                 ★ shared logic — no GUI/TUI deps
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── flash_helper.rs          ★ privileged flash pipeline
│   │       ├── theme/                   ★ single source of truth for all 43 themes
│   │       │   ├── mod.rs
│   │       │   ├── types.rs             AppTheme, Rgb
│   │       │   └── presets.rs           all 43 constructors + THEME_NAMES
│   │       ├── domain/
│   │       │   ├── drive_info.rs
│   │       │   ├── image_info.rs
│   │       │   └── constraints.rs       drive/image compatibility checks
│   │       ├── commands/
│   │       │   ├── drive_detection.rs   async /sys/block enumeration
│   │       │   └── hotplug.rs           USB connect/disconnect watcher
│   │       └── utils/
│   │           └── logger.rs            debug_log!, flash_debug!, status_log! macros
│   │
│   ├── flashkraft-gui/                  Iced desktop application
│   │   ├── examples/
│   │   │   ├── basic_usage.rs
│   │   │   └── custom_theme.rs
│   │   └── src/
│   │       ├── main.rs              entry point + privilege escalation
│   │       ├── lib.rs
│   │       ├── core/                Elm Architecture
│   │       │   ├── state.rs         Model + TEA methods
│   │       │   ├── message.rs       all Message variants
│   │       │   ├── update.rs        state transition logic
│   │       │   ├── flash_runner.rs  Iced Subscription — streams FlashProgress
│   │       │   ├── storage.rs       JSON settings (gui-settings.json)
│   │       │   └── commands/
│   │       │       └── file_selection.rs  async rfd file dialog
│   │       ├── ui/                  presentation layer
│   │       │   ├── mod.rs           view dispatch
│   │       │   ├── theme.rs         theme utilities
│   │       │   ├── screens/         one file per screen
│   │       │   │   ├── select_image.rs
│   │       │   │   ├── select_drive.rs
│   │       │   │   ├── flashing.rs
│   │       │   │   ├── complete.rs
│   │       │   │   └── error.rs
│   │       │   └── components/      reusable widgets
│   │       │       ├── animated_progress.rs
│   │       │       ├── header.rs
│   │       │       ├── progress_line.rs
│   │       │       ├── step_indicators.rs
│   │       │       └── theme_selector.rs
│   │       └── utils/
│   │           └── icons.rs         Bootstrap icon mapper
│   │
│   └── flashkraft-tui/                  Ratatui terminal application
│       ├── examples/
│       │   ├── headless_demo.rs
│       │   ├── tui_demo.rs
│       │   ├── flash_progress_demo.rs  tui-slider progress showcase
│       │   └── theme_demo.rs           full theme switcher + live-preview panel
│       └── src/
│           ├── main.rs              entry point + privilege escalation
│           ├── lib.rs               event loop + terminal setup
│           ├── core/                business logic
│           │   ├── state.rs         App state + screen transitions
│           │   ├── message.rs       AppScreen, FlashEvent, InputMode
│           │   ├── update.rs        keyboard event handler per screen
│           │   ├── flash_runner.rs  async flash task bridge
│           │   └── storage.rs       JSON settings (tui-settings.json)
│           └── ui/                  presentation layer
│               ├── mod.rs           render dispatch + themed_block!, kv_line! macros
│               ├── theme.rs         TuiPalette — derives from flashkraft-core (43 themes)
│               ├── screens/         one file per screen
│               │   ├── select_image.rs
│               │   ├── select_drive.rs
│               │   ├── confirm.rs
│               │   ├── flashing.rs
│               │   ├── complete.rs
│               │   └── error.rs
│               └── components/      reusable widgets
│                   ├── chrome.rs    header, footer, breadcrumbs
│                   ├── helpers.rs   centred_rect, file_icon, piechart
│                   ├── file_ops.rs  file operation modals
│                   └── theme_panel.rs  live-preview theme picker overlay
│
├── scripts/
│   ├── version.nu               print current workspace version
│   ├── bump_version.nu          automated workspace version bump
│   ├── check_publish.nu         pre-publish readiness check
│   ├── release_prepare.nu       generate CHANGELOG.md + RELEASE_NOTES.md
│   ├── upgrade_deps.nu          nightly dependency upgrade
│   ├── setup_gitea.nu           add / update the Gitea remote
│   ├── migrate_to_gitea.nu      dual GitHub + Gitea hosting
│   ├── ci/
│   │   ├── validate_tag.nu      validate vX.Y.Z tag format
│   │   ├── publish_crate.nu     idempotent crates.io publisher
│   │   ├── quality_gate.nu      fmt + clippy + test + nu tests
│   │   ├── release_notes.nu     CHANGELOG + RELEASE_NOTES generator
│   │   └── stage_artifacts.nu   binary staging for release
│   └── tests/
│       ├── runner.nu            shared test runner
│       ├── run_all.nu           runs every test_*.nu suite
│       ├── test_version.nu
│       ├── test_bump_version.nu
│       ├── test_check_publish.nu
│       ├── test_release_prepare.nu
│       ├── test_upgrade_deps.nu
│       ├── test_validate_tag.nu
│       └── test_publish_crate.nu
│
├── .github/workflows/
│   ├── ci.yml
│   ├── deps-update.yml
│   └── release.yml
│
└── .gitea/workflows/
    ├── ci.yml
    ├── deps-update.yml
    └── release.yml
```

Items marked ★ form the flash pipeline and are described in detail above.

## Dependencies

### `flashkraft-core`

| Crate | Version | Purpose |
|-------|---------|------|
| `sysinfo` | 0.38 | Drive enumeration |
| `nix` | 0.31 | `umount2`, `BLKRRPART` ioctl, `fsync` |
| `sha2` | 0.11 | SHA-256 write verification |
| `tokio` | 1 | Async runtime |
| `futures` / `futures-timer` | 0.3 / 3.0 | Async channel primitives |
| `dirs` | 6.0 | XDG data directory resolution |
| `anyhow` | 1 | Error handling |

### `flashkraft-gui`

| Crate | Version | Purpose |
|-------|---------|------|
| `iced` | 0.14.0 | Cross-platform GUI framework (Elm Architecture) |
| `iced_aw` | 0.14.1 | Additional Iced widgets |
| `iced_fonts` | 0.3.0 | Bootstrap icon font |
| `rfd` | 0.17 | Native file/folder dialogs |
| `serde` / `serde_json` | 1 | JSON settings persistence |

### `flashkraft-tui`

| Crate | Version | Purpose |
|-------|---------|----------|
| `ratatui` | 0.30 | Terminal UI framework |
| `crossterm` | 0.29 | Cross-platform terminal control |
| `tui-file-explorer` | 1.0.6 | Built-in keyboard-driven file browser |
| `tui-slider` | 0.3.2 | Flash-progress slider widget |
| `tui-spinner` | 0.2.3 | Animated braille spinners (flux, bar, linear) |
| `tui-piechart` | 0.3.3 | Drive storage pie-chart widget |
| `tui-checkbox` | 0.4.4 | Drive-list and confirm-screen checkboxes |
| `serde` / `serde_json` | 1 | JSON settings persistence |

## Architecture Highlights

- **Shared core crate** — `flashkraft-core` contains all flash logic and all 43 themes; both UIs are thin frontends over the same engine
- **Centralised theme catalogue** — `flashkraft-core::theme` is the single source of truth; adding a theme to core automatically makes it available in both GUI and TUI with no other changes
- **Pure-Rust flash engine** — zero shell scripts or external binaries
- **Self-elevating helper** — single binary per UI, no install-time setup beyond `polkit`
- **Elm Architecture (GUI)** — unidirectional data flow, pure `update`/`view` functions
- **Screen-based state machine (TUI)** — each `AppScreen` variant owns its event handler and render function
- **JSON settings** — human-readable `gui-settings.json` / `tui-settings.json` in the OS config dir
- **557 tests, 0 warnings** — clean `cargo test --workspace` and `cargo clippy --workspace`

## Demo GIFs & Git LFS

All generated GIFs under `examples/vhs/generated/` are tracked by [Git LFS](https://git-lfs.github.com/) (see `.gitattributes`).

```bash
# One-time setup after cloning
git lfs install && git lfs pull   # or: just lfs-pull

# Regenerate all demos (requires vhs installed)
just vhs-all

# Regenerate TUI demos only  (output: crates/flashkraft-tui/examples/vhs/generated/)
just vhs-tui

# Regenerate GUI demos only  (output: crates/flashkraft-gui/examples/vhs/generated/)
just vhs-gui

# Render a single tape by name
just vhs-tape tui-demo
just vhs-tape flash-progress
just vhs-tape theme-switcher
just vhs-tape demo-basic

# List all available tapes and generated GIFs
just vhs-list
```

| Tape | What it shows |
|---|---|
| `tui-demo` | Full keyboard-driven wizard: image → drive → flash → complete |
| `tui-headless` | Headless state-machine demo (no TTY required) |
| `flash-progress` | Animated `tui-slider` progress bar during a simulated write |
| `theme-switcher` | Live-preview theme panel (`Shift+T`), 43-theme catalogue, `t` cycling |
| `demo-basic` | GUI basic usage |
| `demo-build` | GUI build walkthrough |
| `demo-quick` | GUI quick-start |

Install VHS:

```bash
brew install vhs                                   # macOS
go install github.com/charmbracelet/vhs@latest    # any platform
```

To run Rust examples from the workspace root:

```bash
# Core examples
cargo run -p flashkraft-core --example detect_drives
cargo run -p flashkraft-core --example constraints_demo
cargo run -p flashkraft-core --example flash_writer_demo

# GUI examples  (crates/flashkraft-gui/examples/)
cargo run -p flashkraft-gui --example basic_usage
cargo run -p flashkraft-gui --example custom_theme

# TUI examples  (crates/flashkraft-tui/examples/)
cargo run -p flashkraft-tui --example headless_demo
cargo run -p flashkraft-tui --example flash_progress_demo   # animated tui-slider demo
cargo run -p flashkraft-tui --example theme_demo            # file-explorer theme switcher
```

## Contributing

Contributions are welcome! Please:

1. Keep all flash logic in `flashkraft-core` — neither the GUI nor the TUI crate should contain flash pipeline code
2. Follow the Elm Architecture pattern in the GUI — all state changes via `update`
3. Follow the screen-state-machine pattern in the TUI — screen transitions via `App` methods
4. Both frontends share the same `core/` + `ui/screens/` + `ui/components/` layout
5. Keep functions pure where possible
6. Add unit tests for any new logic, especially in `flash_helper.rs`, `core/state.rs`, and `core/update.rs`
7. Run `cargo test --workspace` and `cargo clippy --workspace` before opening a PR

## Learning Resources

- [Iced Documentation](https://docs.rs/iced/)
- [Ratatui Documentation](https://docs.rs/ratatui/)
- [The Elm Architecture Guide](https://guide.elm-lang.org/architecture/)
- [Iced Examples](https://github.com/iced-rs/iced/tree/master/examples)
- [Ratatui Examples](https://github.com/ratatui-org/ratatui/tree/main/examples)
- [The Rust Book](https://doc.rust-lang.org/book/)
- [nix crate](https://docs.rs/nix/) — POSIX ioctls and syscalls from Rust

## License

MIT — see [LICENSE](LICENSE) for details.

## Acknowledgments

- GUI built with [Iced](https://github.com/iced-rs/iced)
- TUI built with [Ratatui](https://github.com/ratatui-org/ratatui)
- Follows [The Elm Architecture](https://guide.elm-lang.org/architecture/)
- Flash pipeline design inspired by [Balena Etcher](https://github.com/balena-io/etcher)
- Terminal widgets: [tui-slider](https://github.com/sorinirimies/tui-slider), [tui-spinner](https://github.com/sorinirimies/tui-spinner), [tui-piechart](https://github.com/sorinirimies/tui-piechart), [tui-checkbox](https://github.com/sorinirimies/tui-checkbox)