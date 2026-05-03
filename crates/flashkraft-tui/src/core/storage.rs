//! TUI Preference Persistence
//!
//! Stores user preferences (currently: theme selection) in a redb embedded
//! database under the OS config directory:
//!
//! | OS      | Path                                                        |
//! |---------|-------------------------------------------------------------|
//! | macOS   | `~/Library/Application Support/flashkraft/tui-prefs.db`    |
//! | Linux   | `~/.config/flashkraft/tui-prefs.db`                        |
//! | Windows | `%APPDATA%\flashkraft\tui-prefs.db`                        |
//!
//! # Usage
//!
//! ```no_run
//! use flashkraft_tui::core::storage::TuiStorage;
//! let storage = TuiStorage::open();          // never panics
//! storage.save_theme("Tokyo Night");
//! let name = storage.load_theme();           // → Some("Tokyo Night")
//! ```

use std::path::PathBuf;

use redb::ReadableDatabase;

/// redb table that holds all TUI preferences as byte key-value pairs.
const TABLE: redb::TableDefinition<&[u8], &[u8]> = redb::TableDefinition::new("tui_prefs");

/// redb key for the active theme name.
const KEY_THEME: &[u8] = b"tui_theme";

/// Redb-backed preference store for the TUI.
///
/// All operations are infallible from the caller's perspective: errors are
/// silently swallowed so a broken/missing DB never crashes the app.
pub struct TuiStorage {
    db: Option<redb::Database>,
}

impl TuiStorage {
    /// Open (or create) the preference database.
    ///
    /// Returns a storage instance even if the database could not be opened;
    /// in that case all reads return `None` and all writes are no-ops.
    pub fn open() -> Self {
        let db = Self::db_path().and_then(|path| redb::Database::create(path).ok());
        Self { db }
    }

    // ── Theme ───────────────────────────────────────────────────────────────────────

    /// Persist the active theme name.
    ///
    /// Silently ignores any I/O errors.
    pub fn save_theme(&self, name: &str) {
        if let Some(db) = &self.db {
            let Ok(write_txn) = db.begin_write() else {
                return;
            };
            {
                let Ok(mut table) = write_txn.open_table(TABLE) else {
                    return;
                };
                if table.insert(KEY_THEME, name.as_bytes()).is_err() {
                    return;
                }
            }
            let _ = write_txn.commit();
        }
    }

    /// Load the previously saved theme name.
    ///
    /// Returns `None` if nothing was saved yet or if an error occurs.
    pub fn load_theme(&self) -> Option<String> {
        let db = self.db.as_ref()?;
        let read_txn = db.begin_read().ok()?;
        let table = read_txn.open_table(TABLE).ok()?;
        let guard = table.get(KEY_THEME).ok()??;
        let bytes = guard.value();
        String::from_utf8(bytes.to_vec()).ok()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────────────

    fn db_path() -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push("flashkraft");
        std::fs::create_dir_all(&path).ok()?;
        path.push("tui-prefs.db");
        Some(path)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a throwaway `TuiStorage` backed by a temporary redb database.
    ///
    /// Returns both the storage and the `TempDir` handle -- the caller must
    /// keep `_tmp` alive so the underlying file is not deleted while tests run.
    fn temp_storage() -> (TuiStorage, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let db = redb::Database::create(tmp.path().join("test.redb")).ok();
        (TuiStorage { db }, tmp)
    }

    #[test]
    fn load_theme_returns_none_when_empty() {
        let (s, _tmp) = temp_storage();
        assert!(s.load_theme().is_none());
    }

    #[test]
    fn save_and_load_theme_roundtrip() {
        let (s, _tmp) = temp_storage();
        s.save_theme("Tokyo Night");
        assert_eq!(s.load_theme().as_deref(), Some("Tokyo Night"));
    }

    #[test]
    fn save_theme_overwrites_previous() {
        let (s, _tmp) = temp_storage();
        s.save_theme("Dracula");
        s.save_theme("Nord");
        assert_eq!(s.load_theme().as_deref(), Some("Nord"));
    }

    #[test]
    fn load_theme_with_no_db_returns_none() {
        let s = TuiStorage { db: None };
        assert!(s.load_theme().is_none());
    }

    #[test]
    fn save_theme_with_no_db_is_noop() {
        let s = TuiStorage { db: None };
        s.save_theme("Catppuccin Mocha"); // must not panic
    }

    #[test]
    fn roundtrip_all_preset_names() {
        use tui_file_explorer::Theme;
        let (s, _tmp) = temp_storage();
        for (name, _, _) in Theme::all_presets() {
            s.save_theme(name);
            assert_eq!(
                s.load_theme().as_deref(),
                Some(name),
                "roundtrip failed for preset '{}'",
                name,
            );
        }
    }
}
