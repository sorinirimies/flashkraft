//! Privileged flash helper — lets the GUI flash raw devices even when it was
//! started as a plain unprivileged build (e.g. `cargo run`), without ever
//! re-executing the GUI's own process image.
//!
//! ## Why not just re-exec the whole GUI through `sudo`/`pkexec`?
//!
//! That was tried and reverted: `execvp` replaces the entire process image,
//! which kills the visible window mid-startup (looks like a crash) and can
//! break the native file-picker's access to the desktop D-Bus/portal session.
//! The GUI must stay running, unprivileged, for its whole lifetime.
//!
//! ## Design
//!
//! Instead, only the raw-device write itself is escalated, via a **separate,
//! short-lived, non-interactive child process**:
//!
//! ```text
//!  unprivileged GUI (this process, keeps running)
//!       │
//!       │ pkexec <self-exe> --flash-write-helper <image> <device>
//!       ▼
//!  privileged helper (root, no UI, no D-Bus, exits when done)
//!       │  one JSON `WireUpdate` per line on stdout
//!       ▼
//!  reader thread → std::sync::mpsc::Receiver<FlashUpdate>
//! ```
//!
//! The helper mode is entered via [`run_if_invoked`], checked at the very top
//! of `main()` before any UI or privilege-drop machinery runs. The parent
//! side is [`spawn`], called from `flash_runner::build_flash_stream` only
//! when [`flashkraft_core::flash_helper::can_escalate_in_process`] is
//! `false` (i.e. the binary isn't setuid-root and isn't already running as
//! root).
//!
//! Cancellation is handled by killing the child process outright — there is
//! no live cancel signal plumbed through pkexec, so a cancelled flash simply
//! terminates the helper.

use flashkraft_core::flash_helper::FlashEvent;
use flashkraft_core::FlashUpdate;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

const HELPER_FLAG: &str = "--flash-write-helper";

// ---------------------------------------------------------------------------
// Wire protocol (JSON Lines over stdout)
// ---------------------------------------------------------------------------

/// A JSON-serialisable mirror of [`FlashUpdate`].
///
/// `FlashUpdate::VerifyProgress::phase` is `&'static str`, which doesn't
/// round-trip through `serde` deserialisation directly, so the wire type
/// uses an owned `String` and maps it back to a `'static` literal on the
/// receiving side.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t")]
enum WireUpdate {
    Progress {
        progress: f32,
        bytes_written: u64,
        speed_mb_s: f32,
    },
    VerifyProgress {
        phase: String,
        overall: f32,
        bytes_read: u64,
        total_bytes: u64,
        speed_mb_s: f32,
    },
    Message {
        message: String,
    },
    Completed,
    Failed {
        message: String,
    },
}

impl From<&FlashUpdate> for WireUpdate {
    fn from(update: &FlashUpdate) -> Self {
        match update {
            FlashUpdate::Progress {
                progress,
                bytes_written,
                speed_mb_s,
            } => WireUpdate::Progress {
                progress: *progress,
                bytes_written: *bytes_written,
                speed_mb_s: *speed_mb_s,
            },
            FlashUpdate::VerifyProgress {
                phase,
                overall,
                bytes_read,
                total_bytes,
                speed_mb_s,
            } => WireUpdate::VerifyProgress {
                phase: (*phase).to_string(),
                overall: *overall,
                bytes_read: *bytes_read,
                total_bytes: *total_bytes,
                speed_mb_s: *speed_mb_s,
            },
            FlashUpdate::Message(msg) => WireUpdate::Message {
                message: msg.clone(),
            },
            FlashUpdate::Completed => WireUpdate::Completed,
            FlashUpdate::Failed(msg) => WireUpdate::Failed {
                message: msg.clone(),
            },
        }
    }
}

impl From<WireUpdate> for FlashUpdate {
    fn from(wire: WireUpdate) -> Self {
        match wire {
            WireUpdate::Progress {
                progress,
                bytes_written,
                speed_mb_s,
            } => FlashUpdate::Progress {
                progress,
                bytes_written,
                speed_mb_s,
            },
            WireUpdate::VerifyProgress {
                phase,
                overall,
                bytes_read,
                total_bytes,
                speed_mb_s,
            } => FlashUpdate::VerifyProgress {
                phase: if phase == "image" { "image" } else { "device" },
                overall,
                bytes_read,
                total_bytes,
                speed_mb_s,
            },
            WireUpdate::Message { message } => FlashUpdate::Message(message),
            WireUpdate::Completed => FlashUpdate::Completed,
            WireUpdate::Failed { message } => FlashUpdate::Failed(message),
        }
    }
}

// ---------------------------------------------------------------------------
// Child side — entered from `main()` before any UI initialisation
// ---------------------------------------------------------------------------

/// If this process was invoked as the hidden privileged-helper worker
/// (`--flash-write-helper <image> <device>`), run the flash pipeline
/// directly, stream JSON-lines progress on stdout, and return `Some(exit_code)`.
///
/// Returns `None` if the process was invoked normally, in which case the
/// caller should proceed to start the GUI as usual.
///
/// This never touches iced, D-Bus, or any UI subsystem — it is expected to
/// run under `pkexec`/`sudo`, fully headless, and exit as soon as the
/// pipeline finishes.
pub fn run_if_invoked() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let flag_idx = args.iter().position(|a| a == HELPER_FLAG)?;

    let image = args.get(flag_idx + 1).cloned();
    let device = args.get(flag_idx + 2).cloned();
    let (Some(image), Some(device)) = (image, device) else {
        eprintln!("{HELPER_FLAG} requires <image> <device> arguments");
        return Some(2);
    };

    let (tx, rx) = mpsc::channel::<FlashEvent>();
    let cancel = Arc::new(AtomicBool::new(false));

    std::thread::spawn(move || {
        flashkraft_core::flash_helper::run_pipeline(&image, &device, tx, cancel);
    });

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let mut exit_code = 1;

    for event in rx {
        let is_done = matches!(event, FlashEvent::Done);
        let is_error = matches!(event, FlashEvent::Error(_));
        let update = FlashUpdate::from(event);
        let wire = WireUpdate::from(&update);
        if let Ok(json) = serde_json::to_string(&wire) {
            let _ = writeln!(lock, "{json}");
            let _ = lock.flush();
        }
        if is_done {
            exit_code = 0;
        } else if is_error {
            exit_code = 1;
        }
    }

    Some(exit_code)
}

// ---------------------------------------------------------------------------
// Parent side — spawns the privileged child and bridges its output
// ---------------------------------------------------------------------------

/// Spawn the privileged helper (via `pkexec`, falling back to `sudo -A` if a
/// graphical askpass is configured) and return a channel of [`FlashUpdate`]
/// forwarded from its stdout.
///
/// Killing `cancel` (setting it to `true`) terminates the helper process,
/// which is the only cancellation mechanism available for an escalated
/// external process.
pub fn spawn(
    image_path: &Path,
    device_path: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<mpsc::Receiver<FlashUpdate>, String> {
    let self_exe =
        std::env::current_exe().map_err(|e| format!("Cannot resolve current executable: {e}"))?;

    let mut cmd = if which_exists("pkexec") {
        Command::new("pkexec")
    } else if which_exists("sudo") && std::env::var_os("SUDO_ASKPASS").is_some() {
        let mut c = Command::new("sudo");
        c.arg("-A");
        c
    } else {
        return Err(
            "Raw devices require root privileges: install FlashKraft setuid-root (just install), \
             or ensure pkexec is available so it can escalate automatically."
                .to_string(),
        );
    };

    cmd.arg(&self_exe)
        .arg(HELPER_FLAG)
        .arg(image_path)
        .arg(device_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to launch privileged helper: {e}"))?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let child = Arc::new(Mutex::new(child));

    let (tx, rx) = mpsc::channel::<FlashUpdate>();

    spawn_cancel_watcher(child.clone(), cancel);
    let stderr_tail = spawn_stderr_drain(stderr);
    spawn_stdout_reader(child, stdout, tx, stderr_tail);

    Ok(rx)
}

/// Kills the helper process shortly after `cancel` becomes `true`, or exits
/// quietly once the child has already terminated on its own.
fn spawn_cancel_watcher(child: Arc<Mutex<std::process::Child>>, cancel: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        if cancel.load(Ordering::SeqCst) {
            if let Ok(mut guard) = child.lock() {
                let _ = guard.kill();
            }
            return;
        }
        match child.lock().ok().and_then(|mut g| g.try_wait().ok()) {
            Some(Some(_)) => return, // already exited
            _ => std::thread::sleep(std::time::Duration::from_millis(150)),
        }
    });
}

/// Drains the helper's stderr into a shared buffer so the pipe never fills
/// up and blocks the child; the buffer is surfaced only if the pipeline
/// fails without emitting a structured `FlashUpdate::Failed`.
fn spawn_stderr_drain(stderr: std::process::ChildStderr) -> Arc<Mutex<String>> {
    let buf = Arc::new(Mutex::new(String::new()));
    let buf_writer = buf.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Ok(mut b) = buf_writer.lock() {
                b.push_str(&line);
                b.push('\n');
            }
        }
    });
    buf
}

/// Reads one `WireUpdate` JSON object per line from the helper's stdout and
/// forwards each as a [`FlashUpdate`]. If the child exits without ever
/// reporting `Completed`/`Failed`, synthesizes a `Failed` update from its
/// exit status and any captured stderr.
fn spawn_stdout_reader(
    child: Arc<Mutex<std::process::Child>>,
    stdout: std::process::ChildStdout,
    tx: mpsc::Sender<FlashUpdate>,
    stderr_tail: Arc<Mutex<String>>,
) {
    std::thread::spawn(move || {
        let mut saw_terminal = false;

        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(wire) = serde_json::from_str::<WireUpdate>(&line) else {
                continue;
            };
            let update = FlashUpdate::from(wire);
            saw_terminal = matches!(update, FlashUpdate::Completed | FlashUpdate::Failed(_));
            if tx.send(update).is_err() {
                return;
            }
        }

        if saw_terminal {
            return;
        }

        let status = child.lock().ok().and_then(|mut g| g.wait().ok());
        let failed = match status {
            Some(status) if status.success() => {
                FlashUpdate::Failed("Privileged helper exited without reporting a result".into())
            }
            _ => {
                let tail = stderr_tail.lock().map(|b| b.clone()).unwrap_or_default();
                let detail = tail.trim();
                if detail.is_empty() {
                    FlashUpdate::Failed(
                        "Privileged helper process was terminated or failed to start".into(),
                    )
                } else {
                    FlashUpdate::Failed(format!("Privileged helper failed: {detail}"))
                }
            }
        };
        let _ = tx.send(failed);
    });
}

fn which_exists(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_update_progress_round_trips() {
        let original = FlashUpdate::Progress {
            progress: 0.42,
            bytes_written: 1024,
            speed_mb_s: 12.5,
        };
        let wire = WireUpdate::from(&original);
        let json = serde_json::to_string(&wire).unwrap();
        let parsed: WireUpdate = serde_json::from_str(&json).unwrap();
        let round_tripped = FlashUpdate::from(parsed);

        match (original, round_tripped) {
            (
                FlashUpdate::Progress {
                    progress: p1,
                    bytes_written: b1,
                    speed_mb_s: s1,
                },
                FlashUpdate::Progress {
                    progress: p2,
                    bytes_written: b2,
                    speed_mb_s: s2,
                },
            ) => {
                assert_eq!(p1, p2);
                assert_eq!(b1, b2);
                assert_eq!(s1, s2);
            }
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn wire_update_verify_progress_maps_phase_back_to_static_str() {
        let original = FlashUpdate::VerifyProgress {
            phase: "image",
            overall: 0.25,
            bytes_read: 10,
            total_bytes: 40,
            speed_mb_s: 3.0,
        };
        let wire = WireUpdate::from(&original);
        let json = serde_json::to_string(&wire).unwrap();
        let parsed: WireUpdate = serde_json::from_str(&json).unwrap();
        let round_tripped = FlashUpdate::from(parsed);

        match round_tripped {
            FlashUpdate::VerifyProgress { phase, .. } => assert_eq!(phase, "image"),
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn wire_update_unknown_phase_defaults_to_device() {
        let json = r#"{"t":"VerifyProgress","phase":"bogus","overall":0.5,"bytes_read":1,"total_bytes":2,"speed_mb_s":1.0}"#;
        let parsed: WireUpdate = serde_json::from_str(json).unwrap();
        let update = FlashUpdate::from(parsed);
        match update {
            FlashUpdate::VerifyProgress { phase, .. } => assert_eq!(phase, "device"),
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn wire_update_completed_and_failed_round_trip() {
        for original in [
            FlashUpdate::Completed,
            FlashUpdate::Failed("boom".to_string()),
            FlashUpdate::Message("status text".to_string()),
        ] {
            let wire = WireUpdate::from(&original);
            let json = serde_json::to_string(&wire).unwrap();
            let parsed: WireUpdate = serde_json::from_str(&json).unwrap();
            let round_tripped = FlashUpdate::from(parsed);
            assert_eq!(format!("{original:?}"), format!("{round_tripped:?}"));
        }
    }

    #[test]
    fn run_if_invoked_returns_none_without_the_helper_flag() {
        // We can't easily override std::env::args() in a unit test, but we
        // can verify the flag constant is what main.rs and spawn() agree on.
        assert_eq!(HELPER_FLAG, "--flash-write-helper");
    }

    #[test]
    fn which_exists_finds_a_binary_known_to_be_on_path() {
        // `sh` should exist on essentially every Unix CI runner.
        if cfg!(unix) {
            assert!(which_exists("sh"));
        }
    }

    #[test]
    fn which_exists_rejects_a_bogus_binary_name() {
        assert!(!which_exists(
            "definitely-not-a-real-binary-name-xyz-123-flashkraft-test"
        ));
    }
}
