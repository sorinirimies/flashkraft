//! Basic Usage Example — FlashKraft GUI
//!
//! Launches the full Iced desktop application demonstrating:
//! - OS image file selection (ISO / IMG / DMG / ZIP)
//! - Automatic USB / SD drive detection
//! - Real-time flash progress with MB/s speed display
//! - Write verification (SHA-256 read-back)
//! - 21 built-in themes
//!
//! # Run
//!
//! ```bash
//! cargo run -p flashkraft-gui --example basic_usage
//! ```

use flashkraft_gui::{FlashKraft, Message};
use iced::{Settings, Task};

fn main() -> iced::Result {
    println!("╔══════════════════════════════════════════╗");
    println!("║   FlashKraft GUI — Basic Usage Example   ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("Launching the FlashKraft desktop application.");
    println!();
    println!("Workflow:");
    println!("  1. Click  [Select Image]  — pick an .iso / .img file");
    println!("  2. Click  [Select Target] — choose a USB or SD drive");
    println!("  3. Click  [Flash!]        — write and verify the image");
    println!("  4. Click  [🎨]            — switch between 21 themes");
    println!();
    println!("Safety features:");
    println!("  • System drives are automatically flagged and blocked");
    println!("  • Read-only drives cannot be selected");
    println!("  • Image size vs drive size is checked before flashing");
    println!("  • SHA-256 verification runs after every write");
    println!();

    iced::application(
        || {
            let initial_state = FlashKraft::new();
            let load_drives = Task::perform(
                flashkraft_core::commands::load_drives(),
                Message::DrivesRefreshed,
            );
            (initial_state, load_drives)
        },
        FlashKraft::update,
        FlashKraft::view,
    )
    .title("FlashKraft — OS Image Writer")
    .subscription(FlashKraft::subscription)
    .theme(|state: &FlashKraft| state.theme.clone())
    .settings(Settings {
        fonts: vec![iced_fonts::BOOTSTRAP_FONT_BYTES.into()],
        ..Default::default()
    })
    .window(iced::window::Settings {
        size: iced::Size::new(1300.0, 700.0),
        resizable: false,
        decorations: true,
        ..Default::default()
    })
    .run()
}
