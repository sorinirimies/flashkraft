//! Custom Theme Example — FlashKraft GUI
//!
//! Launches the FlashKraft desktop application with a pre-selected theme,
//! demonstrating the full theme system:
//!
//! - 21 built-in Iced themes (dark + light variants)
//! - Runtime theme switching via the 🎨 button
//! - Theme preference persisted across sessions via `redb`
//! - Every UI component — buttons, progress bars, panels — adapts instantly
//!
//! # Run
//!
//! ```bash
//! cargo run -p flashkraft-gui --example custom_theme
//! ```

use flashkraft_gui::{FlashKraft, Message};
use iced::{Settings, Task, Theme};

fn main() -> iced::Result {
    println!("╔══════════════════════════════════════════════╗");
    println!("║   FlashKraft GUI — Custom Theme Example      ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();
    println!("Launching with the Tokyo Night theme pre-selected.");
    println!();
    println!("Available dark themes:");
    println!("  • Dark (default)      • Dracula");
    println!("  • Nord                • Solarized Dark");
    println!("  • Gruvbox Dark        • Catppuccin Mocha");
    println!("  • Tokyo Night         • Kanagawa Wave");
    println!("  • Moonfly             • Nightfly");
    println!("  • Oxocarbon");
    println!();
    println!("Available light themes:");
    println!("  • Light               • Solarized Light");
    println!("  • Gruvbox Light       • Catppuccin Latte");
    println!("  • Tokyo Night Light   • Kanagawa Lotus");
    println!();
    println!("How to switch themes at runtime:");
    println!("  1. Click the [🎨] button in the top-right corner");
    println!("  2. Select any theme from the picker");
    println!("  3. The entire UI updates instantly");
    println!("  4. Your choice is saved and restored on next launch");
    println!();

    // Build initial state pre-configured with Tokyo Night.
    let mut initial_state = FlashKraft::new();
    initial_state.theme = Theme::TokyoNight;

    // Persist the pre-selected theme so it survives a restart.
    if let Some(storage) = &initial_state.storage {
        let _ = storage.save_theme(&Theme::TokyoNight);
    }

    iced::application(
        "FlashKraft — Custom Theme Demo",
        FlashKraft::update,
        FlashKraft::view,
    )
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
    .run_with(move || {
        let load_drives = Task::perform(
            flashkraft_core::commands::load_drives(),
            Message::DrivesRefreshed,
        );
        (initial_state, load_drives)
    })
}
