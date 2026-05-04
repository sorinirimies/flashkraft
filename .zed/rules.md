# rules.md — Development Guidelines for FlashKraft

This file is the single source of truth for conventions, patterns, and
architectural decisions across the FlashKraft workspace.

---

## 1. Workspace Layout

```
flashkraft/
├── crates/
│   ├── flashkraft-core/          # Domain types, flash pipeline, theme catalogue (no UI)
│   │   └── src/
│   │       ├── commands/         # Drive detection, USB hotplug
│   │       ├── domain/           # DriveInfo, ImageInfo, constraints
│   │       ├── theme/            # AppTheme, Rgb, all 43 presets (SINGLE SOURCE OF TRUTH)
│   │       ├── utils/            # fmt_bytes, debug macros
│   │       └── flash_helper.rs   # Privileged flash pipeline
│   ├── flashkraft-tui/           # Ratatui terminal UI (lib + bin)
│   │   └── src/
│   │       ├── core/             # State, events/update, storage, flash_runner, message
│   │       └── ui/               # Rendering: mod.rs + components/ + screens/ + theme.rs
│   └── flashkraft-gui/           # Iced graphical UI (lib + bin)
│       └── src/
│           ├── core/             # State, update, storage, flash_runner, message, commands/
│           └── ui/               # Rendering: mod.rs + components/ + screens/ + theme.rs
├── Cargo.toml                    # Workspace manifest — all shared dep versions pinned here
├── justfile                      # Developer task runner
├── cliff.toml                    # git-cliff changelog config
└── .zed/rules.md                 # This file
```

**Rules:**
- All dependency versions live **only** in `[workspace.dependencies]`. Crate
  `Cargo.toml` files reference them with `{ workspace = true }`. No version is
  ever duplicated.
- `flashkraft-core` must never depend on `flashkraft-tui` or `flashkraft-gui`.
  Dependency graph is strictly: `tui` → `core` ← `gui`.

---

## 2. Code Style

### Formatting
- `rustfmt` is mandatory. Run `cargo fmt --all` before every commit.
- CI rejects unformatted code.

### Naming
| Thing | Convention | Example |
|---|---|---|
| Types / Traits | `UpperCamelCase` | `App`, `DriveInfo`, `FlashEvent` |
| Functions / methods | `snake_case` | `handle_key`, `poll_drives` |
| Constants | `SCREAMING_SNAKE_CASE` | `THEME_COUNT`, `MAX_LOG_LINES` |
| Modules | `snake_case` | `flash_runner`, `storage`, `theme` |
| Macros | `snake_case!` | `themed_block!`, `cursor_nav!`, `kv_line!` |

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
- **`domain/`** — pure data: `DriveInfo`, `ImageInfo`, drive constraints. No I/O.
- **`commands/`** — OS queries: drive detection (`sysinfo`/`nix`), USB hotplug.
- **`theme/`** — `AppTheme`, `Rgb`, and all 43 named presets. **Single source of
  truth for every colour in both frontends.** See §4.
- **`flash_helper.rs`** — privileged flash pipeline (`pkexec` target).
- **`utils/`** — `fmt_bytes`, debug logging macros.

### `flashkraft-tui`
- **`core/state.rs`** — `App` state machine. All state mutations. Owns
  `TuiStorage` and calls `persist_theme()` after every theme change.
- **`core/update.rs`** — keyboard events → state transitions. Pure functions;
  no direct I/O. Contains `handle_key` and all `handle_*` screen handlers.
- **`core/flash_runner.rs`** — Tokio task driving the privileged flash child.
- **`core/storage.rs`** — `TuiStorage`: JSON-backed preference store at
  `~/.config/flashkraft/tui-settings.json`. Infallible from caller's perspective.
- **`core/message.rs`** — `AppScreen`, `InputMode`, `ClipOp`, `UsbEntry`, etc.
- **`ui/mod.rs`** — top-level `render()` dispatcher + shared macros
  (`themed_block!`, `kv_line!`, `themed_checkbox!`).
- **`ui/theme.rs`** — `TuiPalette` struct and `all_app_themes()`. Derives
  palettes from `flashkraft_core::AppTheme` — never from `tui_file_explorer`.
- **`ui/components/`** — reusable widgets: `chrome.rs` (header/footer/breadcrumbs),
  `theme_panel.rs`, `helpers.rs`, `file_ops.rs`.
- **`ui/screens/`** — one file per screen: `select_image.rs`, `select_drive.rs`,
  `confirm.rs`, `flashing.rs`, `complete.rs`, `error.rs`.

**TUI Rules:**
- Screen renders must not mutate `App` (except scroll-state sync).
- `handle_*` functions are pure: `(&mut App, KeyEvent) → bool`.
- `flash_runner.rs` communicates only via `tokio::sync::mpsc` channels.
- Storage operations must never panic.

### `flashkraft-gui`
- **`core/state.rs`** — `FlashKraft` struct (Elm state). `begin_flash_state()`,
  `reset()`, `cancel_selections()`.
- **`core/update.rs`** — pure `update(state, message) → Task<Message>`.
- **`core/storage.rs`** — `Storage` + `GuiSettings`: JSON at
  `~/.config/flashkraft/gui-settings.json`. Derives all themes from core —
  see §4 and §10.
- **`core/flash_runner.rs`** — Iced `Subscription` streaming `FlashProgress`.
- **`ui/mod.rs`** — top-level `view()` dispatcher.
- **`ui/theme.rs`** — placeholder; theme logic lives in `core/storage.rs`.
- **`ui/components/`** — `header.rs`, `step_indicators.rs` (uses `step_indicator!`
  macro), `animated_progress.rs`, `theme_selector.rs`, `progress_line.rs`.
- **`ui/screens/`** — `select_image.rs`, `select_drive.rs` (uses `styled_text!`),
  `flashing.rs`, `complete.rs`, `error.rs`.

---

## 4. Theme System — Single Source of Truth

**All 43 themes live in `flashkraft-core::theme::presets`.**
Adding a theme to core automatically makes it available in both frontends
with no other changes required.

```
flashkraft_core::THEME_NAMES  [43 entries]  ← single source of truth
        │
        ├──► flashkraft-tui   all_app_themes()   0..THEME_COUNT  (43 TuiPalettes)
        │
        └──► flashkraft-gui   all_themes()       0..THEME_COUNT  (43 Theme::Custom)
```

### Core theme types (`theme/types.rs`)
- `Rgb` — `const`-constructible `(r, g, b: u8)` triplet.
- `AppTheme` — 12 semantic fields: `background`, `surface`, `border`,
  `selection`, `text_primary`, `text_secondary`, `text_muted`, `accent`,
  `success`, `warning`, `error`, `is_dark`.

### Adding a new theme
1. Add the name to `THEME_NAMES` in `presets.rs`.
2. Bump `THEME_COUNT`.
3. Add an arm to `theme_by_index()`.
4. Write the constructor function with `pub fn my_theme() -> AppTheme { … }`.
5. Add at least one test (light/dark flag, accent colour).
6. **Nothing else** — both frontends pick it up automatically.

### TUI palette mapping (`ui/theme.rs`)
```rust
TuiPalette {
    brand:   rgb(t.accent),          // primary — titles, active elements
    accent:  rgb(t.border),          // secondary — borders, badges, hints
    success: rgb(t.success),
    warn:    rgb(t.warning),
    err:     rgb(t.error),
    dim:     rgb(t.text_muted),
    fg:      rgb(t.text_primary),
    bg:      rgb(t.background),
    sel_bg:  rgb(t.selection),
    dir:     rgb(t.text_secondary),  // directory names in file explorer
}
```

Never hardcode `Color::Rgb(…)` inline in render functions.
Always use `pal.brand`, `pal.accent`, etc.

### GUI theme construction (`core/storage.rs`)
Every GUI theme is `Theme::Custom` built from core via `custom_theme_from_core()`.
No Iced built-in variants (`Theme::Dark`, `Theme::TokyoNight`, etc.) are used.
This guarantees pixel-perfect parity with the TUI colour definitions.

### Theme key bindings (TUI)
| Key | Scope | Action |
|---|---|---|
| `Ctrl+T` | Every screen, every mode | Cycle to next theme and persist |
| `t` | Every screen except SelectImage Editing mode | Cycle to next theme and persist |
| `Shift+T` | Every screen, every mode | Toggle global theme panel |
| Panel ↑/↓ or j/k | Theme panel open | Live-preview the highlighted theme |
| Panel `Enter` | Theme panel open | Confirm and persist live preview |
| Panel `Esc` | Theme panel open | Close and revert live preview |

### Theme persistence
- **TUI**: `TuiStorage::save_theme(name)` → `~/.config/flashkraft/tui-settings.json`
- **GUI**: `Storage::save_theme(theme)` → `~/.config/flashkraft/gui-settings.json`
- Both store the theme as a plain UTF-8 name string matching `THEME_NAMES`.
- Default theme for both is `"Default"` (index 0).

---

## 5. Macros

FlashKraft uses `macro_rules!` macros to eliminate boilerplate. **Prefer macros
over duplicated code blocks.** Existing macros:

### Core (`utils/logger.rs`)
- `debug_log!(…)` — debug-only `eprintln!("[DEBUG] …")`.
- `flash_debug!(…)` — debug-only `eprintln!("[FLASH_DEBUG] …")`.
- `status_log!(…)` — debug-only `eprintln!("[STATUS] …")`.
- `debug_if!(cond, …)` — conditional debug logging.
All three logging macros delegate to `__debug_log_impl!`.

### Core (test-only)
- `skip_device_tests!` — generates `should_skip_device` test table.
- `translate_event_test!` — generates hotplug event test functions.
- `pipeline_test_events!` — runs flash pipeline on temp files.
- `assert_pipeline_emits_stage!` — generates stage-emission tests.
- `busy_check_test!` — generates errno-specific busy-check tests.
- `pipeline_stage_order_test!` — generates platform-gated stage-order tests.

### TUI (`ui/mod.rs`)
- `themed_block!(title, title_color, border_color)` — builds a `Block` with
  rounded borders and a bold styled title. Used ~16 times in render functions.
- `kv_line!(label, value, pal)` / `kv_line!(…, bold color)` — builds a
  key-value `Line` with a dim label and a styled value.
- `themed_checkbox!(label, checked, color, pal)` — builds a palette-styled
  `Checkbox` widget.

### TUI (`core/state.rs`)
- `cursor_nav!(up: fn_up, down: fn_down, cursor: field, list: field)` — generates
  a pair of cursor-clamp navigation methods on `impl App`.

### TUI (`core/update.rs`)
- `try_or_error_screen!(app, call)` — matches `Ok(())`/`Err(msg)` and
  transitions to the Error screen on failure.

### GUI (`ui/components/step_indicators.rs`)
- `step_indicator!(icon, label)` — builds a centred 220px step indicator.

### GUI (`ui/screens/select_drive.rs`)
- `styled_text!(content, size, disabled)` — applies grey colour when disabled.

### Rule
When the same code pattern appears **3 or more times** with only data varying,
extract it into a `macro_rules!` macro. Document it in this section.

---

## 6. Async Patterns

- Async runtime: **Tokio** (`tokio = { version = "1", features = ["full"] }`).
- Single `#[tokio::main]` entry-point in `main.rs`.
- Channel types:
  - Drive detection + flash progress: `tokio::sync::mpsc::channel`.
  - Cancellation: `Arc<AtomicBool>`.
- **No `std::thread::spawn`** inside async code — use `tokio::task::spawn_blocking`.
- Poll channels in `poll_drives()` / `poll_flash()` once per event-loop tick.

---

## 7. Error Handling

- **Application layer** (`tui`, `gui`): `anyhow::Result` for top-level `run()`.
- **Library layer** (`core`): typed errors where variant matters; `anyhow` for
  terminal errors.
- **UI layer**: errors stored as `String` on `App`/`FlashKraft`. Never panic.
- Flash helper subprocess: exits `0` (success) or `2` (bad args). All other
  errors written to stdout as structured lines.
- `panic::set_hook` in `run()` restores the terminal before printing.

---

## 8. Privileged Flash Pipeline

```
[TUI/GUI process]
    └─ flash_runner::start_flash()
           └─ tokio::process::Command → pkexec flashkraft-tui --flash-helper <img> <dev>
                  └─ [privileged child process]
                         └─ flashkraft_core::flash_helper::run(img, dev)
                                └─ flash_writer writes + syncs + verifies
                                   progress lines → child stdout → parent mpsc → UI
```

Structured stdout lines: `SIZE:<bytes>`, `PROGRESS:<pct>`, `STAGE:<label>`,
`LOG:<msg>`, `DONE`, `ERROR:<msg>`.

---

## 9. Testing

> **Rule: every new feature or behaviour change must be accompanied by tests.**
> A PR that adds functionality without tests will not be merged.

### Where tests live
- Unit tests: `#[cfg(test)] mod tests` at the bottom of the same file.
- Integration tests spanning modules: `tests/` at the crate root.
- Use `tempfile::tempdir()` for all filesystem tests.

### What to test (mandatory)
| Area | Required tests |
|---|---|
| **State transitions** | Every `App`/`FlashKraft` method that mutates state: happy path + edge/error path |
| **Key handlers** | Every key binding: `consumed == true/false` + expected state change |
| **Storage roundtrips** | Every persisted value: save → load → assert equal |
| **Theme coverage** | New theme code: iterate `THEME_NAMES` / `all_app_themes()` |
| **Theme parity** | `every_core_theme_is_present_in_gui()` — always keep this passing |
| **Palette invariants** | New `TuiPalette` fields: at least one render-path smoke test |
| **Macro smoke tests** | New macros: at least one compile + output test |

### Test naming
```rust
fn <subject>_<condition>_<expectation>()
// e.g.:
fn theme_panel_esc_reverts_live_preview_to_original_theme()
fn save_and_load_theme_roundtrip()
fn every_core_theme_is_present_in_gui()
fn default_light_is_not_dark()
```

Omit `test_` prefix in `storage.rs`, `update.rs` — names are self-documenting.
Use `test_` prefix in `state.rs` and core domain tests.

### Running
```bash
cargo test -p flashkraft-core
cargo test -p flashkraft-tui
cargo test -p flashkraft
cargo test --workspace          # full suite — must always be green
cargo clippy --workspace -- -D warnings -A deprecated
```

---

## 10. Persistence (JSON Settings)

Both frontends persist preferences as human-readable JSON files.

| Frontend | File | Struct |
|---|---|---|
| GUI | `~/.config/flashkraft/gui-settings.json` | `GuiSettings { theme: String }` |
| TUI | `~/.config/flashkraft/tui-settings.json` | `TuiSettings { theme: String }` |

**Rules:**
- Theme is stored as a plain name string matching `flashkraft_core::THEME_NAMES`.
- Default theme name: `"Default"` for both frontends.
- Missing or corrupt file silently yields defaults — never crash.
- New persistent fields: add to the settings struct with `#[serde(default)]`
  and add a save/load roundtrip test.
- No binary serialisation formats — plain strings only.

---

## 11. Dependency Management

- **One version per dependency, defined in `[workspace.dependencies]`.**
- Prefer minor-version pins (`"1"`, `"0.30"`) over patch pins.
- Run `cargo update` + review `Cargo.lock` diffs before each release.
- Do not add GUI dependencies (`iced`, `rfd`) to `flashkraft-core` or
  `flashkraft-tui`.
- Do not add TUI dependencies (`ratatui`, `crossterm`) to `flashkraft-core`.

---

## 12. Versioning & Release

- All crates share the same version via `version.workspace = true`.
- Bump with: `just bump <version>`.
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

- Never commit `target/`, `*.rs.bk`, `.DS_Store`, `.zed/`, `.vscode/`, `.idea/`.
- Commit messages follow Conventional Commits.
- PRs are squash-merged.
- Tag format: `v<semver>`. Tags are immutable after push.
- `Cargo.lock` is committed for reproducible builds.

---

## 14. Security

- The flash writer requires root. **Never run the full TUI/GUI as root.**
  Always delegate to the helper subprocess via `pkexec`.
- Do not store API keys or credentials in source — use environment variables or
  GitHub Secrets.

---

## 15. Checklist Before Opening a PR

- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --workspace -- -D warnings -A deprecated` passes
- [ ] `cargo test --workspace` passes with **zero failures**
- [ ] **Every new feature or behaviour change has accompanying tests**
- [ ] New theme presets: `THEME_COUNT` bumped, `THEME_NAMES` updated, constructor
      written, at least one test added
- [ ] New `TuiPalette` fields: palette mapping updated in `ui/theme.rs`,
      invariant tests updated
- [ ] New persistent settings fields: `#[serde(default)]` applied, roundtrip
      test added
- [ ] New repeated code patterns (≥3 occurrences): extracted into a macro,
      documented in §5
- [ ] New public items have `///` doc comments
- [ ] No `Color::Rgb(…)` hardcoded in render functions — use `pal.*` fields
- [ ] No new `[dependencies]` added without updating `[workspace.dependencies]`
- [ ] `Cargo.toml` version is **not** bumped in the PR (release workflow owns that)
- [ ] Commit messages follow Conventional Commits
- [ ] `target/` and editor directories not staged
