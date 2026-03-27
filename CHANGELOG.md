# Changelog

All notable changes to this project will be documented in this file.

## 0.8.9 - 2026-03-27
### ✨ Features
- feat: add Nix flake for reproducible builds
### 📦 Other Changes
- Merge pull request #2 from MartinLoeper/main
- Pre-warm cargo registry before upgrading dependencies
### 🔄 Updated
- Update iced_winit to 0.14 and adjust features on Linux
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.8.8...v0.8.9
## 0.8.8 - 2026-03-25
### 📦 Other Changes
- Replace nightly dependency update workflow with improved nightly
### 🔧 Chores
- chore: bump version to 0.8.8
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.8.7...v0.8.8
## 0.8.7 - 2026-03-24
### ➕ Added
- Add nightly dependency upgrade script and tests
### 🔧 Chores
- chore: bump version to 0.8.7
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.8.6...v0.8.7
## 0.8.6 - 2026-03-23
### ➕ Added
- Add pull, pull-gitea, and pull-all recipes to justfile
- Add tempfile dependency to Cargo.toml
### 📦 Other Changes
- Migrate all release and publish scripts from Bash to Nu, add Nu test
- Downgrade flashkraft packages and update tui-file-explorer
### 🔧 Chores
- chore: nightly dependency update 2026-03-18
- chore: nightly dependency update 2026-03-19
- chore: nightly dependency update 2026-03-20
- chore: nightly dependency update 2026-03-22
- chore: nightly dependency update 2026-03-23
- chore: bump version to 0.8.6
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.8.5...v0.8.6
## 0.8.5 - 2026-03-17
### ♻️ Refactor
- Refactor device busy check and add tests for flash pipeline
### 📦 Other Changes
- Bump tui-file-explorer and tui-piechart dependencies
### 🔧 Chores
- chore: bump version to 0.8.5
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.8.4...v0.8.5
## 0.8.4 - 2026-03-17
### ➕ Added
- Add unified FlashUpdate event type for frontend progress
### 🔧 Chores
- chore: bump version to 0.8.4
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.8.3...v0.8.4
## 0.8.3 - 2026-03-17
### ➕ Added
- Add progress_floor and verify_overall_progress helpers
### 🔄 Updated
- Update tui-file-explorer to version 0.6.2
- Update tui-file-explorer to version 0.6.2
### 🔧 Chores
- chore: nightly dependency update 2026-03-14
- chore: nightly dependency update 2026-03-15
- chore: nightly dependency update 2026-03-16
- chore: nightly dependency update 2026-03-17
- chore: bump version to 0.8.3
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.8.2...v0.8.3
## 0.8.2 - 2026-03-13
### 📦 Other Changes
- Bump tui-file-explorer to 0.6.1 and tui-slider to 0.3.2
- Bump tui-file-explorer to 0.6.1
### 🔧 Chores
- chore: bump version to 0.8.2
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.8.0...v0.8.2
## 0.8.0 - 2026-03-13
### 📦 Other Changes
- Handle additional ExplorerOutcome variants in image browser
### 🔄 Updated
- Update tui-file-explorer to version 0.6.0
- Update tui-file-explorer to version 0.6.0
### 🔧 Chores
- chore: nightly dependency update 2026-03-12
- chore: nightly dependency update 2026-03-13
- chore: bump version to 0.8.0
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.9...v0.8.0
## 0.7.9 - 2026-03-12
### 🔧 Chores
- chore: bump version to 0.7.9
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.8...v0.7.9
## 0.7.8 - 2026-03-12
### ➕ Added
- Add Zed-style spinner to log panel in flashing UI
### 📈 Improvements
- Improve confirm flash dialog layout for long names and labels
### 📦 Other Changes
- Log successful udisksctl unmount instead of start
### 🔧 Chores
- chore: bump version to 0.7.8
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.7...v0.7.8
## 0.7.7 - 2026-03-11
### 📦 Other Changes
- Reorder Linux-specific dependencies in Cargo.toml
### 🔧 Chores
- chore: bump version to 0.7.7
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.6...v0.7.7
## 0.7.6 - 2026-03-11
### 📦 Other Changes
- Move nix dependency to unix-specific section
### 🔧 Chores
- chore: bump version to 0.7.6
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.5...v0.7.6
## 0.7.5 - 2026-03-11
### ➕ Added
- Add example targets to GUI and TUI Cargo.toml files
### 🔧 Chores
- chore: bump version to 0.7.5
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.4...v0.7.5
## 0.7.4 - 2026-03-11
### ➕ Added
- Add verification progress reporting to GUI and TUI
- Add verify progress event handling and simplify thread loops
- Add runtime test harness detection to reexec_as_root
### 📦 Other Changes
- Replace nusb with notify for hotplug and privilege escalation
- Skip privilege escalation during tests and update progress display
- Restrict Unix-specific functions to non-test builds
### 🔧 Chores
- chore: nightly dependency update 2026-03-05
- chore: bump version to 0.7.4
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.3...v0.7.4
## 0.7.3 - 2026-03-04
### ➕ Added
- Add USB hotplug detection via nusb and update error view
### 📈 Improvements
- Improve command line detection and clean up comments in status_views
### 🔧 Chores
- chore: bump version to 0.7.3
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.2...v0.7.3
## 0.7.2 - 2026-03-04
### 📦 Other Changes
- Call load_drives_sync directly instead of spawning blocking task
### 🔧 Chores
- chore: bump version to 0.7.2
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.1...v0.7.2
## 0.7.1 - 2026-03-03
### 🐛 Bug Fixes
- Fix macOS parsing helpers to compile under test on all platforms
### 🔧 Chores
- chore: bump version to 0.7.1
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.7.0...v0.7.1
## 0.7.0 - 2026-03-03
### ♻️ Refactor
- Refactor macOS parsing helpers for testability and clarity
### 📦 Other Changes
- Remove nusb dependency and switch to native USB detection
### 🔧 Chores
- chore: nightly dependency update 2026-03-02
- chore: bump version to 0.7.0
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.9...v0.7.0
## 0.6.9 - 2026-03-02
### 🔧 Chores
- chore: bump version to 0.6.9
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.7...v0.6.9
## 0.6.7 - 2026-03-02
### 🐛 Bug Fixes
- fix: replace invalid job-level secrets guard with env var gate in publish job
- fix: drop deprecated --token flag and make cargo publish idempotent on already-exists
### 📦 Other Changes
- Use --follow-tags for push-release-all to trigger workflows reliably
### 🔧 Chores
- chore: bump version to 0.6.7
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.6...v0.6.7
## 0.6.6 - 2026-03-02
### 🐛 Bug Fixes
- Fix drive detection to include all USB devices on Linux
### 🔧 Chores
- chore: bump version to 0.6.6
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.5...v0.6.6
## 0.6.5 - 2026-03-02
### ➕ Added
- Add HashMap and Path imports for Linux drive detection
### 🔧 Chores
- chore: bump version to 0.6.5
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.4...v0.6.5
## 0.6.4 - 2026-03-01
### ♻️ Refactor
- Refactor flash pipeline to run in-process with setuid-root
### ➕ Added
- Add Windows UAC manifest for Administrator elevation
### 🐛 Bug Fixes
- fix: add --allow-dirty to cargo publish to allow copied README.md
### 📦 Other Changes
- Remove unused display_string method and its test
### 🔄 Updated
- Update doc examples to use no_run and fix usage code
### 🔧 Chores
- chore: nightly dependency update 2026-02-27
- chore: nightly dependency update 2026-02-28
- chore: bump version to 0.6.4
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.3...v0.6.4
## 0.6.3 - 2026-02-26
### 🐛 Bug Fixes
- fix: use --follow-tags to push branch and tag in one operation, preventing double release trigger
### 🔧 Chores
- chore: bump version to 0.6.3
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.2...v0.6.3
## 0.6.2 - 2026-02-26
### 🐛 Bug Fixes
- fix: copy README.md into all crate dirs before publishing so crates.io shows it
### 🔧 Chores
- chore: bump version to 0.6.2
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.1...v0.6.2
## 0.6.1 - 2026-02-26
### 🐛 Bug Fixes
- fix: remove gh workflow dispatch from just release — tag push triggers it automatically, add release-retrigger as explicit fallback
### 🔧 Chores
- chore: bump version to 0.6.1
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.6.0...v0.6.1
## 0.6.0 - 2026-02-26
### 🐛 Bug Fixes
- fix: use cargo info instead of cargo search for reliable already-published check
### 🔧 Chores
- chore: bump version to 0.6.0
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.9...v0.6.0
## 0.5.9 - 2026-02-26
### 🐛 Bug Fixes
- fix: copy README.md into crates/flashkraft-gui before publishing to crates.io
### 🔧 Chores
- chore: bump version to 0.5.9
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.8...v0.5.9
## 0.5.8 - 2026-02-26
### 🐛 Bug Fixes
- fix: replace all -p flashkraft-gui references with -p flashkraft across workflows and justfile
### 🔧 Chores
- chore: bump version to 0.5.8
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.7...v0.5.8
## 0.5.7 - 2026-02-26
### 🐛 Bug Fixes
- fix: rename flashkraft-gui package to flashkraft for crates.io publishing
### 🔧 Chores
- chore: bump version to 0.5.7
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.6...v0.5.7
## 0.5.6 - 2026-02-26
### 🐛 Bug Fixes
- fix: skip publish if version already exists on crates.io
### 🔧 Chores
- chore: bump version to 0.5.6
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.5...v0.5.6
## 0.5.5 - 2026-02-26
### 🐛 Bug Fixes
- fix: silence unused variable warnings in flash_subscription.rs
### 🔄 CI
- ci: add flashkraft-gui to Windows check job — Iced supports Windows natively
### 🔧 Chores
- chore: bump version to 0.5.5
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.4...v0.5.5
## 0.5.4 - 2026-02-26
### 🐛 Bug Fixes
- fix: restore flashkraft-core publishing — required by gui and tui on crates.io
- fix: remove --locked from cargo test in all CI workflows
### 🔧 Chores
- chore: bump version to 0.5.4
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.3...v0.5.4
## 0.5.3 - 2026-02-26
### 🔧 Chores
- chore: bump version to 0.5.3
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.2...v0.5.3
## 0.5.2 - 2026-02-26
### ♻️ Refactor
- refactor: consolidate to single published flashkraft crate with gui/tui features
- refactor: publish flashkraft (GUI) and flashkraft-tui, core is internal only
### ✨ Features
- feat: add update-deps just command and nightly dep update workflows
### 🐛 Bug Fixes
- fix: gate nix::libc import behind #[cfg(unix)] for Windows compat
### 🔄 CI
- ci: drop aarch64 and musl targets, keep x86_64-unknown-linux-gnu only
- ci: add Windows x86_64 msvc target, fix cross-platform artifact staging
- ci: add Windows cargo check job for core and tui
### 🔧 Chores
- chore: remove unnecessary sleep between crates.io publishes
- chore: bump version to 0.5.2
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.1...v0.5.2
## 0.5.1 - 2026-02-26
### 🐛 Bug Fixes
- fix: remove secrets condition from publish job — breaks workflow_dispatch
### 🔧 Chores
- chore: bump version to 0.5.1
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.5.0...v0.5.1
## 0.5.0 - 2026-02-26
### 🔄 Updated
- Update release flow to always dispatch workflow via gh CLI
### 🔧 Chores
- chore: bump version to 0.5.0
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.4.8...v0.5.0
## 0.4.8 - 2026-02-26
### ➕ Added
- Add TUI theme system, demos, and persistence
### 🔧 Chores
- chore: bump version to 0.4.8
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.4.7...v0.4.8
## 0.4.7 - 2026-02-25
### 🔧 Chores
- chore: bump version to 0.4.7
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.4.6...v0.4.7
## 0.4.6 - 2026-02-24
### ➕ Added
- Add manual release trigger and tag validation to workflow
### 📦 Other Changes
- Move VHS demos to crate examples, add Git LFS tracking for GIFs
### 🔧 Chores
- chore: bump version to 0.4.6
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.4.5...v0.4.6
## 0.4.5 - 2026-02-24
### 🔄 Updated
- Update docs and release scripts for clarity and workflow
### 🔧 Chores
- chore: bump version to 0.4.5
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.4.4...v0.4.5
## 0.4.4 - 2026-02-24
### 🔧 Chores
- chore: bump version to 0.4.4
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.4.3...v0.4.4
## 0.4.3 - 2026-02-24
### ➕ Added
- Add reusable file-explorer widget for Ratatui
- Add file explorer integration and tests for image selection
### 🔧 Chores
- chore: bump version to 0.4.3
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.4.2...v0.4.3
## 0.4.2 - 2026-02-20
### 📦 Other Changes
- system showcase
### 🔧 Chores
- chore: bump version to 0.4.2
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.4.1...v0.4.2
## 0.4.1 - 2026-02-20
### ♻️ Refactor
- Refactor into workspace with core, GUI, and TUI crates
### ➕ Added
- Add drive/image compatibility checks and warnings
- Add Ratatui-based TUI with async flash support
### 📦 Other Changes
- Remove push and release recipes from justfile and backup
- Simplify is_source_drive mount point check logic
- Run CI and release workflows only on Ubuntu
- Release v0.4.0: pure-Rust flash engine, verification, tests
- Release flashkraft-core 0.4.1 and add example demos
### 🔄 Updated
- Update dependencies in Cargo.lock
### 🔧 Chores
- chore: bump version to 0.3.3
- chore: bump version to 0.3.4
- chore: bump version to 0.3.5
- chore: bump version to 0.3.5
- chore: bump version to 0.4.1
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.3.2...v0.4.1
## 0.3.2 - 2025-10-30
### ➕ Added
- Add repository link, categories, and keywords to Cargo.toml
### 🔧 Chores
- chore: bump version to 0.3.2
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.3.1...v0.3.2
## 0.3.1 - 2025-10-27
### 📦 Other Changes
- - Fix race condition where flash progress updates continued to display
- Use theme_selector_right in status views
### 🔧 Chores
- chore: bump version to 0.3.1
- chore: bump version to 0.3.1
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.3.0...v0.3.1
## 0.3.0 - 2025-10-27
### 📦 Other Changes
- Clarify Electron bloat comparison in features section
### 🔧 Chores
- chore: bump version to 0.3.0
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.2.9...v0.3.0
## 0.2.9 - 2025-10-26
### 📦 Other Changes
- Reduce progress line and glow thickness for sleeker look
- bump version
### 🔧 Chores
- chore: bump version to 0.2.9
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.2.6...v0.2.9
## 0.2.6 - 2025-10-26
### ➕ Added
- Add cancellation support to flash operation
### 🔧 Chores
- chore: bump version to 0.2.6
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.2.5...v0.2.6
## 0.2.5 - 2025-10-25
### 🐛 Bug Fixes
- Fix cargo install command in release changelog generation
### 🔧 Chores
- chore: bump version to 0.2.5
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.2.4...v0.2.5
## 0.2.4 - 2025-10-25
### 🔧 Chores
- chore: bump version to 0.2.4
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.2.3...v0.2.4
## 0.2.3 - 2025-10-25
### 🔄 Updated
- Update README.md
### 🔧 Chores
- chore: bump version to 0.2.2
- chore: bump version to 0.2.3
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.2.1...v0.2.3
## 0.2.1 - 2025-10-25
### ➕ Added
- Add no_run and example variables to doc tests in logger macros
### 🔧 Chores
- chore: bump version to 0.2.1
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.2.0...v0.2.1
## 0.2.0 - 2025-10-25
### ♻️ Refactor
- Refactor examples and project structure for FlashKraft library
### 🔧 Chores
- chore: bump version to 0.2.0
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.1.9...v0.2.0
## 0.1.9 - 2025-10-25
### 🔄 Updated
- Update README.md
### 🔧 Chores
- chore: bump version to 0.1.8
- chore: bump version to 0.1.8
- chore: bump version to 0.1.9
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.1.7...v0.1.9
## 0.1.7 - 2025-10-25
### 📦 Other Changes
- Remove Quick Start example from release notes generation
### 🔧 Chores
- chore: bump version to 0.1.7
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.1.6...v0.1.7
## 0.1.6 - 2025-10-25
### 📦 Other Changes
- Adds VHS demo for Elm Architecture
### 🔄 Updated
- Update README.md
- Update README.md
- Update README.md
- Update README.md, cleanup
- Update README.md
### 🔧 Chores
- chore: bump version to 0.1.6
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.1.5...v0.1.6
## 0.1.5 - 2025-10-24
### 📦 Other Changes
- Scale animation speed based on transfer rate
### 🔧 Chores
- chore: bump version to 0.1.5
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.1.4...v0.1.5
## 0.1.4 - 2025-10-24
### 📦 Other Changes
- Remove screenshots section from README and simplify header color
### 🔧 Chores
- chore: bump version to 0.1.4
**Full Changelog**: https://github.com/sorinirimies/flashkraft/compare/v0.1.3...v0.1.4
## 0.1.3 - 2025-10-24
### ♻️ Refactor
- Refactor Elm Architecture methods into core/state.rs
### ➕ Added
- Add initial project structure with Elm Architecture and real device
- Add theme picker and support for dynamic themes Add theme picker and
- Add CI, release workflow, examples, VHS demos, and docs
- Add persistent theme storage using sled
- Add Cargo.lock and update project metadata and .gitignore
### 📦 Other Changes
- Initial commit
- Make device selector height fill available space
- Remove architecture and UI documentation from docs
- Show flash speed and ETA during flashing process
- Restructure codebase into core, domain, components, and utils modules
- Bump version to 0.1.1 and update bump_version.sh script
- Replace changelog with release header and add cliff.toml config
### 🔄 Updated
- Update README to reflect new project structure and dependencies
### 🔧 Chores
- chore: bump version to 0.1.3
