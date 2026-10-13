//! Flashing from the browser — and every guard that makes that acceptable.
//!
//! A web page that can erase disks is dangerous, so the request path is
//! deliberately paranoid. A flash starts only when **all** of these hold:
//!
//! 1. flashing is enabled — the server is bound to loopback and was not
//!    started with `--read-only`;
//! 2. the `Host` header is a loopback name (defeats DNS rebinding);
//! 3. the request carries this process's random token, which is only ever
//!    embedded in the pages the server itself serves (so another site cannot
//!    read it) — Topcoat's origin policy additionally rejects cross-site POSTs;
//! 4. the body is JSON (a "non-simple" request: browsers refuse to send it
//!    cross-origin without a pre-flight we never grant);
//! 5. every device is re-detected **on the server** and re-checked with the
//!    same rules as the TUI/GUI (no system drives, no read-only media, must
//!    fit) — the browser's idea of a drive is never trusted;
//! 6. the caller confirmed the exact number of drives being erased;
//! 7. no other flash is running.

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use flashkraft_core::{
    FlashHistoryEntry,
    domain::{DriveInfo, ImageInfo, is_drive_valid},
    multi_flash::{DEFAULT_MAX_PARALLEL, MultiState, MultiUpdate, TargetStatus, spawn_multi_flash},
};
use serde::{Deserialize, Serialize};

/// Header carrying the per-process token.
pub const TOKEN_HEADER: &str = "x-flashkraft-token";
/// Most drives one request may target.
pub const MAX_TARGETS: usize = 16;

// ---------------------------------------------------------------------------
// Process-wide configuration
// ---------------------------------------------------------------------------

/// Runtime switches, fixed at start-up.
#[derive(Debug, Clone)]
pub struct FlashConfig {
    /// `false` → every flash endpoint answers 403.
    pub enabled: bool,
    pub token: String,
}

static CONFIG: OnceLock<FlashConfig> = OnceLock::new();
static MANAGER: OnceLock<Manager> = OnceLock::new();

/// Set the start-up policy. First call wins.
pub fn configure(enabled: bool) {
    let _ = CONFIG.set(FlashConfig {
        enabled,
        token: random_token(),
    });
}

/// The active configuration (flashing **disabled** if [`configure`] never ran).
pub fn config() -> &'static FlashConfig {
    CONFIG.get_or_init(|| FlashConfig {
        enabled: false,
        token: random_token(),
    })
}

/// The shared job manager.
pub fn manager() -> &'static Manager {
    MANAGER.get_or_init(Manager::default)
}

/// Flashing is only offered when the server cannot be reached from the network.
pub fn flashing_allowed(host: &str, read_only: bool) -> bool {
    !read_only && is_loopback_host(host)
}

/// `true` for `localhost`, `127.x.x.x` and `::1` (no port).
pub fn is_loopback_host(host: &str) -> bool {
    let h = host.trim().trim_start_matches('[').trim_end_matches(']');
    h.eq_ignore_ascii_case("localhost")
        || h.parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

// ---------------------------------------------------------------------------
// Request guards (pure → unit-testable)
// ---------------------------------------------------------------------------

/// 128 random bits as hex. `/dev/urandom` where available, otherwise std's
/// randomly-seeded hasher mixed with the clock and pid.
pub fn random_token() -> String {
    use std::io::Read;
    let mut buf = [0u8; 16];
    let ok = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .is_ok();
    if !ok {
        use std::hash::{BuildHasher, Hasher};
        for chunk in buf.chunks_mut(8) {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u128(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos()),
            );
            h.write_u32(std::process::id());
            chunk.copy_from_slice(&h.finish().to_le_bytes()[..chunk.len()]);
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Constant-time comparison.
pub fn token_matches(expected: &str, given: &str) -> bool {
    let (a, b) = (expected.as_bytes(), given.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Host header (`localhost:3000`, `[::1]:3000`) → host part without the port.
pub fn host_without_port(header: &str) -> &str {
    let h = header.trim();
    if let Some(rest) = h.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest);
    }
    h.rsplit_once(':').map_or(h, |(host, _)| host)
}

/// Why a request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reject {
    pub status: u16,
    pub message: String,
}

impl Reject {
    fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

/// Guards 1–3: policy, `Host`, token.
pub fn check_caller(
    cfg: &FlashConfig,
    host_header: Option<&str>,
    token: Option<&str>,
) -> Result<(), Reject> {
    if !cfg.enabled {
        return Err(Reject::new(
            403,
            "flashing is disabled (the server is not bound to loopback, or --read-only was given)",
        ));
    }
    if !host_header.is_some_and(|h| is_loopback_host(host_without_port(h))) {
        return Err(Reject::new(403, "unexpected Host header"));
    }
    if !token.is_some_and(|t| token_matches(&cfg.token, t)) {
        return Err(Reject::new(
            403,
            "missing or wrong flash token — reload the page",
        ));
    }
    Ok(())
}

/// Body of `POST /api/flash`.
#[derive(Debug, Clone, Deserialize)]
pub struct StartRequest {
    pub image: String,
    pub devices: Vec<String>,
    /// Must equal the number of distinct devices: the user saw and accepted
    /// "erase N drives".
    pub confirm: usize,
}

/// Guards 5–6: re-check everything against freshly detected drives.
///
/// `drives` and `image` come from the **server's** own detection.
pub fn validate(
    req: &StartRequest,
    drives: &[DriveInfo],
    image: Result<ImageInfo, String>,
) -> Result<(ImageInfo, Vec<DriveInfo>), Reject> {
    if req.devices.is_empty() {
        return Err(Reject::new(400, "no drives selected"));
    }
    if req.devices.len() > MAX_TARGETS {
        return Err(Reject::new(
            400,
            format!("at most {MAX_TARGETS} drives at once"),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let devices: Vec<&String> = req.devices.iter().filter(|d| seen.insert(*d)).collect();
    if req.confirm != devices.len() {
        return Err(Reject::new(
            400,
            format!(
                "confirmation mismatch: you confirmed {} drive(s) but {} were selected",
                req.confirm,
                devices.len()
            ),
        ));
    }
    let image = image.map_err(|e| Reject::new(400, e))?;

    let mut targets = Vec::with_capacity(devices.len());
    for dev in devices {
        let Some(d) = drives.iter().find(|d| &d.device_path == dev) else {
            return Err(Reject::new(
                400,
                format!("{dev} is not a detected drive (was it unplugged?)"),
            ));
        };
        if d.is_system || d.is_read_only || d.disabled || !is_drive_valid(d, Some(&image)) {
            return Err(Reject::new(
                400,
                format!("{} ({dev}) cannot be flashed with this image", d.name),
            ));
        }
        targets.push(d.clone());
    }
    Ok((image, targets))
}

// ---------------------------------------------------------------------------
// Job manager
// ---------------------------------------------------------------------------

struct Job {
    image: String,
    /// `device → "Name (size)"` for history labels and the UI.
    labels: Vec<(String, String)>,
    state: MultiState,
    cancel: Arc<AtomicBool>,
}

/// Holds the one flash job the server allows at a time.
#[derive(Default)]
pub struct Manager {
    job: Mutex<Option<Job>>,
}

/// One drive in a [`FlashStatus`].
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TargetStatusView {
    pub device: String,
    pub label: String,
    /// `queued` | `running` | `done` | `failed`
    pub status: &'static str,
    pub progress: f32,
    pub write: f32,
    pub verify: f32,
    pub speed_mb_s: f32,
    pub message: String,
    pub error: Option<String>,
}

/// `GET /api/flash`.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct FlashStatus {
    /// A job exists (running or finished).
    pub present: bool,
    /// Still writing.
    pub running: bool,
    pub image: String,
    pub progress: f32,
    pub speed_mb_s: f32,
    pub done: usize,
    pub failed: usize,
    pub targets: Vec<TargetStatusView>,
}

impl Manager {
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Job>> {
        self.job.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_running(&self) -> bool {
        self.lock()
            .as_ref()
            .is_some_and(|j| j.state.summary.is_none())
    }

    pub fn status(&self) -> FlashStatus {
        let guard = self.lock();
        let Some(job) = guard.as_ref() else {
            return FlashStatus::default();
        };
        let label_of = |dev: &str| {
            job.labels
                .iter()
                .find(|(d, _)| d == dev)
                .map_or_else(|| dev.to_string(), |(_, l)| l.clone())
        };
        FlashStatus {
            present: true,
            running: job.state.summary.is_none(),
            image: job.image.clone(),
            progress: job.state.overall(),
            speed_mb_s: job.state.total_speed_mb_s(),
            done: job.state.done(),
            failed: job.state.failed(),
            targets: job
                .state
                .targets
                .iter()
                .map(|t| TargetStatusView {
                    device: t.device.clone(),
                    label: label_of(&t.device),
                    status: match t.status {
                        TargetStatus::Queued => "queued",
                        TargetStatus::Running => "running",
                        TargetStatus::Done => "done",
                        TargetStatus::Failed(_) => "failed",
                    },
                    progress: t.overall(),
                    write: t.write,
                    verify: t.verify,
                    speed_mb_s: t.speed_mb_s,
                    message: t.message.clone(),
                    error: match &t.status {
                        TargetStatus::Failed(e) => Some(e.clone()),
                        _ => None,
                    },
                })
                .collect(),
        }
    }

    /// Guard 7, then start writing. `targets` must already be validated.
    pub fn start(&self, image: &ImageInfo, targets: &[DriveInfo]) -> Result<(), Reject> {
        let mut guard = self.lock();
        if guard.as_ref().is_some_and(|j| j.state.summary.is_none()) {
            return Err(Reject::new(409, "a flash is already running"));
        }
        let devices: Vec<String> = targets.iter().map(|d| d.device_path.clone()).collect();
        let cancel = Arc::new(AtomicBool::new(false));
        let paths: Vec<PathBuf> = devices.iter().map(PathBuf::from).collect();
        let rx = spawn_multi_flash(&image.path, &paths, DEFAULT_MAX_PARALLEL, cancel.clone());

        *guard = Some(Job {
            image: image.name.clone(),
            labels: targets
                .iter()
                .map(|d| {
                    (
                        d.device_path.clone(),
                        format!("{} ({:.1} GB)", d.name, d.size_gb),
                    )
                })
                .collect(),
            state: MultiState::new(&devices),
            cancel,
        });
        drop(guard);

        let (image_path, size_mb) = (image.path.display().to_string(), image.size_mb);
        std::thread::Builder::new()
            .name("flashkraft-web-flash".into())
            .spawn(move || drain(manager(), rx, &image_path, size_mb))
            .map_err(|e| Reject::new(500, format!("could not start the flash thread: {e}")))?;
        Ok(())
    }

    /// Ask the running job to stop. Returns whether there was one.
    pub fn cancel(&self) -> bool {
        match self.lock().as_ref() {
            Some(j) if j.state.summary.is_none() => {
                j.cancel.store(true, Ordering::SeqCst);
                true
            }
            _ => false,
        }
    }

    fn apply(&self, update: &MultiUpdate) {
        if let Some(j) = self.lock().as_mut() {
            j.state.apply(update);
        }
    }

    fn label(&self, device: &str) -> String {
        self.lock()
            .as_ref()
            .and_then(|j| j.labels.iter().find(|(d, _)| d == device))
            .map_or_else(|| device.to_string(), |(_, l)| l.clone())
    }
}

/// Feed updates into the job; record history; guarantee a final summary.
fn drain(m: &Manager, rx: std::sync::mpsc::Receiver<MultiUpdate>, image_path: &str, size_mb: f64) {
    while let Ok(update) = rx.recv() {
        if let MultiUpdate::Finished(summary) = &update {
            for ok in &summary.succeeded {
                // A history write must never fail a flash.
                let _ = flashkraft_core::settings::record_flash(FlashHistoryEntry::new(
                    image_path.to_string(),
                    m.label(ok),
                    size_mb,
                ));
            }
        }
        m.apply(&update);
    }
    // Channel closed without a verdict (should not happen): close the job.
    let mut guard = m.lock();
    if let Some(j) = guard.as_mut()
        && j.state.summary.is_none()
    {
        j.state.summary = Some(flashkraft_core::multi_flash::MultiSummary::default());
    }
}

/// Browser-side logic: selection, confirm dialog, progress panel.
pub const SCRIPT: &str = include_str!("flash.js");

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(enabled: bool) -> FlashConfig {
        FlashConfig {
            enabled,
            token: "t0k3n".into(),
        }
    }

    fn drive(dev: &str, gb: f64) -> DriveInfo {
        let mut d = DriveInfo::new(format!("Stick {dev}"), dev.into(), gb, dev.into());
        d.usb_info = None;
        d
    }

    fn image(mb: f64) -> ImageInfo {
        ImageInfo {
            path: "/tmp/x.iso".into(),
            name: "x.iso".into(),
            size_mb: mb,
        }
    }

    fn req(devs: &[&str], confirm: usize) -> StartRequest {
        StartRequest {
            image: "/tmp/x.iso".into(),
            devices: devs.iter().map(|s| s.to_string()).collect(),
            confirm,
        }
    }

    #[test]
    fn loopback_hosts_only() {
        for h in [
            "localhost",
            "LOCALHOST",
            "127.0.0.1",
            "127.1.2.3",
            "::1",
            "[::1]",
        ] {
            assert!(is_loopback_host(h), "{h}");
        }
        for h in [
            "0.0.0.0",
            "192.168.1.5",
            "evil.com",
            "localhost.evil.com",
            "",
        ] {
            assert!(!is_loopback_host(h), "{h}");
        }
    }

    #[test]
    fn host_port_is_stripped() {
        assert_eq!(host_without_port("localhost:3000"), "localhost");
        assert_eq!(host_without_port("127.0.0.1:80"), "127.0.0.1");
        assert_eq!(host_without_port("[::1]:3000"), "::1");
        assert_eq!(host_without_port("evil.com"), "evil.com");
    }

    #[test]
    fn flashing_needs_loopback_and_not_read_only() {
        assert!(flashing_allowed("127.0.0.1", false));
        assert!(!flashing_allowed("0.0.0.0", false));
        assert!(!flashing_allowed("127.0.0.1", true));
    }

    #[test]
    fn tokens_are_random_hex_and_compared_exactly() {
        let (a, b) = (random_token(), random_token());
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
        assert!(token_matches(&a, &a));
        assert!(!token_matches(&a, &b));
        assert!(!token_matches(&a, ""));
        assert!(!token_matches(&a, &a[..31]));
    }

    #[test]
    fn caller_must_pass_every_guard() {
        let ok = cfg(true);
        assert!(check_caller(&ok, Some("localhost:3000"), Some("t0k3n")).is_ok());
        // disabled
        assert_eq!(
            check_caller(&cfg(false), Some("localhost"), Some("t0k3n"))
                .unwrap_err()
                .status,
            403
        );
        // rebinding: attacker's name resolving to 127.0.0.1
        assert!(check_caller(&ok, Some("evil.com:3000"), Some("t0k3n")).is_err());
        assert!(check_caller(&ok, None, Some("t0k3n")).is_err());
        // token
        assert!(check_caller(&ok, Some("localhost"), None).is_err());
        assert!(check_caller(&ok, Some("localhost"), Some("nope")).is_err());
    }

    #[test]
    fn valid_request_yields_targets_in_order() {
        let drives = vec![drive("/dev/sdb", 32.0), drive("/dev/sdc", 16.0)];
        let (img, t) = validate(
            &req(&["/dev/sdc", "/dev/sdb"], 2),
            &drives,
            Ok(image(700.0)),
        )
        .unwrap();
        assert_eq!(img.name, "x.iso");
        assert_eq!(t[0].device_path, "/dev/sdc");
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn duplicates_are_collapsed_before_confirming() {
        let drives = vec![drive("/dev/sdb", 32.0)];
        let (_, t) = validate(&req(&["/dev/sdb", "/dev/sdb"], 1), &drives, Ok(image(1.0))).unwrap();
        assert_eq!(t.len(), 1);
        // confirming the pre-dedupe count is a mismatch
        assert!(validate(&req(&["/dev/sdb", "/dev/sdb"], 2), &drives, Ok(image(1.0))).is_err());
    }

    #[test]
    fn confirmation_must_match_the_count() {
        let drives = vec![drive("/dev/sdb", 32.0), drive("/dev/sdc", 32.0)];
        for bad in [0, 1, 3] {
            let e = validate(
                &req(&["/dev/sdb", "/dev/sdc"], bad),
                &drives,
                Ok(image(1.0)),
            )
            .unwrap_err();
            assert_eq!(e.status, 400);
            assert!(e.message.contains("confirmation"));
        }
    }

    #[test]
    fn unknown_devices_are_refused_even_if_the_path_is_real() {
        let drives = vec![drive("/dev/sdb", 32.0)];
        for evil in ["/dev/sda", "/etc/passwd", "../../dev/sdb", "/dev/sdb "] {
            let e = validate(&req(&[evil], 1), &drives, Ok(image(1.0))).unwrap_err();
            assert!(e.message.contains("not a detected drive"), "{evil}");
        }
    }

    #[test]
    fn system_read_only_and_too_small_drives_are_refused() {
        let mut sys = drive("/dev/sda", 500.0);
        sys.is_system = true;
        let mut ro = drive("/dev/sdb", 32.0);
        ro.is_read_only = true;
        let tiny = drive("/dev/sdc", 0.1);
        let drives = vec![sys, ro, tiny];
        for dev in ["/dev/sda", "/dev/sdb", "/dev/sdc"] {
            assert!(
                validate(&req(&[dev], 1), &drives, Ok(image(700.0))).is_err(),
                "{dev}"
            );
        }
    }

    #[test]
    fn one_bad_drive_rejects_the_whole_request() {
        let mut sys = drive("/dev/sda", 500.0);
        sys.is_system = true;
        let drives = vec![drive("/dev/sdb", 32.0), sys];
        assert!(validate(&req(&["/dev/sdb", "/dev/sda"], 2), &drives, Ok(image(1.0))).is_err());
    }

    #[test]
    fn empty_oversized_and_imageless_requests_are_refused() {
        let drives: Vec<DriveInfo> = (0..20)
            .map(|i| drive(&format!("/dev/sd{i}"), 32.0))
            .collect();
        assert!(validate(&req(&[], 0), &drives, Ok(image(1.0))).is_err());
        let many: Vec<String> = (0..17).map(|i| format!("/dev/sd{i}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        assert!(validate(&req(&many, 17), &drives, Ok(image(1.0))).is_err());
        assert!(validate(&req(&["/dev/sd1"], 1), &drives, Err("no such file".into())).is_err());
    }

    #[test]
    fn status_reflects_an_empty_manager() {
        assert_eq!(Manager::default().status(), FlashStatus::default());
        assert!(!Manager::default().cancel());
        assert!(!Manager::default().is_running());
    }
}
