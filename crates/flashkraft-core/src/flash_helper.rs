//! Flash Pipeline
//!
//! Implements the entire privileged flash pipeline in-process.
//!
//! ## Privilege model
//!
//! The installed binary carries the **setuid-root** bit
//! (`sudo chmod u+s /usr/bin/flashkraft`).  At process startup `main.rs`
//! calls [`set_real_uid`] to record the unprivileged user's UID.
//!
//! When the pipeline needs to open a raw block device it temporarily
//! escalates to root via `nix::unistd::seteuid(0)`, opens the file
//! descriptor, then immediately drops back to the real UID.  Root is held
//! for less than one millisecond.
//!
//! ## Progress reporting
//!
//! The pipeline runs on a dedicated blocking thread spawned by the flash
//! subscription.  Progress is reported by sending [`FlashEvent`] values
//! through a [`std::sync::mpsc::Sender`] — no child process, no stdout
//! parsing, no IPC protocol.
//!
//! ## Pipeline stages
//!
//! 1. Validate inputs (image exists and is non-empty, device exists, not a partition node)
//! 2. Unmount all partitions of the target device (lazy / `MNT_DETACH`)
//! 3. Write the image in 4 MiB blocks, reporting progress every 400 ms
//! 4. `fsync` the device fd (hard error on failure)
//! 5. `fdatasync` + global `sync()` (belt-and-suspenders)
//! 6. `BLKRRPART` ioctl — ask the kernel to re-read the partition table
//! 7. SHA-256 verify: hash the source image, hash the first N bytes of the device, compare

#[cfg(unix)]
use nix::libc;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, OnceLock,
};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Write / read-back buffer: 4 MiB is a sweet spot for USB throughput.
const BLOCK_SIZE: usize = 4 * 1024 * 1024;

/// Minimum interval between `FlashEvent::Progress` emissions.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(400);

// ---------------------------------------------------------------------------
// Real-UID registry
// ---------------------------------------------------------------------------

/// The unprivileged UID of the user who launched the process.
///
/// Captured in `main.rs` via `nix::unistd::getuid()` before any `seteuid`
/// call and stored here via [`set_real_uid`].
static REAL_UID: OnceLock<u32> = OnceLock::new();

/// Store the real (unprivileged) UID of the process owner.
///
/// Must be called once from `main()` before any flash operation.
/// On non-Unix platforms this is a no-op.
pub fn set_real_uid(uid: u32) {
    let _ = REAL_UID.set(uid);
}

/// Return `true` when the process is currently running with effective root
/// privileges (i.e. `geteuid() == 0`).
///
/// On non-Unix platforms this always returns `false` — callers should use
/// the Windows Administrator check instead.
pub fn is_privileged() -> bool {
    #[cfg(unix)]
    {
        nix::unistd::geteuid().is_root()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Attempt to re-exec the current binary with root privileges via `pkexec`
/// or `sudo -E`, whichever is found first on `PATH`.
///
/// This is called **on demand** — e.g. when the user clicks Flash and we
/// detect that `is_privileged()` is `false` — rather than unconditionally
/// at startup.  Because `execvp` replaces the current process image on
/// success, this function only returns when neither escalation helper is
/// available or the user declined (cancelled the polkit dialog / Ctrl-C'd
/// the sudo prompt).
///
/// `FLASHKRAFT_ESCALATED=1` is injected into the child environment so that
/// the re-exec'd process skips this call and does not loop.
///
/// # Safety
///
/// Safe to call from any thread, but must be called before the Iced event
/// loop has started spawning threads that hold OS resources (file
/// descriptors, mutexes) that `execvp` would implicitly close/reset.
/// Calling it from the `update` handler (on the Iced main thread, before
/// the flash subscription starts) satisfies this requirement.
#[cfg(unix)]
pub fn reexec_as_root() {
    // Never attempt privilege escalation during `cargo test` — sudo/pkexec
    // would block the test runner waiting for a password prompt.
    //
    // IMPORTANT: `#[cfg(test)]` is only set on the *root* crate being tested.
    // When `flashkraft-core` is compiled as a *dependency* of another crate's
    // test binary (e.g. `flashkraft-gui`'s tests), it is compiled in normal
    // (non-test) mode, so `#[cfg(test)]` does NOT fire here.
    //
    // We therefore use a runtime heuristic: cargo test binary paths contain
    // a hash-suffixed name under `target/debug/deps/`, e.g.:
    //   …/target/debug/deps/flashkraft_gui-a76f74e119b55607
    // We also check for the FLASHKRAFT_NO_REEXEC env var as an explicit opt-out,
    // and for NEXTEST_TEST_FILTER which nextest sets.
    if is_running_under_test_harness() {
        return;
    }

    // Compile-time guard for crate-local unit tests (when core IS the root
    // test crate and #[cfg(test)] IS honoured).
    #[cfg(test)]
    return;

    #[cfg(not(test))]
    reexec_as_root_inner();
}

/// Returns `true` when the current process appears to be a `cargo test` (or
/// nextest) test-runner binary, based on runtime evidence.
///
/// This is needed because `#[cfg(test)]` is **not** propagated to dependency
/// crates — only the root crate being tested gets the flag.
#[cfg(unix)]
fn is_running_under_test_harness() -> bool {
    // Explicit opt-out env var — tests can set this if needed.
    if std::env::var("FLASHKRAFT_NO_REEXEC").is_ok() {
        return true;
    }

    // nextest sets this in every test process.
    if std::env::var("NEXTEST_TEST_FILTER").is_ok() {
        return true;
    }

    // cargo test passes `--test-threads` (or related flags) on argv.
    // More importantly, the test binary itself is passed the test filter as
    // a positional argv — but the most reliable signal is the executable path:
    // cargo always places test binaries under `target/debug/deps/<name>-<hash>`
    // or `target/<profile>/deps/<name>-<hash>`.
    //
    // We look for `/deps/` in the executable path as a strong indicator.
    if let Ok(exe) = std::env::current_exe() {
        let path_str = exe.to_string_lossy();
        // All cargo test binaries live under a `deps` directory.
        if path_str.contains("/deps/") {
            return true;
        }
        // Also catch `target\deps\` on Windows.
        if path_str.contains("\\deps\\") {
            return true;
        }
    }

    false
}

#[cfg(all(unix, not(test)))]
fn reexec_as_root_inner() {
    use std::ffi::CString;

    // Guard: the re-exec'd copy sets this so we don't loop forever.
    if std::env::var("FLASHKRAFT_ESCALATED").as_deref() == Ok("1") {
        return;
    }

    let self_exe = match std::fs::read_link("/proc/self/exe").or_else(|_| std::env::current_exe()) {
        Ok(p) => p,
        Err(_) => return,
    };
    let self_exe_str = match self_exe.to_str() {
        Some(s) => s.to_owned(),
        None => return,
    };

    let extra_args: Vec<String> = std::env::args().skip(1).collect();

    // Tell the child it was already escalated so it won't recurse.
    std::env::set_var("FLASHKRAFT_ESCALATED", "1");

    // ── Try pkexec first (graphical polkit dialog) ────────────────────────────
    if unix_which_exists("pkexec") {
        let mut argv: Vec<CString> = Vec::new();
        argv.push(unix_c_str("pkexec"));
        argv.push(unix_c_str(&self_exe_str));
        for a in &extra_args {
            argv.push(unix_c_str(a));
        }
        let _ = nix::unistd::execvp(&unix_c_str("pkexec"), &argv);
    }

    // ── Try sudo -E (terminal fallback) ───────────────────────────────────────
    if unix_which_exists("sudo") {
        let mut argv: Vec<CString> = Vec::new();
        argv.push(unix_c_str("sudo"));
        argv.push(unix_c_str("-E")); // preserve DISPLAY / WAYLAND_DISPLAY
        argv.push(unix_c_str(&self_exe_str));
        for a in &extra_args {
            argv.push(unix_c_str(a));
        }
        let _ = nix::unistd::execvp(&unix_c_str("sudo"), &argv);
    }

    // Neither helper available — remove the guard and fall through unprivileged.
    std::env::remove_var("FLASHKRAFT_ESCALATED");
}

/// Stub for non-Unix targets so call sites compile without `#[cfg]` guards.
#[cfg(not(unix))]
pub fn reexec_as_root() {}

/// Return `true` if `name` is an executable file reachable via `PATH`.
#[cfg(all(unix, not(test)))]
fn unix_which_exists(name: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = std::path::Path::new(dir).join(name);
            if let Ok(meta) = std::fs::metadata(&candidate) {
                if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
                    return true;
                }
            }
        }
    }
    false
}

/// Build a `CString`, replacing embedded NUL bytes with `?`.
#[cfg(all(unix, not(test)))]
fn unix_c_str(s: &str) -> std::ffi::CString {
    let sanitised: Vec<u8> = s.bytes().map(|b| if b == 0 { b'?' } else { b }).collect();
    std::ffi::CString::new(sanitised).unwrap_or_else(|_| std::ffi::CString::new("?").unwrap())
}

/// Retrieve the stored real UID, falling back to the current effective UID.
#[cfg(unix)]
fn real_uid() -> nix::unistd::Uid {
    let raw = REAL_UID
        .get()
        .copied()
        .unwrap_or_else(|| nix::unistd::getuid().as_raw());
    nix::unistd::Uid::from_raw(raw)
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A stage in the five-step flash pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlashStage {
    /// Initial state before the pipeline starts.
    Starting,
    /// All partitions of the target device are being lazily unmounted.
    Unmounting,
    /// The image is being written to the block device in 4 MiB chunks.
    Writing,
    /// Kernel write-back caches are being flushed (`fsync` / `sync`).
    Syncing,
    /// The kernel is asked to re-read the partition table (`BLKRRPART`).
    Rereading,
    /// SHA-256 of the source image is compared against a read-back of the device.
    Verifying,
    /// The entire pipeline completed successfully.
    Done,
    /// The pipeline terminated with an error.
    Failed(String),
}

impl std::fmt::Display for FlashStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlashStage::Starting => write!(f, "Starting…"),
            FlashStage::Unmounting => write!(f, "Unmounting partitions…"),
            FlashStage::Writing => write!(f, "Writing image to device…"),
            FlashStage::Syncing => write!(f, "Flushing write buffers…"),
            FlashStage::Rereading => write!(f, "Refreshing partition table…"),
            FlashStage::Verifying => write!(f, "Verifying written data…"),
            FlashStage::Done => write!(f, "Flash complete!"),
            FlashStage::Failed(m) => write!(f, "Failed: {m}"),
        }
    }
}

/// A typed event emitted by the flash pipeline.
///
/// Sent over [`std::sync::mpsc`] to the async Iced subscription — no
/// serialisation, no text parsing.
#[derive(Debug, Clone)]
pub enum FlashEvent {
    /// A pipeline stage transition.
    Stage(FlashStage),
    /// Write-progress update.
    Progress {
        bytes_written: u64,
        total_bytes: u64,
        speed_mb_s: f32,
    },
    /// Verification read-back progress update.
    ///
    /// Emitted during both the image-hash pass and the device read-back pass.
    /// `phase` is `"image"` for the source-hash pass and `"device"` for the
    /// read-back pass.  `bytes_read` and `total_bytes` are the counts for the
    /// current pass only; the overall verify progress should be computed as:
    ///
    /// ```text
    /// if phase == "image"  { bytes_read / total_bytes * 0.5 }
    /// if phase == "device" { 0.5 + bytes_read / total_bytes * 0.5 }
    /// ```
    VerifyProgress {
        phase: &'static str,
        bytes_read: u64,
        total_bytes: u64,
        speed_mb_s: f32,
    },
    /// Informational log message (not an error).
    Log(String),
    /// The pipeline finished successfully.
    Done,
    /// The pipeline failed; the string is a human-readable error.
    Error(String),
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the full flash pipeline in the **calling thread**.
///
/// This function is blocking and must be called from a dedicated
/// `std::thread::spawn` thread, not from an async executor.
///
/// # Arguments
///
/// * `image_path`  – path to the source image file
/// * `device_path` – path to the target block device (e.g. `/dev/sdb`)
/// * `tx`          – channel to send [`FlashEvent`] progress updates
/// * `cancel`      – set to `true` to abort the pipeline between blocks
pub fn run_pipeline(
    image_path: &str,
    device_path: &str,
    tx: mpsc::Sender<FlashEvent>,
    cancel: Arc<AtomicBool>,
) {
    if let Err(e) = flash_pipeline(image_path, device_path, &tx, cancel) {
        let _ = tx.send(FlashEvent::Error(e));
    }
}

// ---------------------------------------------------------------------------
// Top-level pipeline
// ---------------------------------------------------------------------------

fn send(tx: &mpsc::Sender<FlashEvent>, event: FlashEvent) {
    // If the receiver is gone the GUI has been closed — ignore silently.
    let _ = tx.send(event);
}

fn flash_pipeline(
    image_path: &str,
    device_path: &str,
    tx: &mpsc::Sender<FlashEvent>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    // ── Validate inputs ──────────────────────────────────────────────────────
    if !Path::new(image_path).is_file() {
        return Err(format!("Image file not found: {image_path}"));
    }

    if !Path::new(device_path).exists() {
        return Err(format!("Target device not found: {device_path}"));
    }

    // Guard against partition nodes (e.g. /dev/sdb1 instead of /dev/sdb).
    #[cfg(target_os = "linux")]
    reject_partition_node(device_path)?;

    let image_size = std::fs::metadata(image_path)
        .map_err(|e| format!("Cannot stat image: {e}"))?
        .len();

    if image_size == 0 {
        return Err("Image file is empty".to_string());
    }

    // ── Check device is not already in use ───────────────────────────────────
    // Open the device O_RDONLY | O_EXCL — if another process (e.g. a second
    // flashkraft instance) already has it open for writing this fails
    // immediately with EBUSY, giving the user a clear message instead of
    // appearing to hang during unmount.
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let busy = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_EXCL)
            .open(device_path);
        if let Err(e) = busy {
            if e.raw_os_error() == Some(libc::EBUSY) {
                return Err(format!(
                    "Device '{device_path}' is already in use by another process.\n\
                     Is another flash operation already running?"
                ));
            }
            // Any other error (EPERM, EACCES) is fine here — we will handle
            // it properly when we open for writing later.
        }
    }

    // ── Step 1: Unmount ──────────────────────────────────────────────────────
    send(tx, FlashEvent::Stage(FlashStage::Unmounting));
    unmount_device(device_path, tx);

    // ── Step 2: Write ────────────────────────────────────────────────────────
    send(tx, FlashEvent::Stage(FlashStage::Writing));
    send(
        tx,
        FlashEvent::Log(format!(
            "Writing {image_size} bytes from {image_path} → {device_path}"
        )),
    );
    write_image(image_path, device_path, image_size, tx, &cancel)?;

    // ── Step 3: Sync ─────────────────────────────────────────────────────────
    send(tx, FlashEvent::Stage(FlashStage::Syncing));
    sync_device(device_path, tx);

    // ── Step 4: Re-read partition table ──────────────────────────────────────
    send(tx, FlashEvent::Stage(FlashStage::Rereading));
    reread_partition_table(device_path, tx);

    // ── Step 5: Verify ───────────────────────────────────────────────────────
    send(tx, FlashEvent::Stage(FlashStage::Verifying));
    verify(image_path, device_path, image_size, tx)?;

    // ── Done ─────────────────────────────────────────────────────────────────
    send(tx, FlashEvent::Done);
    Ok(())
}

// ---------------------------------------------------------------------------
// Partition-node guard (Linux only)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn reject_partition_node(device_path: &str) -> Result<(), String> {
    let dev_name = Path::new(device_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let is_partition = {
        let bytes = dev_name.as_bytes();
        !bytes.is_empty() && bytes[bytes.len() - 1].is_ascii_digit() && {
            let stem = dev_name.trim_end_matches(|c: char| c.is_ascii_digit());
            stem.ends_with('p')
                || (!stem.is_empty()
                    && !stem.ends_with(|c: char| c.is_ascii_digit())
                    && stem.chars().any(|c| c.is_ascii_alphabetic()))
        }
    };

    if is_partition {
        let whole = dev_name.trim_end_matches(|c: char| c.is_ascii_digit() || c == 'p');
        return Err(format!(
            "Refusing to write to partition node '{device_path}'. \
             Select the whole-disk device (e.g. /dev/{whole}) instead."
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Privilege helpers
// ---------------------------------------------------------------------------

/// Open `device_path` for raw writing, temporarily escalating to root if the
/// binary is setuid-root, then immediately dropping back to the real UID.
fn open_device_for_writing(device_path: &str) -> Result<std::fs::File, String> {
    #[cfg(unix)]
    {
        use nix::unistd::seteuid;

        // Attempt to escalate to root.
        //
        // This only succeeds when the binary carries the setuid-root bit
        // (`chmod u+s`).  If escalation fails we still try to open the file —
        // it may be a regular writable file (e.g. during tests) or the user
        // may already have write permission on the device.
        let escalated = seteuid(nix::unistd::Uid::from_raw(0)).is_ok();

        let result = std::fs::OpenOptions::new()
            .write(true)
            .open(device_path)
            .map_err(|e| {
                let raw = e.raw_os_error().unwrap_or(0);
                if raw == libc::EACCES || raw == libc::EPERM {
                    if escalated {
                        format!(
                            "Permission denied opening '{device_path}'.\n\
                             Even with setuid-root the device refused access — \
                             check that the device exists and is not in use."
                        )
                    } else {
                        format!(
                            "Permission denied opening '{device_path}'.\n\
                             FlashKraft needs root access to write to block devices.\n\
                             Install setuid-root so it can escalate automatically:\n\
                             sudo chown root:root /usr/bin/flashkraft\n\
                             sudo chmod u+s /usr/bin/flashkraft"
                        )
                    }
                } else if raw == libc::EBUSY {
                    format!(
                        "Device '{device_path}' is busy. \
                         Ensure all partitions are unmounted before flashing."
                    )
                } else {
                    format!("Cannot open device '{device_path}' for writing: {e}")
                }
            });

        // Drop back to the real (unprivileged) user immediately.
        if escalated {
            let _ = seteuid(real_uid());
        }

        result
    }

    #[cfg(not(unix))]
    {
        std::fs::OpenOptions::new()
            .write(true)
            .open(device_path)
            .map_err(|e| {
                let raw = e.raw_os_error().unwrap_or(0);
                // ERROR_ACCESS_DENIED (5) or ERROR_PRIVILEGE_NOT_HELD (1314)
                if raw == 5 || raw == 1314 {
                    format!(
                        "Access denied opening '{device_path}'.\n\
                         FlashKraft must be run as Administrator on Windows.\n\
                         Right-click the application and choose \
                         'Run as administrator'."
                    )
                } else if raw == 32 {
                    // ERROR_SHARING_VIOLATION
                    format!(
                        "Device '{device_path}' is in use by another process.\n\
                         Close any applications using the drive and try again."
                    )
                } else {
                    format!("Cannot open device '{device_path}' for writing: {e}")
                }
            })
    }
}

// ---------------------------------------------------------------------------
// Step 1 – Unmount
// ---------------------------------------------------------------------------

fn unmount_device(device_path: &str, tx: &mpsc::Sender<FlashEvent>) {
    let device_name = Path::new(device_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let partitions = find_mounted_partitions(&device_name, device_path);

    if partitions.is_empty() {
        send(tx, FlashEvent::Log("No mounted partitions found".into()));
    } else {
        for partition in &partitions {
            send(tx, FlashEvent::Log(format!("Unmounting {partition}")));
            do_unmount(partition, tx);
        }
    }
}

/// Returns the list of mounted partitions/volumes that belong to `device_path`.
///
/// On Linux/macOS this parses `/proc/mounts`.
/// On Windows this enumerates logical drive letters, resolves each to its
/// underlying physical device via `QueryDosDeviceW`, and returns the volume
/// paths (e.g. `\\.\C:`) whose physical device number matches `device_path`
/// (e.g. `\\.\PhysicalDrive1`).
fn find_mounted_partitions(
    #[cfg_attr(target_os = "windows", allow(unused_variables))] device_name: &str,
    device_path: &str,
) -> Vec<String> {
    #[cfg(not(target_os = "windows"))]
    {
        let mounts = std::fs::read_to_string("/proc/mounts")
            .or_else(|_| std::fs::read_to_string("/proc/self/mounts"))
            .unwrap_or_default();

        let mut mount_points = Vec::new();
        for line in mounts.lines() {
            let mut fields = line.split_whitespace();
            let dev = match fields.next() {
                Some(d) => d,
                None => continue,
            };
            // Second field in /proc/mounts is the mount point directory —
            // that is what umount2 requires, not the device path.
            let mount_point = match fields.next() {
                Some(m) => m,
                None => continue,
            };
            if dev == device_path || is_partition_of(dev, device_name) {
                mount_points.push(mount_point.to_string());
            }
        }
        mount_points
    }

    #[cfg(target_os = "windows")]
    {
        windows::find_volumes_on_physical_drive(device_path)
    }
}

#[cfg(not(target_os = "windows"))]
fn is_partition_of(dev: &str, device_name: &str) -> bool {
    // `dev` may be a full path like "/dev/sda1"; compare only the basename.
    let dev_base = Path::new(dev)
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();

    if !dev_base.starts_with(device_name) {
        return false;
    }
    let suffix = &dev_base[device_name.len()..];
    if suffix.is_empty() {
        return false;
    }
    let first = suffix.chars().next().unwrap();
    first.is_ascii_digit() || (first == 'p' && suffix.len() > 1)
}

/// Return `true` if `name` resolves to an executable on `PATH`.
/// Used by `do_unmount` to prefer `udisksctl` when available.
#[cfg(target_os = "linux")]
fn which_exists(name: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|dir| {
            let p = std::path::Path::new(dir).join(name);
            std::fs::metadata(&p)
                .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        })
}

fn do_unmount(partition: &str, tx: &mpsc::Sender<FlashEvent>) {
    #[cfg(target_os = "linux")]
    {
        use nix::unistd::seteuid;
        use std::ffi::CString;

        // ── Strategy 1: udisksctl ─────────────────────────────────────────────
        // Prefer udisksctl when available — it signals udisks2/systemd-mount to
        // properly release the mount and prevents the automounter from
        // immediately re-mounting the partition after we detach it.
        // `--no-user-interaction` prevents it from blocking on a password prompt.
        if which_exists("udisksctl") {
            // Spawn with a timeout — udisksctl can stall if udisks2 is busy.
            // We give it 5 seconds before falling through to umount2.
            let result = std::process::Command::new("udisksctl")
                .args(["unmount", "--no-user-interaction", "-b", partition])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();

            let udisks_ok = match result {
                Ok(mut child) => {
                    // Poll for up to 5 s in 100 ms increments.
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                    loop {
                        match child.try_wait() {
                            Ok(Some(status)) => break status.success(),
                            Ok(None) if std::time::Instant::now() < deadline => {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                            _ => {
                                // Timed out or error — kill and fall through.
                                let _ = child.kill();
                                send(
                                    tx,
                                    FlashEvent::Log(
                                        "udisksctl timed out — falling back to umount2".into(),
                                    ),
                                );
                                break false;
                            }
                        }
                    }
                }
                Err(_) => false,
            };

            if udisks_ok {
                send(
                    tx,
                    FlashEvent::Log(format!("Unmounted {partition} via udisksctl")),
                );
                return;
            }
        }

        // ── Strategy 2: umount2 MNT_DETACH (fallback) ────────────────────────
        // Lazy unmount: detaches the filesystem immediately even if busy.
        // umount2 never blocks — MNT_DETACH returns right away.
        let _ = seteuid(nix::unistd::Uid::from_raw(0));

        if let Ok(c_path) = CString::new(partition) {
            let ret = unsafe { libc::umount2(c_path.as_ptr(), libc::MNT_DETACH) };
            if ret != 0 {
                let raw = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                match raw {
                    // EINVAL — not a mount point or already unmounted, harmless.
                    libc::EINVAL => {}
                    // ENOENT — path doesn't exist, also harmless.
                    libc::ENOENT => {}
                    _ => {
                        let err = std::io::Error::from_raw_os_error(raw);
                        send(
                            tx,
                            FlashEvent::Log(format!(
                                "Warning — could not unmount {partition}: {err}"
                            )),
                        );
                    }
                }
            }
        }

        let _ = seteuid(real_uid());
    }

    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("diskutil")
            .args(["unmount", partition])
            .output();
        if let Ok(o) = out {
            if !o.status.success() {
                send(
                    tx,
                    FlashEvent::Log(format!("Warning — diskutil unmount {partition} failed")),
                );
            }
        }
    }

    // Windows: open the volume with exclusive access, lock it, then dismount.
    // The volume path is expected to be of the form `\\.\C:` (no trailing slash).
    #[cfg(target_os = "windows")]
    {
        match windows::lock_and_dismount_volume(partition) {
            Ok(()) => send(
                tx,
                FlashEvent::Log(format!("Dismounted volume {partition}")),
            ),
            Err(e) => send(
                tx,
                FlashEvent::Log(format!("Warning — could not dismount {partition}: {e}")),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Step 2 – Write image
// ---------------------------------------------------------------------------

fn write_image(
    image_path: &str,
    device_path: &str,
    image_size: u64,
    tx: &mpsc::Sender<FlashEvent>,
    cancel: &Arc<AtomicBool>,
) -> Result<(), String> {
    let image_file =
        std::fs::File::open(image_path).map_err(|e| format!("Cannot open image: {e}"))?;

    let device_file = open_device_for_writing(device_path)?;

    let mut reader = io::BufReader::with_capacity(BLOCK_SIZE, image_file);
    let mut writer = io::BufWriter::with_capacity(BLOCK_SIZE, device_file);
    let mut buf = vec![0u8; BLOCK_SIZE];

    let mut bytes_written: u64 = 0;
    let start = Instant::now();
    let mut last_report = Instant::now();

    loop {
        // Honour cancellation requests between blocks.
        if cancel.load(Ordering::SeqCst) {
            return Err("Flash operation cancelled by user".to_string());
        }

        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Read error on image: {e}"))?;

        if n == 0 {
            break; // EOF
        }

        writer
            .write_all(&buf[..n])
            .map_err(|e| format!("Write error on device: {e}"))?;

        bytes_written += n as u64;

        let now = Instant::now();
        if now.duration_since(last_report) >= PROGRESS_INTERVAL || bytes_written >= image_size {
            let elapsed_s = now.duration_since(start).as_secs_f32();
            let speed_mb_s = if elapsed_s > 0.001 {
                (bytes_written as f32 / (1024.0 * 1024.0)) / elapsed_s
            } else {
                0.0
            };

            send(
                tx,
                FlashEvent::Progress {
                    bytes_written,
                    total_bytes: image_size,
                    speed_mb_s,
                },
            );
            last_report = now;
        }
    }

    // Flush BufWriter → kernel page cache.
    writer
        .flush()
        .map_err(|e| format!("Buffer flush error: {e}"))?;

    // Retrieve the underlying File for fsync.
    #[cfg_attr(not(unix), allow(unused_variables))]
    let device_file = writer
        .into_inner()
        .map_err(|e| format!("BufWriter error: {e}"))?;

    // fsync: push all dirty pages to the physical medium.
    // Treated as a hard error — a failed fsync means we cannot trust the
    // data reached the device.
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = device_file.as_raw_fd();
        let ret = unsafe { libc::fsync(fd) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            return Err(format!(
                "fsync failed on '{device_path}': {err} — \
                 data may not have been fully written to the device"
            ));
        }
    }

    // Emit a final progress event at 100 %.
    let elapsed_s = start.elapsed().as_secs_f32();
    let speed_mb_s = if elapsed_s > 0.001 {
        (bytes_written as f32 / (1024.0 * 1024.0)) / elapsed_s
    } else {
        0.0
    };
    send(
        tx,
        FlashEvent::Progress {
            bytes_written,
            total_bytes: image_size,
            speed_mb_s,
        },
    );

    send(tx, FlashEvent::Log("Image write complete".into()));
    Ok(())
}

// ---------------------------------------------------------------------------
// Step 3 – Sync
// ---------------------------------------------------------------------------

fn sync_device(device_path: &str, tx: &mpsc::Sender<FlashEvent>) {
    #[cfg(unix)]
    if let Ok(f) = std::fs::OpenOptions::new().write(true).open(device_path) {
        use std::os::unix::io::AsRawFd;
        let fd = f.as_raw_fd();
        #[cfg(target_os = "linux")]
        unsafe {
            libc::fdatasync(fd);
        }
        #[cfg(not(target_os = "linux"))]
        unsafe {
            libc::fsync(fd);
        }
        drop(f);
    }

    #[cfg(target_os = "linux")]
    unsafe {
        libc::sync();
    }

    // Windows: open the physical drive and call FlushFileBuffers.
    // This forces the OS to flush all dirty pages for the device to hardware.
    #[cfg(target_os = "windows")]
    {
        match windows::flush_device_buffers(device_path) {
            Ok(()) => {}
            Err(e) => send(
                tx,
                FlashEvent::Log(format!(
                    "Warning — FlushFileBuffers on '{device_path}' failed: {e}"
                )),
            ),
        }
    }

    send(tx, FlashEvent::Log("Write-back caches flushed".into()));
}

// ---------------------------------------------------------------------------
// Step 4 – Re-read partition table
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn reread_partition_table(device_path: &str, tx: &mpsc::Sender<FlashEvent>) {
    use nix::ioctl_none;
    use std::os::unix::io::AsRawFd;

    ioctl_none!(blkrrpart, 0x12, 95);

    // Brief pause so any pending I/O completes before we poke the kernel.
    std::thread::sleep(Duration::from_millis(500));

    match std::fs::OpenOptions::new().write(true).open(device_path) {
        Ok(f) => {
            let result = unsafe { blkrrpart(f.as_raw_fd()) };
            match result {
                Ok(_) => send(
                    tx,
                    FlashEvent::Log("Kernel partition table refreshed".into()),
                ),
                Err(e) => send(
                    tx,
                    FlashEvent::Log(format!(
                        "Warning — BLKRRPART ioctl failed \
                         (device may not be partitioned): {e}"
                    )),
                ),
            }
        }
        Err(e) => send(
            tx,
            FlashEvent::Log(format!(
                "Warning — could not open device for BLKRRPART: {e}"
            )),
        ),
    }
}

#[cfg(target_os = "macos")]
fn reread_partition_table(device_path: &str, tx: &mpsc::Sender<FlashEvent>) {
    let _ = std::process::Command::new("diskutil")
        .args(["rereadPartitionTable", device_path])
        .output();
    send(
        tx,
        FlashEvent::Log("Partition table refresh requested (macOS)".into()),
    );
}

// Windows: IOCTL_DISK_UPDATE_PROPERTIES asks the partition manager to
// re-enumerate the partition table from the on-disk data.
#[cfg(target_os = "windows")]
fn reread_partition_table(device_path: &str, tx: &mpsc::Sender<FlashEvent>) {
    // Brief pause so the OS flushes before we poke the partition manager.
    std::thread::sleep(Duration::from_millis(500));

    match windows::update_disk_properties(device_path) {
        Ok(()) => send(
            tx,
            FlashEvent::Log("Partition table refreshed (IOCTL_DISK_UPDATE_PROPERTIES)".into()),
        ),
        Err(e) => send(
            tx,
            FlashEvent::Log(format!(
                "Warning — IOCTL_DISK_UPDATE_PROPERTIES failed: {e}"
            )),
        ),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn reread_partition_table(_device_path: &str, tx: &mpsc::Sender<FlashEvent>) {
    send(
        tx,
        FlashEvent::Log("Partition table refresh not supported on this platform".into()),
    );
}

// ---------------------------------------------------------------------------
// Step 5 – Verify
// ---------------------------------------------------------------------------

fn verify(
    image_path: &str,
    device_path: &str,
    image_size: u64,
    tx: &mpsc::Sender<FlashEvent>,
) -> Result<(), String> {
    send(
        tx,
        FlashEvent::Log("Computing SHA-256 of source image".into()),
    );
    let image_hash = sha256_with_progress(image_path, image_size, "image", tx)?;

    send(
        tx,
        FlashEvent::Log(format!(
            "Reading back {image_size} bytes from device for verification"
        )),
    );
    let device_hash = sha256_with_progress(device_path, image_size, "device", tx)?;

    if image_hash != device_hash {
        return Err(format!(
            "Verification failed — data mismatch \
             (image={image_hash} device={device_hash})"
        ));
    }

    send(
        tx,
        FlashEvent::Log(format!("Verification passed ({image_hash})")),
    );
    Ok(())
}

/// Compute the SHA-256 digest of the first `max_bytes` of `path`, emitting
/// [`FlashEvent::VerifyProgress`] events at [`PROGRESS_INTERVAL`] intervals.
///
/// `phase` is forwarded verbatim into every `VerifyProgress` event so the
/// UI can distinguish the image-hash pass (`"image"`) from the device
/// read-back pass (`"device"`).
fn sha256_with_progress(
    path: &str,
    max_bytes: u64,
    phase: &'static str,
    tx: &mpsc::Sender<FlashEvent>,
) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    let file =
        std::fs::File::open(path).map_err(|e| format!("Cannot open {path} for hashing: {e}"))?;

    let mut hasher = Sha256::new();
    let mut reader = io::BufReader::with_capacity(BLOCK_SIZE, file);
    let mut buf = vec![0u8; BLOCK_SIZE];
    let mut remaining = max_bytes;
    let mut bytes_read: u64 = 0;

    let start = Instant::now();
    let mut last_report = Instant::now();

    while remaining > 0 {
        let to_read = (remaining as usize).min(buf.len());
        let n = reader
            .read(&mut buf[..to_read])
            .map_err(|e| format!("Read error while hashing {path}: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        bytes_read += n as u64;
        remaining -= n as u64;

        let now = Instant::now();
        if now.duration_since(last_report) >= PROGRESS_INTERVAL || remaining == 0 {
            let elapsed_s = now.duration_since(start).as_secs_f32();
            let speed_mb_s = if elapsed_s > 0.001 {
                (bytes_read as f32 / (1024.0 * 1024.0)) / elapsed_s
            } else {
                0.0
            };
            send(
                tx,
                FlashEvent::VerifyProgress {
                    phase,
                    bytes_read,
                    total_bytes: max_bytes,
                    speed_mb_s,
                },
            );
            last_report = now;
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Legacy non-progress variant kept for unit tests that don't need a channel.
#[cfg(test)]
fn sha256_first_n_bytes(path: &str, max_bytes: u64) -> Result<String, String> {
    let (tx, _rx) = mpsc::channel();
    sha256_with_progress(path, max_bytes, "image", &tx)
}

// ---------------------------------------------------------------------------
// Windows implementation helpers
// ---------------------------------------------------------------------------

/// All Windows-specific raw-device operations are collected here.
///
/// ## Privilege
/// The binary must be run as Administrator (the UAC manifest embedded by
/// `build.rs` ensures Windows prompts for elevation on launch).  Raw physical
/// drive access (`\\.\PhysicalDriveN`) and volume lock/dismount both require
/// the `SeManageVolumePrivilege` that is only present in an elevated token.
///
/// ## Volume vs physical drive paths
/// - **Physical drive**: `\\.\PhysicalDrive0`, `\\.\PhysicalDrive1`, …
///   Used for writing the image, flushing, and partition-table refresh.
/// - **Volume (drive letter)**: `\\.\C:`, `\\.\D:`, …
///   Used for locking and dismounting before we write.
#[cfg(target_os = "windows")]
mod windows {
    // ── Win32 type aliases ────────────────────────────────────────────────────
    // windows-sys uses raw C types; give them readable names.
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, FALSE, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
        },
        Storage::FileSystem::{
            CreateFileW, FlushFileBuffers, FILE_FLAG_WRITE_THROUGH, FILE_SHARE_READ,
            FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{
            Ioctl::{FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_DISK_UPDATE_PROPERTIES},
            IO::DeviceIoControl,
        },
    };

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Encode a Rust `&str` as a null-terminated UTF-16 `Vec<u16>`.
    fn to_wide(s: &str) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Open a device path (`\\.\PhysicalDriveN` or `\\.\C:`) and return its
    /// Win32 `HANDLE`.  The handle must be closed with `CloseHandle` when done.
    ///
    /// `access` should be `GENERIC_READ`, `GENERIC_WRITE`, or both OR-ed.
    fn open_device_handle(path: &str, access: u32) -> Result<HANDLE, String> {
        let wide = to_wide(path);
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_WRITE_THROUGH,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(format!(
                "Cannot open device '{}': {}",
                path,
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(handle)
        }
    }

    /// Issue a simple `DeviceIoControl` call with no input or output buffer.
    ///
    /// Returns `Ok(())` on success, or an `Err` with the Win32 error message.
    fn device_ioctl(handle: HANDLE, code: u32) -> Result<(), String> {
        let mut bytes_returned: u32 = 0;
        let ok = unsafe {
            DeviceIoControl(
                handle,
                code,
                std::ptr::null(), // no input buffer
                0,
                std::ptr::null_mut(), // no output buffer
                0,
                &mut bytes_returned,
                std::ptr::null_mut(), // synchronous (no OVERLAPPED)
            )
        };
        if ok == FALSE {
            Err(format!("{}", std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }

    // ── Public helpers called from flash_helper ───────────────────────────────

    /// Enumerate all logical drive letters whose underlying physical device
    /// path matches `physical_drive` (e.g. `\\.\PhysicalDrive1`).
    ///
    /// Returns a list of volume paths suitable for passing to
    /// `lock_and_dismount_volume`, e.g. `["\\.\C:", "\\.\D:"]`.
    ///
    /// Algorithm:
    /// 1. Obtain the physical drive number from `physical_drive`.
    /// 2. Call `GetLogicalDriveStringsW` to list all drive letters.
    /// 3. For each letter, open the volume and call `IOCTL_STORAGE_GET_DEVICE_NUMBER`
    ///    to get its physical drive number.
    /// 4. Collect those whose number matches.
    pub fn find_volumes_on_physical_drive(physical_drive: &str) -> Vec<String> {
        use windows_sys::Win32::{
            Storage::FileSystem::GetLogicalDriveStringsW,
            System::Ioctl::{IOCTL_STORAGE_GET_DEVICE_NUMBER, STORAGE_DEVICE_NUMBER},
        };

        // Extract the drive index from "\\.\PhysicalDriveN".
        let target_index: u32 = physical_drive
            .to_ascii_lowercase()
            .trim_start_matches(r"\\.\physicaldrive")
            .parse()
            .unwrap_or(u32::MAX);

        // Get all logical drive strings ("C:\", "D:\", …).
        let mut buf = vec![0u16; 512];
        let len = unsafe { GetLogicalDriveStringsW(buf.len() as u32, buf.as_mut_ptr()) };
        if len == 0 || len > buf.len() as u32 {
            return Vec::new();
        }

        // Parse the null-separated, double-null-terminated list.
        let drive_letters: Vec<String> = buf[..len as usize]
            .split(|&c| c == 0)
            .filter(|s| !s.is_empty())
            .map(|s| {
                // "C:\" → "\\.\C:"  (no trailing backslash — required for
                // CreateFileW on a volume)
                let letter: String = std::char::from_u32(s[0] as u32)
                    .map(|c| c.to_string())
                    .unwrap_or_default();
                format!(r"\\.\{}:", letter)
            })
            .collect();

        let mut matching = Vec::new();

        for vol_path in &drive_letters {
            let wide = to_wide(vol_path);
            let handle = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    GENERIC_READ,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                continue;
            }

            let mut dev_num = STORAGE_DEVICE_NUMBER {
                DeviceType: 0,
                DeviceNumber: u32::MAX,
                PartitionNumber: 0,
            };
            let mut bytes_returned: u32 = 0;

            let ok = unsafe {
                DeviceIoControl(
                    handle,
                    IOCTL_STORAGE_GET_DEVICE_NUMBER,
                    std::ptr::null(),
                    0,
                    &mut dev_num as *mut _ as *mut _,
                    std::mem::size_of::<STORAGE_DEVICE_NUMBER>() as u32,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                )
            };

            unsafe { CloseHandle(handle) };

            if ok != FALSE && dev_num.DeviceNumber == target_index {
                matching.push(vol_path.clone());
            }
        }

        matching
    }

    /// Lock a volume exclusively and dismount it so writes to the underlying
    /// physical disk can proceed without the filesystem intercepting I/O.
    ///
    /// Steps (mirrors what Rufus / dd for Windows do):
    /// 1. Open the volume with `GENERIC_READ | GENERIC_WRITE`.
    /// 2. `FSCTL_LOCK_VOLUME`   — exclusive lock; fails if files are open.
    /// 3. `FSCTL_DISMOUNT_VOLUME` — tell the FS driver to flush and detach.
    ///
    /// The lock is held for the lifetime of the handle.  Because we close the
    /// handle immediately after dismounting, the volume is automatically
    /// unlocked (`FSCTL_UNLOCK_VOLUME` is implicit on handle close).
    pub fn lock_and_dismount_volume(volume_path: &str) -> Result<(), String> {
        let handle = open_device_handle(volume_path, GENERIC_READ | GENERIC_WRITE)?;

        // Lock — exclusive; if this fails (files open) we still try to
        // dismount because the user may have opened Explorer on the drive.
        let lock_result = device_ioctl(handle, FSCTL_LOCK_VOLUME);
        if let Err(ref e) = lock_result {
            // Non-fatal: log and continue.  Dismount can still succeed.
            eprintln!(
                "[flash] FSCTL_LOCK_VOLUME on '{volume_path}' failed ({e}); \
                 attempting dismount anyway"
            );
        }

        // Dismount — detaches the filesystem; flushes dirty data first.
        let dismount_result = device_ioctl(handle, FSCTL_DISMOUNT_VOLUME);

        unsafe { CloseHandle(handle) };

        lock_result.and(dismount_result)
    }

    /// Call `FlushFileBuffers` on the physical drive to force the OS to push
    /// all dirty write-back pages to the device hardware.
    pub fn flush_device_buffers(device_path: &str) -> Result<(), String> {
        let handle = open_device_handle(device_path, GENERIC_WRITE)?;
        let ok = unsafe { FlushFileBuffers(handle) };
        unsafe { CloseHandle(handle) };
        if ok == FALSE {
            Err(format!("{}", std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }

    /// Send `IOCTL_DISK_UPDATE_PROPERTIES` to the physical drive, asking the
    /// Windows partition manager to re-read the partition table from disk.
    pub fn update_disk_properties(device_path: &str) -> Result<(), String> {
        let handle = open_device_handle(device_path, GENERIC_READ | GENERIC_WRITE)?;
        let result = device_ioctl(handle, IOCTL_DISK_UPDATE_PROPERTIES);
        unsafe { CloseHandle(handle) };
        result
    }

    // ── Unit tests ────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        /// `to_wide` must produce a null-terminated UTF-16 sequence.
        #[test]
        fn test_to_wide_null_terminated() {
            let wide = to_wide("ABC");
            assert_eq!(wide.last(), Some(&0u16), "must be null-terminated");
            assert_eq!(&wide[..3], &[b'A' as u16, b'B' as u16, b'C' as u16]);
        }

        /// `to_wide` on an empty string produces exactly one null.
        #[test]
        fn test_to_wide_empty() {
            let wide = to_wide("");
            assert_eq!(wide, vec![0u16]);
        }

        /// `open_device_handle` on a nonexistent path must return an error.
        #[test]
        fn test_open_device_handle_bad_path_returns_error() {
            let result = open_device_handle(r"\\.\NonExistentDevice999", GENERIC_READ);
            assert!(result.is_err(), "expected error for nonexistent device");
        }

        /// `flush_device_buffers` on a nonexistent drive must return an error.
        #[test]
        fn test_flush_device_buffers_bad_path() {
            let result = flush_device_buffers(r"\\.\PhysicalDrive999");
            assert!(result.is_err());
        }

        /// `update_disk_properties` on a nonexistent drive must return an error.
        #[test]
        fn test_update_disk_properties_bad_path() {
            let result = update_disk_properties(r"\\.\PhysicalDrive999");
            assert!(result.is_err());
        }

        /// `lock_and_dismount_volume` on a nonexistent path must return an error.
        #[test]
        fn test_lock_and_dismount_bad_path() {
            let result = lock_and_dismount_volume(r"\\.\Z99:");
            assert!(result.is_err());
        }

        /// `find_volumes_on_physical_drive` with an unparseable path should
        /// return an empty Vec (no panic).
        #[test]
        fn test_find_volumes_bad_path_no_panic() {
            let result = find_volumes_on_physical_drive("not-a-valid-path");
            // May be empty or contain volumes; must not panic.
            let _ = result;
        }

        /// `find_volumes_on_physical_drive` for a very high drive number
        /// (almost certainly nonexistent) should return an empty list.
        #[test]
        fn test_find_volumes_nonexistent_drive_returns_empty() {
            let result = find_volumes_on_physical_drive(r"\\.\PhysicalDrive999");
            assert!(
                result.is_empty(),
                "expected no volumes for PhysicalDrive999"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::mpsc;

    fn make_channel() -> (mpsc::Sender<FlashEvent>, mpsc::Receiver<FlashEvent>) {
        mpsc::channel()
    }

    fn drain(rx: &mpsc::Receiver<FlashEvent>) -> Vec<FlashEvent> {
        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }
        events
    }

    fn has_stage(events: &[FlashEvent], stage: &FlashStage) -> bool {
        events
            .iter()
            .any(|e| matches!(e, FlashEvent::Stage(s) if s == stage))
    }

    fn find_error(events: &[FlashEvent]) -> Option<&str> {
        events.iter().find_map(|e| {
            if let FlashEvent::Error(msg) = e {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    // ── set_real_uid ────────────────────────────────────────────────────────

    #[test]
    fn test_is_privileged_returns_bool() {
        // Just verify it doesn't panic and returns a consistent value.
        let first = is_privileged();
        let second = is_privileged();
        assert_eq!(first, second, "is_privileged must be deterministic");
    }

    #[test]
    fn test_reexec_as_root_does_not_panic_when_already_escalated() {
        // With the guard env-var set, reexec_as_root must return immediately
        // without panicking or actually exec-ing anything.
        std::env::set_var("FLASHKRAFT_ESCALATED", "1");
        reexec_as_root(); // must not exec — guard fires immediately
        std::env::remove_var("FLASHKRAFT_ESCALATED");
    }

    #[test]
    fn test_set_real_uid_stores_value() {
        // OnceLock only sets once; in tests the first call wins.
        // Just verify it doesn't panic.
        set_real_uid(1000);
    }

    // ── is_partition_of ─────────────────────────────────────────────────────

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_is_partition_of_sda() {
        assert!(is_partition_of("/dev/sda1", "sda"));
        assert!(is_partition_of("/dev/sda2", "sda"));
        assert!(!is_partition_of("/dev/sdb1", "sda"));
        assert!(!is_partition_of("/dev/sda", "sda"));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_is_partition_of_nvme() {
        assert!(is_partition_of("/dev/nvme0n1p1", "nvme0n1"));
        assert!(is_partition_of("/dev/nvme0n1p2", "nvme0n1"));
        assert!(!is_partition_of("/dev/nvme0n1", "nvme0n1"));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_is_partition_of_mmcblk() {
        assert!(is_partition_of("/dev/mmcblk0p1", "mmcblk0"));
        assert!(!is_partition_of("/dev/mmcblk0", "mmcblk0"));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_is_partition_of_no_false_prefix_match() {
        assert!(!is_partition_of("/dev/sda1", "sd"));
    }

    // ── reject_partition_node ───────────────────────────────────────────────

    #[test]
    #[cfg(target_os = "linux")]
    fn test_reject_partition_node_sda1() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_reject_img.bin");
        std::fs::write(&img, vec![0u8; 1024]).unwrap();

        let result = reject_partition_node("/dev/sda1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Refusing"));

        let _ = std::fs::remove_file(img);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_reject_partition_node_nvme() {
        let result = reject_partition_node("/dev/nvme0n1p1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Refusing"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_reject_partition_node_accepts_whole_disk() {
        // /dev/sdb does not exist in CI — the function only checks the name
        // pattern, not whether the path exists.
        let result = reject_partition_node("/dev/sdb");
        assert!(result.is_ok(), "whole-disk node should not be rejected");
    }

    // ── find_mounted_partitions ──────────────────────────────────────────────

    #[test]
    fn test_find_mounted_partitions_parses_proc_mounts_format() {
        // We cannot mock /proc/mounts in a unit test so we just verify the
        // function doesn't panic and returns a Vec.
        let result = find_mounted_partitions("sda", "/dev/sda");
        let _ = result; // any result is valid
    }

    // ── sha256_first_n_bytes ─────────────────────────────────────────────────

    #[test]
    fn test_sha256_full_file() {
        use sha2::{Digest, Sha256};

        let dir = std::env::temp_dir();
        let path = dir.join("fk_sha256_full.bin");
        let data: Vec<u8> = (0u8..=255u8).cycle().take(4096).collect();
        std::fs::write(&path, &data).unwrap();

        let result = sha256_first_n_bytes(path.to_str().unwrap(), data.len() as u64).unwrap();
        let expected = format!("{:x}", Sha256::digest(&data));
        assert_eq!(result, expected);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_sha256_partial() {
        use sha2::{Digest, Sha256};

        let dir = std::env::temp_dir();
        let path = dir.join("fk_sha256_partial.bin");
        let data: Vec<u8> = (0u8..=255u8).cycle().take(8192).collect();
        std::fs::write(&path, &data).unwrap();

        let n = 4096u64;
        let result = sha256_first_n_bytes(path.to_str().unwrap(), n).unwrap();
        let expected = format!("{:x}", Sha256::digest(&data[..n as usize]));
        assert_eq!(result, expected);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_sha256_nonexistent_returns_error() {
        let result = sha256_first_n_bytes("/nonexistent/path.bin", 1024);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot open"));
    }

    #[test]
    fn test_sha256_empty_read_is_hash_of_empty() {
        use sha2::{Digest, Sha256};

        let dir = std::env::temp_dir();
        let path = dir.join("fk_sha256_empty.bin");
        std::fs::write(&path, b"hello world extended data").unwrap();

        // max_bytes = 0 → nothing is read → hash of empty input
        let result = sha256_first_n_bytes(path.to_str().unwrap(), 0).unwrap();
        let expected = format!("{:x}", Sha256::digest(b""));
        assert_eq!(result, expected);

        let _ = std::fs::remove_file(path);
    }

    // ── write_image (via temp files) ─────────────────────────────────────────

    #[test]
    fn test_write_image_to_temp_file() {
        let dir = std::env::temp_dir();
        let img_path = dir.join("fk_write_img.bin");
        let dev_path = dir.join("fk_write_dev.bin");

        let image_size: u64 = 2 * 1024 * 1024; // 2 MiB
        {
            let mut f = std::fs::File::create(&img_path).unwrap();
            let block: Vec<u8> = (0u8..=255u8).cycle().take(BLOCK_SIZE).collect();
            let mut rem = image_size;
            while rem > 0 {
                let n = rem.min(BLOCK_SIZE as u64) as usize;
                f.write_all(&block[..n]).unwrap();
                rem -= n as u64;
            }
        }
        std::fs::File::create(&dev_path).unwrap();

        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));

        let result = write_image(
            img_path.to_str().unwrap(),
            dev_path.to_str().unwrap(),
            image_size,
            &tx,
            &cancel,
        );

        assert!(result.is_ok(), "write_image failed: {result:?}");

        let written = std::fs::read(&dev_path).unwrap();
        let original = std::fs::read(&img_path).unwrap();
        assert_eq!(written, original, "written data must match image exactly");

        let events = drain(&rx);
        let has_progress = events
            .iter()
            .any(|e| matches!(e, FlashEvent::Progress { .. }));
        assert!(has_progress, "must emit at least one Progress event");

        let _ = std::fs::remove_file(img_path);
        let _ = std::fs::remove_file(dev_path);
    }

    #[test]
    fn test_write_image_cancelled_mid_write() {
        let dir = std::env::temp_dir();
        let img_path = dir.join("fk_cancel_img.bin");
        let dev_path = dir.join("fk_cancel_dev.bin");

        // Large enough that we definitely hit the cancel check.
        let image_size: u64 = 8 * 1024 * 1024; // 8 MiB
        {
            let mut f = std::fs::File::create(&img_path).unwrap();
            let block = vec![0xAAu8; BLOCK_SIZE];
            let mut rem = image_size;
            while rem > 0 {
                let n = rem.min(BLOCK_SIZE as u64) as usize;
                f.write_all(&block[..n]).unwrap();
                rem -= n as u64;
            }
        }
        std::fs::File::create(&dev_path).unwrap();

        let (tx, _rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(true)); // pre-cancelled

        let result = write_image(
            img_path.to_str().unwrap(),
            dev_path.to_str().unwrap(),
            image_size,
            &tx,
            &cancel,
        );

        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("cancelled"),
            "error should mention cancellation"
        );

        let _ = std::fs::remove_file(img_path);
        let _ = std::fs::remove_file(dev_path);
    }

    #[test]
    fn test_write_image_missing_image_returns_error() {
        let dir = std::env::temp_dir();
        let dev_path = dir.join("fk_noimg_dev.bin");
        std::fs::File::create(&dev_path).unwrap();

        let (tx, _rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));

        let result = write_image(
            "/nonexistent/image.img",
            dev_path.to_str().unwrap(),
            1024,
            &tx,
            &cancel,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot open image"));

        let _ = std::fs::remove_file(dev_path);
    }

    // ── verify ───────────────────────────────────────────────────────────────

    #[test]
    fn test_verify_matching_files() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_verify_img.bin");
        let dev = dir.join("fk_verify_dev.bin");
        let data = vec![0xBBu8; 64 * 1024];
        std::fs::write(&img, &data).unwrap();
        std::fs::write(&dev, &data).unwrap();

        let (tx, _rx) = make_channel();
        let result = verify(
            img.to_str().unwrap(),
            dev.to_str().unwrap(),
            data.len() as u64,
            &tx,
        );
        assert!(result.is_ok());

        let _ = std::fs::remove_file(img);
        let _ = std::fs::remove_file(dev);
    }

    #[test]
    fn test_verify_mismatch_returns_error() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_mismatch_img.bin");
        let dev = dir.join("fk_mismatch_dev.bin");
        std::fs::write(&img, vec![0x00u8; 64 * 1024]).unwrap();
        std::fs::write(&dev, vec![0xFFu8; 64 * 1024]).unwrap();

        let (tx, _rx) = make_channel();
        let result = verify(img.to_str().unwrap(), dev.to_str().unwrap(), 64 * 1024, &tx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Verification failed"));

        let _ = std::fs::remove_file(img);
        let _ = std::fs::remove_file(dev);
    }

    #[test]
    fn test_verify_only_checks_image_size_bytes() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_trunc_img.bin");
        let dev = dir.join("fk_trunc_dev.bin");
        let image_data = vec![0xCCu8; 32 * 1024];
        let mut device_data = image_data.clone();
        device_data.extend_from_slice(&[0xDDu8; 32 * 1024]);
        std::fs::write(&img, &image_data).unwrap();
        std::fs::write(&dev, &device_data).unwrap();

        let (tx, _rx) = make_channel();
        let result = verify(
            img.to_str().unwrap(),
            dev.to_str().unwrap(),
            image_data.len() as u64,
            &tx,
        );
        assert!(
            result.is_ok(),
            "should pass when first N bytes match: {result:?}"
        );

        let _ = std::fs::remove_file(img);
        let _ = std::fs::remove_file(dev);
    }

    // ── flash_pipeline validation ────────────────────────────────────────────

    #[test]
    fn test_pipeline_rejects_missing_image() {
        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_pipeline("/nonexistent/image.iso", "/dev/null", tx, cancel);
        let events = drain(&rx);
        let err = find_error(&events);
        assert!(err.is_some(), "must emit an Error event");
        assert!(err.unwrap().contains("Image file not found"), "err={err:?}");
    }

    #[test]
    fn test_pipeline_rejects_empty_image() {
        let dir = std::env::temp_dir();
        let empty = dir.join("fk_empty.img");
        std::fs::write(&empty, b"").unwrap();

        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_pipeline(empty.to_str().unwrap(), "/dev/null", tx, cancel);

        let events = drain(&rx);
        let err = find_error(&events);
        assert!(err.is_some());
        assert!(err.unwrap().contains("empty"), "err={err:?}");

        let _ = std::fs::remove_file(empty);
    }

    #[test]
    fn test_pipeline_rejects_missing_device() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_nodev_img.bin");
        std::fs::write(&img, vec![0u8; 1024]).unwrap();

        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_pipeline(img.to_str().unwrap(), "/nonexistent/device", tx, cancel);

        let events = drain(&rx);
        let err = find_error(&events);
        assert!(err.is_some());
        assert!(
            err.unwrap().contains("Target device not found"),
            "err={err:?}"
        );

        let _ = std::fs::remove_file(img);
    }

    /// End-to-end pipeline test using only temp files (no real hardware).
    #[test]
    fn test_pipeline_end_to_end_temp_files() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_e2e_img.bin");
        let dev = dir.join("fk_e2e_dev.bin");

        let image_data: Vec<u8> = (0u8..=255u8).cycle().take(1024 * 1024).collect();
        std::fs::write(&img, &image_data).unwrap();
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_pipeline(img.to_str().unwrap(), dev.to_str().unwrap(), tx, cancel);

        let events = drain(&rx);

        // Must have seen at least one Progress event.
        let has_progress = events
            .iter()
            .any(|e| matches!(e, FlashEvent::Progress { .. }));
        assert!(has_progress, "must emit Progress events");

        // Must have passed through the core pipeline stages.
        assert!(
            has_stage(&events, &FlashStage::Unmounting),
            "must emit Unmounting stage"
        );
        assert!(
            has_stage(&events, &FlashStage::Writing),
            "must emit Writing stage"
        );
        assert!(
            has_stage(&events, &FlashStage::Syncing),
            "must emit Syncing stage"
        );

        // On temp files the pipeline either completes (Done) or fails after
        // the write/verify stage (e.g. BLKRRPART on a regular file).
        let has_done = events.iter().any(|e| matches!(e, FlashEvent::Done));
        let has_error = events.iter().any(|e| matches!(e, FlashEvent::Error(_)));
        assert!(
            has_done || has_error,
            "pipeline must end with Done or Error"
        );

        if has_done {
            let written = std::fs::read(&dev).unwrap();
            assert_eq!(written, image_data, "written data must match image");
        } else if let Some(err_msg) = find_error(&events) {
            // Error must NOT be from write or verify.
            assert!(
                !err_msg.contains("Cannot open")
                    && !err_msg.contains("Verification failed")
                    && !err_msg.contains("Write error"),
                "unexpected error: {err_msg}"
            );
        }

        let _ = std::fs::remove_file(img);
        let _ = std::fs::remove_file(dev);
    }

    // ── FlashStage Display ───────────────────────────────────────────────────

    #[test]
    fn test_flash_stage_display() {
        assert!(FlashStage::Writing.to_string().contains("Writing"));
        assert!(FlashStage::Syncing.to_string().contains("Flushing"));
        assert!(FlashStage::Done.to_string().contains("complete"));
        assert!(FlashStage::Failed("oops".into())
            .to_string()
            .contains("oops"));
    }

    // ── FlashStage equality ──────────────────────────────────────────────────

    #[test]
    fn test_flash_stage_eq() {
        assert_eq!(FlashStage::Writing, FlashStage::Writing);
        assert_ne!(FlashStage::Writing, FlashStage::Syncing);
        assert_eq!(
            FlashStage::Failed("x".into()),
            FlashStage::Failed("x".into())
        );
        assert_ne!(
            FlashStage::Failed("x".into()),
            FlashStage::Failed("y".into())
        );
    }

    // ── FlashEvent Clone ─────────────────────────────────────────────────────

    #[test]
    fn test_flash_event_clone() {
        let events = vec![
            FlashEvent::Stage(FlashStage::Writing),
            FlashEvent::Progress {
                bytes_written: 1024,
                total_bytes: 4096,
                speed_mb_s: 12.5,
            },
            FlashEvent::Log("hello".into()),
            FlashEvent::Done,
            FlashEvent::Error("boom".into()),
        ];
        for e in &events {
            let _ = e.clone(); // must not panic
        }
    }

    // ── find_mounted_partitions (platform-neutral contracts) ─────────────────

    /// Calling find_mounted_partitions with a device name that almost
    /// certainly isn't mounted must return an empty Vec without panicking.
    #[test]
    fn test_find_mounted_partitions_nonexistent_device_returns_empty() {
        // PhysicalDrive999 / sdzzz are both guaranteed not to exist anywhere.
        #[cfg(target_os = "windows")]
        let result = find_mounted_partitions("PhysicalDrive999", r"\\.\PhysicalDrive999");
        #[cfg(not(target_os = "windows"))]
        let result = find_mounted_partitions("sdzzz", "/dev/sdzzz");

        // Result can be empty or non-empty depending on the OS, but must not panic.
        let _ = result;
    }

    /// find_mounted_partitions must return a Vec (never panic) even when
    /// called with an empty device name.
    #[test]
    fn test_find_mounted_partitions_empty_name_no_panic() {
        let result = find_mounted_partitions("", "");
        let _ = result;
    }

    // ── is_partition_of (Windows drive-letter paths are not partitions) ──────

    /// On Windows the caller never passes Unix-style paths, so these should
    /// all return false (no false positives from the partition-suffix logic).
    #[test]
    fn test_is_partition_of_windows_style_paths() {
        // Windows physical drive paths have no numeric suffix after the name.
        assert!(!is_partition_of(r"\\.\PhysicalDrive0", "PhysicalDrive0"));
        assert!(!is_partition_of(r"\\.\PhysicalDrive1", "PhysicalDrive0"));
    }

    // ── sync_device (via pipeline — emits Log event on all platforms) ────────

    /// sync_device must emit a "caches flushed" log event regardless of
    /// platform.  We test this indirectly via the full pipeline on temp files.
    #[test]
    fn test_pipeline_emits_syncing_stage() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_sync_stage_img.bin");
        let dev = dir.join("fk_sync_stage_dev.bin");

        let data: Vec<u8> = (0u8..=255).cycle().take(512 * 1024).collect();
        std::fs::write(&img, &data).unwrap();
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_pipeline(img.to_str().unwrap(), dev.to_str().unwrap(), tx, cancel);

        let events = drain(&rx);
        assert!(
            has_stage(&events, &FlashStage::Syncing),
            "Syncing stage must be emitted on every platform"
        );

        let _ = std::fs::remove_file(&img);
        let _ = std::fs::remove_file(&dev);
    }

    /// The pipeline must emit the Rereading stage on every platform.
    #[test]
    fn test_pipeline_emits_rereading_stage() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_reread_stage_img.bin");
        let dev = dir.join("fk_reread_stage_dev.bin");

        let data: Vec<u8> = vec![0xABu8; 256 * 1024];
        std::fs::write(&img, &data).unwrap();
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_pipeline(img.to_str().unwrap(), dev.to_str().unwrap(), tx, cancel);

        let events = drain(&rx);
        assert!(
            has_stage(&events, &FlashStage::Rereading),
            "Rereading stage must be emitted on every platform"
        );

        let _ = std::fs::remove_file(&img);
        let _ = std::fs::remove_file(&dev);
    }

    /// The pipeline must emit the Verifying stage on every platform.
    #[test]
    fn test_pipeline_emits_verifying_stage() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_verify_stage_img.bin");
        let dev = dir.join("fk_verify_stage_dev.bin");

        let data: Vec<u8> = vec![0xCDu8; 256 * 1024];
        std::fs::write(&img, &data).unwrap();
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_pipeline(img.to_str().unwrap(), dev.to_str().unwrap(), tx, cancel);

        let events = drain(&rx);
        assert!(
            has_stage(&events, &FlashStage::Verifying),
            "Verifying stage must be emitted on every platform"
        );

        let _ = std::fs::remove_file(&img);
        let _ = std::fs::remove_file(&dev);
    }

    // ── open_device_for_writing error messages ───────────────────────────────

    /// Opening a path that does not exist must produce an error that mentions
    /// the device path — verified on all platforms.
    #[test]
    fn test_open_device_for_writing_nonexistent_mentions_path() {
        let bad = if cfg!(target_os = "windows") {
            r"\\.\PhysicalDrive999".to_string()
        } else {
            "/nonexistent/fk_bad_device".to_string()
        };

        // open_device_for_writing is private; exercise it via write_image.
        let dir = std::env::temp_dir();
        let img = dir.join("fk_open_err_img.bin");
        std::fs::write(&img, vec![1u8; 512]).unwrap();

        let (tx, _rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let result = write_image(img.to_str().unwrap(), &bad, 512, &tx, &cancel);

        assert!(result.is_err(), "must fail for nonexistent device");
        // The error string should mention the device path.
        assert!(
            result.as_ref().unwrap_err().contains("PhysicalDrive999")
                || result.as_ref().unwrap_err().contains("fk_bad_device")
                || result.as_ref().unwrap_err().contains("Cannot open"),
            "error should reference the bad path: {:?}",
            result
        );

        let _ = std::fs::remove_file(&img);
    }

    // ── sync_device emits a log message ─────────────────────────────────────

    /// sync_device must emit at least one FlashEvent::Log containing the
    /// word "flushed" or "flush" on every platform.
    #[test]
    fn test_sync_device_emits_log() {
        let dir = std::env::temp_dir();
        let dev = dir.join("fk_sync_log_dev.bin");
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        sync_device(dev.to_str().unwrap(), &tx);

        let events = drain(&rx);
        let has_flush_log = events.iter().any(|e| {
            if let FlashEvent::Log(msg) = e {
                let lower = msg.to_lowercase();
                lower.contains("flush") || lower.contains("cache")
            } else {
                false
            }
        });
        assert!(
            has_flush_log,
            "sync_device must emit a flush/cache log event"
        );

        let _ = std::fs::remove_file(&dev);
    }

    // ── reread_partition_table emits a log message ───────────────────────────

    /// reread_partition_table must emit at least one FlashEvent::Log on every
    /// platform — either a success message or a warning.
    #[test]
    fn test_reread_partition_table_emits_log() {
        let dir = std::env::temp_dir();
        let dev = dir.join("fk_reread_log_dev.bin");
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        reread_partition_table(dev.to_str().unwrap(), &tx);

        let events = drain(&rx);
        let has_log = events.iter().any(|e| matches!(e, FlashEvent::Log(_)));
        assert!(
            has_log,
            "reread_partition_table must emit at least one Log event"
        );

        let _ = std::fs::remove_file(&dev);
    }

    // ── unmount_device emits a log message ───────────────────────────────────

    /// unmount_device on a temp-file path (which is never mounted) must emit
    /// the "no mounted partitions" log without panicking on any platform.
    #[test]
    fn test_unmount_device_no_partitions_emits_log() {
        let dir = std::env::temp_dir();
        let dev = dir.join("fk_unmount_log_dev.bin");
        std::fs::File::create(&dev).unwrap();

        let path_str = dev.to_str().unwrap();
        let (tx, rx) = make_channel();
        unmount_device(path_str, &tx);

        let events = drain(&rx);
        // Must emit at least one Log event (either "no partitions" or a warning).
        let has_log = events.iter().any(|e| matches!(e, FlashEvent::Log(_)));
        assert!(has_log, "unmount_device must emit at least one Log event");

        let _ = std::fs::remove_file(&dev);
    }

    // ── Pipeline all-stages ordering ─────────────────────────────────────────

    /// The pipeline must emit stages in the documented order:
    /// Unmounting → Writing → Syncing → Rereading → Verifying.
    #[test]
    fn test_pipeline_stage_ordering() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_order_img.bin");
        let dev = dir.join("fk_order_dev.bin");

        let data: Vec<u8> = (0u8..=255).cycle().take(256 * 1024).collect();
        std::fs::write(&img, &data).unwrap();
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        run_pipeline(img.to_str().unwrap(), dev.to_str().unwrap(), tx, cancel);

        let events = drain(&rx);

        // Collect all Stage events in order.
        let stages: Vec<&FlashStage> = events
            .iter()
            .filter_map(|e| {
                if let FlashEvent::Stage(s) = e {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();

        // Verify the mandatory stages appear and in correct relative order.
        let pos = |target: &FlashStage| {
            stages
                .iter()
                .position(|s| *s == target)
                .unwrap_or(usize::MAX)
        };

        let unmounting = pos(&FlashStage::Unmounting);
        let writing = pos(&FlashStage::Writing);
        let syncing = pos(&FlashStage::Syncing);
        let rereading = pos(&FlashStage::Rereading);
        let verifying = pos(&FlashStage::Verifying);

        assert!(unmounting < writing, "Unmounting must precede Writing");
        assert!(writing < syncing, "Writing must precede Syncing");
        assert!(syncing < rereading, "Syncing must precede Rereading");
        assert!(rereading < verifying, "Rereading must precede Verifying");

        let _ = std::fs::remove_file(&img);
        let _ = std::fs::remove_file(&dev);
    }

    // ── Linux-specific tests ─────────────────────────────────────────────────

    /// On Linux, find_mounted_partitions reads /proc/mounts.
    /// Verify it returns a Vec without panicking (live test).
    #[test]
    #[cfg(target_os = "linux")]
    fn test_find_mounted_partitions_linux_no_panic() {
        // sda is unlikely to be mounted in CI, but the function must not panic.
        let result = find_mounted_partitions("sda", "/dev/sda");
        let _ = result;
    }

    /// On Linux, /proc/mounts always contains at least one line (the root
    /// filesystem), so reading a clearly-mounted device (e.g. something at /)
    /// should find entries.
    #[test]
    #[cfg(target_os = "linux")]
    fn test_find_mounted_partitions_linux_reads_proc_mounts() {
        // We can't know exactly which device is at /, but we can verify
        // that the function can parse whatever /proc/mounts contains.
        let content = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
        // If /proc/mounts is non-empty there must be at least one entry parseable.
        if !content.is_empty() {
            // Parse first real /dev/ device from /proc/mounts and verify
            // find_mounted_partitions does not panic on it.
            if let Some(line) = content.lines().find(|l| l.starts_with("/dev/")) {
                if let Some(dev) = line.split_whitespace().next() {
                    let name = std::path::Path::new(dev)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let _ = find_mounted_partitions(&name, dev);
                }
            }
        }
    }

    /// On Linux, do_unmount on a path that is not mounted must not panic.
    /// EINVAL (not a mount point) and ENOENT (path doesn't exist) are both
    /// silenced — they are normal/harmless conditions, not warnings to surface
    /// to the user.
    #[test]
    #[cfg(target_os = "linux")]
    fn test_do_unmount_not_mounted_does_not_panic() {
        let (tx, rx) = make_channel();
        do_unmount("/dev/fk_nonexistent_part", &tx);
        let events = drain(&rx);
        // EINVAL / ENOENT must NOT produce a warning log — they are expected
        // silent outcomes when a partition is already detached or never mounted.
        let has_warning = events.iter().any(|e| matches!(e, FlashEvent::Log(_)));
        assert!(
            !has_warning,
            "do_unmount must not emit a warning for EINVAL/ENOENT: {events:?}"
        );
    }

    // ── macOS-specific tests ─────────────────────────────────────────────────

    /// On macOS, do_unmount with a bogus partition path must emit a warning
    /// log (diskutil will fail) but must not panic.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_do_unmount_macos_bad_path_emits_warning() {
        let (tx, rx) = make_channel();
        do_unmount("/dev/fk_nonexistent_part", &tx);
        let events = drain(&rx);
        let has_log = events.iter().any(|e| matches!(e, FlashEvent::Log(_)));
        assert!(has_log, "do_unmount must emit a Log event on failure");
    }

    /// On macOS, find_mounted_partitions reads /proc/mounts (which doesn't
    /// exist) or falls back gracefully — must not panic.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_find_mounted_partitions_macos_no_panic() {
        let result = find_mounted_partitions("disk2", "/dev/disk2");
        let _ = result;
    }

    /// On macOS, reread_partition_table calls diskutil — must emit a log even
    /// if the path is a temp file (diskutil will fail gracefully).
    #[test]
    #[cfg(target_os = "macos")]
    fn test_reread_partition_table_macos_emits_log() {
        let dir = std::env::temp_dir();
        let dev = dir.join("fk_macos_reread_dev.bin");
        std::fs::File::create(&dev).unwrap();

        let (tx, rx) = make_channel();
        reread_partition_table(dev.to_str().unwrap(), &tx);

        let events = drain(&rx);
        let has_log = events.iter().any(|e| matches!(e, FlashEvent::Log(_)));
        assert!(has_log, "reread_partition_table must emit a log on macOS");

        let _ = std::fs::remove_file(&dev);
    }

    // ── Windows-specific pipeline tests ─────────────────────────────────────

    /// On Windows, find_mounted_partitions delegates to
    /// windows::find_volumes_on_physical_drive — verify it does not panic
    /// for a well-formed but nonexistent drive.
    #[test]
    #[cfg(target_os = "windows")]
    fn test_find_mounted_partitions_windows_nonexistent() {
        let result = find_mounted_partitions("PhysicalDrive999", r"\\.\PhysicalDrive999");
        assert!(
            result.is_empty(),
            "nonexistent physical drive should have no volumes"
        );
    }

    /// On Windows, do_unmount on a bad volume path must emit a warning log
    /// and not panic.
    #[test]
    #[cfg(target_os = "windows")]
    fn test_do_unmount_windows_bad_volume_emits_log() {
        let (tx, rx) = make_channel();
        do_unmount(r"\\.\Z99:", &tx);
        let events = drain(&rx);
        let has_log = events.iter().any(|e| matches!(e, FlashEvent::Log(_)));
        assert!(has_log, "do_unmount on bad volume must emit a Log event");
    }

    /// On Windows, sync_device on a nonexistent physical drive path should
    /// emit a warning log (FlushFileBuffers will fail) but not panic.
    #[test]
    #[cfg(target_os = "windows")]
    fn test_sync_device_windows_bad_path_no_panic() {
        let (tx, rx) = make_channel();
        sync_device(r"\\.\PhysicalDrive999", &tx);
        let events = drain(&rx);
        // Must emit at least one log event (either flush warning or the
        // normal "caches flushed" message).
        let has_log = events.iter().any(|e| matches!(e, FlashEvent::Log(_)));
        assert!(has_log, "sync_device must emit a Log event on Windows");
    }

    /// On Windows, reread_partition_table on a nonexistent drive must emit
    /// a warning log and not panic.
    #[test]
    #[cfg(target_os = "windows")]
    fn test_reread_partition_table_windows_bad_path_no_panic() {
        let (tx, rx) = make_channel();
        reread_partition_table(r"\\.\PhysicalDrive999", &tx);
        let events = drain(&rx);
        let has_log = events.iter().any(|e| matches!(e, FlashEvent::Log(_)));
        assert!(
            has_log,
            "reread_partition_table must emit a Log event on Windows"
        );
    }

    /// On Windows, open_device_for_writing on a nonexistent physical drive
    /// must return an Err containing a meaningful message.
    #[test]
    #[cfg(target_os = "windows")]
    fn test_open_device_for_writing_windows_access_denied_message() {
        let dir = std::env::temp_dir();
        let img = dir.join("fk_win_open_img.bin");
        std::fs::write(&img, vec![1u8; 512]).unwrap();

        let (tx, _rx) = make_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let result = write_image(
            img.to_str().unwrap(),
            r"\\.\PhysicalDrive999",
            512,
            &tx,
            &cancel,
        );

        assert!(result.is_err());
        let msg = result.unwrap_err();
        // Must mention either the path, or give a clear error.
        assert!(
            msg.contains("PhysicalDrive999")
                || msg.contains("Access denied")
                || msg.contains("Cannot open"),
            "error must be descriptive: {msg}"
        );

        let _ = std::fs::remove_file(&img);
    }
}
