//! Platform-agnostic theme types and presets.
//!
//! Defines [`AppTheme`] — the single source of truth for every colour used by
//! both the Iced GUI and the Ratatui TUI.  Each frontend converts these
//! framework-agnostic [`Rgb`] values to its own colour type at render time.

pub mod presets;
pub mod types;

pub use presets::{theme_by_index, theme_index_by_name, THEME_COUNT, THEME_NAMES};
pub use types::{AppTheme, Rgb};
