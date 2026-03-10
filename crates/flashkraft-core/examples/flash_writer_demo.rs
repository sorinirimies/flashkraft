//! Flash Pipeline Demo — flashkraft-core
//!
//! Demonstrates the in-process flash pipeline API introduced by the
//! architecture refactor.  It runs a simulated flash operation against a
//! temporary file pair (source image → target "device") and prints every
//! [`FlashEvent`] received over the `std::sync::mpsc` channel.
//!
//! No root privileges are required — the target is a regular temp file, so
//! the `seteuid(0)` path is never exercised.
//!
//! # Run
//!
//! ```bash
//! cargo run -p flashkraft-core --example flash_writer_demo
//! ```

use flashkraft_core::flash_helper::{run_pipeline, FlashEvent, FlashStage};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

// ── Colour helpers (ANSI, no external dep) ───────────────────────────────────

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";

fn main() {
    print_header();

    // ── Section 1: FlashStage Display round-trip ──────────────────────────────
    section("FlashStage::Display strings");

    let stages = [
        FlashStage::Starting,
        FlashStage::Unmounting,
        FlashStage::Writing,
        FlashStage::Syncing,
        FlashStage::Rereading,
        FlashStage::Verifying,
        FlashStage::Done,
        FlashStage::Failed("Simulated write error".to_string()),
    ];

    for stage in &stages {
        println!(
            "  {BOLD}{:<12}{RESET}  →  {CYAN}{}{RESET}",
            format!("{stage:?}").split('(').next().unwrap_or(""),
            stage
        );
    }

    // ── Section 2: Live pipeline run against temp files ───────────────────────
    section("Live run_pipeline — source image → temp target (no root needed)");

    // Write a 4 MiB source image so we get at least one progress event.
    let image_file = tempfile("fk_demo_image_", 4 * 1024 * 1024);
    let target_file = tempfile("fk_demo_target_", 4 * 1024 * 1024);

    println!("  {DIM}image  : {}{RESET}", image_file.display());
    println!("  {DIM}target : {}{RESET}", target_file.display());
    println!();

    let (tx, rx) = mpsc::channel::<FlashEvent>();
    let cancel = Arc::new(AtomicBool::new(false));

    let img = image_file.to_string_lossy().into_owned();
    let dev = target_file.to_string_lossy().into_owned();
    let cancel_clone = cancel.clone();

    let thread = std::thread::spawn(move || {
        run_pipeline(&img, &dev, tx, cancel_clone);
    });

    // Receive and pretty-print every event.
    let mut event_count = 0usize;
    let mut completed = false;

    for event in &rx {
        event_count += 1;
        print_event(&event);
        match event {
            FlashEvent::Done | FlashEvent::Error(_) => {
                completed = true;
                break;
            }
            _ => {}
        }
    }

    thread.join().expect("flash thread panicked");

    println!();
    if completed {
        println!("  {GREEN}{BOLD}✓ Pipeline finished ({event_count} events received){RESET}");
    } else {
        println!("  {YELLOW}⚠ Channel closed without Done/Error ({event_count} events){RESET}");
    }

    // ── Section 3: Cancellation demo ─────────────────────────────────────────
    section("Cancellation — cancel token set before pipeline writes anything");

    // Use a large-ish image so the pipeline doesn't finish before we cancel.
    let big_image = tempfile("fk_demo_big_", 16 * 1024 * 1024);
    let big_target = tempfile("fk_demo_big_target_", 16 * 1024 * 1024);

    let (tx2, rx2) = mpsc::channel::<FlashEvent>();
    let cancel2 = Arc::new(AtomicBool::new(false));
    let cancel2_clone = cancel2.clone();

    let img2 = big_image.to_string_lossy().into_owned();
    let dev2 = big_target.to_string_lossy().into_owned();

    // Set the cancel flag immediately so the pipeline aborts after the first
    // block at most.
    cancel2.store(true, Ordering::SeqCst);

    let thread2 = std::thread::spawn(move || {
        run_pipeline(&img2, &dev2, tx2, cancel2_clone);
    });

    let mut cancel_events = Vec::new();
    for event in &rx2 {
        cancel_events.push(event.clone());
        match event {
            FlashEvent::Done | FlashEvent::Error(_) => break,
            _ => {}
        }
    }
    thread2.join().expect("cancel thread panicked");

    for ev in &cancel_events {
        print_event(ev);
    }

    let cancelled = cancel_events
        .iter()
        .any(|e| matches!(e, FlashEvent::Error(msg) if msg.to_lowercase().contains("cancel")));

    println!();
    if cancelled {
        println!("  {GREEN}{BOLD}✓ Pipeline correctly reported cancellation{RESET}");
    } else {
        println!(
            "  {YELLOW}ℹ Pipeline finished without explicit cancel message \
             (may have completed before the flag was checked){RESET}"
        );
    }

    // ── Cleanup ───────────────────────────────────────────────────────────────
    let _ = std::fs::remove_file(&image_file);
    let _ = std::fs::remove_file(&target_file);
    let _ = std::fs::remove_file(&big_image);
    let _ = std::fs::remove_file(&big_target);

    println!();
    println!("{BOLD}Demo complete.{RESET}");
    println!(
        "{DIM}All events above were delivered via \
         flashkraft_core::flash_helper::run_pipeline over std::sync::mpsc.{RESET}"
    );
    println!();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a named temp file filled with `size` zero bytes and return its path.
fn tempfile(prefix: &str, size: usize) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "{prefix}{}.bin",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&path, vec![0u8; size]).expect("failed to create temp file");
    path
}

fn print_header() {
    println!();
    println!("{BOLD}{BLUE}╔══════════════════════════════════════════════════╗{RESET}");
    println!("{BOLD}{BLUE}║  FlashKraft — Flash Pipeline Demo                ║{RESET}");
    println!("{BOLD}{BLUE}╚══════════════════════════════════════════════════╝{RESET}");
    println!();
}

fn section(title: &str) {
    println!();
    println!("{BOLD}{MAGENTA}── {title} {RESET}");
    println!();
}

fn print_event(event: &FlashEvent) {
    match event {
        FlashEvent::Stage(stage) => {
            println!("  {CYAN}[STAGE   ]{RESET}  {}", stage);
        }
        FlashEvent::Progress {
            bytes_written,
            total_bytes,
            speed_mb_s,
        } => {
            let pct = if *total_bytes > 0 {
                (*bytes_written as f64 / *total_bytes as f64 * 100.0) as u32
            } else {
                0
            };
            let bar = progress_bar(pct);
            println!(
                "  {GREEN}[PROGRESS]{RESET}  {bar} {pct:>3}%  \
                 {bytes_written}/{total_bytes} bytes  \
                 @ {speed_mb_s:.1} MB/s"
            );
        }
        FlashEvent::VerifyProgress {
            phase,
            bytes_read,
            total_bytes,
            speed_mb_s,
        } => {
            let pct = if *total_bytes > 0 {
                (*bytes_read as f64 / *total_bytes as f64 * 100.0) as u32
            } else {
                0
            };
            let bar = progress_bar(pct);
            println!(
                "  {GREEN}[VERIFY  ]{RESET}  {bar} {pct:>3}%  \
                 [{phase}]  {bytes_read}/{total_bytes} bytes  \
                 @ {speed_mb_s:.1} MB/s"
            );
        }
        FlashEvent::Log(msg) => {
            println!("  {DIM}[LOG     ]{RESET}  {msg}");
        }
        FlashEvent::Done => {
            println!("  {GREEN}{BOLD}[DONE    ]{RESET}  Flash complete ✓");
        }
        FlashEvent::Error(msg) => {
            println!("  {RED}{BOLD}[ERROR   ]{RESET}  {msg}");
        }
    }
}

fn progress_bar(pct: u32) -> String {
    let filled = (pct as usize * 20 / 100).min(20);
    let empty = 20 - filled;
    format!(
        "[{GREEN}{}{RESET}{}]",
        "█".repeat(filled),
        "░".repeat(empty)
    )
}
