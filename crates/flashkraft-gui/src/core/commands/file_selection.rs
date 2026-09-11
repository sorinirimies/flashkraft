//! File Selection Command
//!
//! This module contains async functions for file selection dialogs.

use rfd::AsyncFileDialog;
use std::path::PathBuf;

/// Open a file dialog to select an image file
///
/// This async function shows a native file picker dialog and waits
/// for the user to select a file (or cancel).
///
/// # Arguments
///
/// * `start_dir` — directory the dialog should open in (typically the
///   directory of the last image the user picked, from persisted
///   settings). Falls back to the OS default when `None` or invalid.
///
/// # Returns
///
/// `Some(PathBuf)` if a file was selected, `None` if cancelled
pub async fn select_image_file(start_dir: Option<PathBuf>) -> Option<PathBuf> {
    let mut dialog = AsyncFileDialog::new()
        .set_title("Select Image File")
        .add_filter(
            "Image Files",
            &["img", "iso", "dmg", "zip", "gz", "xz", "raw"],
        );

    if let Some(dir) = start_dir.filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }

    dialog
        .add_filter("All Files", &["*"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // This test just ensures the module compiles correctly
        // Actual file dialog testing requires user interaction
    }
}
