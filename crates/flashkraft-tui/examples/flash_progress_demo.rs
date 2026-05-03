//! Flash Progress Demo — FlashKraft TUI
//!
//! Simulates the `Flashing` screen end-to-end with a synthetic progress stream
//! so the animated [`tui_slider`] progress bar can be showcased without
//! needing real hardware or a privileged flash helper.
//!
//! The demo:
//! 1. Pre-populates the [`App`] with a fake image and drive.
//! 2. Transitions directly to [`AppScreen::Flashing`].
//! 3. Spawns a Tokio task that sends [`FlashEvent`]s over ~8 seconds,
//!    mimicking the stages a real flash operation produces.
//! 4. Runs the full Ratatui event loop — the slider animates in real time.
//! 5. Transitions automatically to the [`AppScreen::Complete`] screen and
//!    quits after a short pause so the recording captures the end state.
//!
//! # Run
//!
//! ```bash
//! cargo run -p flashkraft-tui --example flash_progress_demo
//! # or via just:
//! just example-flash-progress
//! ```

use std::io;
use std::panic;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use tokio::time::sleep;

use flashkraft_tui::{
    domain::{DriveInfo, ImageInfo},
    render, restore_terminal, App, AppScreen, FlashEvent,
};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    // Restore terminal on panic before the message is printed.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_demo(&mut terminal).await;

    restore_terminal()?;
    terminal.show_cursor()?;
    result
}

// ---------------------------------------------------------------------------
// Demo loop
// ---------------------------------------------------------------------------

async fn run_demo<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut app = build_app();

    loop {
        app.tick_count = app.tick_count.wrapping_add(1);
        app.poll_flash();

        terminal.draw(|frame| render(&mut app, frame))?;

        // Auto-quit a moment after reaching the Complete or Error screen.
        if matches!(app.screen, AppScreen::Complete | AppScreen::Error) {
            sleep(Duration::from_millis(100)).await;
            app.tick_count = app.tick_count.wrapping_add(1);
            terminal.draw(|frame| render(&mut app, frame))?;
            sleep(Duration::from_secs(3)).await;
            break;
        }

        if app.should_quit {
            break;
        }

        // Allow Esc / q to abort early.
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                use crossterm::event::{KeyCode, KeyModifiers};
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// App bootstrap
// ---------------------------------------------------------------------------

fn build_app() -> App {
    let mut app = App::new();

    // ── Fake image ────────────────────────────────────────────────────────────
    app.selected_image = Some(ImageInfo {
        path: std::path::PathBuf::from("/tmp/ubuntu-24.04-desktop-amd64.iso"),
        name: "ubuntu-24.04-desktop-amd64.iso".to_string(),
        size_mb: 1_420.0,
    });

    // ── Fake drive ────────────────────────────────────────────────────────────
    app.selected_drive = Some(DriveInfo {
        name: "Samsung USB 3.1 Flash Drive".to_string(),
        mount_point: "/media/usb".to_string(),
        device_path: "/dev/sdb".to_string(),
        size_gb: 32.0,
        is_system: false,
        is_read_only: false,
        disabled: false,
        usb_info: None,
    });

    // ── Wire up the synthetic flash channel ───────────────────────────────────
    let (tx, rx) = mpsc::unbounded_channel::<FlashEvent>();
    app.flash_rx = Some(rx);
    app.cancel_token = Arc::new(AtomicBool::new(false));
    app.flash_stage = "Starting…".to_string();
    app.screen = AppScreen::Flashing;

    // Spawn the simulator — sends staged events over ~8 seconds.
    tokio::spawn(simulate_flash(tx));

    app
}

// ---------------------------------------------------------------------------
// Synthetic flash event stream
// ---------------------------------------------------------------------------

/// Emits the same sequence of [`FlashEvent`]s that the real flash helper
/// would produce, but driven by `tokio::time::sleep` instead of actual I/O.
async fn simulate_flash(tx: mpsc::UnboundedSender<FlashEvent>) {
    // Helper: send or silently drop if the receiver is gone.
    let send = |tx: &mpsc::UnboundedSender<FlashEvent>, ev: FlashEvent| {
        let _ = tx.send(ev);
    };

    // ── Stage 1: Preparing ────────────────────────────────────────────────────
    sleep(Duration::from_millis(400)).await;
    send(&tx, FlashEvent::Message("Preparing…".to_string()));
    send(
        &tx,
        FlashEvent::Message("Opened image: ubuntu-24.04-desktop-amd64.iso (1.4 GB)".to_string()),
    );
    send(
        &tx,
        FlashEvent::Message(
            "Target device: /dev/sdb (Samsung USB 3.1 Flash Drive, 32 GB)".to_string(),
        ),
    );
    sleep(Duration::from_millis(600)).await;

    // ── Stage 2: Writing ─────────────────────────────────────────────────────
    send(&tx, FlashEvent::Message("Writing…".to_string()));
    send(
        &tx,
        FlashEvent::Message("dd: writing to /dev/sdb …".to_string()),
    );

    // Simulate progress ticks — accelerate slightly through the middle.
    let total_bytes: u64 = 1_420 * 1_024 * 1_024; // 1 420 MB in bytes
    let steps = 80u32;
    for step in 1..=steps {
        let frac = step as f32 / steps as f32;

        // Vary speed: slow start, fast middle, slow finish.
        let speed_mb = if frac < 0.1 {
            80.0 + frac * 400.0
        } else if frac < 0.85 {
            120.0 + (frac - 0.1) * 80.0
        } else {
            100.0 - (frac - 0.85) * 200.0_f32.max(40.0)
        };

        let bytes_written = (total_bytes as f32 * frac) as u64;

        send(
            &tx,
            FlashEvent::Progress {
                progress: frac,
                bytes_written,
                speed_mb_s: speed_mb,
            },
        );

        // Occasional log lines to fill the log panel.
        match step {
            10 => send(
                &tx,
                FlashEvent::Message(format!(
                    "Written {:.0} MB — buffer ok",
                    bytes_written / 1_048_576
                )),
            ),
            25 => send(&tx, FlashEvent::Message("Write speed stable.".to_string())),
            40 => send(
                &tx,
                FlashEvent::Message(format!(
                    "{:.0} MB / {:.0} MB written",
                    bytes_written / 1_048_576,
                    total_bytes / 1_048_576
                )),
            ),
            55 => send(
                &tx,
                FlashEvent::Message("Halfway — no errors detected.".to_string()),
            ),
            70 => send(
                &tx,
                FlashEvent::Message(format!("Write speed: {speed_mb:.1} MB/s")),
            ),
            78 => send(
                &tx,
                FlashEvent::Message("Flushing kernel buffers…".to_string()),
            ),
            _ => {}
        }

        // Each step covers ~100 ms → total write phase ≈ 8 s.
        sleep(Duration::from_millis(100)).await;
    }

    // ── Stage 3: Verifying ────────────────────────────────────────────────────
    send(&tx, FlashEvent::Message("Verifying…".to_string()));
    send(
        &tx,
        FlashEvent::Message("Checksumming written data…".to_string()),
    );
    sleep(Duration::from_millis(800)).await;
    send(
        &tx,
        FlashEvent::Message("SHA-256 checksum: match ✓".to_string()),
    );
    sleep(Duration::from_millis(400)).await;

    // ── Done ──────────────────────────────────────────────────────────────────
    send(
        &tx,
        FlashEvent::Message("Flash complete — device is safe to remove.".to_string()),
    );
    send(&tx, FlashEvent::Completed);
}
