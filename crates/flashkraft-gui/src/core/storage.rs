//! Storage Module - Theme Persistence
//!
//! This module handles persistent storage of user preferences using redb,
//! an embedded database. Currently stores theme selection.

use iced::Theme;
use redb::{Database, ReadableDatabase, TableDefinition};
use std::path::PathBuf;

/// Key for storing the theme preference
const THEME_KEY: &[u8] = b"theme";

/// Table definition for the preferences table
const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("preferences");

/// Storage manager for application preferences
pub struct Storage {
    db: Database,
}

impl std::fmt::Debug for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Storage")
            .field("db", &"<redb::Database>")
            .finish()
    }
}

impl Storage {
    /// Create a new storage instance
    ///
    /// Opens or creates the redb database in the user's config directory
    pub fn new() -> Result<Self, String> {
        let db_path = Self::get_db_path()?;
        let db =
            Database::create(db_path).map_err(|e| format!("Failed to open database: {}", e))?;
        Ok(Self { db })
    }

    /// Get the database path
    ///
    /// Uses the appropriate config directory based on the OS
    fn get_db_path() -> Result<PathBuf, String> {
        let mut path =
            dirs::config_dir().ok_or_else(|| "Could not determine config directory".to_string())?;
        path.push("flashkraft");
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
        path.push("preferences.db");
        Ok(path)
    }

    /// Load the saved theme
    ///
    /// Returns the saved theme or None if no theme is saved
    pub fn load_theme(&self) -> Option<Theme> {
        let read_txn = self.db.begin_read().ok()?;
        let table = read_txn.open_table(TABLE).ok()?;
        let value = table.get(THEME_KEY).ok()??;
        let theme_name = String::from_utf8(value.value().to_vec()).ok()?;
        Self::theme_from_string(&theme_name)
    }

    /// Save the current theme
    ///
    /// Persists the theme selection to disk
    pub fn save_theme(&self, theme: &Theme) -> Result<(), String> {
        let theme_name = Self::theme_to_string(theme);
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| format!("Failed to save theme: {}", e))?;
        {
            let mut table = write_txn
                .open_table(TABLE)
                .map_err(|e| format!("Failed to save theme: {}", e))?;
            table
                .insert(THEME_KEY, theme_name.as_bytes())
                .map_err(|e| format!("Failed to save theme: {}", e))?;
        }
        write_txn
            .commit()
            .map_err(|e| format!("Failed to save theme: {}", e))?;
        Ok(())
    }

    /// Convert a Theme to a string for storage
    fn theme_to_string(theme: &Theme) -> String {
        match theme {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Dracula => "Dracula",
            Theme::Nord => "Nord",
            Theme::SolarizedLight => "SolarizedLight",
            Theme::SolarizedDark => "SolarizedDark",
            Theme::GruvboxLight => "GruvboxLight",
            Theme::GruvboxDark => "GruvboxDark",
            Theme::CatppuccinLatte => "CatppuccinLatte",
            Theme::CatppuccinFrappe => "CatppuccinFrappe",
            Theme::CatppuccinMacchiato => "CatppuccinMacchiato",
            Theme::CatppuccinMocha => "CatppuccinMocha",
            Theme::TokyoNight => "TokyoNight",
            Theme::TokyoNightStorm => "TokyoNightStorm",
            Theme::TokyoNightLight => "TokyoNightLight",
            Theme::KanagawaWave => "KanagawaWave",
            Theme::KanagawaDragon => "KanagawaDragon",
            Theme::KanagawaLotus => "KanagawaLotus",
            Theme::Moonfly => "Moonfly",
            Theme::Nightfly => "Nightfly",
            Theme::Oxocarbon => "Oxocarbon",
            _ => "Dark",
        }
        .to_string()
    }

    /// Convert a string to a Theme
    fn theme_from_string(s: &str) -> Option<Theme> {
        match s {
            "Dark" => Some(Theme::Dark),
            "Light" => Some(Theme::Light),
            "Dracula" => Some(Theme::Dracula),
            "Nord" => Some(Theme::Nord),
            "SolarizedLight" => Some(Theme::SolarizedLight),
            "SolarizedDark" => Some(Theme::SolarizedDark),
            "GruvboxLight" => Some(Theme::GruvboxLight),
            "GruvboxDark" => Some(Theme::GruvboxDark),
            "CatppuccinLatte" => Some(Theme::CatppuccinLatte),
            "CatppuccinFrappe" => Some(Theme::CatppuccinFrappe),
            "CatppuccinMacchiato" => Some(Theme::CatppuccinMacchiato),
            "CatppuccinMocha" => Some(Theme::CatppuccinMocha),
            "TokyoNight" => Some(Theme::TokyoNight),
            "TokyoNightStorm" => Some(Theme::TokyoNightStorm),
            "TokyoNightLight" => Some(Theme::TokyoNightLight),
            "KanagawaWave" => Some(Theme::KanagawaWave),
            "KanagawaDragon" => Some(Theme::KanagawaDragon),
            "KanagawaLotus" => Some(Theme::KanagawaLotus),
            "Moonfly" => Some(Theme::Moonfly),
            "Nightfly" => Some(Theme::Nightfly),
            "Oxocarbon" => Some(Theme::Oxocarbon),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_to_string() {
        assert_eq!(Storage::theme_to_string(&Theme::Dark), "Dark");
        assert_eq!(Storage::theme_to_string(&Theme::Light), "Light");
        assert_eq!(Storage::theme_to_string(&Theme::Dracula), "Dracula");
    }

    #[test]
    fn test_theme_from_string() {
        assert!(matches!(
            Storage::theme_from_string("Dark"),
            Some(Theme::Dark)
        ));
        assert!(matches!(
            Storage::theme_from_string("Light"),
            Some(Theme::Light)
        ));
        assert!(Storage::theme_from_string("Invalid").is_none());
    }

    #[test]
    fn test_roundtrip() {
        let themes = vec![
            Theme::Dark,
            Theme::Light,
            Theme::Dracula,
            Theme::Nord,
            Theme::CatppuccinMocha,
        ];

        for theme in themes {
            let name = Storage::theme_to_string(&theme);
            let restored = Storage::theme_from_string(&name);
            assert!(restored.is_some());
        }
    }
}
