//! TUI Message Types
//!
//! Pure data types shared across the TUI crate: screen identifiers,
//! input modes, flash-event aliases, and file-operation helpers.
//! Extracted from the former monolithic `app.rs` to keep the
//! state module focused on behaviour.

// ---------------------------------------------------------------------------
// Flash progress events
// ---------------------------------------------------------------------------

/// Progress events produced by the background flash task.
///
/// This is a type alias for [`flashkraft_core::FlashUpdate`] — the
/// normalised frontend event defined in core so both the TUI and GUI share
/// the same representation.
pub use flashkraft_core::FlashUpdate as FlashEvent;

// ---------------------------------------------------------------------------
// USB content entry (shown on the completion screen)
// ---------------------------------------------------------------------------

/// A single entry in the post-flash USB content listing.
#[derive(Debug, Clone)]
pub struct UsbEntry {
    pub name: String,
    pub size_bytes: u64,
    pub is_dir: bool,
    /// Nesting depth for tree-style display (0 = root)
    pub depth: usize,
}

// ---------------------------------------------------------------------------
// Application screens
// ---------------------------------------------------------------------------

/// The currently active screen / step.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum AppScreen {
    /// Step 1 — type (or paste) the path to an OS image.
    #[default]
    SelectImage,
    /// Step 1b — interactive file-browser overlay for picking the OS image.
    BrowseImage,
    /// Step 2 — choose a USB drive from the detected list.
    SelectDrive,
    /// Step 2½ — pie-chart overview of the selected drive's storage.
    DriveInfo,
    /// Step 3 — confirmation dialog before writing.
    ConfirmFlash,
    /// Step 4 — flash operation in progress (tui-slider).
    Flashing,
    /// Step 5 — flash complete; show USB contents + pie-chart.
    Complete,
    /// Error screen — displayed whenever a fatal error occurs.
    Error,
}

// ---------------------------------------------------------------------------
// Input mode
// ---------------------------------------------------------------------------

/// Whether the app is currently capturing keyboard input for a text field.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// Normal navigation mode.
    #[default]
    Normal,
    /// Typing into the image-path text field.
    Editing,
}

// ---------------------------------------------------------------------------
// File-explorer clipboard / operation mode
// ---------------------------------------------------------------------------

/// Whether a file is being copied or moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipOp {
    Copy,
    Cut,
}

/// Pending clipboard entry for the file explorer.
#[derive(Debug, Clone)]
pub struct FileClipboard {
    pub path: std::path::PathBuf,
    pub op: ClipOp,
}

/// Confirmation modal state for the file explorer.
#[derive(Debug, Default, Clone)]
pub enum FileOpMode {
    #[default]
    Normal,
    ConfirmDelete(std::path::PathBuf),
    ConfirmOverwrite {
        src: std::path::PathBuf,
        dst: std::path::PathBuf,
        op: ClipOp,
    },
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // -- Default impls --------------------------------------------------------

    #[test]
    fn app_screen_default_is_select_image() {
        assert_eq!(AppScreen::default(), AppScreen::SelectImage);
    }

    #[test]
    fn input_mode_default_is_normal() {
        assert_eq!(InputMode::default(), InputMode::Normal);
    }

    #[test]
    fn file_op_mode_default_is_normal() {
        assert!(matches!(FileOpMode::default(), FileOpMode::Normal));
    }

    // -- ClipOp variants ------------------------------------------------------

    #[test]
    fn clip_op_copy_variant_exists() {
        let op = ClipOp::Copy;
        assert_eq!(op, ClipOp::Copy);
    }

    #[test]
    fn clip_op_cut_variant_exists() {
        let op = ClipOp::Cut;
        assert_eq!(op, ClipOp::Cut);
    }

    #[test]
    fn clip_op_copy_differs_from_cut() {
        assert_ne!(ClipOp::Copy, ClipOp::Cut);
    }

    // -- UsbEntry construction ------------------------------------------------

    #[test]
    fn usb_entry_can_be_constructed_with_all_fields() {
        let entry = UsbEntry {
            name: "ubuntu-24.04.iso".into(),
            size_bytes: 4_200_000_000,
            is_dir: false,
            depth: 0,
        };
        assert_eq!(entry.name, "ubuntu-24.04.iso");
        assert_eq!(entry.size_bytes, 4_200_000_000);
        assert!(!entry.is_dir);
        assert_eq!(entry.depth, 0);
    }

    #[test]
    fn usb_entry_directory() {
        let entry = UsbEntry {
            name: "boot".into(),
            size_bytes: 0,
            is_dir: true,
            depth: 1,
        };
        assert!(entry.is_dir);
        assert_eq!(entry.depth, 1);
    }

    // -- AppScreen variants ---------------------------------------------------

    #[test]
    fn app_screen_all_variants_compile() {
        // Constructing every variant proves the enum is exhaustive as expected.
        let screens = [
            AppScreen::SelectImage,
            AppScreen::BrowseImage,
            AppScreen::SelectDrive,
            AppScreen::DriveInfo,
            AppScreen::ConfirmFlash,
            AppScreen::Flashing,
            AppScreen::Complete,
            AppScreen::Error,
        ];
        assert_eq!(screens.len(), 8);
    }

    // -- AppScreen derives Clone + PartialEq + Eq -----------------------------

    #[test]
    fn app_screen_clone_and_eq() {
        let a = AppScreen::Flashing;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn app_screen_ne_for_different_variants() {
        assert_ne!(AppScreen::SelectImage, AppScreen::Error);
    }

    // -- InputMode variants ---------------------------------------------------

    #[test]
    fn input_mode_editing_differs_from_normal() {
        assert_ne!(InputMode::Normal, InputMode::Editing);
    }

    // -- FileClipboard --------------------------------------------------------

    #[test]
    fn file_clipboard_stores_path_and_op() {
        let cb = FileClipboard {
            path: PathBuf::from("/tmp/file.txt"),
            op: ClipOp::Copy,
        };
        assert_eq!(cb.path, PathBuf::from("/tmp/file.txt"));
        assert_eq!(cb.op, ClipOp::Copy);
    }

    // -- FileOpMode -----------------------------------------------------------

    #[test]
    fn file_op_mode_confirm_delete() {
        let mode = FileOpMode::ConfirmDelete(PathBuf::from("/tmp/delete_me"));
        assert!(
            matches!(mode, FileOpMode::ConfirmDelete(p) if p.as_path() == std::path::Path::new("/tmp/delete_me"))
        );
    }

    #[test]
    fn file_op_mode_confirm_overwrite() {
        let mode = FileOpMode::ConfirmOverwrite {
            src: PathBuf::from("/a"),
            dst: PathBuf::from("/b"),
            op: ClipOp::Cut,
        };
        assert!(matches!(
            mode,
            FileOpMode::ConfirmOverwrite {
                op: ClipOp::Cut,
                ..
            }
        ));
    }
}
