//! Native window behaviour beyond the window settings in `lib.rs`.
//!
//! * **macOS**: winit ignores a window icon, so [`setup`] sets the Dock icon (a bare
//!   binary outside a `.app` bundle, e.g. from `cargo install`, would have none), and adds
//!   the Window menu whose absence disables *Minimize*, *Zoom*, *Move & Resize* and the
//!   tiling shortcuts.
//! * **Everywhere**: [`brand_image`] is the icon shown in the header and About dialog.
//!
//! Pixels come from `assets/icons/flashkraft.svg` via `just icons`.

#[cfg(target_os = "macos")]
pub mod macos;

/// One-time window setup as an iced task (a no-op where the platform needs nothing).
#[cfg(target_os = "macos")]
pub fn setup<M: Send + 'static>() -> iced::Task<M> {
    iced::window::oldest()
        .and_then(|id| {
            iced::window::run(id, |_window| {
                macos::install_window_menu();
                macos::set_dock_icon(DOCK_ICON_PNG);
            })
        })
        .discard()
}

#[cfg(not(target_os = "macos"))]
pub fn setup<M: Send + 'static>() -> iced::Task<M> {
    iced::Task::none()
}

#[cfg(target_os = "macos")]
const DOCK_ICON_PNG: &[u8] = include_bytes!("../../assets/flashkraft-macos-512.png");

/// The app icon as an image widget handle (header, About). Built once: iced uploads a
/// new texture for every fresh RGBA handle, so a handle made per frame would leak.
#[must_use]
pub fn brand_image() -> iced::widget::image::Handle {
    static HANDLE: std::sync::OnceLock<iced::widget::image::Handle> = std::sync::OnceLock::new();
    HANDLE
        .get_or_init(|| {
            iced::widget::image::Handle::from_rgba(
                crate::ICON_SIZE,
                crate::ICON_SIZE,
                crate::ICON_RGBA,
            )
        })
        .clone()
}

#[cfg(test)]
mod tests {
    #[test]
    fn brand_image_is_built_once() {
        // A fresh RGBA handle gets a fresh texture id; the cache must hand out the same one.
        assert_eq!(super::brand_image(), super::brand_image());
    }
}
