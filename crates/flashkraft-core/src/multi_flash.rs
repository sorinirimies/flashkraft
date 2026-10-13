//! Flash one image to several devices at once.
//!
//! [`spawn_multi_flash`] runs one [`spawn_flash_worker`] per target, at most
//! `max_parallel` at a time (USB hubs share bandwidth, so unlimited fan-out
//! is not always faster), and merges every worker's updates into a single
//! channel tagged with the target. When every target has reached a verdict a
//! final [`MultiUpdate::Finished`] carries the summary, then the channel
//! closes.
//!
//! One failing device never stops the others; a single `cancel` flag stops
//! them all (running targets abort between blocks, queued ones never start
//! and are reported as cancelled).
//!
//! [`MultiState`] folds the stream into per-target state for a UI.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};

use crate::flash_helper::FlashUpdate;
use crate::flash_worker::{is_terminal, spawn_flash_worker, CANCELLED_MSG};

/// Default number of devices written simultaneously.
pub const DEFAULT_MAX_PARALLEL: usize = 4;

/// One message of a multi-device flash.
#[derive(Debug, Clone)]
pub enum MultiUpdate {
    /// An update for target `index` (position in the list given to the spawner).
    Target {
        index: usize,
        device: String,
        update: FlashUpdate,
    },
    /// Every target finished; always the last message.
    Finished(MultiSummary),
}

/// Outcome of a multi-device flash.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultiSummary {
    pub succeeded: Vec<String>,
    /// `(device, error)`.
    pub failed: Vec<(String, String)>,
}

impl MultiSummary {
    /// `true` when nothing failed.
    pub fn all_ok(&self) -> bool {
        self.failed.is_empty()
    }
    pub fn total(&self) -> usize {
        self.succeeded.len() + self.failed.len()
    }
}

/// Remove duplicate device paths, keeping first-seen order.
pub fn dedupe_devices(devices: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    devices
        .iter()
        .filter(|d| seen.insert((*d).clone()))
        .cloned()
        .collect()
}

/// Flash `image` to every device in `devices`.
///
/// Duplicates are dropped. An empty list yields just an empty
/// [`MultiUpdate::Finished`]. `max_parallel` is clamped to `1..=devices`.
pub fn spawn_multi_flash(
    image: &Path,
    devices: &[PathBuf],
    max_parallel: usize,
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<MultiUpdate> {
    let (tx, rx) = mpsc::channel::<MultiUpdate>();
    let devices = dedupe_devices(devices);
    if devices.is_empty() {
        let _ = tx.send(MultiUpdate::Finished(MultiSummary::default()));
        return rx;
    }
    let lanes = max_parallel.clamp(1, devices.len());
    let queue: Arc<Mutex<VecDeque<(usize, PathBuf)>>> =
        Arc::new(Mutex::new(devices.into_iter().enumerate().collect()));
    let summary = Arc::new(Mutex::new(MultiSummary::default()));
    let image = image.to_path_buf();

    let handles: Vec<_> = (0..lanes)
        .filter_map(|lane| {
            let (queue, summary, tx, cancel, image) = (
                queue.clone(),
                summary.clone(),
                tx.clone(),
                cancel.clone(),
                image.clone(),
            );
            std::thread::Builder::new()
                .name(format!("flashkraft-lane-{lane}"))
                .spawn(move || loop {
                    let next = queue.lock().ok().and_then(|mut q| q.pop_front());
                    let Some((index, device)) = next else { break };
                    run_target(&image, index, &device, &cancel, &tx, &summary);
                })
                .ok()
        })
        .collect();

    // Finisher: waits for every lane, then reports.
    let spawned = std::thread::Builder::new()
        .name("flashkraft-multi".into())
        .spawn(move || {
            for h in handles {
                let _ = h.join();
            }
            // Anything still queued (a lane failed to start) never ran.
            let mut s = summary.lock().map(|g| g.clone()).unwrap_or_default();
            if let Ok(mut q) = queue.lock() {
                for (_, d) in q.drain(..) {
                    s.failed
                        .push((d.display().to_string(), "Flash lane unavailable".into()));
                }
            }
            let _ = tx.send(MultiUpdate::Finished(s));
        });
    if spawned.is_err() {
        // Cannot happen short of resource exhaustion; the receiver just closes.
    }
    rx
}

fn run_target(
    image: &Path,
    index: usize,
    device: &Path,
    cancel: &Arc<AtomicBool>,
    tx: &mpsc::Sender<MultiUpdate>,
    summary: &Arc<Mutex<MultiSummary>>,
) {
    let name = device.display().to_string();
    let send = |update: FlashUpdate| {
        let _ = tx.send(MultiUpdate::Target {
            index,
            device: name.clone(),
            update,
        });
    };
    let record = |ok: Result<(), String>| {
        if let Ok(mut s) = summary.lock() {
            match ok {
                Ok(()) => s.succeeded.push(name.clone()),
                Err(e) => s.failed.push((name.clone(), e)),
            }
        }
    };

    // A target that has not started when the user cancels never starts.
    if cancel.load(Ordering::SeqCst) {
        send(FlashUpdate::Failed(CANCELLED_MSG.into()));
        record(Err(CANCELLED_MSG.into()));
        return;
    }

    let rx = spawn_flash_worker(image, device, cancel.clone());
    let mut verdict: Option<Result<(), String>> = None;
    for update in rx {
        let terminal = is_terminal(&update);
        match &update {
            FlashUpdate::Completed => verdict = Some(Ok(())),
            FlashUpdate::Failed(e) => verdict = Some(Err(e.clone())),
            _ => {}
        }
        send(update);
        if terminal {
            break;
        }
    }
    record(verdict.unwrap_or_else(|| Err("Flash worker vanished".into())));
}

// ---------------------------------------------------------------------------
// UI-side state
// ---------------------------------------------------------------------------

/// Where one target is in its flash.
#[derive(Debug, Clone, PartialEq)]
pub enum TargetStatus {
    Queued,
    /// `progress` is the overall `0.0–1.0` (write counts for the first 50 %
    /// of the bar, verification for the second — see [`TargetState`]).
    Running,
    Done,
    Failed(String),
}

/// Live state of one target.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetState {
    pub device: String,
    pub status: TargetStatus,
    /// Write fraction `0.0–1.0`.
    pub write: f32,
    /// Verify fraction `0.0–1.0`.
    pub verify: f32,
    pub speed_mb_s: f32,
    /// Latest status line (stage / log).
    pub message: String,
}

impl TargetState {
    fn new(device: String) -> Self {
        Self {
            device,
            status: TargetStatus::Queued,
            write: 0.0,
            verify: 0.0,
            speed_mb_s: 0.0,
            message: String::new(),
        }
    }

    /// Combined `0.0–1.0`: writing is the first half, verifying the second.
    pub fn overall(&self) -> f32 {
        match self.status {
            TargetStatus::Done => 1.0,
            _ => (self.write * 0.5 + self.verify * 0.5).clamp(0.0, 1.0),
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.status, TargetStatus::Done | TargetStatus::Failed(_))
    }
}

/// Folds a [`MultiUpdate`] stream into per-target state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MultiState {
    pub targets: Vec<TargetState>,
    pub summary: Option<MultiSummary>,
}

impl MultiState {
    /// All targets `Queued`.
    pub fn new(devices: &[String]) -> Self {
        Self {
            targets: devices.iter().cloned().map(TargetState::new).collect(),
            summary: None,
        }
    }

    /// Apply one update. Unknown indices are ignored.
    pub fn apply(&mut self, update: &MultiUpdate) {
        match update {
            MultiUpdate::Finished(s) => self.summary = Some(s.clone()),
            MultiUpdate::Target { index, update, .. } => {
                let Some(t) = self.targets.get_mut(*index) else {
                    return;
                };
                if t.is_finished() {
                    return;
                }
                if t.status == TargetStatus::Queued {
                    t.status = TargetStatus::Running;
                }
                match update {
                    FlashUpdate::Progress {
                        progress,
                        speed_mb_s,
                        ..
                    } => {
                        t.write = *progress;
                        t.speed_mb_s = *speed_mb_s;
                    }
                    FlashUpdate::VerifyProgress {
                        overall,
                        speed_mb_s,
                        ..
                    } => {
                        t.write = 1.0;
                        t.verify = *overall;
                        t.speed_mb_s = *speed_mb_s;
                    }
                    FlashUpdate::Message(m) => t.message = m.clone(),
                    FlashUpdate::Completed => {
                        t.status = TargetStatus::Done;
                        t.write = 1.0;
                        t.verify = 1.0;
                    }
                    FlashUpdate::Failed(e) => t.status = TargetStatus::Failed(e.clone()),
                }
            }
        }
    }

    /// Mean of every target's [`TargetState::overall`].
    pub fn overall(&self) -> f32 {
        if self.targets.is_empty() {
            return 0.0;
        }
        self.targets.iter().map(TargetState::overall).sum::<f32>() / self.targets.len() as f32
    }

    pub fn running(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| t.status == TargetStatus::Running)
            .count()
    }
    pub fn done(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| t.status == TargetStatus::Done)
            .count()
    }
    pub fn failed(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| matches!(t.status, TargetStatus::Failed(_)))
            .count()
    }
    /// Combined throughput of everything currently running.
    pub fn total_speed_mb_s(&self) -> f32 {
        self.targets
            .iter()
            .filter(|t| t.status == TargetStatus::Running)
            .map(|t| t.speed_mb_s)
            .sum()
    }
    /// `true` once [`MultiUpdate::Finished`] arrived.
    pub fn is_finished(&self) -> bool {
        self.summary.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(rx: mpsc::Receiver<MultiUpdate>) -> Vec<MultiUpdate> {
        rx.iter().collect()
    }

    fn fixture(n: usize, size: usize) -> (tempfile::TempDir, PathBuf, Vec<PathBuf>) {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.img");
        let data: Vec<u8> = (0..=255u8).cycle().take(size).collect();
        std::fs::write(&img, data).unwrap();
        let devs = (0..n)
            .map(|i| {
                let p = dir.path().join(format!("dev{i}.bin"));
                std::fs::File::create(&p).unwrap();
                p
            })
            .collect();
        (dir, img, devs)
    }

    fn finished(all: &[MultiUpdate]) -> &MultiSummary {
        match all.last() {
            Some(MultiUpdate::Finished(s)) => s,
            other => panic!("last message must be Finished, got {other:?}"),
        }
    }

    #[test]
    fn dedupe_keeps_first_seen_order() {
        let d = dedupe_devices(&["/a".into(), "/b".into(), "/a".into(), "/c".into()]);
        assert_eq!(d, vec![PathBuf::from("/a"), "/b".into(), "/c".into()]);
    }

    #[test]
    fn empty_list_finishes_immediately() {
        let rx = spawn_multi_flash(Path::new("x"), &[], 4, Arc::new(AtomicBool::new(false)));
        let all = collect(rx);
        assert_eq!(all.len(), 1);
        assert_eq!(finished(&all).total(), 0);
        assert!(finished(&all).all_ok());
    }

    #[test]
    fn three_targets_each_get_a_verdict_and_one_finished() {
        let (_d, img, devs) = fixture(3, 600 * 1024);
        let rx = spawn_multi_flash(&img, &devs, 3, Arc::new(AtomicBool::new(false)));
        let all = collect(rx);
        let summary = finished(&all).clone();
        assert_eq!(summary.total(), 3, "{summary:?}");
        for i in 0..3 {
            let terminals = all
                .iter()
                .filter(|m| matches!(m, MultiUpdate::Target { index, update, .. } if *index == i && is_terminal(update)))
                .count();
            assert_eq!(terminals, 1, "target {i} has exactly one terminal");
        }
        assert_eq!(
            all.iter()
                .filter(|m| matches!(m, MultiUpdate::Finished(_)))
                .count(),
            1
        );
        let data = std::fs::read(&img).unwrap();
        for ok in &summary.succeeded {
            assert_eq!(std::fs::read(ok).unwrap(), data, "{ok} matches the image");
        }
    }

    #[test]
    fn one_bad_target_does_not_stop_the_others() {
        let (dir, img, mut devs) = fixture(2, 300 * 1024);
        devs.insert(1, dir.path().join("missing.bin"));
        let rx = spawn_multi_flash(&img, &devs, 2, Arc::new(AtomicBool::new(false)));
        let all = collect(rx);
        let s = finished(&all);
        assert_eq!(s.total(), 3);
        assert_eq!(
            s.failed.len(),
            1 + s
                .failed
                .iter()
                .filter(|(d, _)| !d.contains("missing"))
                .count()
        );
        assert!(
            s.failed
                .iter()
                .any(|(d, e)| d.contains("missing") && e.contains("not found")),
            "{s:?}"
        );
    }

    #[test]
    fn sequential_lane_runs_every_target() {
        let (_d, img, devs) = fixture(3, 200 * 1024);
        let rx = spawn_multi_flash(&img, &devs, 1, Arc::new(AtomicBool::new(false)));
        let all = collect(rx);
        assert_eq!(finished(&all).total(), 3);
    }

    #[test]
    fn pre_cancelled_reports_every_target_cancelled() {
        let (_d, img, devs) = fixture(3, 100 * 1024);
        let rx = spawn_multi_flash(&img, &devs, 2, Arc::new(AtomicBool::new(true)));
        let all = collect(rx);
        let s = finished(&all);
        assert_eq!(s.failed.len(), 3);
        assert!(s.failed.iter().all(|(_, e)| e.contains("cancelled")));
    }

    #[test]
    fn duplicate_devices_are_flashed_once() {
        let (_d, img, devs) = fixture(1, 100 * 1024);
        let twice = vec![devs[0].clone(), devs[0].clone()];
        let all = collect(spawn_multi_flash(
            &img,
            &twice,
            2,
            Arc::new(AtomicBool::new(false)),
        ));
        assert_eq!(finished(&all).total(), 1);
    }

    // ── MultiState ───────────────────────────────────────────────────────────

    fn up(i: usize, u: FlashUpdate) -> MultiUpdate {
        MultiUpdate::Target {
            index: i,
            device: format!("/dev/sd{i}"),
            update: u,
        }
    }

    #[test]
    fn state_tracks_progress_and_completion() {
        let mut st = MultiState::new(&["/dev/sd0".into(), "/dev/sd1".into()]);
        assert_eq!(st.running(), 0);
        st.apply(&up(
            0,
            FlashUpdate::Progress {
                progress: 0.5,
                bytes_written: 1,
                speed_mb_s: 30.0,
            },
        ));
        st.apply(&up(
            1,
            FlashUpdate::Progress {
                progress: 1.0,
                bytes_written: 2,
                speed_mb_s: 20.0,
            },
        ));
        assert_eq!(st.running(), 2);
        assert!((st.total_speed_mb_s() - 50.0).abs() < 1e-3);
        assert!((st.targets[0].overall() - 0.25).abs() < 1e-6);
        st.apply(&up(0, FlashUpdate::Completed));
        st.apply(&up(1, FlashUpdate::Failed("boom".into())));
        assert_eq!((st.done(), st.failed(), st.running()), (1, 1, 0));
        assert!((st.overall() - 0.5 * (1.0 + 0.5)).abs() < 1e-6);
        assert!(!st.is_finished());
        st.apply(&MultiUpdate::Finished(MultiSummary::default()));
        assert!(st.is_finished());
    }

    #[test]
    fn finished_targets_ignore_late_updates_and_bad_indices_are_ignored() {
        let mut st = MultiState::new(&["/dev/sd0".into()]);
        st.apply(&up(0, FlashUpdate::Completed));
        st.apply(&up(0, FlashUpdate::Failed("late".into())));
        assert_eq!(st.targets[0].status, TargetStatus::Done);
        st.apply(&up(9, FlashUpdate::Completed));
        assert_eq!(st.targets.len(), 1);
    }

    #[test]
    fn verify_progress_fills_the_second_half() {
        let mut st = MultiState::new(&["/dev/sd0".into()]);
        st.apply(&up(
            0,
            FlashUpdate::VerifyProgress {
                phase: "device",
                overall: 0.5,
                bytes_read: 1,
                total_bytes: 2,
                speed_mb_s: 1.0,
            },
        ));
        assert!((st.targets[0].overall() - 0.75).abs() < 1e-6);
        assert_eq!(MultiState::default().overall(), 0.0);
    }
}
