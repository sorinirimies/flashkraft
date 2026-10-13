//! TUI Flash Runner
//!
//! Provides a Tokio-task-based flash operation that mirrors the logic in
//! the Iced `flash_subscription` module but uses [`tokio::sync::mpsc`] channels
//! instead of Iced subscriptions, making it suitable for the Ratatui TUI.
//!
//! ## Architecture
//!
//! ```text
//!   Tokio task            spawn_blocking          flashkraft-core worker
//!   ──────────            ──────────────          ──────────────────────
//!   run_flash()  ───────► for update in rx  ◄──── spawn_flash_worker()
//!        ▲                      │                  (pipeline thread inside)
//!        └── FlashEvent (= FlashUpdate) over tokio unbounded channel
//! ```
//!
//! The pipeline runs entirely in-process; no child process, no pkexec.
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
    sync::{atomic::AtomicBool, Arc},
};

use tokio::sync::mpsc;

use super::message::FlashEvent;
use flashkraft_core::flash_worker::{is_terminal, spawn_flash_worker};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run a flash operation and forward structured [`FlashEvent`]s over `tx`.
///
/// `async` so it can be handed directly to `tokio::spawn`. The heavy lifting
/// lives in [`flashkraft_core::flash_worker`], shared with the GUI: it
/// guarantees exactly one terminal event (`Completed` / `Failed`) as the last
/// item, converts a panicking pipeline into `Failed`, and only reports a
/// cancel once the device has really stopped being written.
///
/// Updates are forwarded as they are produced (blocking `recv` on a Tokio
/// blocking thread) — there is no polling interval.
pub async fn run_flash(
    image_path: PathBuf,
    device_path: PathBuf,
    cancel_token: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<FlashEvent>,
) {
    let rx = spawn_flash_worker(&image_path, &device_path, cancel_token);
    let forward_tx = tx.clone();
    let forwarded = tokio::task::spawn_blocking(move || {
        for update in rx {
            let last = is_terminal(&update);
            if forward_tx.send(update).is_err() || last {
                break;
            }
        }
    })
    .await;
    if forwarded.is_err() {
        let _ = tx.send(FlashEvent::Failed("Flash forwarder crashed".into()));
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use flashkraft_core::FlashUpdate;
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
    fn core_event_stage_maps_to_message() {
        use flashkraft_core::flash_helper::{FlashEvent as CoreFlashEvent, FlashStage};
        let label = FlashStage::Writing.to_string();
        let update = FlashUpdate::from(CoreFlashEvent::Stage(FlashStage::Writing));
        assert!(matches!(update, FlashUpdate::Message(s) if s == label));
    }

    #[test]
    fn core_event_progress_maps_correctly() {
        use flashkraft_core::flash_helper::FlashEvent as CoreFlashEvent;
        let update = FlashUpdate::from(CoreFlashEvent::Progress {
            bytes_written: 512,
            total_bytes: 1024,
            speed_mb_s: 42.0,
        });
        match update {
            FlashUpdate::Progress {
                progress,
                bytes_written,
                speed_mb_s,
            } => {
                assert!((progress - 0.5).abs() < 1e-6);
                assert_eq!(bytes_written, 512);
                assert!((speed_mb_s - 42.0).abs() < 1e-6);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn core_event_log_maps_to_message() {
        use flashkraft_core::flash_helper::FlashEvent as CoreFlashEvent;
        let msg = "hello from pipeline".to_string();
        let update = FlashUpdate::from(CoreFlashEvent::Log(msg.clone()));
        assert!(matches!(update, FlashUpdate::Message(m) if m == msg));
    }

    #[test]
    fn core_event_done_maps_to_completed() {
        use flashkraft_core::flash_helper::FlashEvent as CoreFlashEvent;
        let update = FlashUpdate::from(CoreFlashEvent::Done);
        assert!(matches!(update, FlashUpdate::Completed));
    }

    #[test]
    fn core_event_error_maps_to_failed() {
        use flashkraft_core::flash_helper::FlashEvent as CoreFlashEvent;
        let msg = "something broke".to_string();
        let update = FlashUpdate::from(CoreFlashEvent::Error(msg.clone()));
        assert!(matches!(update, FlashUpdate::Failed(m) if m == msg));
    }

    // ── Cancellation & end-to-end ─────────────────────────────────────────────

    fn only_terminal_is_last(events: &[FlashEvent]) {
        let terminals = events.iter().filter(|e| is_terminal(e)).count();
        assert_eq!(terminals, 1, "exactly one terminal event: {events:?}");
        assert!(is_terminal(events.last().unwrap()), "terminal comes last");
    }

    #[tokio::test]
    async fn run_flash_cancelled_before_start_reports_cancelled() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), vec![1u8; 1024 * 1024]).unwrap();
        let dev = tempfile::NamedTempFile::new().unwrap();

        let (tx, mut rx) = make_channel();
        run_flash(
            tmp.path().to_path_buf(),
            dev.path().to_path_buf(),
            Arc::new(AtomicBool::new(true)),
            tx,
        )
        .await;

        let events = drain(&mut rx);
        only_terminal_is_last(&events);
        assert!(
            matches!(events.last(), Some(FlashEvent::Failed(m)) if m.contains("cancelled")),
            "{events:?}"
        );
    }

    #[tokio::test]
    async fn run_flash_end_to_end_streams_progress_then_one_terminal() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.img");
        let dev = dir.path().join("dev.bin");
        let data: Vec<u8> = (0..=255u8).cycle().take(768 * 1024).collect();
        std::fs::write(&img, &data).unwrap();
        std::fs::File::create(&dev).unwrap();

        let (tx, mut rx) = make_channel();
        run_flash(img, dev.clone(), Arc::new(AtomicBool::new(false)), tx).await;

        let events = drain(&mut rx);
        only_terminal_is_last(&events);
        assert!(events
            .iter()
            .any(|e| matches!(e, FlashEvent::Progress { .. })));
        if matches!(events.last(), Some(FlashEvent::Completed)) {
            assert_eq!(std::fs::read(&dev).unwrap(), data);
        }
    }

    #[tokio::test]
    async fn run_flash_stops_when_receiver_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.img");
        let dev = dir.path().join("dev.bin");
        std::fs::write(&img, vec![3u8; 512 * 1024]).unwrap();
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        drop(rx);
        // Must return rather than hang or panic.
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            run_flash(img, dev, Arc::new(AtomicBool::new(false)), tx),
        )
        .await
        .expect("run_flash returns when nobody listens");
    }
}
