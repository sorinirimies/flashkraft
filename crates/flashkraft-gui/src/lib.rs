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
pub mod privileged_helper;
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
            let should_check_update = initial_state.should_check_for_update();
            let mut tasks = vec![Task::perform(
                flashkraft_core::commands::load_drives(),
                Message::DrivesRefreshed,
            )];
            if should_check_update {
                tasks.push(Task::perform(
                    core::commands::check_for_update(env!("CARGO_PKG_VERSION")),
                    Message::UpdateCheckCompleted,
                ));
            }
            (initial_state, Task::batch(tasks))
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
    .window(window_settings())
    .run()
}

/// Wayland `app_id` and X11 `WM_CLASS` of the window. Desktops match it against the
/// `.desktop` file name and its `StartupWMClass` for the icon and window grouping.
pub const APP_ID: &str = "flashkraft";

/// Side of the embedded window icon, in pixels.
const ICON_SIZE: u32 = 128;

/// The window icon: the title bar and taskbar on Windows, X11 window managers.
/// Raw RGBA, written by `just icons` (scripts/icons.nu) from
/// assets/icons/flashkraft.svg, so no image decoder is linked for it. macOS takes
/// the Dock icon from the `.app` bundle, Wayland from the `.desktop` entry.
static ICON_RGBA: &[u8] = include_bytes!("../assets/flashkraft-128.rgba");

/// The window icon, or `None` when the embedded bytes are not 128 × 128 RGBA.
#[must_use]
pub fn window_icon() -> Option<iced::window::Icon> {
    iced::window::icon::from_rgba(ICON_RGBA.to_vec(), ICON_SIZE, ICON_SIZE).ok()
}

/// The main window: size, minimum size, icon and, on Linux, [`APP_ID`].
#[must_use]
pub fn window_settings() -> iced::window::Settings {
    iced::window::Settings {
        icon: window_icon(),
        size: iced::Size::new(ui::layout::DEFAULT_WINDOW.0, ui::layout::DEFAULT_WINDOW.1),
        min_size: Some(iced::Size::new(
            ui::layout::MIN_WINDOW.0,
            ui::layout::MIN_WINDOW.1,
        )),
        resizable: true,
        decorations: true,
        #[cfg(target_os = "linux")]
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: APP_ID.to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// Re-export domain types from core so downstream code can do
// `use flashkraft_gui::{DriveInfo, ImageInfo}` without knowing about
// flashkraft-core directly.
pub use flashkraft_core::{DriveInfo, ImageInfo};

#[cfg(test)]
mod window_tests {
    use super::*;

    #[test]
    fn the_window_icon_is_the_embedded_rgba() {
        assert_eq!(ICON_RGBA.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
        let icon = window_icon().expect("the embedded icon decodes");
        let (rgba, size) = icon.into_raw();
        assert_eq!((size.width, size.height), (ICON_SIZE, ICON_SIZE));
        let alpha = |x: u32, y: u32| rgba[((y * ICON_SIZE + x) * 4 + 3) as usize];
        assert_eq!(
            alpha(ICON_SIZE / 2, ICON_SIZE / 2),
            255,
            "the tile is opaque"
        );
        assert_eq!(alpha(0, 0), 0, "outside the rounded corner is transparent");
    }

    #[test]
    fn the_window_carries_the_icon() {
        assert!(window_settings().icon.is_some());
    }

    #[test]
    fn the_window_keeps_its_size_and_frame() {
        let s = window_settings();
        assert!(s.resizable && s.decorations);
        assert!(
            s.min_size
                .is_some_and(|m| m.width <= s.size.width && m.height <= s.size.height)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_window_names_its_desktop_entry_on_linux() {
        assert_eq!(window_settings().platform_specific.application_id, APP_ID);
    }
}
