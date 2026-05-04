//! GUI Settings Persistence
//!
//! Stores user preferences as a human-readable JSON file:
//!
//! | OS      | Path                                                              |
//! |---------|-------------------------------------------------------------------|
//! | macOS   | `~/Library/Application Support/flashkraft/gui-settings.json`     |
//! | Linux   | `~/.config/flashkraft/gui-settings.json`                         |
//! | Windows | `%APPDATA%\flashkraft\gui-settings.json`                          |
//!
//! # Theme catalogue
//!
//! All themes come directly from [`flashkraft_core::THEME_NAMES`] — **the
//! single source of truth** shared with the TUI.  Every core theme is
//! represented as an [`iced::Theme::Custom`] built via
//! [`theme_from_core`], so GUI and TUI always show exactly the same set.

use iced::Theme;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

// ── Core → Iced conversion ────────────────────────────────────────────────────

/// Build an Iced [`Theme::Custom`] from a core [`AppTheme`] preset.
///
/// This is the **only** theme-construction path in the GUI — every theme,
/// including those that happen to share a name with an Iced built-in, is
/// represented as a `Custom` theme so colours always match the core definition.
pub fn custom_theme_from_core(name: &str, t: &flashkraft_core::AppTheme) -> Theme {
    use iced::theme::{Custom, Palette};
    let palette = Palette {
        background: iced::Color::from_rgb8(t.background.r, t.background.g, t.background.b),
        text: iced::Color::from_rgb8(t.text_primary.r, t.text_primary.g, t.text_primary.b),
        primary: iced::Color::from_rgb8(t.accent.r, t.accent.g, t.accent.b),
        success: iced::Color::from_rgb8(t.success.r, t.success.g, t.success.b),
        warning: iced::Color::from_rgb8(t.warning.r, t.warning.g, t.warning.b),
        danger: iced::Color::from_rgb8(t.error.r, t.error.g, t.error.b),
    };
    Theme::Custom(Arc::new(Custom::new(name.to_string(), palette)))
}

/// Build the Iced theme for core preset at `index`.
///
/// Convenience wrapper used by [`all_themes`] and [`theme_from_string`].
fn theme_from_core(index: usize) -> Theme {
    let t = flashkraft_core::theme_by_index(index);
    custom_theme_from_core(flashkraft_core::THEME_NAMES[index], &t)
}

// ── Theme catalogue (single source of truth = core) ───────────────────────────

/// Return all supported GUI themes, derived entirely from
/// [`flashkraft_core::THEME_NAMES`].
///
/// The list is always in sync with the TUI — adding a theme to core
/// automatically makes it available here with no further changes.
pub fn all_themes() -> Vec<Theme> {
    (0..flashkraft_core::THEME_COUNT)
        .map(theme_from_core)
        .collect()
}

/// Convert an Iced [`Theme`] to the string name used for persistence.
///
/// Only `Theme::Custom` can be produced by this crate, so we extract the
/// name from the custom wrapper.  Fallback for any unexpected variant is
/// `"Default"`.
fn theme_to_string(theme: &Theme) -> String {
    match theme {
        Theme::Custom(c) => c.to_string(),
        // Should never occur — all our themes are Custom — but handle
        // gracefully in case a user somehow has a raw Iced variant saved.
        other => {
            // Best-effort: match against known Iced variant names
            let s = format!("{other:?}");
            // Iced Debug output is the variant name (e.g. "Dark", "Dracula")
            // Check if core knows this name; if so use it, else "Default".
            if flashkraft_core::theme_index_by_name(&s).is_some() {
                s
            } else {
                "Default".to_string()
            }
        }
    }
}

/// Reconstruct an Iced [`Theme`] from its persisted string name.
///
/// Looks up the name in the core catalogue and builds a `Theme::Custom`.
/// Returns `None` only if the name is not found in core.
fn theme_from_string(s: &str) -> Option<Theme> {
    let idx = flashkraft_core::theme_index_by_name(s)?;
    Some(theme_from_core(idx))
}

// ── Settings struct ───────────────────────────────────────────────────────────

/// Persistent GUI user preferences.
///
/// Serialised to / deserialised from `gui-settings.json` in the OS config dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    /// Name of the active Iced theme (e.g. `"TokyoNight"`).
    #[serde(default = "default_theme_name")]
    pub theme: String,
}

fn default_theme_name() -> String {
    "Default".to_string()
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            theme: default_theme_name(),
        }
    }
}

// ── Storage ───────────────────────────────────────────────────────────────────

/// Manages loading and saving of [`GuiSettings`].
#[derive(Debug)]
pub struct Storage {
    path: PathBuf,
    settings: GuiSettings,
}

impl Storage {
    /// Load settings from disk, creating the file with defaults if absent.
    ///
    /// Never panics — a missing or corrupt file silently yields defaults.
    pub fn new() -> Result<Self, String> {
        let path = Self::settings_path()?;
        let settings = Self::load_from(&path);
        Ok(Self { path, settings })
    }

    /// Return the active [`Theme`], or `None` if the stored name is unrecognised.
    pub fn load_theme(&self) -> Option<Theme> {
        theme_from_string(&self.settings.theme)
    }

    /// Persist a new theme choice to disk.
    ///
    /// Returns `Err` only if the file cannot be written.
    pub fn save_theme(&mut self, theme: &Theme) -> Result<(), String> {
        self.settings.theme = theme_to_string(theme);
        self.flush()
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    fn settings_path() -> Result<PathBuf, String> {
        let mut path =
            dirs::config_dir().ok_or_else(|| "Could not determine config directory".to_string())?;
        path.push("flashkraft");
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
        path.push("gui-settings.json");
        Ok(path)
    }

    fn load_from(path: &PathBuf) -> GuiSettings {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn flush(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.settings)
            .map_err(|e| format!("Failed to serialise settings: {e}"))?;
        std::fs::write(&self.path, json).map_err(|e| format!("Failed to write settings file: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_storage() -> (Storage, TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("gui-settings.json");
        let settings = GuiSettings::default();
        let storage = Storage { path, settings };
        (storage, tmp)
    }

    #[test]
    fn test_theme_to_string() {
        // All themes are now Custom — the name comes from the Custom wrapper
        let default_theme = theme_from_core(0);
        assert_eq!(theme_to_string(&default_theme), "Default");
        let dracula = theme_from_string("Dracula").unwrap();
        assert_eq!(theme_to_string(&dracula), "Dracula");
    }

    #[test]
    fn test_theme_from_string() {
        // All themes resolve to Theme::Custom
        let default_theme = theme_from_string("Default");
        assert!(default_theme.is_some());
        assert!(matches!(default_theme.unwrap(), Theme::Custom(_)));

        let dracula = theme_from_string("Dracula");
        assert!(dracula.is_some());
        assert!(matches!(dracula.unwrap(), Theme::Custom(_)));

        assert!(theme_from_string("Invalid").is_none());
    }

    #[test]
    fn test_roundtrip() {
        for theme in all_themes() {
            let name = theme_to_string(&theme);
            let restored = theme_from_string(&name);
            assert!(restored.is_some(), "roundtrip failed for {name}");
        }
    }

    #[test]
    fn test_all_themes_count() {
        // Exactly matches flashkraft_core::THEME_COUNT — single source of truth
        assert_eq!(all_themes().len(), flashkraft_core::THEME_COUNT);
        assert_eq!(all_themes().len(), 43);
    }

    #[test]
    fn test_save_and_load_theme() {
        let (mut storage, _tmp) = temp_storage();
        let tokyo = theme_from_core(flashkraft_core::theme_index_by_name("Tokyo Night").unwrap());
        storage.save_theme(&tokyo).unwrap();
        let loaded = storage.load_theme().unwrap();
        assert_eq!(theme_to_string(&loaded), "Tokyo Night");
    }

    #[test]
    fn test_save_writes_json_file() {
        let (mut storage, _tmp) = temp_storage();
        let nord = theme_from_string("Nord").unwrap();
        storage.save_theme(&nord).unwrap();
        let contents = std::fs::read_to_string(&storage.path).unwrap();
        assert!(contents.contains("Nord"), "JSON should contain theme name");
    }

    #[test]
    fn test_missing_file_yields_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.json");
        let settings = Storage::load_from(&path);
        assert_eq!(settings.theme, "Default");
    }

    #[test]
    fn test_corrupt_file_yields_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("corrupt.json");
        std::fs::write(&path, "not valid json {{{{").unwrap();
        let settings = Storage::load_from(&path);
        assert_eq!(settings.theme, "Default");
    }

    #[test]
    fn test_all_themes_roundtrip() {
        for theme in all_themes() {
            let name = theme_to_string(&theme);
            let restored = theme_from_string(&name).expect("roundtrip should succeed");
            assert_eq!(
                theme_to_string(&restored),
                name,
                "roundtrip failed for {name}"
            );
        }
    }

    #[test]
    fn every_core_theme_is_present_in_gui() {
        let gui_names: Vec<String> = all_themes().iter().map(theme_to_string).collect();
        for name in flashkraft_core::THEME_NAMES {
            assert!(
                gui_names.iter().any(|n| n == name),
                "Core theme '{name}' is missing from GUI all_themes()"
            );
        }
    }
}
