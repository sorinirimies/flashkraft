//! Drive Detection Example — flashkraft-core
//!
//! Runs the same async drive-detection logic used by both the GUI and TUI
//! at startup, then pretty-prints every discovered block device in a table.
//!
//! # What it shows
//!
//! - How to call [`flashkraft_core::commands::load_drives`] from an async context
//! - The full [`DriveInfo`] model: name, device path, mount point, size, flags
//! - Which drives are flagged as system, read-only, or safe to flash
//!
//! # Run
//!
//! ```bash
//! cargo run -p flashkraft-core --example detect_drives
//! ```
//!
//! No root privileges are required — this is a read-only scan.

use flashkraft_core::DriveInfo;
use flashkraft_core::commands::load_drives;

// ── ANSI colour helpers (no external dep) ─────────────────────────────────────

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    print_header();

    println!("{DIM}Scanning block devices via /sys/block …{RESET}");
    println!();

    let drives = load_drives().await;

    if drives.is_empty() {
        println!("{YELLOW}No block devices found.{RESET}");
        println!();
        println!(
            "{DIM}This example scans /sys/block and requires Linux.\n\
             On macOS or Windows, the scan returns an empty list.{RESET}"
        );
        return;
    }

    print_summary(&drives);
    println!();
    print_table(&drives);
    println!();
    print_flash_candidates(&drives);
    println!();
    print_legend();
}

// ── Summary line ──────────────────────────────────────────────────────────────

fn print_summary(drives: &[DriveInfo]) {
    let total = drives.len();
    let system = drives.iter().filter(|d| d.is_system).count();
    let read_only = drives.iter().filter(|d| d.is_read_only).count();
    let flashable = drives
        .iter()
        .filter(|d| !d.is_system && !d.is_read_only)
        .count();

    println!(
        "Found {BOLD}{total}{RESET} device(s):  \
         {GREEN}{flashable} flashable{RESET}  \
         {YELLOW}{system} system{RESET}  \
         {RED}{read_only} read-only{RESET}"
    );
}

// ── Table ─────────────────────────────────────────────────────────────────────

fn print_table(drives: &[DriveInfo]) {
    // Column widths — widen dynamically based on content.
    let w_name = drives
        .iter()
        .map(|d| d.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let w_dev = drives
        .iter()
        .map(|d| d.device_path.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let w_mount = drives
        .iter()
        .map(|d| d.mount_point.len())
        .max()
        .unwrap_or(5)
        .max(5);

    // Header
    println!(
        "{BOLD}{CYAN}{:<w_name$}  {:<w_dev$}  {:<w_mount$}  {:>8}  {:<8}  Status{RESET}",
        "Name",
        "Device",
        "Mount",
        "Size",
        "Flags",
        w_name = w_name,
        w_dev = w_dev,
        w_mount = w_mount,
    );

    println!(
        "{DIM}{}{RESET}",
        "─".repeat(w_name + w_dev + w_mount + 8 + 8 + 10 + 10)
    );

    // Rows
    for drive in drives {
        let size_str = format_size(drive.size_gb);
        let (flags, flag_colour) = format_flags(drive);
        let (status, status_colour) = format_status(drive);

        println!(
            "{BOLD}{:<w_name$}{RESET}  {CYAN}{:<w_dev$}{RESET}  {DIM}{:<w_mount$}{RESET}  \
             {:>8}  {flag_colour}{:<8}{RESET}  {status_colour}{}{RESET}",
            drive.name,
            drive.device_path,
            drive.mount_point,
            size_str,
            flags,
            status,
            w_name = w_name,
            w_dev = w_dev,
            w_mount = w_mount,
        );
    }
}

// ── Flash candidates ──────────────────────────────────────────────────────────

fn print_flash_candidates(drives: &[DriveInfo]) {
    let candidates: Vec<&DriveInfo> = drives
        .iter()
        .filter(|d| !d.is_system && !d.is_read_only)
        .collect();

    if candidates.is_empty() {
        println!("{YELLOW}⚠  No flashable drives found.{RESET}");
        println!("{DIM}Insert a USB drive or SD card and re-run this example.{RESET}");
        return;
    }

    println!("{BOLD}{GREEN}✓  Flash candidates{RESET}");
    println!();

    for drive in candidates {
        println!("  {GREEN}▶{RESET}  {BOLD}{}{RESET}", drive.name);
        println!("     Device : {CYAN}{}{RESET}", drive.device_path);
        println!("     Mount  : {DIM}{}{RESET}", drive.mount_point);
        println!("     Size   : {}", format_size(drive.size_gb));
        println!();
    }
}

// ── Legend ────────────────────────────────────────────────────────────────────

fn print_legend() {
    println!("{BOLD}Legend{RESET}");
    println!("  {GREEN}●  Flashable{RESET}   — safe to use as a write target");
    println!("  {YELLOW}●  System{RESET}      — mounted at a critical path (/, /boot, …)");
    println!("  {RED}●  Read-only{RESET}   — kernel marks the device as read-only (ro=1)");
    println!("  {DIM}●  Disabled{RESET}    — any combination of the above flags");
    println!();
    println!("{DIM}Flags key:  S = system  R = read-only  D = disabled{RESET}");
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format size in GiB or MiB depending on magnitude.
fn format_size(size_gb: f64) -> String {
    if size_gb >= 1.0 {
        format!("{:.1} GiB", size_gb)
    } else if size_gb > 0.0 {
        format!("{:.0} MiB", size_gb * 1024.0)
    } else {
        "  —".to_string()
    }
}

/// Build a short flags string from drive constraints.
///
/// Returns `(flags_str, colour_escape)`.
fn format_flags(drive: &DriveInfo) -> (String, &'static str) {
    let mut flags = String::new();
    if drive.is_system {
        flags.push('S');
    }
    if drive.is_read_only {
        flags.push('R');
    }
    if drive.disabled {
        flags.push('D');
    }
    if flags.is_empty() {
        flags.push_str("none");
        (flags, DIM)
    } else if drive.is_system {
        (flags, YELLOW)
    } else {
        (flags, RED)
    }
}

/// Return a human-readable status label and its colour.
fn format_status(drive: &DriveInfo) -> (String, &'static str) {
    if drive.is_system {
        ("⚠ System drive".to_string(), YELLOW)
    } else if drive.is_read_only {
        ("🔒 Read-only".to_string(), RED)
    } else if drive.disabled {
        ("✗ Disabled".to_string(), RED)
    } else {
        ("✓ Flashable".to_string(), GREEN)
    }
}

// ── Header chrome ─────────────────────────────────────────────────────────────

fn print_header() {
    println!();
    println!("{BOLD}{CYAN}┌──────────────────────────────────────────────────────┐{RESET}");
    println!(
        "{BOLD}{CYAN}│{RESET}  {BOLD}flashkraft-core  ·  Drive Detection Example{RESET}        \
         {BOLD}{CYAN}│{RESET}"
    );
    println!("{BOLD}{CYAN}└──────────────────────────────────────────────────────┘{RESET}");
    println!();
}
