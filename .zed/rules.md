# rules.md — Development Guidelines for FlashKraft

This file is the single source of truth for conventions, patterns, and
architectural decisions across the FlashKraft workspace.

---

## 1. Workspace Layout

```
flashkraft/
├── crates/
│   ├── flashkraft-core/   # Domain types, commands, flash writer, utils (no UI)
│   ├── flashkraft-tui/    # Ratatui terminal UI (lib + bin)
│   └── flashkraft-gui/    # Iced graphical UI
├── Cargo.toml             # Workspace manifest — all shared dep versions pinned here
├── justfile               # Developer task runner
├── cliff.toml             # git-cliff changelog config
└── rules.md               # This file
```

**Rules:**
- All dependency versions live **only** in `[workspace.dependencies]`. Crate
  `Cargo.toml` files reference them with `{ workspace = true }`. No version is
  ever duplicated.
- `flashkraft-core` must never depend on `flashkraft-tui` or `flashkraft-gui`.
  The dependency graph is strictly: `tui` → `core` ← `gui`.
- External standalone crates (e.g. `tui-file-explorer`) are added to the
  workspace as path dependencies during development, then switched to a version
  reference on publication.

---

## 2. Code Style

### Formatting
- `rustfmt` is mandatory. Run `cargo fmt --all` before every commit.
- `max_width = 100` (see `rustfmt.toml` in each crate).
- CI rejects unformatted code.

### Naming
| Thing | Convention | Example |
|---|---|---|
| Types / Traits | `UpperCamelCase` | `App`, `DriveInfo`, `FlashEvent` |
| Functions / methods | `snake_case` | `handle_key`, `poll_drives` |
| Constants | `SCREAMING_SNAKE_CASE` | `KEY_THEME`, `MAX_LOG_LINES` |
| Modules | `snake_case` | `flash_runner`, `storage`, `theme` |
| Async tasks | `snake_case` with `_task` / `_runner` suffix | `flash_runner` |

### Clippy
- Zero warnings in CI: `cargo clippy -- -D warnings -A deprecated`.
- Never suppress a lint without a comment.
- `#[allow(dead_code)]` is not allowed on production code.

### Section dividers (80 chars wide)
```rust
// ── Section title ──────────────────────────────────────────────────────────
```

---

## 3. Module Responsibilities

### `flashkraft-core`
- **Domain** (`domain/`) — pure data: `DriveInfo`, `ImageInfo`. No I/O.
- **Commands** (`commands/`) — OS queries: drive detection via `sysinfo`/`nix`.
- **Flash writer** (`flash_writer/`) — synchronous raw-write pipeline.
- **Flash helper** (`flash_helper/`) — privileged subprocess entry-point
  (`pkexec` target).
- **Utils** (`utils/`) — shared helpers (hashing, size formatting, etc.).

### `flashkraft-tui`
- **`lib.rs`** — crate root, re-exports, async `run()` entry-point.
- **`tui/app.rs`** — `App` state machine. All state mutations and transitions
  live here. Owns `TuiStorage` and calls `persist_theme()` after every theme
  change.
- **`tui/events.rs`** — keyboard events → state transitions. Pure functions
  only; no direct I/O.
- **`tui/flash_runner.rs`** — Tokio task driving the privileged flash child.
- **`tui/ui.rs`** — all ratatui `Frame` rendering. No state mutation. Receives
  a cloned `TuiPalette` from `app.palette()` at the top of `render()` and
  threads it through every `render_*` function.
- **`tui/theme.rs`** — `TuiPalette` struct and `all_app_themes()`. Maps each
  `tui_file_explorer::Theme` preset to a `TuiPalette`, adding `bg`, `warn`,
  and `err` colours that the explorer theme model does not carry.
- **`tui/storage.rs`** — `TuiStorage`: redb-backed preference store. Persists
  the active theme name across restarts. All operations are infallible from the
  caller's perspective — a missing or corrupt DB is silently ignored.

**Rules:**
- `ui.rs` must not mutate `App` — it receives `&mut App` only for ratatui
  scroll state synchronisation.
- `events.rs` functions are pure: given `(&mut App, KeyEvent)` they return
  a `bool` (consumed flag); all side-effects go through `App` state fields.
- `flash_runner.rs` communicates exclusively via `tokio::sync::mpsc` channels.
  Never share state directly between the runner task and the UI.
- `storage.rs` operations must never panic. Wrap every redb call in `.ok()` or
  `.unwrap_or_default()`.

---

## 4. Theme System (TUI)

The TUI colour system is built around `TuiPalette`, derived from
`tui_file_explorer::Theme::all_presets()`.

### Data flow
```
tui_file_explorer::Theme::all_presets()
    ├─ App::explorer_themes  Vec<(String, Theme)>   ← drives the file explorer
    └─ App::app_themes       Vec<(String, TuiPalette)>  ← drives the full TUI
         (both indexed by App::explorer_theme_idx — one source of truth)
```

### Palette threading
```rust
pub fn render(app: &mut App, frame: &mut Frame) {
    let pal = app.palette().clone();      // cloned once at the top
    render_select_image(app, frame, area, &pal, theme_name);
    // every render_* receives &TuiPalette
}
```

Never hardcode `Color::Rgb(...)` inline in render functions.
Always reference `pal.brand`, `pal.accent`, `pal.dim`, etc.

### TuiPalette fields
| Field | Role |
|---|---|
| `brand` | Primary accent (titles, active elements) |
| `accent` | Secondary accent (borders, key hints) |
| `success` | Positive / success state |
| `warn` | Warning / caution state |
| `err` | Error / destructive state |
| `dim` | Muted / secondary text |
| `fg` | Default foreground |
| `bg` | Terminal background fill |
| `sel_bg` | Selected-row background |
| `dir` | Directory names in the file explorer |

### Theme key bindings (TUI)
| Key | Scope | Action |
|---|---|---|
| `Ctrl+T` | Every screen, every mode | Cycle to next theme and persist |
| `t` | Every screen except SelectImage Editing mode | Cycle to next theme and persist |
| `[` | BrowseImage only | Cycle to previous theme and persist |
| `Shift+T` | Every screen, every mode | Toggle global theme panel |

### Theme persistence
Every theme change (`next_explorer_theme`, `prev_explorer_theme`,
`theme_panel_confirm`) calls `App::persist_theme()`, which writes the theme
name string to redb via `TuiStorage::save_theme`. On startup `App::new()`
reads the saved name back and resolves its index in `all_presets()`.

Config DB paths:
- **macOS:** `~/Library/Application Support/flashkraft/tui-prefs.db`
- **Linux:** `~/.config/flashkraft/tui-prefs.db`
- **Windows:** `%APPDATA%\flashkraft\tui-prefs.db`

---

## 5. Async Patterns

- The async runtime is **Tokio** (`tokio = { version = "1", features = ["full"] }`).
- The single `#[tokio::main]` entry-point is in `main.rs`. All async logic
  flows from there.
- Channel types:
  - Drive detection: `tokio::sync::mpsc::channel` — one sender in a spawned
    task, one receiver polled in the event loop.
  - Flash progress: `tokio::sync::mpsc::channel` — same pattern.
  - Cancellation: `Arc<AtomicBool>`.
- **No `std::thread::spawn`** inside async code. Use `tokio::task::spawn_blocking`
  for synchronous I/O that would block the executor.
- Poll channels in `App::poll_drives()` and `App::poll_flash()` — these are
  called once per event-loop tick (100 ms). Do not block in them.

---

## 6. Error Handling

- **Application layer** (`flashkraft-tui`, `flashkraft-gui`): use `anyhow::Result`
  for the top-level `run()` and `main()` return types. Propagate with `?`.
- **Library layer** (`flashkraft-core`): use typed errors where the caller needs
  to distinguish variants; use `anyhow` where errors are terminal.
- **UI layer**: errors shown to the user are stored as `String` fields on `App`
  (e.g. `app.error_message`). Never panic in UI code.
- The flash helper subprocess exits with code `0` (success) or `2` (bad args).
  All other errors are written to stdout as structured lines and parsed by
  `flash_runner.rs`.
- Install a `panic::set_hook` in `run()` that restores the terminal before
  printing the panic message (already done in `lib.rs`).

---

## 7. Privileged Flash Pipeline

```
[TUI process]
    └─ flash_runner::start_flash()
           └─ tokio::process::Command → pkexec flashkraft-tui --flash-helper <img> <dev>
                  └─ [privileged child process]
                         └─ flashkraft_core::flash_helper::run(img, dev)
                                └─ flash_writer writes + syncs + verifies
                                   progress lines → child stdout → parent mpsc → UI
```

- The `--flash-helper` branch is entered in `main.rs` **before** any TUI code runs.
- Structured stdout lines from the child: `SIZE:<bytes>`, `PROGRESS:<pct>`,
  `STAGE:<label>`, `LOG:<msg>`, `DONE`, `ERROR:<msg>`.
- The parent never writes to the child's stdin.
- Cancellation sends `SIGTERM` to the child process group.

---

## 8. Testing

> **Rule: every new feature or behaviour change must be accompanied by tests.**
> A PR that adds functionality without tests will not be merged.

### Where tests live
- Unit tests go in `#[cfg(test)] mod tests` at the **bottom of the same file**
  as the code under test.
- Integration-style tests that span multiple modules go in `tests/` at the
  crate root.
- Use `tempfile::tempdir()` for all filesystem tests (including redb storage).

### What to test
- **State transitions** — every `App` method that mutates state gets at least
  one happy-path test and one edge/error test.
- **Key handlers** — every key binding in `events.rs` gets a test asserting
  `consumed == true/false` and the expected state change.
- **Storage roundtrips** — every value written to `TuiStorage` must have a
  save/load roundtrip test.
- **Theme coverage** — when adding theme-aware code, iterate over
  `Theme::all_presets()` to ensure no preset is forgotten.
- **Palette invariants** — new `TuiPalette` fields must be exercised in at
  least one render path; smoke-test via the existing theme cycling tests.

### Test naming
```rust
fn <subject>_<condition>_<expectation>()
// e.g.:
fn theme_panel_enter_applies_cursor_theme_and_closes()
fn save_and_load_theme_roundtrip()
fn t_inserts_char_on_select_image_editing_mode()
```

Prefix with `test_` for `app.rs` / `core` tests; omit it in `events.rs` and
`storage.rs` where the name is self-documenting without the prefix.

### Running
```bash
cargo test -p flashkraft-core
cargo test -p flashkraft-tui
cargo test --workspace
```

---

## 9. Ratatui Conventions

- Each screen has one rendering function in `tui/ui.rs`:
  `render_select_image`, `render_select_drive`, etc.
  The top-level `render(app, frame)` dispatches to these.
- **Do not use module-level colour constants.** All colours come from
  `TuiPalette`, cloned once at the top of `render()` and passed as
  `pal: &TuiPalette` to every `render_*` function. This is what makes
  runtime theme switching work.
- Widget construction is always local to the render function — no widgets
  stored in `App`.
- The `tick_count` field on `App` drives animations. Use
  `app.tick_count.wrapping_add(1)` to prevent overflow.
- The event-loop poll timeout is 100 ms. This is the UI update cadence.
  Do not change it without considering animation smoothness vs CPU usage.
- Overlay panels (e.g. the global theme panel) are rendered **after** the
  active screen's render function in `render()`, so they float on top.
  Use `frame.render_widget(Clear, area)` before drawing the overlay.

---

## 10. Persistence (redb)

- Both `flashkraft-gui` and `flashkraft-tui` use `redb` for user preferences.
  The DB files are separate:
  - GUI: `flashkraft/preferences.db`
  - TUI: `flashkraft/tui-prefs.db`
- Keys are `&[u8]` constants defined at the top of the storage module:
  ```rust
  const KEY_THEME: &[u8] = b"tui_theme";
  ```
- Values are UTF-8 strings. No binary serialisation formats (no serde, no
  bincode) — plain strings are enough.
- Always commit the write transaction after inserts to guarantee durability.
- Wrap every redb operation in `.ok()` — storage failures must never crash
  the application.

---

## 11. Dependency Management

- **One version per dependency, defined in `[workspace.dependencies]`.**
- Prefer minor-version pins (`"1"`, `"0.30"`) over patch pins.
- Run `cargo update` + review `Cargo.lock` diffs before each release.
- Do not add GUI dependencies (`iced`, `rfd`, etc.) to `flashkraft-core` or
  `flashkraft-tui`.
- `dirs` and `redb` are workspace-wide — used by both GUI and TUI for config
  directory resolution and preference storage respectively.
- Git dependencies (`tui-slider`, `tui-piechart`, `tui-checkbox`) must be
  switched to crates.io versions when those crates are published.

---

## 12. Versioning & Release

- All crates share the same version via `version.workspace = true`.
- Bump with: `just bump <version>` (updates `Cargo.toml`, generates changelog,
  commits, tags).
- Push the tag to trigger the release workflow:
  ```bash
  just release <version>       # GitHub only
  just release-all <version>   # GitHub + Gitea
  ```
- Changelog is auto-generated by `git-cliff` from Conventional Commits.

### Commit prefixes
| Prefix | When |
|---|---|
| `feat:` | New user-visible feature |
| `fix:` | Bug fix |
| `doc:` | Docs only |
| `refactor:` | Internal restructure |
| `perf:` | Performance improvement |
| `chore:` | CI, tooling, deps |
| `BREAKING CHANGE:` | Major version bump required |

---

## 13. Git Hygiene

- Never commit `target/`, `*.rs.bk`, `.DS_Store`, `.zed/`, `.vscode/`,
  `.idea/` — all in `.gitignore`.
- Commit messages follow Conventional Commits.
- PRs are squash-merged.
- Tag format: `v<semver>`. Tags are immutable after push.
- The `Cargo.lock` is committed for reproducible builds.

---

## 14. Security

- The flash writer requires root. **Never run the full TUI as root.**
  Always delegate the privileged operation to the helper subprocess via `pkexec`.
- Do not store API keys, tokens, or credentials in source. Use environment
  variables or secret management (GitHub Secrets for CI).
- `CRATES_IO_TOKEN` lives only in GitHub/Gitea repository secrets, never in
  any file tracked by git.

---

## 15. Checklist Before Opening a PR

- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --workspace -- -D warnings -A deprecated` passes
- [ ] `cargo test --workspace` passes with **zero failures**
- [ ] **Every new feature or behaviour change has accompanying tests**
- [ ] New `TuiPalette` fields or theme presets are covered by tests
- [ ] New `TuiStorage` keys have save/load roundtrip tests
- [ ] New public items have `///` doc comments
- [ ] No new `[dependencies]` added without updating `[workspace.dependencies]`
- [ ] No inline `Color::Rgb(...)` added to render functions — use `pal.*` fields
- [ ] `Cargo.toml` version is **not** bumped in the PR (release workflow owns that)
- [ ] Commit messages follow Conventional Commits
- [ ] `target/` and editor directories not staged