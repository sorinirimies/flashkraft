//! Drive Constraints Demo — flashkraft-core
//!
//! Demonstrates the complete drive/image compatibility checking system used by
//! both the GUI and TUI to decide which drives are selectable, which need a
//! warning, and which must be disabled.
//!
//! ## What it covers
//!
//! | Function | Purpose |
//! |----------|---------|
//! | [`is_system_drive`] | Detects OS / boot drives |
//! | [`is_source_drive`] | Detects drives that contain the source image |
//! | [`is_drive_large_enough`] | Checks drive capacity vs image size |
//! | [`is_drive_size_large`] | Warns about drives > 128 GiB |
//! | [`get_drive_image_compatibility_statuses`] | Full compatibility report |
//! | [`mark_invalid_drives`] | Bulk-marks drives as disabled |
//!
//! ## Run
//!
//! ```bash
//! cargo run -p flashkraft-core --example constraints_demo
//! ```

use std::path::PathBuf;

use flashkraft_core::domain::constraints::{
    CompatibilityStatusType, LARGE_DRIVE_SIZE, get_drive_image_compatibility_statuses,
    is_drive_large_enough, is_drive_size_large, is_source_drive, is_system_drive,
    mark_invalid_drives,
};
use flashkraft_core::domain::{DriveInfo, ImageInfo};

// ── ANSI colour helpers (no external dep) ─────────────────────────────────────

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    print_header();

    // ── Section 1: Individual predicate functions ─────────────────────────────
    section("1. Individual predicate functions");

    let image_4gb = make_image("/media/usb/ubuntu-24.04-desktop-amd64.iso", 4_096.0);

    println!(
        "  {DIM}Test image:{RESET}  {CYAN}{}{RESET}  ({:.0} MB / {:.2} GiB)",
        image_4gb.name,
        image_4gb.size_mb,
        image_4gb.size_mb / 1024.0,
    );
    println!();

    // Eagerly evaluate each predicate so no closures or lifetime constraints are needed.
    let large_drive_desc = format!(
        "256 GiB drive → above {}GiB threshold",
        LARGE_DRIVE_SIZE as u32
    );
    let small_drive_desc = format!(
        "64 GiB drive → below {}GiB threshold",
        LARGE_DRIVE_SIZE as u32
    );

    let source_usb = make_drive_full("Source USB", "/media/usb", 64.0, "/dev/sdb", false, false);
    let target_usb = make_drive_full("Target USB", "/media/other", 32.0, "/dev/sdc", false, false);
    let big_usb = make_drive("big_usb", "/dev/sdb", 32.0);
    let tiny_usb = make_drive("tiny_usb", "/dev/sdb", 2.0);
    let big_hdd = make_drive("big_hdd", "/dev/sdb", 256.0);
    let normal_usb = make_drive("normal_usb", "/dev/sdb", 64.0);

    let predicates: &[(&str, bool, &str)] = &[
        (
            "is_system_drive",
            is_system_drive(&make_drive_full(
                "System NVMe",
                "/",
                8_000.0,
                "/dev/nvme0n1",
                true,
                false,
            )),
            "8 GiB system drive (/)",
        ),
        (
            "is_system_drive",
            is_system_drive(&make_drive_full(
                "USB Stick",
                "/media/usb",
                32.0,
                "/dev/sdb",
                false,
                false,
            )),
            "32 GiB USB stick (not system)",
        ),
        (
            "is_source_drive",
            is_source_drive(&source_usb, Some(&image_4gb)),
            "drive whose mount point contains the image",
        ),
        (
            "is_source_drive",
            is_source_drive(&target_usb, Some(&image_4gb)),
            "different drive (not source)",
        ),
        (
            "is_drive_large_enough",
            is_drive_large_enough(&big_usb, Some(&image_4gb)),
            "32 GiB drive for 4 GiB image → fits",
        ),
        (
            "is_drive_large_enough",
            is_drive_large_enough(&tiny_usb, Some(&image_4gb)),
            "2 GiB drive for 4 GiB image → too small",
        ),
        (
            "is_drive_size_large",
            is_drive_size_large(&big_hdd),
            &large_drive_desc,
        ),
        (
            "is_drive_size_large",
            is_drive_size_large(&normal_usb),
            &small_drive_desc,
        ),
    ];

    let name_width = predicates
        .iter()
        .map(|(n, _, _)| n.len())
        .max()
        .unwrap_or(0);

    for (name, result, desc) in predicates {
        let (colour, symbol) = if *result {
            (GREEN, "✓ true ")
        } else {
            (RED, "✗ false")
        };
        println!("  {BOLD}{name:<name_width$}{RESET}  {colour}{symbol}{RESET}  {DIM}{desc}{RESET}");
    }

    // ── Section 2: Compatibility status reports ───────────────────────────────
    section("2. Compatibility status reports — get_drive_image_compatibility_statuses");

    let scenario_image = make_image("/tmp/fedora-40-server.iso", 2_048.0); // 2 GiB

    let scenarios: &[(&str, DriveInfo)] = &[
        (
            "Perfect match (32 GiB, non-system, writable)",
            make_drive("usb_perfect", "/dev/sdb", 32.0),
        ),
        (
            "System drive (always warned/rejected)",
            make_drive_full("sys_disk", "/", 500.0, "/dev/sda", true, false),
        ),
        (
            "Read-only / write-protected drive",
            make_drive_full("ro_usb", "/media/ro_usb", 8.0, "/dev/sdb", false, true),
        ),
        (
            "Too small (1 GiB drive for 2 GiB image)",
            make_drive("tiny", "/dev/sdc", 1.0),
        ),
        (
            "Large drive warning (>128 GiB, otherwise fine)",
            make_drive("backup_hdd", "/dev/sdd", 500.0),
        ),
        (
            "Source drive (image is stored on this drive)",
            make_drive_full(
                "source_usb",
                "/media/source",
                64.0,
                "/dev/sde",
                false,
                false,
            ),
        ),
    ];

    // The last scenario needs an image whose path starts with the drive's mount point.
    let source_image = make_image("/media/source/fedora-40-server.iso", 2_048.0);

    for (i, (label, drive)) in scenarios.iter().enumerate() {
        let img = if i == scenarios.len() - 1 {
            &source_image
        } else {
            &scenario_image
        };

        let statuses = get_drive_image_compatibility_statuses(drive, Some(img));

        println!();
        println!(
            "  {BOLD}Drive:{RESET} {CYAN}{}{RESET}  {DIM}({}){RESET}",
            drive.name, label
        );
        println!(
            "  {DIM}  device={} size={:.0}GiB system={} ro={}{RESET}",
            drive.device_path, drive.size_gb, drive.is_system, drive.is_read_only
        );

        if statuses.is_empty() {
            println!("  {GREEN}  ✓  No compatibility issues — drive is ready to flash.{RESET}");
        } else {
            for status in &statuses {
                let (colour, tag) = match status.status_type {
                    CompatibilityStatusType::Error => (RED, "ERROR  "),
                    CompatibilityStatusType::Warning => (YELLOW, "WARNING"),
                };
                println!("  {colour}  ▶  [{tag}] {}{RESET}", status.message);
            }
        }
    }

    // ── Section 3: mark_invalid_drives — bulk operation ───────────────────────
    section("3. mark_invalid_drives — bulk disable based on compatibility");

    let bulk_image = make_image("/tmp/alpine-3.20-virt.iso", 256.0); // 256 MB

    let mut fleet: Vec<DriveInfo> = vec![
        make_drive("flash_usb_8g", "/dev/sdb", 8.0),
        make_drive("flash_usb_32g", "/dev/sdc", 32.0),
        make_drive_full("system_nvme", "/", 500.0, "/dev/nvme0n1", true, false),
        make_drive_full(
            "write_protected",
            "/media/ro",
            16.0,
            "/dev/sdd",
            false,
            true,
        ),
        make_drive("tiny_stick_64mb", "/dev/sde", 0.064), // 64 MiB
        make_drive("large_external_1t", "/dev/sdf", 1_000.0),
    ];

    println!();
    println!(
        "  {DIM}Image: {} ({:.0} MB){RESET}",
        bulk_image.name, bulk_image.size_mb
    );
    println!();
    println!(
        "  {BOLD}{:<24}  {:>8}  {:>6}  {:>6}  Status before mark{RESET}",
        "Drive", "Size", "System", "R/O"
    );

    let sep = "─".repeat(70);
    println!("  {DIM}{sep}{RESET}");

    for d in &fleet {
        let status_before = if d.disabled { "disabled" } else { "enabled " };
        println!(
            "  {BOLD}{:<24}{RESET}  {:>6.1}GiB  {:>6}  {:>6}  {DIM}{}{RESET}",
            d.name, d.size_gb, d.is_system, d.is_read_only, status_before
        );
    }

    // Apply bulk marking
    mark_invalid_drives(&mut fleet, Some(&bulk_image));

    println!();
    println!("  {DIM}After mark_invalid_drives():{RESET}");
    println!();
    println!(
        "  {BOLD}{:<24}  {:>8}  {:>8}  Reason{RESET}",
        "Drive", "Size", "Disabled"
    );
    println!("  {DIM}{sep}{RESET}");

    for d in &fleet {
        let (colour, status) = if d.disabled {
            (RED, format!("{RED}✗ disabled{RESET}"))
        } else {
            (GREEN, format!("{GREEN}✓ enabled {RESET}"))
        };

        let reason = reason_for(d, Some(&bulk_image));
        println!(
            "  {BOLD}{colour}{:<24}{RESET}  {:>6.1}GiB  {}  {DIM}{}{RESET}",
            d.name, d.size_gb, status, reason
        );
    }

    // ── Section 4: No-image case ───────────────────────────────────────────────
    section("4. Constraint checks with no image selected (image = None)");

    let no_image_drives = vec![
        make_drive("any_usb", "/dev/sdb", 32.0),
        make_drive_full("sys_disk", "/boot", 500.0, "/dev/sda", true, false),
        make_drive_full("locked", "/media/ro", 8.0, "/dev/sdc", false, true),
    ];

    for drive in &no_image_drives {
        let statuses = get_drive_image_compatibility_statuses(drive, None);
        let summary = if statuses.is_empty() {
            format!("{GREEN}No issues{RESET}")
        } else {
            let errors = statuses
                .iter()
                .filter(|s| s.status_type == CompatibilityStatusType::Error)
                .count();
            let warnings = statuses
                .iter()
                .filter(|s| s.status_type == CompatibilityStatusType::Warning)
                .count();
            format!("{RED}{errors} error(s){RESET}  {YELLOW}{warnings} warning(s){RESET}")
        };
        println!(
            "  {BOLD}{:<20}{RESET}  system={:<5}  ro={:<5}  → {summary}",
            drive.name,
            drive.is_system.to_string(),
            drive.is_read_only.to_string(),
        );
    }

    // ── Section 5: Edge cases ─────────────────────────────────────────────────
    section("5. Edge cases");

    // Drive with size_gb == 0.0 (unknown size)
    let zero_size_drive = DriveInfo::new(
        "unknown_size".into(),
        "/media/unknown".into(),
        0.0,
        "/dev/sdz".into(),
    );
    let edge_image = make_image("/tmp/test.img", 100.0);
    let zero_size_statuses =
        get_drive_image_compatibility_statuses(&zero_size_drive, Some(&edge_image));
    let has_error = zero_size_statuses
        .iter()
        .any(|s| s.status_type == CompatibilityStatusType::Error);
    println!(
        "  Zero-size drive against 100 MB image → {}",
        if has_error {
            format!("{RED}error (correct — size unknown/zero){RESET}")
        } else {
            format!("{YELLOW}no error{RESET}")
        }
    );

    // Image with size_mb == 0.0 (degenerate)
    let degenerate_image = make_image("/tmp/empty.img", 0.0);
    let small_drive = make_drive("small", "/dev/sdb", 1.0);
    let degen_statuses =
        get_drive_image_compatibility_statuses(&small_drive, Some(&degenerate_image));
    println!(
        "  1 GiB drive against 0 MB image → {} status(es)",
        degen_statuses.len()
    );

    // Exact-fit: drive size == image size
    let exact_drive = make_drive("exact_fit", "/dev/sdb", 4.0); // 4 GiB
    let exact_image = make_image("/tmp/exact.img", 4.0 * 1024.0); // 4 GiB
    let exact_statuses = get_drive_image_compatibility_statuses(&exact_drive, Some(&exact_image));
    let exact_ok = exact_statuses.is_empty();
    println!(
        "  Exact-fit (4 GiB drive, 4 GiB image) → {}",
        if exact_ok {
            format!("{GREEN}✓ compatible{RESET}")
        } else {
            format!("{RED}✗ {} issue(s){RESET}", exact_statuses.len())
        }
    );

    // ── Done ──────────────────────────────────────────────────────────────────
    println!();
    println!("{BOLD}{CYAN}Demo complete.{RESET}");
    println!("{DIM}All checks above exercise flashkraft_core::domain::constraints.{RESET}");
    println!();
}

// ── Builder helpers ───────────────────────────────────────────────────────────

/// Create a simple drive with default flags (not system, writable).
fn make_drive(name: &str, device: &str, size_gb: f64) -> DriveInfo {
    DriveInfo::new(
        name.into(),
        format!("/media/{name}"),
        size_gb,
        device.into(),
    )
}

/// Create a drive with all fields specified.
fn make_drive_full(
    name: &str,
    mount_point: &str,
    size_gb: f64,
    device: &str,
    is_system: bool,
    is_read_only: bool,
) -> DriveInfo {
    DriveInfo::with_constraints(
        name.into(),
        mount_point.into(),
        size_gb,
        device.into(),
        is_system,
        is_read_only,
    )
}

/// Create an ImageInfo with a known path and size.
fn make_image(path: &str, size_mb: f64) -> ImageInfo {
    let p = PathBuf::from(path);
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.img")
        .to_string();
    ImageInfo {
        path: p,
        name,
        size_mb,
    }
}

/// Derive a short human-readable reason string for the drive's disabled state.
fn reason_for(drive: &DriveInfo, image: Option<&ImageInfo>) -> String {
    let statuses = get_drive_image_compatibility_statuses(drive, image);
    if statuses.is_empty() {
        return "—".into();
    }
    // Pick the first error; fall back to the first warning.
    let primary = statuses
        .iter()
        .find(|s| s.status_type == CompatibilityStatusType::Error)
        .or_else(|| statuses.first())
        .expect("non-empty statuses has at least one entry");

    // Truncate long messages for table display
    let msg = &primary.message;
    if msg.len() > 55 {
        format!("{}…", &msg[..54])
    } else {
        msg.clone()
    }
}

// ── UI chrome ─────────────────────────────────────────────────────────────────

fn print_header() {
    println!();
    println!("{BOLD}{CYAN}┌──────────────────────────────────────────────────────────┐{RESET}");
    println!(
        "{BOLD}{CYAN}│{RESET}  {BOLD}flashkraft-core  ·  Drive Constraints Demo{RESET}           \
         {BOLD}{CYAN}│{RESET}"
    );
    println!("{BOLD}{CYAN}└──────────────────────────────────────────────────────────┘{RESET}");
    println!();
    println!(
        "Exercises {CYAN}flashkraft_core::domain::constraints{RESET} — \
         the same checks used by both the GUI and TUI\n\
         to decide which drives are selectable, warned, or disabled."
    );
    println!();
}

fn section(title: &str) {
    let pad = "─".repeat(65usize.saturating_sub(title.len() + 4));
    println!();
    println!("{BOLD}{MAGENTA}── {title} {DIM}{pad}{RESET}");
    println!();
}
