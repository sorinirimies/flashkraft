//! TUI UI Rendering
//!
//! All ratatui `Frame` rendering lives here. Each screen in [`AppScreen`]
//! has a dedicated `render_*` function called from the top-level [`render`]
//! entry point.
//!
//! Widget usage:
//! - [`tui_slider::Slider`]     — flash-progress bar (Flashing screen)
//! - [`tui_piechart::PieChart`] — drive-storage overview (DriveInfo screen)
//!   and file-type breakdown (Complete screen)
//! - [`tui_checkbox::Checkbox`] — drive-list items (SelectDrive screen)
//!   and confirmation checklist (ConfirmFlash screen)

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
    },
    Frame,
};

use tui_checkbox::Checkbox;
use tui_piechart::{LegendLayout, LegendPosition, PieChart, PieSlice};
use tui_slider::{Slider, SliderOrientation, SliderState};
use tui_spinner::{BarSpinner, FluxFrames, FluxSpinner, Spin as SpinDir};

use self::theme::TuiPalette;
use crate::core::message::{AppScreen, FileOpMode, InputMode};
use crate::core::state::App;
use tui_file_explorer::render_themed;

/// Build a [`Block`] with a bold styled title, rounded borders, and coloured border.
macro_rules! themed_block {
    ($title:expr, $title_color:expr, $border_color:expr) => {
        Block::default()
            .title(Span::styled(
                $title,
                Style::default()
                    .fg($title_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg($border_color))
    };
}

/// Build a key-value [`Line`] with a dim label and a styled value.
macro_rules! kv_line {
    ($label:expr, $value:expr, $pal:expr) => {
        Line::from(vec![
            Span::styled($label, Style::default().fg($pal.dim)),
            Span::styled($value, Style::default().fg($pal.fg)),
        ])
    };
    ($label:expr, $value:expr, $pal:expr, $color:expr) => {
        Line::from(vec![
            Span::styled($label, Style::default().fg($pal.dim)),
            Span::styled($value, Style::default().fg($color)),
        ])
    };
    ($label:expr, $value:expr, $pal:expr, bold $color:expr) => {
        Line::from(vec![
            Span::styled($label, Style::default().fg($pal.dim)),
            Span::styled(
                $value,
                Style::default().fg($color).add_modifier(Modifier::BOLD),
            ),
        ])
    };
}

/// Build a palette-styled [`Checkbox`].
macro_rules! themed_checkbox {
    ($label:expr, $checked:expr, $color:expr, $pal:expr) => {
        Checkbox::new($label, $checked)
            .checkbox_style(Style::default().fg($color).add_modifier(Modifier::BOLD))
            .label_style(Style::default().fg($pal.dim))
            .checked_symbol("☑ ")
            .unchecked_symbol("☐ ")
    };
    ($label:expr, $checked:expr, $color:expr, $pal:expr, $check_sym:expr, $uncheck_sym:expr) => {
        Checkbox::new($label, $checked)
            .checkbox_style(Style::default().fg($color).add_modifier(Modifier::BOLD))
            .label_style(Style::default().fg($pal.dim))
            .checked_symbol($check_sym)
            .unchecked_symbol($uncheck_sym)
    };
}

// ── Module declarations (AFTER macros so children can use them) ───────────────

mod components;
pub mod responsive;
mod screens;
pub mod theme;

use components::helpers::{build_filetype_piechart, centred_rect, file_icon};
use components::*;
use screens::*;

// ── Pie-chart slice palette (theme-independent) ───────────────────────────────
pub(in crate::ui) const SLICE_COLORS: &[Color] = &[
    Color::Rgb(80, 200, 255),
    Color::Rgb(255, 100, 30),
    Color::Rgb(80, 220, 120),
    Color::Rgb(255, 200, 50),
    Color::Rgb(200, 80, 255),
    Color::Rgb(255, 80, 130),
    Color::Rgb(80, 255, 200),
    Color::Rgb(255, 180, 80),
];

pub(in crate::ui) fn slice_color(i: usize) -> Color {
    SLICE_COLORS[i % SLICE_COLORS.len()]
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Top-level render function — called on every frame from the event loop.
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let pal = app.palette().clone();
    let theme_name = app.current_theme_name().to_string();
    frame.render_widget(Block::default().style(Style::default().bg(pal.bg)), area);

    if responsive::too_small(area) {
        render_too_small(frame, area, &pal);
        return;
    }

    match app.screen {
        AppScreen::SelectImage => render_select_image(app, frame, area, &pal, &theme_name),
        AppScreen::BrowseImage => render_browse_image(app, frame, area, &pal, &theme_name),
        AppScreen::SelectDrive => render_select_drive(app, frame, area, &pal, &theme_name),
        AppScreen::DriveInfo => render_drive_info(app, frame, area, &pal, &theme_name),
        AppScreen::ConfirmFlash => render_confirm_flash(app, frame, area, &pal, &theme_name),
        AppScreen::Flashing => render_flashing(app, frame, area, &pal, &theme_name),
        AppScreen::Complete => render_complete(app, frame, area, &pal, &theme_name),
        AppScreen::Error => render_error(app, frame, area, &pal, &theme_name),
    }

    // The global theme panel floats on top of any screen.
    if app.show_app_theme_panel {
        render_app_theme_panel(app, frame, area, &pal);
    }

    // The "update available" banner floats above everything, top row.
    if let Some(banner) = &app.update_banner {
        render_update_banner(banner, frame, area, &pal);
    }
}

/// Fallback shown when the terminal is smaller than the minimum usable size.
fn render_too_small(frame: &mut Frame, area: Rect, pal: &TuiPalette) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let msg = format!(
        "Terminal too small ({}x{}). Need {}x{}.",
        area.width,
        area.height,
        responsive::MIN_WIDTH,
        responsive::MIN_HEIGHT
    );
    frame.render_widget(
        Paragraph::new(msg)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center)
            .style(Style::default().fg(pal.warn).bg(pal.bg)),
        area,
    );
}

/// Render the dismissible "a new version is available" banner as a single
/// highlighted row across the very top of the terminal.
///
/// Auto-hides after `UPDATE_BANNER_DURATION` (checked every tick in
/// `App::poll_update_banner_expiry`) or immediately on Ctrl+U.
fn render_update_banner(
    banner: &crate::core::state::UpdateBanner,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
) {
    if area.height == 0 {
        return;
    }

    let banner_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };

    let text = format!(
        " \u{1F680} FlashKraft {} is available — you're on {}.  (Ctrl+U to dismiss) ",
        banner.latest_version,
        env!("CARGO_PKG_VERSION"),
    );

    frame.render_widget(Clear, banner_area);
    frame.render_widget(
        Paragraph::new(text).style(
            Style::default()
                .fg(pal.bg)
                .bg(pal.accent)
                .add_modifier(Modifier::BOLD),
        ),
        banner_area,
    );
}

#[cfg(test)]
mod responsive_tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    const SCREENS: [AppScreen; 8] = [
        AppScreen::SelectImage,
        AppScreen::BrowseImage,
        AppScreen::SelectDrive,
        AppScreen::DriveInfo,
        AppScreen::ConfirmFlash,
        AppScreen::Flashing,
        AppScreen::Complete,
        AppScreen::Error,
    ];

    const SIZES: [(u16, u16); 16] = [
        (0, 0),
        (1, 1),
        (10, 3),
        (29, 9),
        (30, 10),
        (30, 10),
        (40, 12),
        (60, 20),
        (69, 24),
        (79, 24),
        (80, 24),
        (99, 30),
        (100, 30),
        (140, 40),
        (250, 70),
        (300, 5),
    ];

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn every_screen_renders_at_every_size_without_panic() {
        for screen in SCREENS {
            for (w, h) in SIZES {
                for panel in [false, true] {
                    let mut app = App::new();
                    app.screen = screen.clone();
                    app.show_app_theme_panel = panel;
                    let _ = draw(&mut app, w, h);
                }
            }
        }
    }

    #[test]
    fn tiny_terminal_shows_too_small_message() {
        let mut app = App::new();
        let out = draw(&mut app, 29, 9);
        assert!(out.contains("too small"), "{out}");
    }

    #[test]
    fn usable_terminal_does_not_show_too_small_message() {
        let mut app = App::new();
        let out = draw(&mut app, 30, 10);
        assert!(!out.contains("too small"), "{out}");
    }

    #[test]
    fn footer_hints_are_dropped_not_cut_mid_word() {
        let mut app = App::new();
        app.screen = AppScreen::SelectDrive;
        let wide = draw(&mut app, 140, 30);
        assert!(wide.contains("Refresh"));
        let narrow = draw(&mut app, 40, 30);
        assert!(!narrow.contains("Refres "), "{narrow}");
    }

    #[test]
    fn resizing_between_frames_is_safe() {
        let mut app = App::new();
        app.screen = AppScreen::SelectDrive;
        for (w, h) in SIZES.iter().chain(SIZES.iter().rev()) {
            let _ = draw(&mut app, *w, *h);
        }
    }
}
