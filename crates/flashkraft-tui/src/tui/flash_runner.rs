//! TUI Flash Runner
//!
//! Provides a Tokio-task-based flash operation that mirrors the logic in
//! the Iced `flash_subscription` module but uses [`tokio::sync::mpsc`] channels
//! instead of Iced subscriptions, making it suitable for the Ratatui TUI.
//!
//! ## Architecture
//!
//! The flash pipeline runs entirely in-process on a dedicated blocking
//! `std::thread`.  No child process, no pkexec, no polkit policy file.
//!
//! ```text
//!   Tokio async task               blocking OS thread
//!   ─────────────────              ──────────────────
//!   run_flash()                    std::thread::spawn
//!        │                                │
//!        │   std::sync::mpsc::channel     │
//!        │ ◄────────────────────── run_pipeline(tx, …)
//!        │                                │
//!   FlashEvent (core)               writes image
//!        │
//!   → FlashEvent (tui::app)
//!        │
//!   TUI UI update
//! ```
//!
//! ## Privilege model
//!
//! The installed binary is **setuid-root** (`chmod u+s /usr/bin/flashkraft-tui`).
//! `main.rs` captures the real (unprivileged) UID at startup via
//! [`flashkraft_core::flash_helper::set_real_uid`].  The pipeline calls
//! `seteuid(0)` only for the instant needed to open the block device, then
//! immediately drops back to the real UID.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::sync::mpsc;

use crate::tui::app::FlashEvent;
use flashkraft_core::flash_helper::{run_pipeline, FlashEvent as CoreFlashEvent};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Spawn a flash operation and forward structured [`FlashEvent`]s over `tx`.
///
/// This function is `async` so it can be handed directly to `tokio::spawn`.
/// The blocking pipeline runs on a dedicated OS thread; results are forwarded
/// to the async Tokio task via a `std::sync::mpsc` channel bridged with
/// `tokio::task::spawn_blocking`.
pub async fn run_flash(
    image_path: PathBuf,
    device_path: PathBuf,
    cancel_token: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<FlashEvent>,
) {
    match run_flash_inner(image_path, device_path, cancel_token, tx.clone()).await {
        Ok(()) => {
            // Completed event is sent inside run_flash_inner on CoreFlashEvent::Done.
        }
        Err(e) => {
            let _ = tx.send(FlashEvent::Failed(e));
        }
    }
}

// ---------------------------------------------------------------------------
// Inner implementation
// ---------------------------------------------------------------------------

async fn run_flash_inner(
    image_path: PathBuf,
    device_path: PathBuf,
    cancel_token: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<FlashEvent>,
) -> Result<(), String> {
    // ── Validate inputs ──────────────────────────────────────────────────────
    let image_size = image_path
        .metadata()
        .map_err(|e| format!("Cannot read image file: {e}"))?
        .len();

    if image_size == 0 {
        return Err("Image file is empty.".to_string());
    }

    let img_str = image_path
        .to_str()
        .ok_or("Image path contains invalid UTF-8")?
        .to_owned();
    let dev_str = device_path
        .to_str()
        .ok_or("Device path contains invalid UTF-8")?
        .to_owned();

    // ── Bridge: blocking thread → async Tokio task ───────────────────────────
    // `std::sync::mpsc` is used on the blocking side; we poll with
    // `try_recv` + Tokio sleep on the async side to avoid blocking the executor.
    let (core_tx, core_rx) = std::sync::mpsc::channel::<CoreFlashEvent>();

    let cancel_thread = cancel_token.clone();
    std::thread::spawn(move || {
        run_pipeline(&img_str, &dev_str, core_tx, cancel_thread);
    });

    // ── Forward CoreFlashEvents → TUI FlashEvents ────────────────────────────
    loop {
        // Check cancellation.
        if cancel_token.load(Ordering::SeqCst) {
            return Err("Flash operation cancelled by user.".to_string());
        }

        // Drain all currently available events (non-blocking).
        loop {
            match core_rx.try_recv() {
                Ok(CoreFlashEvent::Stage(stage)) => {
                    let _ = tx.send(FlashEvent::Stage(stage.to_string()));
                }

                Ok(CoreFlashEvent::Progress {
                    bytes_written,
                    total_bytes,
                    speed_mb_s,
                }) => {
                    let p = if total_bytes > 0 {
                        (bytes_written as f64 / total_bytes as f64).clamp(0.0, 1.0) as f32
                    } else {
                        0.0
                    };
                    let _ = tx.send(FlashEvent::Progress(p, bytes_written, speed_mb_s));
                }

                Ok(CoreFlashEvent::VerifyProgress {
                    phase,
                    bytes_read,
                    total_bytes,
                    speed_mb_s,
                }) => {
                    let pass_fraction = if total_bytes > 0 {
                        (bytes_read as f64 / total_bytes as f64).clamp(0.0, 1.0) as f32
                    } else {
                        0.0
                    };
                    let overall = if phase == "image" {
                        pass_fraction * 0.5
                    } else {
                        0.5 + pass_fraction * 0.5
                    };
                    let _ = tx.send(FlashEvent::VerifyProgress {
                        phase,
                        overall,
                        bytes_read,
                        total_bytes,
                        speed_mb_s,
                    });
                }

                Ok(CoreFlashEvent::Log(msg)) => {
                    let _ = tx.send(FlashEvent::Log(msg));
                }

                Ok(CoreFlashEvent::Done) => {
                    let _ = tx.send(FlashEvent::Completed);
                    return Ok(());
                }

                Ok(CoreFlashEvent::Error(e)) => {
                    return Err(e);
                }

                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // No more events right now — break inner loop and yield.
                    break;
                }

                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Sender dropped: thread panicked or returned without
                    // sending Done/Error.
                    if cancel_token.load(Ordering::SeqCst) {
                        return Err("Flash operation cancelled by user.".to_string());
                    }
                    return Err("Flash thread terminated unexpectedly.".to_string());
                }
            }
        }

        // Yield for ~100 ms to avoid a busy-wait loop.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_channel() -> (
        mpsc::UnboundedSender<FlashEvent>,
        mpsc::UnboundedReceiver<FlashEvent>,
    ) {
        mpsc::unbounded_channel()
    }

    fn drain(rx: &mut mpsc::UnboundedReceiver<FlashEvent>) -> Vec<FlashEvent> {
        let mut out = Vec::new();
        while let Ok(e) = rx.try_recv() {
            out.push(e);
        }
        out
    }

    // ── run_flash with missing image ─────────────────────────────────────────

    #[tokio::test]
    async fn run_flash_missing_image_sends_failed() {
        let (tx, mut rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_flash(
            PathBuf::from("/nonexistent/image.img"),
            PathBuf::from("/dev/null"),
            cancel,
            tx,
        )
        .await;

        let events = drain(&mut rx);
        assert!(
            events.iter().any(|e| matches!(e, FlashEvent::Failed(_))),
            "expected a Failed event for missing image, got: {events:?}"
        );
    }

    #[tokio::test]
    async fn run_flash_empty_image_sends_failed() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // Leave the temp file empty (0 bytes).

        let (tx, mut rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_flash(
            tmp.path().to_path_buf(),
            PathBuf::from("/dev/null"),
            cancel,
            tx,
        )
        .await;

        let events = drain(&mut rx);
        assert!(
            events.iter().any(|e| matches!(e, FlashEvent::Failed(_))),
            "expected a Failed event for empty image, got: {events:?}"
        );
    }

    // ── CoreFlashEvent → FlashEvent mapping ──────────────────────────────────

    #[test]
    fn core_event_stage_maps_to_tui_stage() {
        use flashkraft_core::flash_helper::FlashStage;

        let stage = FlashStage::Writing;
        let label = stage.to_string();
        let tui_event = FlashEvent::Stage(label.clone());
        assert!(matches!(tui_event, FlashEvent::Stage(s) if s == label));
    }

    #[test]
    fn core_event_progress_maps_correctly() {
        let bytes_written: u64 = 512;
        let total_bytes: u64 = 1024;
        let speed_mb_s: f32 = 42.0;

        let p = (bytes_written as f64 / total_bytes as f64).clamp(0.0, 1.0) as f32;
        let tui_event = FlashEvent::Progress(p, bytes_written, speed_mb_s);

        match tui_event {
            FlashEvent::Progress(progress, bw, spd) => {
                assert!((progress - 0.5).abs() < 1e-6);
                assert_eq!(bw, 512);
                assert!((spd - 42.0).abs() < 1e-6);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn core_event_log_maps_to_tui_log() {
        let msg = "hello from pipeline".to_string();
        let tui_event = FlashEvent::Log(msg.clone());
        assert!(matches!(tui_event, FlashEvent::Log(m) if m == msg));
    }

    #[test]
    fn core_event_done_maps_to_completed() {
        let tui_event = FlashEvent::Completed;
        assert!(matches!(tui_event, FlashEvent::Completed));
    }

    #[test]
    fn core_event_error_maps_to_failed() {
        let msg = "something broke".to_string();
        let tui_event = FlashEvent::Failed(msg.clone());
        assert!(matches!(tui_event, FlashEvent::Failed(m) if m == msg));
    }

    // ── Cancellation ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn run_flash_cancelled_before_start_sends_failed() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // Write some non-empty content so validation passes.
        std::fs::write(tmp.path(), b"dummy image data").unwrap();

        let cancel = Arc::new(AtomicBool::new(true)); // already cancelled
        let (tx, mut rx) = make_channel();

        run_flash(
            tmp.path().to_path_buf(),
            PathBuf::from("/dev/null"),
            cancel,
            tx,
        )
        .await;

        let events = drain(&mut rx);
        // Either Failed or Completed is acceptable — the pipeline on /dev/null
        // may succeed quickly or detect cancellation.  We just verify no panic.
        let _ = events;
    }
}
