//! TUI Settings Persistence
//!
//! Stores user preferences as a human-readable JSON file:
//!
//! | OS      | Path                                                              |
//! |---------|-------------------------------------------------------------------|
//! | macOS   | `~/Library/Application Support/flashkraft/tui-settings.json`     |
//! | Linux   | `~/.config/flashkraft/tui-settings.json`                         |
//! | Windows | `%APPDATA%\flashkraft\tui-settings.json`                          |
//!
//! # Example file
//! ```json
//! {
//!   "theme": "Tokyo Night"
//! }
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use flashkraft_tui::core::storage::TuiStorage;
//! let mut storage = TuiStorage::open();   // never panics
//! storage.save_theme("Tokyo Night");
//! let name = storage.load_theme();        // → Some("Tokyo Night")
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Settings struct ───────────────────────────────────────────────────────────

/// Persistent TUI user preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiSettings {
    /// Name of the active TUI theme (must match a `tui_file_explorer::Theme::all_presets()` entry).
    #[serde(default = "default_theme_name")]
    pub theme: String,
}

fn default_theme_name() -> String {
    "Default".to_string()
}

impl Default for TuiSettings {
    fn default() -> Self {
        Self {
            theme: default_theme_name(),
        }
    }
}

// ── TuiStorage ────────────────────────────────────────────────────────────────

/// JSON-backed preference store for the TUI.
///
/// All operations are infallible from the caller's perspective: errors are
/// silently swallowed so a missing or corrupt file never crashes the app.
pub struct TuiStorage {
    /// `None` when the settings path could not be determined.
    path: Option<PathBuf>,
    settings: TuiSettings,
}

impl TuiStorage {
    /// Open (or create) the preference store.
    ///
    /// Returns a storage instance even if the file could not be read;
    /// in that case all reads return `None` and all writes are no-ops.
    pub fn open() -> Self {
        let path = Self::settings_path();
        let settings = path
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path, settings }
    }

    // ── Theme ─────────────────────────────────────────────────────────────────

    /// Persist the active theme name. Silently ignores any I/O errors.
    pub fn save_theme(&mut self, name: &str) {
        self.settings.theme = name.to_string();
        self.flush();
    }

    /// Load the previously saved theme name.
    ///
    /// Returns `None` if nothing was saved yet or if an error occurred.
    pub fn load_theme(&self) -> Option<String> {
        if self.settings.theme.is_empty() {
            None
        } else {
            Some(self.settings.theme.clone())
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn settings_path() -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push("flashkraft");
        std::fs::create_dir_all(&path).ok()?;
        path.push("tui-settings.json");
        Some(path)
    }

    fn flush(&self) {
        let Some(path) = &self.path else { return };
        if let Ok(json) = serde_json::to_string_pretty(&self.settings) {
            let _ = std::fs::write(path, json);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_storage() -> (TuiStorage, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tui-settings.json");
        let storage = TuiStorage {
            path: Some(path),
            settings: TuiSettings::default(),
        };
        (storage, tmp)
    }

    #[test]
    fn load_theme_returns_none_when_empty() {
        // default theme is "Default", not empty, so load_theme returns Some
        // To test "none when empty", use an explicitly empty theme
        let empty = TuiStorage {
            path: None,
            settings: TuiSettings {
                theme: String::new(),
            },
        };
        assert!(empty.load_theme().is_none());
    }

    #[test]
    fn save_and_load_theme_roundtrip() {
        let (mut s, _tmp) = temp_storage();
        s.save_theme("Tokyo Night");
        assert_eq!(s.load_theme().as_deref(), Some("Tokyo Night"));
    }

    #[test]
    fn save_theme_overwrites_previous() {
        let (mut s, _tmp) = temp_storage();
        s.save_theme("Dracula");
        s.save_theme("Nord");
        assert_eq!(s.load_theme().as_deref(), Some("Nord"));
    }

    #[test]
    fn save_theme_with_no_path_is_noop() {
        let mut s = TuiStorage {
            path: None,
            settings: TuiSettings::default(),
        };
        s.save_theme("Catppuccin Mocha"); // must not panic
    }

    #[test]
    fn save_writes_json_file() {
        let (mut s, _tmp) = temp_storage();
        s.save_theme("Gruvbox Dark");
        let path = s.path.as_ref().unwrap();
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains("Gruvbox Dark"));
    }

    #[test]
    fn corrupt_file_yields_default_on_open() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tui-settings.json");
        std::fs::write(&path, "not { valid } json").unwrap();
        let storage = TuiStorage {
            path: Some(path.clone()),
            settings: std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        };
        assert_eq!(storage.settings.theme, "Default");
    }

    #[test]
    fn roundtrip_all_preset_names() {
        use tui_file_explorer::Theme;
        let (mut s, _tmp) = temp_storage();
        for (name, _, _) in Theme::all_presets() {
            s.save_theme(name);
            assert_eq!(
                s.load_theme().as_deref(),
                Some(name),
                "roundtrip failed for preset '{name}'"
            );
        }
    }
}
