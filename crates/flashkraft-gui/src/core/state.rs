//! Application State Module
//!
//! NOTE: `hotplug_stream` is defined at the bottom of this file as a free
//! function so it can be passed as a `fn() -> impl Stream` pointer to
//! `Subscription::run`.
//!
//! ## Hotplug
//!
//! [`FlashKraft::subscription`] includes a persistent USB hotplug watch
//! powered by `watch_usb_events()` (inotify on Linux, FSEvents on macOS,
//! ReadDirectoryChangesW on Windows — no elevated privileges required).
//! When any block device is connected or disconnected the subscription emits
//! [`Message::UsbHotplugDetected`], which `update.rs` handles by re-running
//! drive detection — identical to the user pressing Refresh, but automatic.
//!
//! This module contains the main application state (FlashKraft struct)
//! which represents the complete state of the application at any point in time.
//!
//! Following The Elm Architecture, this module also implements the core
//! application methods: update, view, and subscription.

use crate::core::flash_runner::FlashProgress;
use crate::core::storage::Storage;
use crate::core::{update, Message};
use crate::domain::{DriveInfo, ImageInfo};
use crate::ui;
use crate::ui::components::animated_progress::AnimatedProgress;
use flashkraft_core::commands::watch_usb_events;
use futures::StreamExt as _;
use iced::Color;
use iced::{stream, Element, Subscription, Task, Theme};
use std::sync::{atomic::AtomicBool, Arc};

/// The main application state
///
/// This struct represents the complete state of the FlashKraft application.
/// All state is managed immutably and changes only through the `update` function.
#[derive(Debug)]
pub struct FlashKraft {
    /// Currently selected image file
    pub selected_image: Option<ImageInfo>,

    /// Currently selected target drive
    pub selected_target: Option<DriveInfo>,

    /// List of available drives detected on the system
    pub available_drives: Vec<DriveInfo>,

    /// Current flash progress (0.0 to 1.0), None if not flashing
    pub flash_progress: Option<f32>,

    /// Bytes written during flash operation
    pub flash_bytes_written: u64,

    /// Current transfer speed in MB/s
    pub flash_speed_mb_s: f32,

    /// Current pipeline stage label (e.g. "Writing image to device…", "Verifying written data…")
    pub flash_stage: String,

    /// Verification overall progress (0.0–1.0 across both image-hash and device read-back passes).
    /// `None` when verification has not started yet.
    pub verify_progress: Option<f32>,

    /// Current verification read speed in MB/s (0.0 when not verifying).
    pub verify_speed_mb_s: f32,

    /// Which verification pass is active: `"image"` or `"device"`. Empty when not verifying.
    pub verify_phase: &'static str,

    /// Error message if an error occurred
    pub error_message: Option<String>,

    /// Whether the device selection view is currently open
    pub device_selection_open: bool,

    /// Whether a flash operation is currently active (for subscription).
    /// Remains true through the entire pipeline including verification —
    /// only set to false when `FlashCompleted` arrives.
    pub flashing_active: bool,

    /// Set to true when `FlashCompleted(Ok(()))` arrives.
    /// Distinct from `flashing_active` so the view can show the complete
    /// screen without killing the subscription prematurely.
    pub flash_complete: bool,

    /// Cancellation token for flash operation
    pub flash_cancel_token: Arc<AtomicBool>,

    /// Monotonically increasing counter incremented on every new flash attempt.
    /// Included in the subscription ID hash so that flashing the same image to
    /// the same device a second time always creates a fresh subscription instead
    /// of reusing the completed (pending) one from the previous run.
    pub flash_run_id: u64,

    /// Currently selected theme
    pub theme: Theme,

    /// Storage for persistent preferences
    pub storage: Option<Storage>,

    /// Animated progress bar for flash operations
    pub animated_progress: AnimatedProgress,

    /// Separate green animated progress bar shown during verification.
    pub verify_animated_progress: AnimatedProgress,

    /// Animation time for progress line glow effects (0.0 to infinity)
    pub animation_time: f32,
}

impl FlashKraft {
    /// Create a new FlashKraft instance with default values
    pub fn new() -> Self {
        // Try to initialize storage and load saved theme
        let storage = Storage::new().ok();
        let theme = storage
            .as_ref()
            .and_then(|s| s.load_theme())
            .unwrap_or(Theme::Dark);

        // Initialize animated progress with theme
        let mut animated_progress = AnimatedProgress::new();
        animated_progress.set_theme(theme.clone());

        // Verification bar is always green regardless of theme.
        let verify_animated_progress =
            AnimatedProgress::new_with_color(Color::from_rgb(0.18, 0.78, 0.45));

        Self {
            selected_image: None,
            selected_target: None,
            available_drives: Vec::new(),
            flash_progress: None,
            flash_bytes_written: 0,
            flash_speed_mb_s: 0.0,
            flash_stage: String::new(),
            verify_progress: None,
            verify_speed_mb_s: 0.0,
            verify_phase: "",
            error_message: None,
            device_selection_open: false,
            flashing_active: false,
            flash_complete: false,
            flash_cancel_token: Arc::new(AtomicBool::new(false)),
            flash_run_id: 0,
            theme,
            storage,
            animated_progress,
            verify_animated_progress,
            animation_time: 0.0,
        }
    }

    /// Check if the application is ready to flash
    ///
    /// Returns true if both an image and target are selected
    pub fn is_ready_to_flash(&self) -> bool {
        self.selected_image.is_some() && self.selected_target.is_some()
    }

    /// Check if a flash operation is currently in progress
    pub fn is_flashing(&self) -> bool {
        self.flash_progress.is_some()
    }

    /// Check if the flash operation is complete (pipeline finished successfully).
    pub fn is_flash_complete(&self) -> bool {
        self.flash_complete
    }

    /// Check if there is an error
    pub fn has_error(&self) -> bool {
        self.error_message.is_some()
    }

    /// Reset the application state
    pub fn reset(&mut self) {
        self.selected_image = None;
        self.selected_target = None;
        self.flash_progress = None;
        self.flash_bytes_written = 0;
        self.flash_speed_mb_s = 0.0;
        self.flash_stage = String::new();
        self.verify_progress = None;
        self.verify_speed_mb_s = 0.0;
        self.verify_phase = "";
        self.error_message = None;
        self.device_selection_open = false;
        self.flashing_active = false;
        self.flash_complete = false;
        self.flash_cancel_token = Arc::new(AtomicBool::new(false));
        // Do NOT reset flash_run_id — it must keep incrementing across resets
        // so that a re-flash after Reset always gets a fresh subscription.
    }

    /// Cancel current selections
    pub fn cancel_selections(&mut self) {
        self.reset();
    }

    /// Prepare state for a new flash attempt.
    pub fn begin_flash_state(&mut self) {
        self.flash_cancel_token = Arc::new(AtomicBool::new(false));
        self.flash_run_id = self.flash_run_id.wrapping_add(1);
        self.flash_progress = Some(0.0);
        self.error_message = None;
        self.flashing_active = true;
        self.flash_complete = false;
    }
}

impl Default for FlashKraft {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Elm Architecture Implementation
// ============================================================================

impl FlashKraft {
    /// Update the application state based on a message
    ///
    /// This is the core of The Elm Architecture. All state changes
    /// flow through this function.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to process
    ///
    /// # Returns
    ///
    /// A Task that may trigger async operations, or Task::none()
    pub fn update(&mut self, message: Message) -> Task<Message> {
        // Optionally log messages for debugging (exclude AnimationTick to avoid spam)
        if !matches!(message, Message::AnimationTick) {
            #[cfg(debug_assertions)]
            println!("[DEBUG] Message: {:?}", message);
        }

        // Delegate to the update function
        update::update(self, message)
    }

    /// Render the user interface
    ///
    /// This is a pure function that describes what the UI should look
    /// like based on the current state.
    ///
    /// # Returns
    ///
    /// An Element describing the UI to render
    pub fn view(&self) -> Element<'_, Message> {
        // Delegate to the view function
        ui::view(self)
    }

    /// Subscribe to long-running operations
    ///
    /// This enables streaming progress updates from the flash operation,
    /// animation ticks for the progress bar, and automatic USB hotplug
    /// detection (so the drive list updates without the user pressing Refresh).
    ///
    /// # Returns
    ///
    /// A Subscription that emits messages for ongoing operations
    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = Vec::new();

        // ── USB hotplug — always active ───────────────────────────────────────
        //
        // watch_usb_events() watches /sys/block via inotify on Linux, /dev via
        // FSEvents on macOS, and a directory via ReadDirectoryChangesW on
        // Windows.  No elevated privileges are required for any of these
        // watches.  When the watch cannot be created (path missing or sandboxed)
        // we simply produce no events rather than crashing.
        let hotplug_sub = Subscription::run(hotplug_stream);
        subscriptions.push(hotplug_sub);

        // ── Flash progress ────────────────────────────────────────────────────
        if self.flashing_active {
            if let (Some(image), Some(target)) = (&self.selected_image, &self.selected_target) {
                let flash_sub = crate::core::flash_runner::flash_progress(
                    image.path.clone(),
                    target.device_path.clone().into(),
                    self.flash_cancel_token.clone(),
                    self.flash_run_id,
                )
                .map(|progress| match progress {
                    FlashProgress::Progress {
                        progress,
                        bytes_written,
                        speed_mb_s,
                    } => Message::FlashProgressUpdate(progress, bytes_written, speed_mb_s),
                    FlashProgress::VerifyProgress {
                        phase,
                        overall,
                        bytes_read,
                        total_bytes,
                        speed_mb_s,
                    } => Message::VerifyProgressUpdate(
                        overall,
                        phase,
                        bytes_read,
                        total_bytes,
                        speed_mb_s,
                    ),
                    FlashProgress::Message(msg) => Message::Status(msg),
                    FlashProgress::Completed => Message::FlashCompleted(Ok(())),
                    FlashProgress::Failed(err) => Message::FlashCompleted(Err(err)),
                });
                subscriptions.push(flash_sub);
            }
        }

        // Animation tick — always active for progress-line glow and spinner effects
        subscriptions.push(iced::window::frames().map(|_| Message::AnimationTick));

        Subscription::batch(subscriptions)
    }
}

/// Free function that creates the USB hotplug event stream.
///
/// Defined as a named `fn` (not a closure) so it can be passed directly to
/// [`Subscription::run`], which requires a `fn() -> S` pointer.
fn hotplug_stream() -> impl futures::Stream<Item = Message> {
    stream::channel(4, async |mut output| {
        use futures::SinkExt as _;
        match watch_usb_events() {
            Ok(mut events) => {
                while let Some(_event) = events.next().await {
                    let _ = output.send(Message::UsbHotplugDetected).await;
                }
            }
            Err(e) => {
                eprintln!("[hotplug] watch_usb_events failed: {e}");
                // Park the future so the subscription stays alive but
                // silent — avoids Iced restarting it in a tight loop.
                std::future::pending::<()>().await;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_state() {
        let state = FlashKraft::new();
        assert!(state.selected_image.is_none());
        assert!(state.selected_target.is_none());
        assert!(state.available_drives.is_empty());
        assert!(!state.is_ready_to_flash());
        assert!(!state.device_selection_open);
    }

    #[test]
    fn test_is_ready_to_flash() {
        let mut state = FlashKraft::new();
        assert!(!state.is_ready_to_flash());

        state.selected_image = Some(ImageInfo {
            path: PathBuf::from("/tmp/test.img"),
            name: "test.img".to_string(),
            size_mb: 100.0,
        });
        assert!(!state.is_ready_to_flash());

        state.selected_target = Some(DriveInfo::new(
            "USB".to_string(),
            "/media/usb".to_string(),
            32.0,
            "/dev/sdb".to_string(),
        ));
        assert!(state.is_ready_to_flash());
    }

    #[test]
    fn test_is_flashing() {
        let mut state = FlashKraft::new();
        assert!(!state.is_flashing());

        state.flash_progress = Some(0.5);
        assert!(state.is_flashing());
    }

    #[test]
    fn test_reset() {
        let mut state = FlashKraft::new();
        state.selected_image = Some(ImageInfo {
            path: PathBuf::from("/tmp/test.img"),
            name: "test.img".to_string(),
            size_mb: 100.0,
        });
        state.flash_progress = Some(0.5);
        state.error_message = Some("Error".to_string());
        state.device_selection_open = true;

        state.reset();

        assert!(state.selected_image.is_none());
        assert!(state.flash_progress.is_none());
        assert!(state.error_message.is_none());
        assert!(!state.device_selection_open);
    }
}
