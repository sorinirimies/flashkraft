//! Utility modules for FlashKraft Core
//!
//! Contains debug-logging macros shared across all crates.
//! GUI-specific utilities (icon mappers, etc.) live in `flashkraft-gui`.

pub mod logger;
pub use logger::fmt_bytes;

// Macros defined with #[macro_export] in logger.rs are automatically
// available at the crate root as `flashkraft_core::debug_log!` etc.
