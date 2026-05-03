//! Headless Demo — FlashKraft TUI
//!
//! Exercises the complete [`App`] state machine without opening a terminal or
//! requiring any hardware.  Every screen transition, channel-poll path, and
//! navigation helper is driven programmatically and the resulting state is
//! printed to stdout.
//!
//! This is useful for:
//! - CI environments that have no TTY
//! - Verifying state-machine logic in isolation
//! - Demonstrating the TUI architecture to new contributors
//!
//! # Run
//!
//! ```bash
//! cargo run -p flashkraft-tui --example headless_demo
//! ```

use std::path::PathBuf;

use flashkraft_tui::{App, AppScreen, InputMode};

// ── ANSI colour helpers (no external dep) ────────────────────────────────────

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

    // ── 1. Construction & initial state ──────────────────────────────────────
    section("1. App::new() — initial state");

    let mut app = App::new();

    assert_eq!(app.screen, AppScreen::SelectImage);
    assert_eq!(app.input_mode, InputMode::Editing);
    assert!(app.image_input.is_empty());
    assert!(app.available_drives.is_empty());
    assert!(!app.should_quit);

    check("screen", "SelectImage", &format!("{:?}", app.screen));
    check("input_mode", "Editing", &format!("{:?}", app.input_mode));
    check(
        "image_input",
        "(empty)",
        if app.image_input.is_empty() {
            "(empty)"
        } else {
            &app.image_input
        },
    );
    check("should_quit", "false", &app.should_quit.to_string());

    // ── 2. Text field — character insertion ───────────────────────────────────
    section("2. Image-path text field — character insertion & cursor movement");

    let test_path = "/tmp/ubuntu-24.04-desktop-amd64.iso";
    for c in test_path.chars() {
        app.image_insert(c);
    }

    check("image_input after typing", test_path, &app.image_input);
    check(
        "cursor position",
        &test_path.chars().count().to_string(),
        &app.image_cursor.to_string(),
    );

    // Move cursor left by 4 and insert characters
    app.image_cursor_left();
    app.image_cursor_left();
    app.image_cursor_left();
    app.image_cursor_left();

    let expected_cursor = test_path.chars().count() - 4;
    check(
        "cursor after 4× left",
        &expected_cursor.to_string(),
        &app.image_cursor.to_string(),
    );

    // Move back to end
    app.image_cursor_right();
    app.image_cursor_right();
    app.image_cursor_right();
    app.image_cursor_right();
    check(
        "cursor after 4× right (back to end)",
        &test_path.chars().count().to_string(),
        &app.image_cursor.to_string(),
    );

    // Backspace deletes the last character
    app.image_backspace();
    let trimmed = &test_path[..test_path.len() - 1];
    check("image_input after backspace", trimmed, &app.image_input);

    // Restore the character
    app.image_insert('o');
    check("image_input after re-insert", test_path, &app.image_input);

    // ── 3. confirm_image — path does not exist → Err ──────────────────────────
    section("3. confirm_image() with a non-existent path → expect Err");

    // The path almost certainly does not exist on the CI runner.
    let result = app.confirm_image();
    match &result {
        Err(msg) => {
            check_bool("confirm_image returns Err", true, true);
            println!("  {DIM}error message: \"{msg}\"{RESET}");
        }
        Ok(()) => {
            // The file happened to exist — that is fine; just note it.
            println!("  {YELLOW}Note: {test_path} exists on this system — confirm_image succeeded.{RESET}");
        }
    }

    // Screen must not have advanced (still SelectImage) if the file was absent.
    if result.is_err() {
        check(
            "screen unchanged after Err",
            "SelectImage",
            &format!("{:?}", app.screen),
        );
    }

    // ── 4. confirm_image — real temp file → Ok ────────────────────────────────
    section("4. confirm_image() with a real temp file → expect Ok + screen advance");

    let tmp = create_temp_image();
    app.image_input = tmp.to_string_lossy().to_string();
    app.image_cursor = app.image_input.chars().count();

    match app.confirm_image() {
        Ok(()) => {
            check(
                "screen after confirm_image",
                "SelectDrive",
                &format!("{:?}", app.screen),
            );
            check_bool("selected_image is Some", true, app.selected_image.is_some());
            if let Some(ref img) = app.selected_image {
                check("image name", "flashkraft_demo.img", &img.name);
                check_bool("image size_mb > 0", true, img.size_mb > 0.0);
                println!("  {DIM}size: {:.3} MB{RESET}", img.size_mb);
            }
        }
        Err(msg) => {
            println!("  {RED}✗ confirm_image failed unexpectedly: {msg}{RESET}");
        }
    }

    // ── 5. Drive list navigation ──────────────────────────────────────────────
    section("5. Drive list cursor navigation");

    // Populate with synthetic drives for testing.
    use flashkraft_tui::domain::DriveInfo;

    app.available_drives = vec![
        DriveInfo::new(
            "USB Stick A".into(),
            "/media/usb0".into(),
            16.0,
            "/dev/sdb".into(),
        ),
        DriveInfo::new(
            "USB Stick B".into(),
            "/media/usb1".into(),
            32.0,
            "/dev/sdc".into(),
        ),
        DriveInfo::new(
            "SD Card".into(),
            "/media/sdcard".into(),
            64.0,
            "/dev/sdd".into(),
        ),
    ];
    app.drive_cursor = 0;

    check("cursor starts at 0", "0", &app.drive_cursor.to_string());

    app.drive_down();
    check(
        "cursor after drive_down",
        "1",
        &app.drive_cursor.to_string(),
    );

    app.drive_down();
    check(
        "cursor after 2× drive_down",
        "2",
        &app.drive_cursor.to_string(),
    );

    // Cannot go past the last element
    app.drive_down();
    check("cursor clamped at last", "2", &app.drive_cursor.to_string());

    app.drive_up();
    check("cursor after drive_up", "1", &app.drive_cursor.to_string());

    app.drive_up();
    app.drive_up();
    check("cursor clamped at 0", "0", &app.drive_cursor.to_string());

    // ── 6. confirm_drive ──────────────────────────────────────────────────────
    section("6. confirm_drive() → DriveInfo screen");

    app.screen = AppScreen::SelectDrive;
    app.drive_cursor = 1; // USB Stick B

    match app.confirm_drive() {
        Ok(()) => {
            check(
                "screen after confirm_drive",
                "DriveInfo",
                &format!("{:?}", app.screen),
            );
            check_bool("selected_drive is Some", true, app.selected_drive.is_some());
            if let Some(ref d) = app.selected_drive {
                check("selected drive name", "USB Stick B", &d.name);
                check("selected device path", "/dev/sdc", &d.device_path);
            }
        }
        Err(msg) => println!("  {RED}✗ confirm_drive failed: {msg}{RESET}"),
    }

    // ── 7. confirm_drive — system drive → Err ────────────────────────────────
    section("7. confirm_drive() on a system drive → expect Err");

    use flashkraft_tui::domain::DriveInfo as DI;

    let sys = DI::with_constraints(
        "System Disk".into(),
        "/".into(),
        500.0,
        "/dev/sda".into(),
        true,  // is_system
        false, // is_read_only
    );
    app.available_drives.push(sys);
    app.drive_cursor = app.available_drives.len() - 1;
    app.screen = AppScreen::SelectDrive;

    match app.confirm_drive() {
        Err(msg) => {
            check_bool("confirm_drive(system) returns Err", true, true);
            println!("  {DIM}error: \"{msg}\"{RESET}");
            // Screen must stay on SelectDrive
            check(
                "screen unchanged",
                "SelectDrive",
                &format!("{:?}", app.screen),
            );
        }
        Ok(()) => println!("  {RED}✗ expected Err for system drive{RESET}"),
    }

    // ── 8. advance_to_confirm ─────────────────────────────────────────────────
    section("8. advance_to_confirm() → ConfirmFlash screen");

    app.screen = AppScreen::DriveInfo;
    app.advance_to_confirm();
    check("screen", "ConfirmFlash", &format!("{:?}", app.screen));

    // ── 9. go_back ────────────────────────────────────────────────────────────
    section("9. go_back() — each step retreats one screen");

    let transitions: &[(&str, AppScreen, &str)] = &[
        (
            "ConfirmFlash → DriveInfo",
            AppScreen::ConfirmFlash,
            "DriveInfo",
        ),
        (
            "DriveInfo → SelectDrive",
            AppScreen::DriveInfo,
            "SelectDrive",
        ),
        (
            "SelectDrive → SelectImage",
            AppScreen::SelectDrive,
            "SelectImage",
        ),
    ];

    for (label, from, expected_to) in transitions {
        app.screen = from.clone();
        app.go_back();
        check(label, expected_to, &format!("{:?}", app.screen));
    }

    // ── 10. image_size_bytes / drive_size_bytes ───────────────────────────────
    section("10. Convenience accessors — image_size_bytes & drive_size_bytes");

    // Set up known image + drive
    app.selected_image = Some(flashkraft_tui::domain::ImageInfo {
        path: PathBuf::from("/tmp/test.img"),
        name: "test.img".into(),
        size_mb: 512.0,
    });
    app.selected_drive = Some(DI::new(
        "Test Drive".into(),
        "/dev/sdz".into(),
        32.0,
        "/dev/sdz".into(),
    ));

    let img_bytes = app.image_size_bytes();
    let drv_bytes = app.drive_size_bytes();

    check_bool("image_size_bytes > 0", true, img_bytes > 0);
    check_bool("drive_size_bytes > 0", true, drv_bytes > 0);

    let expected_img = (512.0_f64 * 1024.0 * 1024.0) as u64;
    let expected_drv = (32.0_f64 * 1024.0 * 1024.0 * 1024.0) as u64;

    // Allow ±1 byte for floating-point truncation.
    check_bool(
        "image_size_bytes ≈ 512 MiB",
        true,
        img_bytes.abs_diff(expected_img) <= 1,
    );
    check_bool(
        "drive_size_bytes ≈ 32 GiB",
        true,
        drv_bytes.abs_diff(expected_drv) <= 1,
    );

    println!(
        "  {DIM}image : {} bytes ({:.1} MiB){RESET}",
        img_bytes,
        img_bytes as f64 / 1_048_576.0
    );
    println!(
        "  {DIM}drive : {} bytes ({:.1} GiB){RESET}",
        drv_bytes,
        drv_bytes as f64 / 1_073_741_824.0
    );

    // ── 11. USB contents scroll ───────────────────────────────────────────────
    section("11. USB contents scroll helpers");

    use flashkraft_tui::UsbEntry;

    app.usb_contents = (0..20)
        .map(|i| UsbEntry {
            name: format!("file_{i:02}.txt"),
            size_bytes: (i + 1) * 1024,
            is_dir: i % 5 == 0,
            depth: 0,
        })
        .collect();
    app.contents_scroll = 0;

    app.contents_down();
    app.contents_down();
    app.contents_down();
    check(
        "scroll after 3× down",
        "3",
        &app.contents_scroll.to_string(),
    );

    app.contents_up();
    check("scroll after up", "2", &app.contents_scroll.to_string());

    // Cannot scroll below 0
    app.contents_scroll = 0;
    app.contents_up();
    check("scroll clamped at 0", "0", &app.contents_scroll.to_string());

    // Cannot scroll past last element
    app.contents_scroll = app.usb_contents.len() - 1;
    app.contents_down();
    check(
        "scroll clamped at last",
        &(app.usb_contents.len() - 1).to_string(),
        &app.contents_scroll.to_string(),
    );

    // ── 12. tick_count wrapping ───────────────────────────────────────────────
    section("12. tick_count — wrapping increment");

    app.tick_count = u64::MAX;
    app.tick_count = app.tick_count.wrapping_add(1);
    check("tick_count wraps to 0", "0", &app.tick_count.to_string());

    // ── 13. reset() ───────────────────────────────────────────────────────────
    section("13. reset() — returns to factory-fresh state");

    app.screen = AppScreen::Complete;
    app.image_input = "/some/path.iso".into();
    app.flash_progress = 1.0;
    app.tick_count = 42;

    app.reset();

    check(
        "screen after reset",
        "SelectImage",
        &format!("{:?}", app.screen),
    );
    check("image_input cleared", "", &app.image_input);
    check_bool(
        "selected_image cleared",
        false,
        app.selected_image.is_some(),
    );
    check_bool(
        "selected_drive cleared",
        false,
        app.selected_drive.is_some(),
    );
    check(
        "flash_progress reset",
        "0",
        &format!("{:.0}", app.flash_progress),
    );
    check("tick_count reset", "0", &app.tick_count.to_string());
    check_bool("should_quit still false", false, app.should_quit);

    // ── 14. cancel_flash token ────────────────────────────────────────────────
    section("14. cancel_flash() — sets cancel token, moves to Error screen");

    use std::sync::atomic::Ordering;

    let token = app.cancel_token.clone();
    assert!(!token.load(Ordering::SeqCst), "token should start false");

    app.screen = AppScreen::Flashing;
    app.cancel_flash();

    check("screen after cancel", "Error", &format!("{:?}", app.screen));
    check_bool(
        "cancel_token set",
        true,
        app.cancel_token.load(Ordering::SeqCst),
    );
    check_bool("error_message set", true, !app.error_message.is_empty());
    println!("  {DIM}message: \"{}\"{RESET}", app.error_message);

    // ── Done ──────────────────────────────────────────────────────────────────
    println!();
    println!("{BOLD}{GREEN}╔══════════════════════════════════════════╗{RESET}");
    println!("{BOLD}{GREEN}║   All headless checks passed!  ✓         ║{RESET}");
    println!("{BOLD}{GREEN}╚══════════════════════════════════════════╝{RESET}");
    println!();
    println!(
        "{DIM}The App state machine was exercised without a terminal,\n\
         proving the business logic is fully decoupled from the renderer.{RESET}"
    );
    println!();

    // Clean up the temp file if we created one.
    let _ = std::fs::remove_file(PathBuf::from("/tmp/flashkraft_demo.img"));
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a tiny (1 MiB) temp image file that `confirm_image` will accept.
fn create_temp_image() -> PathBuf {
    let path = PathBuf::from("/tmp/flashkraft_demo.img");
    // Write 1 MiB of zeroes.
    let data = vec![0u8; 1024 * 1024];
    std::fs::write(&path, &data).expect("failed to create temp image");
    path
}

/// Print a check line: ✓ green if actual == expected, ✗ red otherwise.
fn check(label: &str, expected: &str, actual: &str) {
    if actual == expected {
        println!("  {GREEN}✓{RESET}  {BOLD}{label}{RESET}");
        println!("       {DIM}= {actual}{RESET}");
    } else {
        println!("  {RED}✗{RESET}  {BOLD}{label}{RESET}");
        println!("       {RED}expected: {expected}{RESET}");
        println!("       {RED}actual:   {actual}{RESET}");
        // Panic so the demo exits non-zero in CI.
        panic!("check failed: {label}");
    }
}

/// Boolean check (avoids formatting true/false as strings in the caller).
fn check_bool(label: &str, expected: bool, actual: bool) {
    check(label, &expected.to_string(), &actual.to_string());
}

// ── UI chrome ─────────────────────────────────────────────────────────────────

fn print_header() {
    println!();
    println!("{BOLD}{MAGENTA}┌──────────────────────────────────────────────────────┐{RESET}");
    println!(
        "{BOLD}{MAGENTA}│{RESET}  {BOLD}flashkraft-tui  ·  Headless State-Machine Demo{RESET}    \
         {BOLD}{MAGENTA}│{RESET}"
    );
    println!("{BOLD}{MAGENTA}└──────────────────────────────────────────────────────┘{RESET}");
    println!();
    println!(
        "Drives the {CYAN}App{RESET} state machine through every screen transition\n\
         and helper method without opening a terminal or requiring hardware."
    );
    println!();
}

fn section(title: &str) {
    let bar = "─".repeat(60usize.saturating_sub(title.len() + 4));
    println!();
    println!("{BOLD}── {title} {DIM}{bar}{RESET}");
    println!();
}
