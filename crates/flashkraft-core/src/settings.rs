//! Read-only access to the settings files the GUI and TUI write.
//!
//! The front-ends each persist `gui-settings.json` / `tui-settings.json` in
//! the OS config directory (`…/flashkraft/`). Anything that only needs to
//! *read* them — the web dashboard, scripts — uses this module so the shape
//! is defined once and a corrupt or missing file degrades to defaults.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::flash_history::{FlashHistoryEntry, MAX_HISTORY_ENTRIES};

/// File written by the Ratatui front-end.
pub const TUI_SETTINGS_FILE: &str = "tui-settings.json";
/// File written by the Iced front-end.
pub const GUI_SETTINGS_FILE: &str = "gui-settings.json";

/// The subset of a settings file other tools care about.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SharedSettings {
    /// Theme name as saved by the TUI (the GUI stores an Iced theme name).
    #[serde(default)]
    pub theme: String,
    /// Successful flashes, newest first.
    #[serde(default)]
    pub flash_history: Vec<FlashHistoryEntry>,
}

/// `<OS config dir>/flashkraft`, if the OS has a config dir.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("flashkraft"))
}

/// Parse a settings document; anything unparsable yields defaults.
pub fn parse(json: &str) -> SharedSettings {
    serde_json::from_str(json).unwrap_or_default()
}

/// Read one settings file (`None` if absent or unreadable).
pub fn read_file(path: &Path) -> Option<SharedSettings> {
    std::fs::read_to_string(path).ok().map(|s| parse(&s))
}

/// Combine several histories: newest first, exact duplicates removed (the
/// same flash recorded by both front-ends), capped at [`MAX_HISTORY_ENTRIES`].
pub fn merge_history(
    lists: impl IntoIterator<Item = Vec<FlashHistoryEntry>>,
) -> Vec<FlashHistoryEntry> {
    let mut all: Vec<FlashHistoryEntry> = lists.into_iter().flatten().collect();
    all.sort_by_key(|e| std::cmp::Reverse(e.completed_at_unix));
    all.dedup();
    all.truncate(MAX_HISTORY_ENTRIES);
    all
}

/// Merged flash history of both front-ends found in `dir`.
pub fn load_history_from(dir: &Path) -> Vec<FlashHistoryEntry> {
    merge_history(
        [TUI_SETTINGS_FILE, GUI_SETTINGS_FILE]
            .iter()
            .filter_map(|f| read_file(&dir.join(f)))
            .map(|s| s.flash_history),
    )
}

/// Merged flash history from the real config directory.
pub fn load_history() -> Vec<FlashHistoryEntry> {
    config_dir()
        .map(|d| load_history_from(&d))
        .unwrap_or_default()
}

/// The TUI's saved theme name found in `dir`, if any.
pub fn saved_theme_name_from(dir: &Path) -> Option<String> {
    read_file(&dir.join(TUI_SETTINGS_FILE))
        .map(|s| s.theme)
        .filter(|t| !t.is_empty())
}

/// The TUI's saved theme name from the real config directory.
pub fn saved_theme_name() -> Option<String> {
    config_dir().and_then(|d| saved_theme_name_from(&d))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, at: u64) -> FlashHistoryEntry {
        FlashHistoryEntry {
            image_path: path.into(),
            drive_label: "Stick — 32 GB".into(),
            size_mb: 700.0,
            completed_at_unix: at,
        }
    }

    #[test]
    fn parse_tolerates_garbage_and_missing_fields() {
        assert!(parse("not json").flash_history.is_empty());
        assert!(parse("{}").theme.is_empty());
        let s = parse(r#"{"theme":"Nord","extra":1}"#);
        assert_eq!(s.theme, "Nord");
    }

    #[test]
    fn merge_sorts_newest_first_dedups_and_caps() {
        let a = vec![entry("a.iso", 10), entry("b.iso", 30)];
        let b = vec![entry("b.iso", 30), entry("c.iso", 20)];
        let m = merge_history([a, b]);
        let order: Vec<_> = m.iter().map(|e| e.image_path.as_str()).collect();
        assert_eq!(order, ["b.iso", "c.iso", "a.iso"]);

        let many = (0..(MAX_HISTORY_ENTRIES as u64 + 5))
            .map(|i| entry(&format!("{i}.iso"), i))
            .collect::<Vec<_>>();
        assert_eq!(merge_history([many]).len(), MAX_HISTORY_ENTRIES);
    }

    #[test]
    fn loads_both_files_from_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(TUI_SETTINGS_FILE),
            r#"{"theme":"Dracula","flash_history":[{"image_path":"t.iso","drive_label":"x","size_mb":1.0,"completed_at_unix":5}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join(GUI_SETTINGS_FILE),
            r#"{"theme":"TokyoNight","flash_history":[{"image_path":"g.iso","drive_label":"x","size_mb":1.0,"completed_at_unix":9}]}"#,
        )
        .unwrap();
        let h = load_history_from(dir.path());
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].image_path, "g.iso");
        assert_eq!(
            saved_theme_name_from(dir.path()).as_deref(),
            Some("Dracula")
        );
    }

    #[test]
    fn empty_directory_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_history_from(dir.path()).is_empty());
        assert!(saved_theme_name_from(dir.path()).is_none());
    }
}
