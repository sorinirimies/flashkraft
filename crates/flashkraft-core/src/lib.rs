//! FlashKraft Core
//!
//! This crate contains all shared, framework-free logic that is reused by
//! both the Iced desktop GUI (`flashkraft-gui`) and the Ratatui TUI
//! (`flashkraft-tui`).
//!
//! ## Contents
//!
//! | Module | What lives here |
//! |--------|-----------------|
//! | [`domain`] | Domain models — [`DriveInfo`], [`ImageInfo`], drive constraints |
//! | [`flash_helper`] | In-process flash pipeline — `run_pipeline`, [`FlashEvent`], [`FlashStage`] |
//! | [`commands`] | Async helpers — drive detection |
//! | [`utils`] | Debug-logging macros (`debug_log!`, `flash_debug!`, …) |
//!
//! ## Dependency policy
//!
//! This crate intentionally has **no** GUI or TUI dependencies (no `iced`,
//! no `ratatui`, no `crossterm`).  It may only depend on:
//! - OS / system crates (`sysinfo`, `nix`, `sha2`, …)
//! - Async utilities (`tokio`, `futures`, `futures-timer`)
//! - Config directory resolution (`dirs`)

// Utility macros must be declared first so they are available to every
// subsequent module via the implicit `#[macro_use]` on the crate root.
#[macro_use]
pub mod utils;

pub mod commands;
pub mod domain;
pub mod flash_helper;
pub mod theme;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use domain::{DriveInfo, ImageInfo};
pub use theme::{theme_by_index, theme_index_by_name, AppTheme, Rgb, THEME_COUNT, THEME_NAMES};
pub use utils::fmt_bytes;

/// Re-export the flash pipeline event types so consumers only need to import
/// from `flashkraft_core` rather than the sub-module path.
pub use flash_helper::{verify_overall_progress, FlashEvent, FlashStage, FlashUpdate};
