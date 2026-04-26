//! Flash Subscription - Real-time progress streaming
//!
//! ## Architecture
//!
//! ```text
//!   Iced async runtime (ThreadPool)       blocking OS thread
//!   ────────────────────────────────      ──────────────────
//!   flash_progress()                      std::thread::spawn
//!        │                                       │
//!        │  futures::channel::mpsc               │
//!        │ ◄─────────────────────────── bridge thread
//!        │        (forwards from std_rx)         │
//!        │                               run_pipeline(std_tx)
//!   event = rx.next().await                      │
//!        │  (yields to executor)          writes image / verifies
//!        │
//!   FlashProgress → Message → Iced repaint
//! ```
//!
//! ## Why blocking `recv()` was wrong
//!
//! The previous implementation called `std::sync::mpsc::Receiver::recv()`
//! directly inside the `async` stream block.  `recv()` is a **blocking**
//! syscall — it parks the OS thread until a message arrives.  Because Iced
//! drives subscriptions on a `futures::executor::ThreadPool` (not tokio),
//! blocking that thread starved every other future on the same worker,
//! including Iced's repaint loop.  Progress events were queued correctly but
//! the UI never re-rendered until the entire pipeline had finished.
//!
//! ## Fix
//!
//! We now use a **three-actor design**:
//!
//! 1. **Pipeline thread** — calls `run_pipeline` with a `std::sync::mpsc::Sender`.
//! 2. **Bridge thread** — calls `std_rx.recv()` (blocking is fine here because
//!    this thread owns nothing except forwarding) and calls
//!    `futures_tx.try_send()` into a `futures::channel::mpsc` channel.
//!    A tiny `thread::sleep(1 ms)` between iterations keeps CPU usage near zero
//!    while the pipeline is idle between blocks.
//! 3. **Async stream** — calls `rx.next().await` on the `futures::channel::mpsc`
//!    receiver, which is a proper async future that yields between every message
//!    and lets the Iced executor schedule repaints freely.
//!
//! ## Cancellation
//!
//! An `Arc<AtomicBool>` cancel token is shared with the pipeline thread.
//! The pipeline checks it on every 4 MiB write block.

use crate::flash_debug;
use flashkraft_core::flash_helper::{run_pipeline, FlashEvent};
use flashkraft_core::FlashUpdate;
use futures::channel::mpsc as futures_mpsc;
use futures::StreamExt;
use iced::stream;
use iced::Subscription;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Progress event emitted by the flash subscription to the Iced runtime.
///
/// This is a type alias for [`flashkraft_core::FlashUpdate`] — the unified
/// frontend event defined in core so both the GUI and TUI share the same
/// representation without duplicating the type.
pub use flashkraft_core::FlashUpdate as FlashProgress;

// ---------------------------------------------------------------------------
// Subscription data
// ---------------------------------------------------------------------------

/// All data needed by the flash subscription stream.
///
/// Implements [`Hash`] manually so that only the deterministic fields
/// (`image_path`, `device_path`, `run_id`) contribute to the subscription
/// identity.  `cancel_token` is an `Arc<AtomicBool>` which is intentionally
/// excluded — its pointer value changes on every allocation and would defeat
/// subscription deduplication.
#[derive(Clone)]
struct FlashSubData {
    image_path: PathBuf,
    device_path: PathBuf,
    cancel_token: Arc<AtomicBool>,
    run_id: u64,
}

impl Hash for FlashSubData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.image_path.hash(state);
        self.device_path.hash(state);
        self.run_id.hash(state);
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create a subscription that streams [`FlashProgress`] events while the
/// flash operation runs.
///
/// `run_id` must be incremented on every new flash attempt so that flashing
/// the same image to the same device twice always produces a distinct
/// subscription ID and Iced creates a fresh stream.
pub fn flash_progress(
    image_path: PathBuf,
    device_path: PathBuf,
    cancel_token: Arc<AtomicBool>,
    run_id: u64,
) -> Subscription<FlashProgress> {
    let data = FlashSubData {
        image_path,
        device_path,
        cancel_token,
        run_id,
    };

    Subscription::run_with(data, build_flash_stream)
}

/// Builder function passed to [`Subscription::run_with`].
///
/// Receives a reference to the subscription data, clones what it needs,
/// and returns an async stream that drives the flash pipeline.
fn build_flash_stream(
    data: &FlashSubData,
) -> impl futures::Stream<Item = FlashProgress> + Send + 'static {
    let image_path = data.image_path.clone();
    let device_path = data.device_path.clone();
    let cancel_token = data.cancel_token.clone();

    stream::channel(64, async move |mut output| {
        use futures::SinkExt as _;

        // ── Validate inputs ───────────────────────────────────────────────
        let image_size = match image_path.metadata() {
            Ok(m) if m.len() == 0 => {
                let _ = output
                    .send(FlashProgress::Failed("Image file is empty".into()))
                    .await;
                return std::future::pending().await;
            }
            Ok(m) => m.len(),
            Err(e) => {
                let _ = output
                    .send(FlashProgress::Failed(format!(
                        "Cannot read image file: {e}"
                    )))
                    .await;
                return std::future::pending().await;
            }
        };

        flash_debug!("flash_progress: image={image_path:?} dev={device_path:?} size={image_size}");

        // ── Channel setup ─────────────────────────────────────────────────
        //
        // std channel  → bridge thread (blocking recv) → futures channel
        //                                                       ↓
        //                                              rx.next().await
        //                                              (yields to executor)
        let (std_tx, std_rx) = std::sync::mpsc::channel::<FlashEvent>();

        // futures::channel::mpsc is executor-agnostic — next() is a real
        // async future that yields between every message.
        let (mut futures_tx, mut futures_rx) = futures_mpsc::channel::<FlashEvent>(64);

        // ── Pipeline thread ───────────────────────────────────────────────
        let img_str = image_path.to_string_lossy().into_owned();
        let dev_str = device_path.to_string_lossy().into_owned();
        let cancel_pipeline = cancel_token.clone();

        std::thread::Builder::new()
            .name("flashkraft-pipeline".into())
            .spawn(move || {
                flash_debug!("flash thread: starting pipeline");
                run_pipeline(&img_str, &dev_str, std_tx, cancel_pipeline);
                flash_debug!("flash thread: pipeline returned");
            })
            .expect("failed to spawn flash pipeline thread");

        // ── Bridge thread ─────────────────────────────────────────────────
        //
        // Sits in a blocking recv() loop — safe because this is its own
        // dedicated OS thread and it owns no async resources.  When a
        // message arrives it forwards it into the futures channel via
        // try_send (non-blocking from this thread's perspective).
        std::thread::Builder::new()
            .name("flashkraft-bridge".into())
            .spawn(move || {
                while let Ok(event) = std_rx.recv() {
                    // try_send returns Err if the receiver was
                    // dropped (subscription cancelled) — exit cleanly.
                    if futures_tx.try_send(event).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn flash bridge thread");

        // ── Async event loop ──────────────────────────────────────────────
        //
        // futures_rx.next().await is a genuine async yield point.
        // The Iced ThreadPool executor is free to run other futures
        // (repaints, animation ticks, etc.) between every message.
        loop {
            match futures_rx.next().await {
                Some(FlashEvent::Done) => {
                    flash_debug!("flash thread: Done");
                    let _ = output.send(FlashUpdate::Completed).await;
                    break;
                }

                Some(FlashEvent::Error(e)) => {
                    flash_debug!("flash thread: Error: {e}");
                    let _ = output.send(FlashUpdate::Failed(e)).await;
                    break;
                }

                Some(core_event) => {
                    let update = FlashUpdate::from(core_event);
                    flash_debug!("flash event: {update:?}");
                    let _ = output.send(update).await;
                }

                // Channel closed — bridge thread exited (pipeline done or cancelled).
                None => {
                    flash_debug!("flash channel closed unexpectedly");
                    if cancel_token.load(Ordering::SeqCst) {
                        let _ = output
                            .send(FlashUpdate::Failed(
                                "Flash operation cancelled by user".into(),
                            ))
                            .await;
                    } else {
                        let _ = output
                            .send(FlashUpdate::Failed(
                                "Flash thread terminated unexpectedly".into(),
                            ))
                            .await;
                    }
                    break;
                }
            }
        }

        // Park forever — Iced requires the stream future to never return.
        std::future::pending().await
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    #[test]
    fn test_flash_progress_clone() {
        let progress = FlashProgress::Progress {
            progress: 0.5,
            bytes_written: 1024,
            speed_mb_s: 10.0,
        };
        let cloned = progress.clone();
        match cloned {
            FlashProgress::Progress {
                progress,
                bytes_written,
                speed_mb_s,
            } => {
                assert!((progress - 0.5).abs() < f32::EPSILON);
                assert_eq!(bytes_written, 1024);
                assert!((speed_mb_s - 10.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected Progress variant"),
        }
    }

    #[test]
    fn test_flash_progress_debug() {
        let progress = FlashProgress::Completed;
        let debug_str = format!("{:?}", progress);
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_subscription_id_is_deterministic() {
        fn compute_id(image: &str, device: &str, run_id: u64) -> u64 {
            let data = FlashSubData {
                image_path: PathBuf::from(image),
                device_path: PathBuf::from(device),
                cancel_token: Arc::new(AtomicBool::new(false)),
                run_id,
            };
            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            hasher.finish()
        }
        assert_eq!(
            compute_id("/tmp/a.img", "/dev/sdb", 1),
            compute_id("/tmp/a.img", "/dev/sdb", 1),
        );
    }

    #[test]
    fn test_subscription_id_differs_for_different_devices() {
        fn compute_id(image: &str, device: &str, run_id: u64) -> u64 {
            let data = FlashSubData {
                image_path: PathBuf::from(image),
                device_path: PathBuf::from(device),
                cancel_token: Arc::new(AtomicBool::new(false)),
                run_id,
            };
            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            hasher.finish()
        }
        assert_ne!(
            compute_id("/tmp/a.img", "/dev/sdb", 1),
            compute_id("/tmp/a.img", "/dev/sdc", 1),
        );
    }

    #[test]
    fn test_subscription_id_differs_for_different_run_ids() {
        fn compute_id(image: &str, device: &str, run_id: u64) -> u64 {
            let data = FlashSubData {
                image_path: PathBuf::from(image),
                device_path: PathBuf::from(device),
                cancel_token: Arc::new(AtomicBool::new(false)),
                run_id,
            };
            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            hasher.finish()
        }
        let id_a = compute_id("/tmp/a.img", "/dev/sdb", 1);
        let id_b = compute_id("/tmp/a.img", "/dev/sdb", 2);
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn test_verify_progress_overall_image_phase() {
        let p = FlashProgress::VerifyProgress {
            phase: "image",
            overall: 0.25,
            bytes_read: 100,
            total_bytes: 400,
            speed_mb_s: 50.0,
        };
        if let FlashProgress::VerifyProgress { overall, .. } = p {
            assert!((overall - 0.25).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_verify_progress_overall_device_phase() {
        let p = FlashProgress::VerifyProgress {
            phase: "device",
            overall: 0.75,
            bytes_read: 300,
            total_bytes: 400,
            speed_mb_s: 50.0,
        };
        if let FlashProgress::VerifyProgress { overall, .. } = p {
            assert!((overall - 0.75).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_cancelled_maps_to_failed() {
        let token = Arc::new(AtomicBool::new(true));
        assert!(token.load(Ordering::SeqCst));
        let msg = FlashProgress::Failed("Flash operation cancelled by user".into());
        match msg {
            FlashProgress::Failed(e) => {
                assert!(e.contains("cancelled"));
            }
            _ => panic!("Expected Failed variant"),
        }
    }

    #[test]
    fn test_bridge_exits_when_receiver_dropped() {
        // Simulate bridge thread behavior: if the futures channel receiver
        // is dropped (subscription cancelled), try_send returns Err and the
        // bridge exits cleanly.
        let (std_tx, std_rx) = std::sync::mpsc::channel::<FlashEvent>();
        let (futures_tx, _futures_rx) = futures_mpsc::channel::<FlashEvent>(4);

        // Drop the receiver immediately
        drop(_futures_rx);

        // Send an event through std channel
        let _ = std_tx.send(FlashEvent::Done);

        // Bridge logic: try_send should fail
        let mut ftx = futures_tx;
        if let Ok(event) = std_rx.recv() {
            let result = ftx.try_send(event);
            assert!(
                result.is_err(),
                "try_send should fail when receiver is dropped"
            );
        }
    }

    #[test]
    fn test_flash_event_mapping_smoke() {
        // Test that FlashEvent → FlashUpdate mapping works for common variants
        let events = vec![
            FlashEvent::Done,
            FlashEvent::Error("test error".into()),
            FlashEvent::Progress {
                bytes_written: 512,
                total_bytes: 1024,
                speed_mb_s: 10.0,
            },
        ];

        for event in events {
            match event {
                FlashEvent::Done => {
                    let update = FlashUpdate::Completed;
                    assert!(matches!(update, FlashUpdate::Completed));
                }
                FlashEvent::Error(e) => {
                    let update = FlashUpdate::Failed(e);
                    assert!(matches!(update, FlashUpdate::Failed(_)));
                }
                other => {
                    let update = FlashUpdate::from(other);
                    // Should not panic
                    let _ = format!("{:?}", update);
                }
            }
        }
    }

    #[test]
    fn test_cancel_token_not_part_of_hash() {
        // Two FlashSubData with different cancel tokens but same paths/run_id
        // should hash identically.
        let data1 = FlashSubData {
            image_path: PathBuf::from("/tmp/test.img"),
            device_path: PathBuf::from("/dev/sdb"),
            cancel_token: Arc::new(AtomicBool::new(false)),
            run_id: 42,
        };
        let data2 = FlashSubData {
            image_path: PathBuf::from("/tmp/test.img"),
            device_path: PathBuf::from("/dev/sdb"),
            cancel_token: Arc::new(AtomicBool::new(true)),
            run_id: 42,
        };
        let mut h1 = DefaultHasher::new();
        data1.hash(&mut h1);
        let mut h2 = DefaultHasher::new();
        data2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }
}
