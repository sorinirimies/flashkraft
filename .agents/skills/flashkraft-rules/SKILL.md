---
name: flashkraft-rules
description: Rules for work in the flashkraft repository that the code can't show: constraints, ownership and must-keep behaviour. Use before changing, fixing, reviewing or extending code here, together with the rust-in-projects skill.
---

# flashkraft: rules the code can't show

Hard constraints for this repository. The general conventions (orientation, tests,
scripting, releasing) are in the `rust-in-projects` skill; this file holds only what is
specific to flashkraft.

* **Root:** the flash writer needs root. Never run the TUI or GUI as root; the
  helper subprocess goes through `pkexec`.
* **The helper** reports progress on stdout lines (`PROGRESS:`/`DONE`/`ERROR:`).
  `flash_runner.rs` talks only through `tokio::sync::mpsc`.
* **Palettes** come from `flashkraft_core::AppTheme`, never from
  `tui_file_explorer`.
