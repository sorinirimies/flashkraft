//! GUI Utility Modules
//!
//! Contains utilities that are specific to the Iced desktop application.
//! Framework-agnostic utilities (debug logging macros) live in
//! `flashkraft-core::utils`.

pub mod icons;

// Re-export the icon helper for convenient access
pub use icons::icon;
