//! GUI Commands Module — File Selection
//!
//! This module contains async command functions that are specific to the
//! Iced desktop application.
//!
//! ## What belongs here
//!
//! - Native file-picker dialogs ([`file_selection`]) — these depend on
//!   `rfd` which requires a running windowing system and is therefore
//!   GUI-only.
//!
//! ## What does NOT belong here
//!
//! - Drive / block-device detection — that is framework-free and lives in
//!   `flashkraft_core::commands::drive_detection` so it can be shared with
//!   the TUI frontend.

pub mod file_selection;
pub mod update_check;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use file_selection::select_image_file;
pub use update_check::check_for_update;

// Re-export the shared drive-detection command so GUI code can call
// `commands::load_drives()` without importing from flashkraft_core directly.
pub use flashkraft_core::commands::load_drives;
