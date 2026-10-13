//! Commands Module — Core Async Side Effects
//!
//! Contains async helper functions that perform OS-level side effects.
//! These are shared between both the GUI and TUI frontends.
//!
//! ## What belongs here
//!
//! - Drive / block-device detection ([`drive_detection`])
//! - USB hotplug event stream ([`hotplug`])
//!
//! ## What does NOT belong here
//!
//! - File-picker dialogs (`rfd`) — those are GUI-only and live in
//!   `flashkraft-gui::core::commands::file_selection`.

pub mod demo;
pub mod drive_detection;
pub mod hotplug;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use drive_detection::{load_drives, load_drives_sync};
pub use hotplug::{watch_usb_events, UsbHotplugEvent};
