//! Update Logic - The Elm Architecture
//!
//! This module contains the update function that processes messages
//! and updates the application state. This is the core of The Elm
//! Architecture where all state transitions occur.

use crate::flash_debug;
use crate::status_log;
use iced::Task;

use crate::core::commands;
use crate::core::message::Message;
use crate::core::state::FlashKraft;
use crate::domain::{constraints, ImageInfo};

/// Update the application state based on a message
///
/// This is the heart of The Elm Architecture. It's a pure function that:
/// 1. Takes the current state and a message
/// 2. Updates the state
/// 3. Returns a Command for side effects (or Task::none())
///
/// # Arguments
///
/// * `state` - Mutable reference to the application state
/// * `message` - The message to process
///
/// # Returns
///
/// A Command that may trigger async operations, or Task::none()
pub fn update(state: &mut FlashKraft, message: Message) -> Task<Message> {
    match message {
        // ====================================================================
        // User Interaction Messages
        // ====================================================================
        Message::SelectImageClicked => {
            // Spawn async file selection dialog
            Task::perform(commands::select_image_file(), Message::ImageSelected)
        }

        Message::RefreshDrivesClicked => {
            // Spawn async drive detection
            Task::perform(commands::load_drives(), Message::DrivesRefreshed)
        }

        Message::UsbHotplugDetected => {
            // A USB device was connected or disconnected — re-enumerate drives
            // exactly as if the user had pressed Refresh.  The drive list will
            // update automatically without any user interaction.
            Task::perform(commands::load_drives(), Message::DrivesRefreshed)
        }

        Message::TargetDriveClicked(drive) => {
            // Update selected target
            state.selected_target = Some(drive);
            state.error_message = None;
            // Close the device selection view
            state.device_selection_open = false;
            Task::none()
        }

        Message::OpenDeviceSelection => {
            // Open the device selection view
            state.device_selection_open = true;
            // Refresh drives when opening the view
            Task::perform(commands::load_drives(), Message::DrivesRefreshed)
        }

        Message::CloseDeviceSelection => {
            // Close the device selection view
            state.device_selection_open = false;
            Task::none()
        }

        Message::FlashClicked => {
            // Validate we can flash
            if state.is_ready_to_flash() {
                // If we are not already running as root, attempt a transparent
                // re-exec via pkexec / sudo so the user gets a polkit password
                // prompt now — before the flash subscription starts — rather
                // than hitting a hard permission error mid-flash.
                if !flashkraft_core::flash_helper::is_privileged() {
                    return Task::perform(async { Message::EscalateAndFlash }, |m| m);
                }

                state.begin_flash_state();

                Task::none()
            } else {
                // Set error if trying to flash without selections
                state.error_message =
                    Some("Please select both an image file and a target drive".to_string());
                Task::none()
            }
        }

        Message::EscalateAndFlash => {
            // Called when FlashClicked detects we are not running as root.
            //
            // `reexec_as_root()` calls execvp, which replaces this process
            // image entirely on success — the Iced runtime, all threads, and
            // all open file descriptors are replaced by the new privileged
            // process, which restarts from main() and finds geteuid() == 0.
            //
            // If this function returns it means:
            //   • neither pkexec nor sudo was found on PATH, or
            //   • the user dismissed the polkit dialog / Ctrl-C'd sudo.
            //
            // In that case we fall through and start the subscription anyway;
            // the flash pipeline will hit EACCES when it tries to open the
            // block device and show the clear error message with install
            // instructions.
            flashkraft_core::flash_helper::reexec_as_root();

            // reexec returned → escalation unavailable or declined.
            // Proceed with the flash attempt so the user sees the specific
            // error (and the setuid install instructions) rather than nothing.
            state.begin_flash_state();

            Task::none()
        }

        Message::ResetClicked => {
            // Reset all state
            state.reset();

            // Refresh drives list
            Task::perform(commands::load_drives(), Message::DrivesRefreshed)
        }

        Message::CancelClicked => {
            // Cancel current selections
            state.cancel_selections();
            Task::none()
        }

        Message::CancelFlash => {
            // Signal cancellation to the flash operation
            state
                .flash_cancel_token
                .store(true, std::sync::atomic::Ordering::SeqCst);

            // Cancel ongoing flash operation
            state.flashing_active = false;
            state.flash_progress = None;
            state.flash_bytes_written = 0;
            state.flash_speed_mb_s = 0.0;
            state.error_message = Some("Flash operation cancelled by user".to_string());
            Task::none()
        }

        // ====================================================================
        // Async Result Messages
        // ====================================================================
        Message::ImageSelected(maybe_path) => {
            // Update selected image
            state.selected_image = maybe_path.map(ImageInfo::from_path);
            state.error_message = None;

            // Mark invalid drives based on the new image selection
            constraints::mark_invalid_drives(
                &mut state.available_drives,
                state.selected_image.as_ref(),
            );

            Task::none()
        }

        Message::DrivesRefreshed(mut drives) => {
            // Mark invalid drives based on current image selection
            constraints::mark_invalid_drives(&mut drives, state.selected_image.as_ref());

            // Update available drives list
            state.available_drives = drives;
            Task::none()
        }

        Message::FlashProgressUpdate(progress, bytes, speed) => {
            // Update flash progress from subscription.
            // Writing phase is mapped to 0–80 % so post-write stages have room.
            let mapped = (progress * 0.80).clamp(0.0, 0.80);
            // Only advance — never go backwards.
            let current = state.flash_progress.unwrap_or(0.0);
            let new_progress = mapped.max(current);
            state.flash_progress = Some(new_progress);
            state.flash_bytes_written = bytes;
            state.flash_speed_mb_s = speed;
            // Update animated progress bar
            state.animated_progress.set_progress(new_progress);
            Task::none()
        }

        Message::AnimationTick => {
            // Tick animation for progress bar effects with speed-based scaling
            state.animated_progress.tick(state.flash_speed_mb_s);

            // Tick the green verification bar during the verify phase.
            if state.verify_progress.is_some() {
                state.verify_animated_progress.tick(state.verify_speed_mb_s);
            }

            // Increment animation time for progress line glow effects
            // Scale based on transfer speed for dynamic animations
            let speed_multiplier = (state.flash_speed_mb_s / 20.0).clamp(0.15, 1.2);
            state.animation_time += 0.016 * speed_multiplier; // ~60 FPS baseline, slowed down
            Task::none()
        }

        Message::VerifyProgressUpdate(overall, phase, bytes_read, total_bytes, speed_mb_s) => {
            // Update verification progress fields.
            state.verify_progress = Some(overall);
            state.verify_speed_mb_s = speed_mb_s;
            state.verify_phase = phase;

            // Drive the green verification bar (0–100 %).
            state.verify_animated_progress.set_progress(overall);
            state.verify_animated_progress.tick(speed_mb_s);

            // Also nudge the main bar through the verify window (92–100 %)
            // so the overall bar keeps moving while the verify bar is the
            // primary visual focus.
            let bar_progress = 0.92 + overall * 0.08;
            let current = state.flash_progress.unwrap_or(0.0);
            if bar_progress > current {
                state.flash_progress = Some(bar_progress);
                state.animated_progress.set_progress(bar_progress);
            }

            flash_debug!(
                "verify phase={phase} overall={:.1}% ({bytes_read}/{total_bytes}) @ {speed_mb_s:.1} MB/s",
                overall * 100.0
            );

            Task::none()
        }

        Message::Status(message) => {
            // Log status message in debug builds
            status_log!("{}", message);

            // Advance the overall progress bar floor for post-write pipeline
            // stages so the bar keeps moving after writing finishes.
            //
            // Stage → overall-progress floor:
            //   "Flushing write buffers…"      → 80 %
            //   "Refreshing partition table…"  → 88 %
            //   "Verifying written data…"      → 92 %
            //   (92–100 % is driven by VerifyProgressUpdate events instead)
            let floor: Option<f32> = {
                let syncing_str = flashkraft_core::FlashStage::Syncing.to_string();
                let rereading_str = flashkraft_core::FlashStage::Rereading.to_string();
                let verifying_str = flashkraft_core::FlashStage::Verifying.to_string();
                if message == syncing_str {
                    Some(flashkraft_core::FlashStage::Syncing.progress_floor())
                } else if message == rereading_str {
                    Some(flashkraft_core::FlashStage::Rereading.progress_floor())
                } else if message == verifying_str {
                    Some(flashkraft_core::FlashStage::Verifying.progress_floor())
                } else {
                    None
                }
            };
            if let Some(floor) = floor {
                let current = state.flash_progress.unwrap_or(0.0);
                if floor > current {
                    state.flash_progress = Some(floor);
                    state.animated_progress.set_progress(floor);
                }
            }

            // Track the current stage label so the flashing view can show it.
            // Only update for the meaningful stage transitions, not raw log lines.
            let is_stage = matches!(
                message.as_str(),
                "Unmounting partitions…"
                    | "Writing image to device…"
                    | "Flushing write buffers…"
                    | "Refreshing partition table…"
                    | "Verifying written data…"
                    | "Flash complete!"
            );
            if is_stage {
                state.flash_stage = message;
            }

            Task::none()
        }

        Message::FlashCompleted(result) => {
            // The pipeline is fully done (write + sync + reread + verify).
            // Now it is safe to deactivate the subscription.
            state.flashing_active = false;

            match result {
                Ok(()) => {
                    // Drive progress to exactly 100 % and mark complete.
                    state.flash_progress = Some(1.0);
                    state.animated_progress.set_progress(1.0);
                    state.flash_stage = "Flash complete!".to_string();
                    state.flash_complete = true;
                    state.error_message = None;
                }
                Err(error_message) => {
                    // Flash failed — clear progress so the error view is shown.
                    state.flash_progress = None;
                    state.flash_stage = String::new();
                    state.flash_complete = false;
                    state.error_message = Some(error_message);
                }
            }
            Task::none()
        }

        Message::ThemeChanged(theme) => {
            // Update the application theme
            state.theme = theme.clone();

            // Update animated progress bar theme
            state.animated_progress.set_theme(theme.clone());

            // Save theme to persistent storage
            if let Some(storage) = &state.storage {
                if let Err(e) = storage.save_theme(&theme) {
                    eprintln!("Failed to save theme preference: {}", e);
                }
            }

            Task::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DriveInfo;
    use std::path::PathBuf;

    #[test]
    fn test_cancel_clicked() {
        let mut state = FlashKraft::new();
        state.selected_image = Some(ImageInfo {
            path: PathBuf::from("/tmp/test.img"),
            name: "test.img".to_string(),
            size_mb: 100.0,
        });

        let _ = update(&mut state, Message::CancelClicked);

        assert!(state.selected_image.is_none());
    }

    #[test]
    fn test_target_drive_clicked() {
        let mut state = FlashKraft::new();
        let drive = DriveInfo::new(
            "USB".to_string(),
            "/media/usb".to_string(),
            32.0,
            "/dev/sdb".to_string(),
        );

        let _ = update(&mut state, Message::TargetDriveClicked(drive.clone()));

        assert_eq!(state.selected_target.as_ref().unwrap().name, "USB");
    }

    #[test]
    fn test_image_selected() {
        let mut state = FlashKraft::new();
        let path = PathBuf::from("/tmp/test.img");

        let _ = update(&mut state, Message::ImageSelected(Some(path.clone())));

        assert!(state.selected_image.is_some());
    }

    #[test]
    fn test_flash_clicked_without_selections() {
        let mut state = FlashKraft::new();

        let _ = update(&mut state, Message::FlashClicked);

        assert!(state.error_message.is_some());
        assert!(state.flash_progress.is_none());
    }

    #[test]
    fn test_escalate_and_flash_starts_flashing() {
        // When EscalateAndFlash fires (reexec_as_root already returned without
        // exec-ing — i.e. escalation was unavailable), the subscription must
        // still be activated so the user sees the permission error.
        let mut state = FlashKraft::new();
        state.selected_image = Some(crate::domain::ImageInfo {
            path: std::path::PathBuf::from("/tmp/test.img"),
            name: "test.img".to_string(),
            size_mb: 100.0,
        });
        state.selected_target = Some(DriveInfo::new(
            "USB".to_string(),
            "/media/usb".to_string(),
            32.0,
            "/dev/sdb".to_string(),
        ));

        let _ = update(&mut state, Message::EscalateAndFlash);

        assert!(state.flashing_active);
        assert_eq!(state.flash_progress, Some(0.0));
        assert!(state.error_message.is_none());
    }

    #[test]
    fn test_flash_completed_success() {
        let mut state = FlashKraft::new();
        state.flash_progress = Some(0.5);

        let _ = update(&mut state, Message::FlashCompleted(Ok(())));

        assert_eq!(state.flash_progress, Some(1.0));
        assert!(state.error_message.is_none());
    }

    #[test]
    fn test_flash_completed_error() {
        let mut state = FlashKraft::new();
        state.flash_progress = Some(0.5);

        let _ = update(
            &mut state,
            Message::FlashCompleted(Err("Test error".to_string())),
        );

        assert!(state.flash_progress.is_none());
        assert_eq!(state.error_message.as_deref(), Some("Test error"));
    }

    #[test]
    fn test_reset_clicked() {
        let mut state = FlashKraft::new();
        state.selected_image = Some(ImageInfo {
            path: PathBuf::from("/tmp/test.img"),
            name: "test.img".to_string(),
            size_mb: 100.0,
        });
        state.flash_progress = Some(1.0);

        let _ = update(&mut state, Message::ResetClicked);

        assert!(state.selected_image.is_none());
        assert!(state.flash_progress.is_none());
    }

    #[test]
    fn test_cancel_flash_sets_cancellation_token() {
        use std::sync::atomic::Ordering;

        let mut state = FlashKraft::new();
        state.flashing_active = true;
        state.flash_progress = Some(0.5);
        state.flash_bytes_written = 1024;
        state.flash_speed_mb_s = 10.0;

        // Verify token is not cancelled initially
        assert!(!state.flash_cancel_token.load(Ordering::SeqCst));

        let _ = update(&mut state, Message::CancelFlash);

        // Verify cancellation token is set
        assert!(state.flash_cancel_token.load(Ordering::SeqCst));

        // Verify state is cleaned up
        assert!(!state.flashing_active);
        assert!(state.flash_progress.is_none());
        assert_eq!(state.flash_bytes_written, 0);
        assert_eq!(state.flash_speed_mb_s, 0.0);
        assert!(state.error_message.is_some());
        assert!(state.error_message.as_ref().unwrap().contains("cancelled"));
    }

    #[test]
    fn test_flash_clicked_creates_new_cancel_token() {
        use std::sync::atomic::Ordering;

        let mut state = FlashKraft::new();

        // Set up valid selections
        state.selected_image = Some(ImageInfo {
            path: PathBuf::from("/tmp/test.img"),
            name: "test.img".to_string(),
            size_mb: 100.0,
        });
        state.selected_target = Some(DriveInfo::new(
            "USB".to_string(),
            "/media/usb".to_string(),
            32.0,
            "/dev/sdb".to_string(),
        ));

        // Set the old token to cancelled
        state.flash_cancel_token.store(true, Ordering::SeqCst);
        let old_token = state.flash_cancel_token.clone();

        // When not running as root, FlashClicked dispatches EscalateAndFlash
        // without modifying the token yet. EscalateAndFlash is the handler
        // that actually resets the token and activates the flash subscription
        // (after reexec_as_root() returns without exec-ing in test mode).
        let _ = update(&mut state, Message::EscalateAndFlash);

        // Verify a new token was created (different Arc)
        assert!(!std::sync::Arc::ptr_eq(
            &old_token,
            &state.flash_cancel_token
        ));

        // Verify new token is not cancelled
        assert!(!state.flash_cancel_token.load(Ordering::SeqCst));

        // Verify flashing is active
        assert!(state.flashing_active);
        assert_eq!(state.flash_progress, Some(0.0));
    }

    #[test]
    fn test_cancel_clicked_resets_cancel_token() {
        use std::sync::atomic::Ordering;

        let mut state = FlashKraft::new();
        state.selected_image = Some(ImageInfo {
            path: PathBuf::from("/tmp/test.img"),
            name: "test.img".to_string(),
            size_mb: 100.0,
        });

        // Set the token to cancelled
        state.flash_cancel_token.store(true, Ordering::SeqCst);

        let _ = update(&mut state, Message::CancelClicked);

        // Verify a new token was created (not cancelled)
        assert!(!state.flash_cancel_token.load(Ordering::SeqCst));
        assert!(state.selected_image.is_none());
    }

    #[test]
    fn test_reset_clicked_resets_cancel_token() {
        use std::sync::atomic::Ordering;

        let mut state = FlashKraft::new();
        state.flash_progress = Some(0.5);
        state.flashing_active = true;

        // Set the token to cancelled
        state.flash_cancel_token.store(true, Ordering::SeqCst);

        let _ = update(&mut state, Message::ResetClicked);

        // Verify a new token was created (not cancelled)
        assert!(!state.flash_cancel_token.load(Ordering::SeqCst));
        assert!(!state.flashing_active);
        assert!(state.flash_progress.is_none());
    }

    // ====================================================================
    // FlashProgressUpdate
    // ====================================================================

    #[test]
    fn test_flash_progress_update_sets_fields() {
        let mut state = FlashKraft::new();

        let _ = update(&mut state, Message::FlashProgressUpdate(0.5, 1024, 10.0));

        // progress 0.5 is mapped to 0.5 * 0.80 = 0.40
        assert_eq!(state.flash_progress, Some(0.40));
        assert_eq!(state.flash_bytes_written, 1024);
        assert_eq!(state.flash_speed_mb_s, 10.0);
    }

    #[test]
    fn test_flash_progress_update_never_goes_backwards() {
        let mut state = FlashKraft::new();

        // First update to 60 % raw → mapped 0.48
        let _ = update(&mut state, Message::FlashProgressUpdate(0.6, 2048, 15.0));
        assert_eq!(state.flash_progress, Some(0.6 * 0.80));

        // Second update to 40 % raw → mapped 0.32 — must NOT go backwards
        let _ = update(&mut state, Message::FlashProgressUpdate(0.4, 1024, 5.0));
        assert_eq!(state.flash_progress, Some(0.6 * 0.80));
        // But speed and bytes still update
        assert_eq!(state.flash_bytes_written, 1024);
        assert_eq!(state.flash_speed_mb_s, 5.0);
    }

    #[test]
    fn test_flash_progress_update_clamps_to_80_percent() {
        let mut state = FlashKraft::new();

        // Raw progress beyond 1.0 should be clamped to 0.80 mapped
        let _ = update(&mut state, Message::FlashProgressUpdate(1.5, 4096, 20.0));
        assert_eq!(state.flash_progress, Some(0.80));
    }

    // ====================================================================
    // VerifyProgressUpdate
    // ====================================================================

    #[test]
    fn test_verify_progress_update_sets_fields() {
        let mut state = FlashKraft::new();
        state.flash_progress = Some(0.90);

        let _ = update(
            &mut state,
            Message::VerifyProgressUpdate(0.5, "device", 500_000, 1_000_000, 42.0),
        );

        assert_eq!(state.verify_progress, Some(0.5));
        assert_eq!(state.verify_speed_mb_s, 42.0);
        assert_eq!(state.verify_phase, "device");
        // Main bar should advance: 0.92 + 0.5 * 0.08 = 0.96
        let progress = state.flash_progress.unwrap();
        assert!(
            (progress - 0.96).abs() < 1e-5,
            "expected ~0.96, got {}",
            progress
        );
    }

    // ====================================================================
    // AnimationTick
    // ====================================================================

    #[test]
    fn test_animation_tick_increments_animation_time() {
        let mut state = FlashKraft::new();
        assert_eq!(state.animation_time, 0.0);

        state.flash_speed_mb_s = 20.0;
        let _ = update(&mut state, Message::AnimationTick);

        // speed_multiplier = (20.0 / 20.0).clamp(0.15, 1.2) = 1.0
        // animation_time += 0.016 * 1.0 = 0.016
        assert!(
            (state.animation_time - 0.016).abs() < 1e-6,
            "expected ~0.016, got {}",
            state.animation_time
        );
    }

    #[test]
    fn test_animation_tick_scales_with_speed() {
        let mut state = FlashKraft::new();
        state.flash_speed_mb_s = 3.0; // low speed
        let _ = update(&mut state, Message::AnimationTick);

        // speed_multiplier = (3.0 / 20.0).clamp(0.15, 1.2) = 0.15
        let expected = 0.016 * 0.15;
        assert!(
            (state.animation_time - expected).abs() < 1e-6,
            "expected ~{}, got {}",
            expected,
            state.animation_time
        );
    }

    // ====================================================================
    // DrivesRefreshed
    // ====================================================================

    #[test]
    fn test_drives_refreshed_populates_drives() {
        let mut state = FlashKraft::new();
        assert!(state.available_drives.is_empty());

        let drives = vec![
            DriveInfo::new(
                "USB-A".to_string(),
                "/media/a".to_string(),
                16.0,
                "/dev/sdb".to_string(),
            ),
            DriveInfo::new(
                "USB-B".to_string(),
                "/media/b".to_string(),
                32.0,
                "/dev/sdc".to_string(),
            ),
        ];

        let _ = update(&mut state, Message::DrivesRefreshed(drives));

        assert_eq!(state.available_drives.len(), 2);
        assert_eq!(state.available_drives[0].name, "USB-A");
        assert_eq!(state.available_drives[1].name, "USB-B");
    }

    #[test]
    fn test_drives_refreshed_empty_clears_drives() {
        let mut state = FlashKraft::new();
        // Pre-populate with a drive
        state.available_drives = vec![DriveInfo::new(
            "Old".to_string(),
            "/media/old".to_string(),
            8.0,
            "/dev/sda".to_string(),
        )];

        let _ = update(&mut state, Message::DrivesRefreshed(vec![]));

        assert!(state.available_drives.is_empty());
    }

    // ====================================================================
    // Status (stage floor logic)
    // ====================================================================

    #[test]
    fn test_status_writing_sets_flash_stage() {
        let mut state = FlashKraft::new();

        let _ = update(
            &mut state,
            Message::Status("Writing image to device\u{2026}".to_string()),
        );

        assert_eq!(state.flash_stage, "Writing image to device\u{2026}");
    }

    #[test]
    fn test_status_syncing_sets_progress_floor_80() {
        let mut state = FlashKraft::new();
        state.flash_progress = Some(0.70);

        let syncing = flashkraft_core::FlashStage::Syncing.to_string();
        let _ = update(&mut state, Message::Status(syncing.clone()));

        assert_eq!(state.flash_progress, Some(0.80));
        assert_eq!(state.flash_stage, syncing);
    }

    #[test]
    fn test_status_rereading_sets_progress_floor_88() {
        let mut state = FlashKraft::new();
        state.flash_progress = Some(0.80);

        let rereading = flashkraft_core::FlashStage::Rereading.to_string();
        let _ = update(&mut state, Message::Status(rereading.clone()));

        assert_eq!(state.flash_progress, Some(0.88));
        assert_eq!(state.flash_stage, rereading);
    }

    #[test]
    fn test_status_verifying_sets_progress_floor_92() {
        let mut state = FlashKraft::new();
        state.flash_progress = Some(0.88);

        let verifying = flashkraft_core::FlashStage::Verifying.to_string();
        let _ = update(&mut state, Message::Status(verifying.clone()));

        assert_eq!(state.flash_progress, Some(0.92));
        assert_eq!(state.flash_stage, verifying);
    }

    #[test]
    fn test_status_floor_does_not_go_backwards() {
        let mut state = FlashKraft::new();
        // Progress already past the syncing floor
        state.flash_progress = Some(0.90);

        let syncing = flashkraft_core::FlashStage::Syncing.to_string();
        let _ = update(&mut state, Message::Status(syncing));

        // Floor is 0.80, but current is 0.90 — must NOT go backwards
        assert_eq!(state.flash_progress, Some(0.90));
    }

    #[test]
    fn test_status_non_stage_message_does_not_update_flash_stage() {
        let mut state = FlashKraft::new();
        state.flash_stage = "Writing image to device\u{2026}".to_string();

        let _ = update(
            &mut state,
            Message::Status("some debug log line".to_string()),
        );

        // flash_stage should not change for non-stage messages
        assert_eq!(state.flash_stage, "Writing image to device\u{2026}");
    }

    // ====================================================================
    // ThemeChanged
    // ====================================================================

    #[test]
    fn test_theme_changed_updates_theme() {
        let mut state = FlashKraft::new();
        // Default is Dark (loaded from storage or fallback)
        assert!(matches!(state.theme, iced::Theme::Dark));

        let _ = update(&mut state, Message::ThemeChanged(iced::Theme::Light));

        assert!(matches!(state.theme, iced::Theme::Light));
    }

    // ====================================================================
    // OpenDeviceSelection / CloseDeviceSelection
    // ====================================================================

    #[test]
    fn test_open_device_selection() {
        let mut state = FlashKraft::new();
        assert!(!state.device_selection_open);

        let _ = update(&mut state, Message::OpenDeviceSelection);

        assert!(state.device_selection_open);
    }

    #[test]
    fn test_close_device_selection() {
        let mut state = FlashKraft::new();
        state.device_selection_open = true;

        let _ = update(&mut state, Message::CloseDeviceSelection);

        assert!(!state.device_selection_open);
    }
}
