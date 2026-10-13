//! Shaping drives, the chosen image and the flash history into one
//! [`Snapshot`] a page can render (pure — only [`load`] touches the system).

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use flashkraft_core::domain::constraints::{
    CompatibilityStatusType, get_drive_image_compatibility_statuses,
};
use flashkraft_core::{DriveInfo, FlashHistoryEntry, ImageInfo, fmt_bytes};
use serde::Serialize;

/// How often a connected page re-reads the drive list (hot-plug).
pub const REFRESH_EVERY: std::time::Duration = std::time::Duration::from_secs(3);

/// How an image relates to a drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// No image chosen, nothing to compare.
    Unknown,
    /// The image fits and nothing is wrong.
    Ok,
    /// Usable, but with a warning (e.g. a very large drive).
    Warn,
    /// Must not be flashed (system disk, too small, read-only, …).
    Blocked,
}

impl Fit {
    pub fn label(self) -> &'static str {
        match self {
            Fit::Unknown => "—",
            Fit::Ok => "ready",
            Fit::Warn => "check",
            Fit::Blocked => "blocked",
        }
    }
    pub fn css(self) -> &'static str {
        match self {
            Fit::Unknown => "unknown",
            Fit::Ok => "ok",
            Fit::Warn => "warn",
            Fit::Blocked => "err",
        }
    }
}

/// One drive, ready to render.
#[derive(Debug, Clone, Serialize)]
pub struct DriveRow {
    pub name: String,
    pub device_path: String,
    pub mount_point: String,
    pub size_bytes: u64,
    pub size_label: String,
    /// `"SanDisk Ultra · SuperSpeed (5 Gbps)"` for USB drives, `None` otherwise.
    pub usb: Option<String>,
    pub is_system: bool,
    pub is_read_only: bool,
    pub fit: Fit,
    /// Constraint messages (`"error"` / `"warning"` + text).
    pub notes: Vec<(String, String)>,
    /// Capacity relative to the largest drive, 0–100 (for the bar).
    pub capacity_pct: u8,
    /// Image size relative to this drive's capacity, 0–100 (the fit meter);
    /// `None` without an image.
    pub image_pct: Option<u8>,
}

/// The chosen image, if any.
#[derive(Debug, Clone, Serialize)]
pub struct ImageView {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub size_label: String,
}

/// One history line.
#[derive(Debug, Clone, Serialize)]
pub struct HistoryRow {
    pub image_name: String,
    pub image_path: String,
    pub drive_label: String,
    pub size_label: String,
    pub completed_at_unix: u64,
    /// `"3 h ago"`.
    pub ago: String,
}

/// Everything a page render needs.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub drives: Vec<DriveRow>,
    pub image: Option<ImageView>,
    /// A path was asked for but is not a readable, non-empty file.
    pub image_error: Option<String>,
    pub history: Vec<HistoryRow>,
    pub usb_count: usize,
    pub usable_count: usize,
    pub total_bytes: u64,
    pub demo: bool,
    pub updated_at: String,
}

/// `"just now"`, `"5 min ago"`, `"3 h ago"`, `"2 d ago"`.
pub fn ago(now: u64, then: u64) -> String {
    let d = now.saturating_sub(then);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86_399 => format!("{} h ago", d / 3600),
        _ => format!("{} d ago", d / 86_400),
    }
}

/// Percentage `part / whole`, clamped to 0–100.
pub fn pct(part: u64, whole: u64) -> u8 {
    if whole == 0 {
        return 0;
    }
    ((part as f64 / whole as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8
}

/// Resolve a user-typed image path: `~/` is expanded, only a non-empty regular
/// file counts. Only metadata is read — never the file's contents.
pub fn inspect_image(raw: &str) -> Result<ImageInfo, String> {
    let raw = raw.trim();
    let path = match raw.strip_prefix("~/") {
        Some(rest) => dirs_home()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| raw.into()),
        None => PathBuf::from(raw),
    };
    let meta = path
        .metadata()
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a file", path.display()));
    }
    if meta.len() == 0 {
        return Err(format!("{} is empty", path.display()));
    }
    Ok(ImageInfo::from_path(path))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Shape raw inputs into a [`Snapshot`] (pure).
pub fn build_snapshot(
    drives: Vec<DriveInfo>,
    image: Option<ImageInfo>,
    image_error: Option<String>,
    history: Vec<FlashHistoryEntry>,
    now: u64,
    demo: bool,
    updated_at: String,
) -> Snapshot {
    let largest = drives.iter().map(DriveInfo::size_bytes).max().unwrap_or(0);
    let image_bytes = image.as_ref().map(ImageInfo::size_bytes);

    let rows: Vec<DriveRow> = drives
        .iter()
        .map(|d| {
            let statuses = get_drive_image_compatibility_statuses(d, image.as_ref());
            let has_err = statuses
                .iter()
                .any(|s| s.status_type == CompatibilityStatusType::Error);
            let has_warn = statuses
                .iter()
                .any(|s| s.status_type == CompatibilityStatusType::Warning);
            let fit = if d.disabled || has_err {
                Fit::Blocked
            } else if image.is_none() {
                Fit::Unknown
            } else if has_warn {
                Fit::Warn
            } else {
                Fit::Ok
            };
            DriveRow {
                name: d.name.clone(),
                device_path: d.device_path.clone(),
                mount_point: d.mount_point.clone(),
                size_bytes: d.size_bytes(),
                size_label: fmt_bytes(d.size_bytes()),
                usb: d.usb_info.as_ref().map(|u| match &u.speed {
                    Some(s) => format!("{} · {s}", u.display_label()),
                    None => u.display_label(),
                }),
                is_system: d.is_system,
                is_read_only: d.is_read_only,
                fit,
                notes: statuses
                    .into_iter()
                    .map(|s| {
                        let kind = match s.status_type {
                            CompatibilityStatusType::Error => "error",
                            CompatibilityStatusType::Warning => "warning",
                        };
                        (kind.to_string(), s.message)
                    })
                    .collect(),
                capacity_pct: pct(d.size_bytes(), largest),
                image_pct: image_bytes.map(|b| pct(b, d.size_bytes())),
            }
        })
        .collect();

    Snapshot {
        usb_count: drives.iter().filter(|d| d.usb_info.is_some()).count(),
        usable_count: rows
            .iter()
            .filter(|r| matches!(r.fit, Fit::Ok | Fit::Warn | Fit::Unknown) && !r.is_system)
            .count(),
        total_bytes: drives.iter().map(DriveInfo::size_bytes).sum(),
        image: image.map(|i| ImageView {
            path: i.path.display().to_string(),
            size_bytes: i.size_bytes(),
            size_label: fmt_bytes(i.size_bytes()),
            name: i.name,
        }),
        image_error,
        history: history
            .into_iter()
            .map(|h| HistoryRow {
                image_name: h
                    .image_path
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&h.image_path)
                    .to_string(),
                size_label: fmt_bytes((h.size_mb * 1_048_576.0) as u64),
                ago: ago(now, h.completed_at_unix),
                completed_at_unix: h.completed_at_unix,
                image_path: h.image_path,
                drive_label: h.drive_label,
            })
            .collect(),
        drives: rows,
        demo,
        updated_at,
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Detect drives and read history on a blocking thread, then shape them.
/// `image` is the (possibly empty) path the user typed.
pub async fn load(image: Option<String>) -> Snapshot {
    tokio::task::spawn_blocking(move || {
        let (info, err) = match image.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => (None, None),
            Some(p) => match inspect_image(p) {
                Ok(i) => (Some(i), None),
                Err(e) => (None, Some(e)),
            },
        };
        let mut drives = flashkraft_core::commands::load_drives_sync();
        flashkraft_core::domain::mark_invalid_drives(&mut drives, info.as_ref());
        build_snapshot(
            drives,
            info,
            err,
            flashkraft_core::settings::load_history(),
            now_unix(),
            flashkraft_core::commands::demo::is_demo(),
            clock(),
        )
    })
    .await
    .unwrap_or_else(|_| build_snapshot(vec![], None, None, vec![], 0, false, clock()))
}

/// Wall-clock `HH:MM:SS` (UTC) without pulling in a date crate.
fn clock() -> String {
    let s = now_unix() % 86_400;
    format!("{:02}:{:02}:{:02} UTC", s / 3600, (s % 3600) / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashkraft_core::commands::demo::demo_drives;

    fn img(mb: f64) -> ImageInfo {
        ImageInfo {
            path: "/tmp/x.iso".into(),
            name: "x.iso".into(),
            size_mb: mb,
        }
    }

    fn snap(image: Option<ImageInfo>) -> Snapshot {
        let mut drives = demo_drives();
        flashkraft_core::domain::mark_invalid_drives(&mut drives, image.as_ref());
        build_snapshot(
            drives,
            image,
            None,
            vec![],
            1_000_000,
            true,
            "00:00:00".into(),
        )
    }

    #[test]
    fn ago_buckets() {
        assert_eq!(ago(100, 100), "just now");
        assert_eq!(ago(100, 200), "just now", "future clamps");
        assert_eq!(ago(1000, 700), "5 min ago");
        assert_eq!(ago(20_000, 20_000 - 7_300), "2 h ago");
        assert_eq!(ago(500_000, 500_000 - 200_000), "2 d ago");
    }

    #[test]
    fn pct_is_clamped_and_total() {
        assert_eq!(pct(5, 0), 0);
        assert_eq!(pct(50, 100), 50);
        assert_eq!(pct(500, 100), 100);
    }

    #[test]
    fn no_image_means_unknown_fit_except_system_disk() {
        let s = snap(None);
        let sys = s.drives.iter().find(|d| d.is_system).unwrap();
        assert_eq!(sys.fit, Fit::Blocked);
        assert!(
            s.drives
                .iter()
                .filter(|d| !d.is_system)
                .all(|d| d.fit == Fit::Unknown)
        );
        assert!(s.drives.iter().all(|d| d.image_pct.is_none()));
        assert_eq!(s.usable_count, 3);
        assert_eq!(s.usb_count, 3);
    }

    #[test]
    fn a_big_image_blocks_small_drives_and_fits_large_ones() {
        // 20 GB image: the 14.6 GB and 29.8 GB… only the 29.8 GB and 119 GB fit.
        let s = snap(Some(img(20.0 * 1024.0)));
        let by = |p: &str| s.drives.iter().find(|d| d.device_path == p).unwrap();
        assert_eq!(by("/dev/sdc").fit, Fit::Blocked, "14.6 GB is too small");
        assert!(matches!(by("/dev/sdb").fit, Fit::Ok | Fit::Warn));
        assert!(matches!(by("/dev/sdd").fit, Fit::Ok | Fit::Warn));
        assert!(by("/dev/sdd").image_pct.unwrap() < by("/dev/sdb").image_pct.unwrap());
        assert!(by("/dev/sdc").notes.iter().any(|(k, _)| k == "error"));
    }

    #[test]
    fn capacity_bar_is_relative_to_the_largest_drive() {
        let s = snap(None);
        assert_eq!(s.drives.iter().map(|d| d.capacity_pct).max(), Some(100));
        let big = s
            .drives
            .iter()
            .find(|d| d.device_path == "/dev/nvme0n1")
            .unwrap();
        assert_eq!(big.capacity_pct, 100);
    }

    #[test]
    fn usb_label_includes_speed() {
        let s = snap(None);
        let sdb = s
            .drives
            .iter()
            .find(|d| d.device_path == "/dev/sdb")
            .unwrap();
        assert_eq!(
            sdb.usb.as_deref(),
            Some("SanDisk Ultra · SuperSpeed (5 Gbps)")
        );
        assert!(s.drives.iter().find(|d| d.is_system).unwrap().usb.is_none());
    }

    #[test]
    fn history_rows_show_file_name_and_age() {
        let h = vec![FlashHistoryEntry {
            image_path: "/home/me/isos/debian.iso".into(),
            drive_label: "SanDisk — 32 GB".into(),
            size_mb: 1024.0,
            completed_at_unix: 1_000_000 - 7_200,
        }];
        let s = build_snapshot(vec![], None, None, h, 1_000_000, false, "t".into());
        assert_eq!(s.history[0].image_name, "debian.iso");
        assert_eq!(s.history[0].ago, "2 h ago");
        assert_eq!(
            s.history[0].size_label,
            flashkraft_core::fmt_bytes(1024 * 1_048_576)
        );
    }

    #[test]
    fn inspect_image_accepts_files_and_explains_failures() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.iso");
        std::fs::write(&f, vec![0u8; 2048]).unwrap();
        let info = inspect_image(f.to_str().unwrap()).unwrap();
        assert_eq!(info.name, "a.iso");
        assert_eq!(info.size_bytes(), 2048);
        assert!(
            inspect_image(dir.path().to_str().unwrap())
                .unwrap_err()
                .contains("not a file")
        );
        assert!(
            inspect_image("/definitely/not/here.iso")
                .unwrap_err()
                .contains("cannot read")
        );
        let e = dir.path().join("e.iso");
        std::fs::write(&e, b"").unwrap();
        assert!(
            inspect_image(e.to_str().unwrap())
                .unwrap_err()
                .contains("empty")
        );
    }

    #[test]
    fn snapshot_serialises_to_json() {
        let j = serde_json::to_value(snap(None)).unwrap();
        assert_eq!(j["drives"].as_array().unwrap().len(), 4);
        assert_eq!(j["drives"][0]["fit"], "blocked");
    }
}
