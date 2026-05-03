//! FlashKraft GUI Library
//!
//! This crate contains the Iced desktop application for FlashKraft.
//!
//! ## Contents
//!
//! | Module | What lives here |
//! |--------|-----------------|
//! | [`core`] | Iced app state, messages, update logic, flash runner, storage |
//! | [`ui`] | View dispatch, per-screen views, reusable widgets |
//! | [`utils`] | GUI-specific utilities (Bootstrap icon mapper) |
//!
//! ## Dependency on `flashkraft-core`
//!
//! All domain models, the flash pipeline, and drive-detection logic live in
//! the `flashkraft-core` crate.  This crate re-exports the most commonly
//! used types so callers only need to import from `flashkraft_gui`.

// GUI-specific utilities (Bootstrap icon mapper uses iced types)
#[macro_use]
pub mod utils;

pub mod core;
pub mod ui;

// ── Core re-exports ───────────────────────────────────────────────────────────

// Re-export `flashkraft_core::domain` at the crate root so that submodules can
// use `crate::domain::DriveInfo` / `crate::domain::ImageInfo` etc.
pub use flashkraft_core::domain;

// Re-export the `flash_debug!` macro from flashkraft_core so that
// `use crate::flash_debug;` in flash_runner.rs resolves correctly.
pub use flashkraft_core::debug_log;
pub use flashkraft_core::flash_debug;
pub use flashkraft_core::status_log;

// Re-export Iced app entry points
pub use core::{FlashKraft, Message};

// ── GUI entry point ───────────────────────────────────────────────────────────

/// Entry point for the Iced desktop GUI.
///
/// The binary must be installed **setuid-root** for the flash pipeline to be
/// able to open block devices:
///
/// ```text
/// sudo chown root:root /usr/bin/flashkraft
/// sudo chmod u+s       /usr/bin/flashkraft
/// ```
///
/// The real UID is captured in `main.rs` before this function is called.
pub fn run_gui() -> iced::Result {
    use iced::{Settings, Task};

    iced::application(
        || {
            let initial_state = FlashKraft::new();
            let initial_command = Task::perform(
                flashkraft_core::commands::load_drives(),
                Message::DrivesRefreshed,
            );
            (initial_state, initial_command)
        },
        FlashKraft::update,
        FlashKraft::view,
    )
    .title("FlashKraft - OS Image Writer")
    .subscription(FlashKraft::subscription)
    .theme(|state: &FlashKraft| state.theme.clone())
    .settings(Settings {
        fonts: vec![iced_fonts::BOOTSTRAP_FONT_BYTES.into()],
        ..Default::default()
    })
    .window(iced::window::Settings {
        size: iced::Size::new(1300.0, 700.0),
        resizable: false,
        decorations: true,
        ..Default::default()
    })
    .run()
}

// Re-export domain types from core so downstream code can do
// `use flashkraft_gui::{DriveInfo, ImageInfo}` without knowing about
// flashkraft-core directly.
pub use flashkraft_core::{DriveInfo, ImageInfo};
