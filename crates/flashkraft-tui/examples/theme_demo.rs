//! Theme Switcher Demo — FlashKraft TUI
//!
//! Opens directly on the [`AppScreen::BrowseImage`] (file-explorer) screen so
//! the animated theme switcher can be showcased without going through the
//! normal wizard flow.
//!
//! Key bindings active on the BrowseImage screen:
//!
//! | Key | Action                          |
//! |-----|---------------------------------|
//! | `t` | Cycle to the next theme         |
//! | `[` | Cycle to the previous theme     |
//! | `T` | Toggle the theme-picker panel   |
//! | `↑`/`k` / `↓`/`j` | Navigate entries |
//! | `Esc` / `q` | Exit the demo          |
//!
//! # Run
//!
//! ```bash
//! cargo run -p flashkraft-tui --example theme_demo
//! # or via just:
//! just example-theme-demo
//! ```

use std::io;
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use flashkraft_tui::{handle_key, render, restore_terminal, App, AppScreen};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
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

        terminal.draw(|frame| render(&mut app, frame))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                handle_key(&mut app, key);

                // If the explorer dismissed us back to SelectImage, re-open it
                // so the demo stays focused on the theme switcher.
                if app.screen == AppScreen::SelectImage {
                    if app.should_quit {
                        break;
                    }
                    app.open_file_explorer();
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

    // Start on the first theme and open the file explorer immediately.
    app.explorer_theme_idx = 0;
    app.open_app_theme_panel();
    app.open_file_explorer();

    app
}
