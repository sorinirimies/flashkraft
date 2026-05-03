//! Message - Events in The Elm Architecture
//!
//! This module defines all possible messages (events) that can occur
//! in the FlashKraft application. Messages are the only way to trigger
//! state changes, making the application predictable and debuggable.

use std::path::PathBuf;

use crate::domain::DriveInfo;
use iced::Theme;

/// All possible messages in the application
///
/// Messages represent events that can occur, either from user interactions
/// or as results of asynchronous operations (Commands).
#[derive(Debug, Clone)]
pub enum Message {
    // ========================================================================
    // User Interaction Messages
    // ========================================================================
    /// User clicked the "Select Image" button
    SelectImageClicked,

    /// User clicked the "Refresh Drives" button
    RefreshDrivesClicked,

    /// A USB device was connected or disconnected — triggers re-enumeration.
    ///
    /// Emitted by the hotplug subscription in [`crate::core::state`] and
    /// handled by immediately re-running drive detection, exactly as if the
    /// user had pressed Refresh.
    UsbHotplugDetected,

    /// User clicked on a specific target drive
    TargetDriveClicked(DriveInfo),

    /// User clicked to open the device selection view
    OpenDeviceSelection,

    /// User clicked to close the device selection view
    CloseDeviceSelection,

    /// User clicked the "Flash" button
    FlashClicked,

    /// User clicked Flash but the process is not privileged — attempt to
    /// re-exec via pkexec / sudo so the user gets a password prompt, then
    /// restart the app with the same selections intact.
    EscalateAndFlash,

    /// User clicked the "Reset" button (start over)
    ResetClicked,

    /// User clicked the "Cancel" button
    CancelClicked,

    /// User clicked "Cancel" during flash operation
    CancelFlash,

    // ========================================================================
    // Animation Messages
    // ========================================================================
    /// Animation tick for progress bar effects
    AnimationTick,

    // ========================================================================
    // Async Result Messages
    // ========================================================================
    /// Result from async image file selection
    ///
    /// Contains `Some(path)` if user selected a file, `None` if cancelled
    ImageSelected(Option<PathBuf>),

    /// Result from async drive detection
    ///
    /// Contains a list of detected drives
    DrivesRefreshed(Vec<DriveInfo>),

    /// Progress update from flash subscription
    ///
    /// Contains (progress 0.0-1.0, bytes_written, speed_mb_per_sec)
    FlashProgressUpdate(f32, u64, f32),

    /// Verification read-back progress update.
    ///
    /// Contains (overall 0.0–1.0 across both passes, phase, bytes_read, total_bytes, speed_mb_s)
    VerifyProgressUpdate(f32, &'static str, u64, u64, f32),

    /// Status message from flash operation
    Status(String),

    /// Result from async flash operation
    ///
    /// Contains `Ok(())` on success or `Err(message)` on failure
    FlashCompleted(Result<(), String>),

    /// User changed the application theme
    ThemeChanged(Theme),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── Debug + Clone ────────────────────────────────────────────────────

    #[test]
    fn message_implements_debug() {
        let msg = Message::SelectImageClicked;
        // Debug formatting should not panic.
        let debug_str = format!("{:?}", msg);
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn message_implements_clone() {
        let msg = Message::AnimationTick;
        let cloned = msg.clone();
        // Both should produce identical debug output.
        assert_eq!(format!("{:?}", msg), format!("{:?}", cloned));
    }

    // ── Unit variant construction ────────────────────────────────────────

    #[test]
    fn select_image_clicked_can_be_constructed() {
        let _msg = Message::SelectImageClicked;
    }

    #[test]
    fn animation_tick_can_be_constructed() {
        let _msg = Message::AnimationTick;
    }

    // ── Tuple variant construction ───────────────────────────────────────

    #[test]
    fn flash_progress_update_holds_values() {
        let msg = Message::FlashProgressUpdate(0.5, 1024, 10.0);
        match msg {
            Message::FlashProgressUpdate(progress, bytes, speed) => {
                assert!((progress - 0.5).abs() < f32::EPSILON);
                assert_eq!(bytes, 1024);
                assert!((speed - 10.0).abs() < f32::EPSILON);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn flash_completed_ok() {
        let msg = Message::FlashCompleted(Ok(()));
        match msg {
            Message::FlashCompleted(Ok(())) => {} // success
            _ => panic!("expected FlashCompleted(Ok(()))"),
        }
    }

    #[test]
    fn flash_completed_err() {
        let msg = Message::FlashCompleted(Err("test".into()));
        match msg {
            Message::FlashCompleted(Err(e)) => assert_eq!(e, "test"),
            _ => panic!("expected FlashCompleted(Err(..))"),
        }
    }

    #[test]
    fn status_holds_string() {
        let msg = Message::Status("test".into());
        match msg {
            Message::Status(s) => assert_eq!(s, "test"),
            _ => panic!("expected Status"),
        }
    }

    // ── ThemeChanged ─────────────────────────────────────────────────────

    #[test]
    fn theme_changed_holds_theme() {
        let msg = Message::ThemeChanged(Theme::Light);
        match msg {
            Message::ThemeChanged(t) => {
                // Just verify the variant can hold a theme.
                let _ = format!("{:?}", t);
            }
            _ => panic!("expected ThemeChanged"),
        }
    }

    // ── ImageSelected ────────────────────────────────────────────────────

    #[test]
    fn image_selected_none() {
        let msg = Message::ImageSelected(None);
        match msg {
            Message::ImageSelected(None) => {} // ok
            _ => panic!("expected ImageSelected(None)"),
        }
    }

    #[test]
    fn image_selected_some_path() {
        let msg = Message::ImageSelected(Some(PathBuf::from("/test")));
        match msg {
            Message::ImageSelected(Some(p)) => assert_eq!(p, PathBuf::from("/test")),
            _ => panic!("expected ImageSelected(Some(..))"),
        }
    }
}
