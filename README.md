# FlashKraft ⚡

[![Crates.io](https://img.shields.io/crates/v/flashkraft)](https://crates.io/crates/flashkraft)
[![Documentation](https://docs.rs/flashkraft/badge.svg)](https://docs.rs/flashkraft)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Release](https://github.com/sorinirimies/flashkraft/actions/workflows/release.yml/badge.svg)](https://github.com/sorinirimies/flashkraft/actions/workflows/release.yml)
[![CI](https://github.com/sorinirimies/flashkraft/actions/workflows/ci.yml/badge.svg)](https://github.com/sorinirimies/flashkraft/actions/workflows/ci.yml)

A lightning-fast, lightweight OS image writer built entirely in Rust. Choose your interface:

| | `flashkraft` (GUI) | `flashkraft-tui` (TUI) |
|---|---|---|
| Framework | [Iced](https://github.com/iced-rs/iced) 0.13 | [Ratatui](https://github.com/ratatui-org/ratatui) 0.30 |
| Input | Mouse + keyboard | Keyboard only |
| Themes | 21 built-in Iced themes | Multiple themes via `tui-file-explorer` |
| Best for | Desktop users | SSH / headless / minimal setups |

No Electron, no shell scripts, no external tooling — pure Rust from UI to block device.

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

- 🎨 **21 beautiful Iced themes** to choose from, persisted across sessions via `redb`
- 🖱️ **Native file picker** — powered by `rfd` for OS-native open dialogs

### TUI extras

- ⌨️ **Fully keyboard-driven** — vim-style `j/k` navigation, `b`/`Esc` to go back
- 📂 **Built-in file explorer** — browse and pick ISO files without leaving the terminal (`Tab` / `Ctrl+F`)
- 📊 **Pie-chart drive overview** — storage breakdown rendered inline using `tui-piechart`
- ✅ **Checkbox confirmation screen** — safety checklist before every flash via `tui-checkbox`
- 🎚️ **Slider progress bar** — smooth flash-progress widget via [`tui-slider`](https://crates.io/crates/tui-slider)
- 🎨 **Multiple file-explorer themes** — switchable live with `t` / `[`, panel toggled with `T`, powered by [`tui-file-explorer`](https://crates.io/crates/tui-file-explorer)
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
│   │       ├── flash_helper.rs          ★ privileged flash pipeline (pkexec)
│   │       ├── flash_writer.rs          ★ wire-protocol parser & speed normaliser
│   │       ├── domain/
│   │       │   ├── drive_info.rs
│   │       │   ├── image_info.rs
│   │       │   └── constraints.rs       drive/image compatibility checks
│   │       ├── commands/
│   │       │   └── drive_detection.rs   async /sys/block enumeration
│   │       └── utils/
│   │           └── logger.rs            debug_log!, flash_debug!, status_log! macros
│   │
│   ├── flashkraft-gui/                  Iced desktop application
│   │   ├── examples/
│   │   │   ├── basic_usage.rs
│   │   │   ├── custom_theme.rs
│   │   └── vhs/                    GUI VHS tapes
│   │       ├── demo-basic.tape
│   │       ├── demo-build.tape
│   │       ├── demo-quick.tape
│   │       └── generated/          output GIFs (Git LFS)
│   │   └── src/
│   │       ├── main.rs                  entry point + --flash-helper dispatch
│   │       ├── lib.rs
│   │       ├── view.rs                  view orchestration
│   │       ├── core/                    Elm Architecture
│   │       │   ├── state.rs             Model + TEA methods
│   │       │   ├── message.rs           all Message variants
│   │       │   ├── update.rs            state transition logic
│   │       │   ├── storage.rs           redb-backed theme persistence
│   │       │   ├── flash_subscription.rs Iced Subscription — streams FlashProgress
│   │       │   └── commands/
│   │       │       └── file_selection.rs async rfd file dialog
│   │       └── components/              UI widgets
│   │           ├── animated_progress.rs
│   │           ├── device_selector.rs
│   │           ├── header.rs
│   │           ├── progress_line.rs
│   │           ├── selection_panels.rs
│   │           ├── status_views.rs
│   │           ├── step_indicators.rs
│   │           └── theme_selector.rs
│   │
│   └── flashkraft-tui/                  Ratatui terminal application
│       ├── examples/
│       │   ├── headless_demo.rs
│       │   ├── tui_demo.rs
│       │   ├── flash_progress_demo.rs  ← tui-slider progress showcase
│       │   └── theme_demo.rs           ← file-explorer theme switcher showcase
│       └── vhs/                    TUI VHS tapes
│           ├── tui-demo.tape
│           ├── tui-headless.tape
│           ├── flash-progress.tape
│           ├── theme-switcher.tape
│           └── generated/          output GIFs (Git LFS)
│       └── src/
│           ├── main.rs                  entry point + --flash-helper dispatch
│           ├── lib.rs
│           ├── tui/
│           │   ├── app.rs               App state + all screen transitions
│           │   ├── ui.rs                ratatui Frame rendering (all screens)
│           │   ├── events.rs            keyboard event handler per screen
│           │   ├── flash_runner.rs      async pkexec supervisor + line parser
│           │   └── mod.rs
│           └── file_explorer/
│               └── mod.rs              built-in keyboard-driven file browser
│
├── scripts/
│   ├── version.nu               print current workspace version
│   ├── bump_version.nu          automated workspace version bump
│   ├── check_publish.nu         pre-publish readiness check (all 3 crates)
│   ├── release_prepare.nu       generate CHANGELOG.md + RELEASE_NOTES.md
│   ├── setup_gitea.nu           add / update the Gitea remote
│   ├── migrate_to_gitea.nu      migrate project to dual GitHub + Gitea hosting
│   └── tests/
│       ├── runner.nu            shared test runner (discovers "test …" commands)
│       ├── run_all.nu           runs every test_*.nu suite with a summary table
│       ├── test_version.nu      tests for version.nu parsing logic
│       ├── test_bump_version.nu tests for bump_version.nu update logic
│       ├── test_check_publish.nu tests for check_publish.nu checks
│       └── test_release_prepare.nu tests for release_prepare.nu notes generation
│
└── .github/workflows/
    ├── ci.yml
    └── release.yml
```

Items marked ★ form the flash pipeline and are described in detail above.

## Dependencies

### `flashkraft-core`

| Crate | Version | Purpose |
|-------|---------|---------|
| `sysinfo` | 0.30 | Drive enumeration |
| `nix` | 0.29 | `umount2`, `BLKRRPART` ioctl, `fsync` |
| `sha2` | 0.10 | SHA-256 write verification |
| `tokio` | 1 | Async runtime |
| `futures` / `futures-timer` | 0.3 / 3.0 | Async channel primitives |
| `redb` | 4.0 | Embedded key-value store |
| `dirs` | 5.0 | XDG data directory resolution |
| `anyhow` | 1 | Error handling |

### `flashkraft-gui`

| Crate | Version | Purpose |
|-------|---------|---------|
| `iced` | 0.13 | Cross-platform GUI framework (Elm Architecture) |
| `iced_aw` | 0.12 | Additional Iced widgets |
| `iced_fonts` | 0.1 | Bootstrap icon font |
| `rfd` | 0.15 | Native file/folder dialogs |

### `flashkraft-tui`

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.30 | Terminal UI framework |
| `crossterm` | 0.29 | Cross-platform terminal control |
| `tui-slider` | git | Flash-progress slider widget |
| `tui-piechart` | git | Drive storage pie-chart widget |
| `tui-checkbox` | git | Drive-list and confirm-screen checkboxes |

## Architecture Highlights

- **Shared core crate** — `flashkraft-core` contains all flash logic; both UIs are thin frontends over the same engine
- **Pure-Rust flash engine** — zero shell scripts or external binaries
- **Self-elevating helper** — single binary per UI, no install-time setup beyond `polkit`
- **Elm Architecture (GUI)** — unidirectional data flow, pure `update`/`view` functions
- **Screen-based state machine (TUI)** — each `AppScreen` variant owns its event handler and render function
- **0 warnings** — clean `cargo build --workspace` and `cargo test --workspace`

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
| `theme-switcher` | Live file-explorer theme cycling (`t` / `[`) and theme panel (`T`) |
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
4. Keep functions pure where possible
5. Add unit tests for any new logic, especially in `flash_helper.rs`, `flash_writer.rs`, `app.rs`, and `events.rs`
6. Run `cargo test --workspace` and `cargo clippy --workspace` before opening a PR

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
- Terminal widgets: [tui-slider](https://github.com/sorinirimies/tui-slider), [tui-piechart](https://github.com/sorinirimies/tui-piechart), [tui-checkbox](https://github.com/sorinirimies/tui-checkbox)