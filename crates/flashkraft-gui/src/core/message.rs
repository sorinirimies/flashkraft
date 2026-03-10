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
