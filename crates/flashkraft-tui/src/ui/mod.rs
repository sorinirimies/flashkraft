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
}
