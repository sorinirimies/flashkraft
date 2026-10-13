//! Flash worker — the one place that turns the blocking pipeline into a
//! stream of [`FlashUpdate`]s.
//!
//! Both front-ends (Iced GUI, Ratatui TUI) used to hand-roll their own bridge
//! between [`run_pipeline`] and their UI loop. That duplicated logic had
//! three weaknesses this module fixes once:
//!
//! * **No guaranteed terminal event.** If the pipeline thread panicked or
//!   returned early the UI could wait forever. [`spawn_flash_worker`] always
//!   delivers *exactly one* of [`FlashUpdate::Completed`] / [`FlashUpdate::Failed`]
//!   as the last item, then closes the channel.
//! * **Polling.** The TUI woke every 100 ms to `try_recv`; the worker blocks
//!   on `recv`, so progress is forwarded the instant it is produced and an
//!   idle flash costs no CPU.
//! * **Cancel without waiting.** The TUI used to report "cancelled" while the
//!   pipeline thread was still writing to the device. The worker only reports
//!   the end once the pipeline thread has actually stopped (the pipeline
//!   checks the cancel flag every block).
//!
//! ```text
//!   caller thread        worker thread                 pipeline thread
//!   ─────────────        ─────────────                 ───────────────
//!   spawn_flash_worker ─►  for event in rx  ◄── mpsc ── run_pipeline()
//!        │                 FlashUpdate::from(event)
//!   Receiver<FlashUpdate> ◄─ send (exactly one terminal)
//! ```

use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

use crate::flash_helper::{run_pipeline, FlashEvent, FlashUpdate};

/// Message reported when the user cancelled the flash.
pub const CANCELLED_MSG: &str = "Flash operation cancelled by user";
/// Message reported when the pipeline stopped without a verdict.
pub const TERMINATED_MSG: &str = "Flash thread terminated unexpectedly";
/// Message reported when the pipeline thread panicked.
pub const PANICKED_MSG: &str = "Flash pipeline panicked";

/// `true` for the two updates that end a flash.
pub fn is_terminal(update: &FlashUpdate) -> bool {
    matches!(update, FlashUpdate::Completed | FlashUpdate::Failed(_))
}

/// Cheap pre-flight checks that give a precise error before any thread starts.
///
/// Returns the image size in bytes.
pub fn preflight(image: &Path, device: &Path) -> Result<u64, String> {
    if image.to_str().is_none() {
        return Err("Image path contains invalid UTF-8".into());
    }
    if device.to_str().is_none() {
        return Err("Device path contains invalid UTF-8".into());
    }
    let meta = image
        .metadata()
        .map_err(|e| format!("Cannot read image file: {e}"))?;
    if !meta.is_file() {
        return Err("Image path is not a regular file".into());
    }
    if meta.len() == 0 {
        return Err("Image file is empty".into());
    }
    Ok(meta.len())
}

/// Convert raw pipeline events into [`FlashUpdate`]s and hand them to `sink`.
///
/// Stops after the first terminal event (anything the pipeline says after
/// `Done`/`Error` is ignored). If `events` ends *without* a terminal event a
/// synthetic [`FlashUpdate::Failed`] is delivered — "cancelled" when `cancel`
/// is set, "terminated unexpectedly" otherwise.
///
/// `sink` returns `false` when the consumer is gone; forwarding then stops
/// and the function returns `false`. Otherwise it returns `true`.
pub fn forward_events<I>(
    events: I,
    cancel: &AtomicBool,
    mut sink: impl FnMut(FlashUpdate) -> bool,
) -> bool
where
    I: IntoIterator<Item = FlashEvent>,
{
    for event in events {
        let update = FlashUpdate::from(event);
        let terminal = is_terminal(&update);
        if !sink(update) {
            return false;
        }
        if terminal {
            return true;
        }
    }
    let msg = if cancel.load(Ordering::SeqCst) {
        CANCELLED_MSG
    } else {
        TERMINATED_MSG
    };
    sink(FlashUpdate::Failed(msg.into()))
}

/// Start flashing `image` → `device` on background threads.
///
/// Returns a receiver of [`FlashUpdate`]s. The last item is always exactly one
/// [`FlashUpdate::Completed`] or [`FlashUpdate::Failed`]; after it the channel
/// is closed. Dropping the receiver abandons the updates but does **not**
/// stop the flash — set `cancel` for that.
pub fn spawn_flash_worker(
    image: &Path,
    device: &Path,
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<FlashUpdate> {
    let (tx, rx) = mpsc::channel::<FlashUpdate>();

    if let Err(e) = preflight(image, device) {
        let _ = tx.send(FlashUpdate::Failed(e));
        return rx;
    }
    let img = image.to_string_lossy().into_owned();
    let dev = device.to_string_lossy().into_owned();

    // Demo mode never touches a device: play a believable simulation instead.
    if crate::commands::demo::is_demo() {
        let size = preflight(image, device).unwrap_or(0);
        let tx = tx.clone();
        let _ = std::thread::Builder::new()
            .name("flashkraft-demo-flash".into())
            .spawn(move || simulate_flash(&dev, size, demo_step_ms(), &cancel, &tx));
        return rx;
    }

    let spawned = std::thread::Builder::new()
        .name("flashkraft-worker".into())
        .spawn({
            let tx = tx.clone();
            move || {
                let (ev_tx, ev_rx) = mpsc::channel::<FlashEvent>();
                let panic_tx = ev_tx.clone();
                let cancel_pipeline = cancel.clone();
                let pipeline = std::thread::Builder::new()
                    .name("flashkraft-pipeline".into())
                    .spawn(move || {
                        let run = std::panic::AssertUnwindSafe(|| {
                            run_pipeline(&img, &dev, ev_tx, cancel_pipeline)
                        });
                        if std::panic::catch_unwind(run).is_err() {
                            let _ = panic_tx.send(FlashEvent::Error(PANICKED_MSG.into()));
                        }
                    });

                let pipeline = match pipeline {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = tx.send(FlashUpdate::Failed(format!(
                            "Cannot start flash pipeline: {e}"
                        )));
                        return;
                    }
                };

                // `ev_rx` ends once the pipeline thread has fully returned, so
                // a cancel is only reported after the device stopped being
                // written.
                forward_events(ev_rx, &cancel, |u| tx.send(u).is_ok());
                let _ = pipeline.join();
            }
        });

    if let Err(e) = spawned {
        let _ = tx.send(FlashUpdate::Failed(format!(
            "Cannot start flash worker: {e}"
        )));
    }
    rx
}

/// Milliseconds per simulated progress step in demo mode
/// (`FLASHKRAFT_DEMO_STEP_MS` overrides the 60 ms default).
fn demo_step_ms() -> u64 {
    std::env::var("FLASHKRAFT_DEMO_STEP_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
}

/// Play a fake flash of `size` bytes onto `device`: write 0→100 %, then verify
/// 0→100 %, then complete. Speed differs per device so parallel bars diverge.
/// Honors `cancel` between steps. Used only in demo mode.
pub fn simulate_flash(
    device: &str,
    size: u64,
    step_ms: u64,
    cancel: &AtomicBool,
    tx: &mpsc::Sender<FlashUpdate>,
) {
    use std::time::Duration;
    // Deterministic per-device pace in 70–130 % of nominal.
    let pace = 70 + device.bytes().fold(0u64, |a, b| a.wrapping_add(b as u64)) % 61;
    let steps_total = 100 * 100 / pace.max(1);
    let speed = 25.0 + (pace as f32) / 4.0;
    let send = |u: FlashUpdate| tx.send(u).is_ok();
    let cancelled = || cancel.load(Ordering::SeqCst);

    if !send(FlashUpdate::Message("Unmounting".into())) {
        return;
    }
    for i in 0..=steps_total {
        if cancelled() {
            let _ = tx.send(FlashUpdate::Failed(CANCELLED_MSG.into()));
            return;
        }
        let f = i as f32 / steps_total as f32;
        if i == 0 && !send(FlashUpdate::Message("Writing".into())) {
            return;
        }
        if !send(FlashUpdate::Progress {
            progress: f,
            bytes_written: (size as f64 * f as f64) as u64,
            speed_mb_s: speed,
        }) {
            return;
        }
        std::thread::sleep(Duration::from_millis(step_ms));
    }
    let _ = send(FlashUpdate::Message("Verifying".into()));
    for i in 0..=steps_total / 2 {
        if cancelled() {
            let _ = tx.send(FlashUpdate::Failed(CANCELLED_MSG.into()));
            return;
        }
        let f = i as f32 / (steps_total / 2).max(1) as f32;
        if !send(FlashUpdate::VerifyProgress {
            phase: if f < 0.5 { "image" } else { "device" },
            overall: f,
            bytes_read: (size as f64 * f as f64) as u64,
            total_bytes: size,
            speed_mb_s: speed * 2.0,
        }) {
            return;
        }
        std::thread::sleep(Duration::from_millis(step_ms));
    }
    let _ = tx.send(FlashUpdate::Completed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flash_helper::FlashStage;

    fn collect(rx: mpsc::Receiver<FlashUpdate>) -> Vec<FlashUpdate> {
        rx.iter().collect()
    }

    fn run(events: Vec<FlashEvent>, cancelled: bool) -> Vec<FlashUpdate> {
        let cancel = AtomicBool::new(cancelled);
        let mut out = Vec::new();
        forward_events(events, &cancel, |u| {
            out.push(u);
            true
        });
        out
    }

    #[test]
    fn terminal_detection() {
        assert!(is_terminal(&FlashUpdate::Completed));
        assert!(is_terminal(&FlashUpdate::Failed("x".into())));
        assert!(!is_terminal(&FlashUpdate::Message("x".into())));
    }

    #[test]
    fn forwards_in_order_and_stops_at_done() {
        let out = run(
            vec![
                FlashEvent::Stage(FlashStage::Writing),
                FlashEvent::Progress {
                    bytes_written: 1,
                    total_bytes: 2,
                    speed_mb_s: 1.0,
                },
                FlashEvent::Done,
                FlashEvent::Log("late".into()),
            ],
            false,
        );
        assert_eq!(out.len(), 3, "events after Done are dropped: {out:?}");
        assert!(matches!(out[2], FlashUpdate::Completed));
    }

    #[test]
    fn error_is_terminal() {
        let out = run(
            vec![FlashEvent::Error("boom".into()), FlashEvent::Done],
            false,
        );
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0], FlashUpdate::Failed(m) if m == "boom"));
    }

    #[test]
    fn missing_verdict_becomes_terminated() {
        let out = run(vec![FlashEvent::Log("hi".into())], false);
        assert!(matches!(out.last(), Some(FlashUpdate::Failed(m)) if m == TERMINATED_MSG));
    }

    #[test]
    fn missing_verdict_while_cancelled_becomes_cancelled() {
        let out = run(vec![], true);
        assert!(matches!(&out[..], [FlashUpdate::Failed(m)] if m == CANCELLED_MSG));
    }

    #[test]
    fn stops_when_consumer_is_gone() {
        let cancel = AtomicBool::new(false);
        let mut seen = 0;
        let ok = forward_events(
            vec![FlashEvent::Log("a".into()), FlashEvent::Log("b".into())],
            &cancel,
            |_| {
                seen += 1;
                false
            },
        );
        assert!(!ok);
        assert_eq!(seen, 1);
    }

    #[test]
    fn preflight_rejects_missing_empty_and_directory() {
        let dir = tempfile::tempdir().unwrap();
        let dev = dir.path().join("dev");
        assert!(preflight(&dir.path().join("nope"), &dev)
            .unwrap_err()
            .contains("Cannot read image"));
        let empty = dir.path().join("empty.img");
        std::fs::write(&empty, b"").unwrap();
        assert!(preflight(&empty, &dev).unwrap_err().contains("empty"));
        assert!(preflight(dir.path(), &dev).unwrap_err().contains("regular"));
        let ok = dir.path().join("ok.img");
        std::fs::write(&ok, b"abc").unwrap();
        assert_eq!(preflight(&ok, &dev), Ok(3));
    }

    #[test]
    fn bad_input_yields_single_failed_and_closes() {
        let rx = spawn_flash_worker(
            Path::new("/nonexistent/x.img"),
            Path::new("/dev/null"),
            Arc::new(AtomicBool::new(false)),
        );
        let all = collect(rx);
        assert_eq!(all.len(), 1);
        assert!(matches!(all[0], FlashUpdate::Failed(_)));
    }

    #[test]
    fn end_to_end_on_temp_files_ends_with_one_terminal() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.img");
        let dev = dir.path().join("dev.bin");
        let data: Vec<u8> = (0..=255u8).cycle().take(512 * 1024).collect();
        std::fs::write(&img, &data).unwrap();
        std::fs::File::create(&dev).unwrap();

        let rx = spawn_flash_worker(&img, &dev, Arc::new(AtomicBool::new(false)));
        let all = collect(rx);
        assert!(all
            .iter()
            .any(|u| matches!(u, FlashUpdate::Progress { .. })));
        let terminals = all.iter().filter(|u| is_terminal(u)).count();
        assert_eq!(terminals, 1, "exactly one terminal update: {all:?}");
        assert!(is_terminal(all.last().unwrap()), "terminal is last");
        if matches!(all.last(), Some(FlashUpdate::Completed)) {
            assert_eq!(std::fs::read(&dev).unwrap(), data);
        }
    }

    #[test]
    fn pre_cancelled_flash_fails_with_cancel_message_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.img");
        let dev = dir.path().join("dev.bin");
        std::fs::write(&img, vec![7u8; 1024 * 1024]).unwrap();
        std::fs::File::create(&dev).unwrap();

        let rx = spawn_flash_worker(&img, &dev, Arc::new(AtomicBool::new(true)));
        let all = collect(rx);
        match all.last() {
            Some(FlashUpdate::Failed(m)) => assert!(m.contains("cancelled"), "{m}"),
            other => panic!("expected cancelled failure, got {other:?}"),
        }
        assert!(
            std::fs::read(&dev).unwrap().len() <= 4 * 1024 * 1024,
            "at most one block may be written before the cancel is seen"
        );
    }

    #[test]
    fn simulation_ends_with_one_completed_and_monotonic_progress() {
        let (tx, rx) = mpsc::channel();
        simulate_flash("/dev/sdx", 1000, 0, &AtomicBool::new(false), &tx);
        drop(tx);
        let all: Vec<_> = rx.iter().collect();
        assert!(matches!(all.last(), Some(FlashUpdate::Completed)));
        assert_eq!(all.iter().filter(|u| is_terminal(u)).count(), 1);
        let writes: Vec<f32> = all
            .iter()
            .filter_map(|u| match u {
                FlashUpdate::Progress { progress, .. } => Some(*progress),
                _ => None,
            })
            .collect();
        assert!(writes.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(writes.last(), Some(&1.0));
    }

    #[test]
    fn simulation_honors_cancel() {
        let (tx, rx) = mpsc::channel();
        simulate_flash("/dev/sdx", 1000, 0, &AtomicBool::new(true), &tx);
        drop(tx);
        let all: Vec<_> = rx.iter().collect();
        assert!(matches!(all.last(), Some(FlashUpdate::Failed(m)) if m == CANCELLED_MSG));
    }

    #[test]
    fn devices_get_different_paces() {
        let count = |d: &str| {
            let (tx, rx) = mpsc::channel();
            simulate_flash(d, 10, 0, &AtomicBool::new(false), &tx);
            drop(tx);
            rx.iter().count()
        };
        assert_ne!(count("/dev/sda"), count("/dev/sdq"));
    }
}
