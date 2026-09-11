//! Flash History
//!
//! A small, append-only record of past flash operations, shared by both the
//! GUI and TUI settings persistence so the on-disk shape (and the "most
//! recent N entries" trimming rule) stays identical across frontends.

use serde::{Deserialize, Serialize};

/// Maximum number of entries retained in a flash history list.
///
/// Older entries are dropped once this limit is exceeded so the settings
/// file never grows unbounded on machines that flash frequently.
pub const MAX_HISTORY_ENTRIES: usize = 20;

/// One completed (successful) flash operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlashHistoryEntry {
    /// Path to the image file that was flashed.
    pub image_path: String,
    /// Human-readable label of the target drive (e.g. "Kingston DataTraveler — 32 GB").
    pub drive_label: String,
    /// Size of the image in megabytes, for quick display without re-reading the file.
    pub size_mb: f64,
    /// Unix timestamp (seconds) of when the flash completed successfully.
    pub completed_at_unix: u64,
}

impl FlashHistoryEntry {
    /// Build a new entry stamped with the current wall-clock time.
    ///
    /// Falls back to `0` if the system clock is somehow before the Unix
    /// epoch (never happens in practice, but avoids a panic/unwrap).
    pub fn new(
        image_path: impl Into<String>,
        drive_label: impl Into<String>,
        size_mb: f64,
    ) -> Self {
        let completed_at_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            image_path: image_path.into(),
            drive_label: drive_label.into(),
            size_mb,
            completed_at_unix,
        }
    }
}

/// Push `entry` to the front of `history` (most recent first) and trim to
/// [`MAX_HISTORY_ENTRIES`].
pub fn push_entry(history: &mut Vec<FlashHistoryEntry>, entry: FlashHistoryEntry) {
    history.insert(0, entry);
    history.truncate(MAX_HISTORY_ENTRIES);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_entry_has_fields_set() {
        let e = FlashHistoryEntry::new("/tmp/image.iso", "USB Drive — 8 GB", 1024.0);
        assert_eq!(e.image_path, "/tmp/image.iso");
        assert_eq!(e.drive_label, "USB Drive — 8 GB");
        assert_eq!(e.size_mb, 1024.0);
        assert!(e.completed_at_unix > 0);
    }

    #[test]
    fn push_entry_prepends_most_recent_first() {
        let mut history = Vec::new();
        push_entry(
            &mut history,
            FlashHistoryEntry::new("a.img", "Drive A", 1.0),
        );
        push_entry(
            &mut history,
            FlashHistoryEntry::new("b.img", "Drive B", 2.0),
        );

        assert_eq!(history[0].image_path, "b.img");
        assert_eq!(history[1].image_path, "a.img");
    }

    #[test]
    fn push_entry_trims_to_max_len() {
        let mut history = Vec::new();
        for i in 0..(MAX_HISTORY_ENTRIES + 5) {
            push_entry(
                &mut history,
                FlashHistoryEntry::new(format!("img-{i}.img"), "Drive", 1.0),
            );
        }
        assert_eq!(history.len(), MAX_HISTORY_ENTRIES);
        // Most recent entry (last pushed) must be first.
        assert_eq!(
            history[0].image_path,
            format!("img-{}.img", MAX_HISTORY_ENTRIES + 4)
        );
    }
}
