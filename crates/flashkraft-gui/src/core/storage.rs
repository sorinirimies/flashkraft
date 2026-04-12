//! Storage Module - Theme Persistence
//!
//! This module handles persistent storage of user preferences using redb,
//! an embedded database. Currently stores theme selection.

use iced::Theme;
use redb::{Database, ReadableDatabase, TableDefinition};
use std::path::PathBuf;

/// Map any error to a descriptive storage-error string and propagate with `?`.
macro_rules! map_storage_err {
    ($expr:expr, $ctx:literal) => {
        $expr.map_err(|e| format!("{}: {}", $ctx, e))?
    };
}

/// Key for storing the theme preference
const THEME_KEY: &[u8] = b"theme";

/// Table definition for the preferences table
const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("preferences");

/// Declare the canonical list of supported Iced themes.
/// Generates `theme_to_string`, `theme_from_string`, and `all_themes`.
macro_rules! theme_variants {
    ( $( $variant:ident ),+ $(,)? ) => {
        /// Convert a [`Theme`] to its storage string.
        fn theme_to_string(theme: &Theme) -> String {
            match theme {
                $( Theme::$variant => stringify!($variant).to_string(), )+
                _ => "Dark".to_string(),
            }
        }

        /// Convert a storage string back to a [`Theme`].
        fn theme_from_string(s: &str) -> Option<Theme> {
            match s {
                $( stringify!($variant) => Some(Theme::$variant), )+
                _ => None,
            }
        }

        /// All supported themes, in display order.
        pub fn all_themes() -> Vec<Theme> {
            vec![ $( Theme::$variant, )+ ]
        }
    };
}

theme_variants!(
    Dark,
    Light,
    Dracula,
    Nord,
    SolarizedLight,
    SolarizedDark,
    GruvboxLight,
    GruvboxDark,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
    TokyoNight,
    TokyoNightStorm,
    TokyoNightLight,
    KanagawaWave,
    KanagawaDragon,
    KanagawaLotus,
    Moonfly,
    Nightfly,
    Oxocarbon,
);

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
        theme_from_string(&theme_name)
    }

    /// Save the current theme
    ///
    /// Persists the theme selection to disk
    pub fn save_theme(&self, theme: &Theme) -> Result<(), String> {
        let theme_name = theme_to_string(theme);
        let write_txn = map_storage_err!(self.db.begin_write(), "Failed to save theme");
        {
            let mut table = map_storage_err!(write_txn.open_table(TABLE), "Failed to save theme");
            map_storage_err!(
                table.insert(THEME_KEY, theme_name.as_bytes()),
                "Failed to save theme"
            );
        }
        map_storage_err!(write_txn.commit(), "Failed to save theme");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_to_string() {
        assert_eq!(theme_to_string(&Theme::Dark), "Dark");
        assert_eq!(theme_to_string(&Theme::Light), "Light");
        assert_eq!(theme_to_string(&Theme::Dracula), "Dracula");
    }

    #[test]
    fn test_theme_from_string() {
        assert!(matches!(theme_from_string("Dark"), Some(Theme::Dark)));
        assert!(matches!(theme_from_string("Light"), Some(Theme::Light)));
        assert!(theme_from_string("Invalid").is_none());
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
            let name = theme_to_string(&theme);
            let restored = theme_from_string(&name);
            assert!(restored.is_some());
        }
    }

    #[test]
    fn test_all_themes_count() {
        let themes = all_themes();
        assert_eq!(themes.len(), 21);
    }

    #[test]
    fn test_all_themes_roundtrip() {
        for theme in all_themes() {
            let name = theme_to_string(&theme);
            let restored = theme_from_string(&name).expect("roundtrip should succeed");
            assert_eq!(
                theme_to_string(&restored),
                name,
                "roundtrip failed for {}",
                name
            );
        }
    }
}
