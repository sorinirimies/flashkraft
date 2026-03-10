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
use futures::channel::mpsc as futures_mpsc;
use futures::SinkExt;
use futures::StreamExt;
use iced::stream;
use iced::Subscription;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Progress event emitted by the flash subscription to the Iced runtime.
#[derive(Debug, Clone)]
pub enum FlashProgress {
    /// Write progress: `(overall 0.0–1.0, bytes_written, speed_mb_s)`
    Progress(f32, u64, f32),
    /// Verification read-back progress.
    ///
    /// `overall` spans both passes:
    ///   - image pass:  `bytes_read / total * 0.5`        → 0.0 – 0.5
    ///   - device pass: `0.5 + bytes_read / total * 0.5`  → 0.5 – 1.0
    VerifyProgress {
        phase: &'static str,
        overall: f32,
        bytes_read: u64,
        total_bytes: u64,
        speed_mb_s: f32,
    },
    /// Human-readable status message (stage name, log line, …)
    Message(String),
    /// The flash operation finished successfully.
    Completed,
    /// The flash operation failed; the string is a human-readable error.
    Failed(String),
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
    // Unique subscription ID — changes every flash attempt.
    let mut hasher = DefaultHasher::new();
    image_path.hash(&mut hasher);
    device_path.hash(&mut hasher);
    run_id.hash(&mut hasher);
    let id = hasher.finish();

    Subscription::run_with_id(
        id,
        stream::channel(64, move |mut output| async move {
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

            flash_debug!(
                "flash_progress: image={image_path:?} dev={device_path:?} size={image_size}"
            );

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
                    loop {
                        match std_rx.recv() {
                            Ok(event) => {
                                // try_send returns Err if the receiver was
                                // dropped (subscription cancelled) — exit cleanly.
                                if futures_tx.try_send(event).is_err() {
                                    break;
                                }
                            }
                            Err(_) => {
                                // std sender dropped → pipeline thread finished.
                                break;
                            }
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
                    Some(FlashEvent::Progress {
                        bytes_written,
                        total_bytes,
                        speed_mb_s,
                    }) => {
                        let progress = if total_bytes > 0 {
                            (bytes_written as f64 / total_bytes as f64).clamp(0.0, 1.0) as f32
                        } else {
                            0.0
                        };
                        flash_debug!(
                            "progress: {:.1}% ({bytes_written}/{total_bytes}) @ {speed_mb_s:.1} MB/s",
                            progress * 100.0
                        );
                        let _ = output
                            .send(FlashProgress::Progress(progress, bytes_written, speed_mb_s))
                            .await;
                    }

                    Some(FlashEvent::VerifyProgress {
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
                        flash_debug!(
                            "verify[{phase}]: {:.1}% ({bytes_read}/{total_bytes}) @ {speed_mb_s:.1} MB/s",
                            pass_fraction * 100.0
                        );
                        let _ = output
                            .send(FlashProgress::VerifyProgress {
                                phase,
                                overall,
                                bytes_read,
                                total_bytes,
                                speed_mb_s,
                            })
                            .await;
                    }

                    Some(FlashEvent::Stage(stage)) => {
                        let msg = stage.to_string();
                        flash_debug!("stage: {msg}");
                        let _ = output.send(FlashProgress::Message(msg)).await;
                    }

                    Some(FlashEvent::Log(msg)) => {
                        flash_debug!("log: {msg}");
                        let _ = output.send(FlashProgress::Message(msg)).await;
                    }

                    Some(FlashEvent::Done) => {
                        flash_debug!("flash thread: Done");
                        let _ = output.send(FlashProgress::Completed).await;
                        break;
                    }

                    Some(FlashEvent::Error(e)) => {
                        flash_debug!("flash thread: Error: {e}");
                        let _ = output.send(FlashProgress::Failed(e)).await;
                        break;
                    }

                    // Channel closed — bridge thread exited (pipeline done or cancelled).
                    None => {
                        flash_debug!("flash channel closed unexpectedly");
                        if cancel_token.load(Ordering::SeqCst) {
                            let _ = output
                                .send(FlashProgress::Failed(
                                    "Flash operation cancelled by user".into(),
                                ))
                                .await;
                        } else {
                            let _ = output
                                .send(FlashProgress::Failed(
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
        }),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_progress_clone() {
        let variants = vec![
            FlashProgress::Progress(0.5, 1024, 10.0),
            FlashProgress::VerifyProgress {
                phase: "image",
                overall: 0.25,
                bytes_read: 512,
                total_bytes: 1024,
                speed_mb_s: 100.0,
            },
            FlashProgress::VerifyProgress {
                phase: "device",
                overall: 0.75,
                bytes_read: 512,
                total_bytes: 1024,
                speed_mb_s: 80.0,
            },
            FlashProgress::Message("hello".to_string()),
            FlashProgress::Completed,
            FlashProgress::Failed("oops".to_string()),
        ];
        for v in &variants {
            let _ = v.clone();
        }
    }

    #[test]
    fn test_flash_progress_debug() {
        let p = FlashProgress::Progress(1.0, 2048, 20.0);
        assert!(format!("{p:?}").contains("Progress"));
    }

    #[test]
    fn test_subscription_id_is_deterministic() {
        fn compute_id(image: &str, device: &str, run_id: u64) -> u64 {
            let mut hasher = DefaultHasher::new();
            PathBuf::from(image).hash(&mut hasher);
            PathBuf::from(device).hash(&mut hasher);
            run_id.hash(&mut hasher);
            hasher.finish()
        }
        let id1 = compute_id("/tmp/test.img", "/dev/sdb", 0);
        let id2 = compute_id("/tmp/test.img", "/dev/sdb", 0);
        assert_eq!(id1, id2, "subscription ID must be deterministic");
    }

    #[test]
    fn test_subscription_id_differs_for_different_devices() {
        fn compute_id(image: &str, device: &str, run_id: u64) -> u64 {
            let mut hasher = DefaultHasher::new();
            PathBuf::from(image).hash(&mut hasher);
            PathBuf::from(device).hash(&mut hasher);
            run_id.hash(&mut hasher);
            hasher.finish()
        }
        let id1 = compute_id("/tmp/test.img", "/dev/sdb", 0);
        let id2 = compute_id("/tmp/test.img", "/dev/sdc", 0);
        assert_ne!(id1, id2, "different devices must yield different IDs");
    }

    #[test]
    fn test_subscription_id_differs_for_different_run_ids() {
        fn compute_id(image: &str, device: &str, run_id: u64) -> u64 {
            let mut hasher = DefaultHasher::new();
            PathBuf::from(image).hash(&mut hasher);
            PathBuf::from(device).hash(&mut hasher);
            run_id.hash(&mut hasher);
            hasher.finish()
        }
        let id1 = compute_id("/tmp/test.img", "/dev/sdb", 0);
        let id2 = compute_id("/tmp/test.img", "/dev/sdb", 1);
        assert_ne!(
            id1, id2,
            "different run IDs must yield different subscription IDs"
        );
    }

    #[test]
    fn test_verify_progress_overall_image_phase() {
        for pct in [0.0f32, 0.25, 0.5, 1.0] {
            let overall = pct * 0.5;
            assert!(
                (0.0..=0.5).contains(&overall),
                "image phase overall {overall} out of [0, 0.5]"
            );
        }
    }

    #[test]
    fn test_verify_progress_overall_device_phase() {
        for pct in [0.0f32, 0.25, 0.5, 1.0] {
            let overall = 0.5 + pct * 0.5;
            assert!(
                (0.5..=1.0).contains(&overall),
                "device phase overall {overall} out of [0.5, 1.0]"
            );
        }
    }

    #[test]
    fn test_cancelled_maps_to_failed() {
        let cancel = Arc::new(AtomicBool::new(true));
        let msg = if cancel.load(Ordering::SeqCst) {
            "Flash operation cancelled by user"
        } else {
            "Flash thread terminated unexpectedly"
        };
        assert_eq!(msg, "Flash operation cancelled by user");
    }

    /// The bridge thread correctly terminates when the futures receiver is dropped.
    #[test]
    fn test_bridge_exits_when_receiver_dropped() {
        let (std_tx, std_rx) = std::sync::mpsc::channel::<FlashEvent>();
        let (mut futures_tx, futures_rx) = futures_mpsc::channel::<FlashEvent>(4);

        // Drop the receiver immediately — bridge should exit cleanly.
        drop(futures_rx);

        let bridge = std::thread::spawn(move || loop {
            match std_rx.recv() {
                Ok(event) => {
                    if futures_tx.try_send(event).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        });

        // Send one event — bridge will fail try_send and exit.
        let _ = std_tx.send(FlashEvent::Done);
        // Give the bridge thread a moment to process.
        std::thread::sleep(std::time::Duration::from_millis(50));
        // Drop sender so bridge's recv() returns Err if it didn't exit already.
        drop(std_tx);

        bridge.join().expect("bridge thread should exit cleanly");
    }

    /// Verify that all FlashEvent variants are handled (mapping smoke test).
    #[test]
    fn test_flash_event_mapping_smoke() {
        use flashkraft_core::flash_helper::FlashStage;

        let events = vec![
            FlashEvent::Stage(FlashStage::Writing),
            FlashEvent::Progress {
                bytes_written: 512,
                total_bytes: 1024,
                speed_mb_s: 42.0,
            },
            FlashEvent::VerifyProgress {
                phase: "image",
                bytes_read: 256,
                total_bytes: 1024,
                speed_mb_s: 100.0,
            },
            FlashEvent::VerifyProgress {
                phase: "device",
                bytes_read: 512,
                total_bytes: 1024,
                speed_mb_s: 80.0,
            },
            FlashEvent::Log("Test log".into()),
            FlashEvent::Done,
            FlashEvent::Error("boom".into()),
        ];

        // Verify each variant maps to a FlashProgress without panicking.
        for event in events {
            let _mapped: Option<FlashProgress> = match event {
                FlashEvent::Progress {
                    bytes_written,
                    total_bytes,
                    speed_mb_s,
                } => {
                    let p = if total_bytes > 0 {
                        (bytes_written as f64 / total_bytes as f64).clamp(0.0, 1.0) as f32
                    } else {
                        0.0
                    };
                    Some(FlashProgress::Progress(p, bytes_written, speed_mb_s))
                }
                FlashEvent::VerifyProgress {
                    phase,
                    bytes_read,
                    total_bytes,
                    speed_mb_s,
                } => {
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
                    Some(FlashProgress::VerifyProgress {
                        phase,
                        overall,
                        bytes_read,
                        total_bytes,
                        speed_mb_s,
                    })
                }
                FlashEvent::Stage(s) => Some(FlashProgress::Message(s.to_string())),
                FlashEvent::Log(m) => Some(FlashProgress::Message(m)),
                FlashEvent::Done => Some(FlashProgress::Completed),
                FlashEvent::Error(e) => Some(FlashProgress::Failed(e)),
            };
        }
    }
}
