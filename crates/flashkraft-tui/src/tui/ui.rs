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

use super::app::{App, AppScreen, FileOpMode, InputMode, UsbEntry};
use super::theme::TuiPalette;
use tui_file_explorer::render_themed;

// ── Pie-chart slice palette (theme-independent) ───────────────────────────────
const SLICE_COLORS: &[Color] = &[
    Color::Rgb(80, 200, 255),
    Color::Rgb(255, 100, 30),
    Color::Rgb(80, 220, 120),
    Color::Rgb(255, 200, 50),
    Color::Rgb(200, 80, 255),
    Color::Rgb(255, 80, 130),
    Color::Rgb(80, 255, 200),
    Color::Rgb(255, 180, 80),
];

fn slice_color(i: usize) -> Color {
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

// ── Shared chrome ─────────────────────────────────────────────────────────────

fn render_header(
    frame: &mut Frame,
    area: Rect,
    subtitle: &str,
    theme_name: &str,
    pal: &TuiPalette,
) {
    // Split header into [left gap | centre title | right theme badge]
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area);

    // Centre: brand title + subtitle
    let title = Line::from(vec![
        Span::styled(
            "⚡ Flash",
            Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Kraft",
            Style::default().fg(pal.fg).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(subtitle, Style::default().fg(pal.dim)),
    ]);

    // Outer block with the bottom border spans the full width
    let border_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(pal.brand))
        .border_type(BorderType::Thick);
    frame.render_widget(border_block, area);

    // Centre title (no border — sits inside the outer block's visual row)
    frame.render_widget(Paragraph::new(title).alignment(Alignment::Center), cols[1]);

    // Right: theme badge — "🎨 <ThemeName>"
    let badge = Paragraph::new(Line::from(vec![
        Span::styled("🎨 ", Style::default()),
        Span::styled(
            theme_name,
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Right);
    frame.render_widget(badge, cols[2]);
}

fn render_footer(frame: &mut Frame, area: Rect, hints: &[(&str, &str)], pal: &TuiPalette) {
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default()));
        }
        spans.push(Span::styled(
            format!("[{key}]"),
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*desc, Style::default().fg(pal.dim)));
    }

    let para = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(pal.dim)),
        );

    frame.render_widget(para, area);
}

fn render_breadcrumbs(frame: &mut Frame, area: Rect, active: usize, pal: &TuiPalette) {
    let steps: &[(usize, &str)] = &[(1, "Select Image"), (2, "Select Drive"), (3, "Flash")];

    let mut spans: Vec<Span> = Vec::new();
    for (i, (num, label)) in steps.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ──  ", Style::default().fg(pal.dim)));
        }
        let is_active = *num == active;
        let style = if is_active {
            Style::default()
                .fg(pal.brand)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if *num < active {
            Style::default().fg(pal.success)
        } else {
            Style::default().fg(pal.dim)
        };
        let bullet = if *num < active {
            "✓".to_string()
        } else {
            num.to_string()
        };
        spans.push(Span::styled(format!("{bullet}. {label}"), style));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

/// Split `area` into [header, breadcrumbs, body, footer].
fn chrome_layout(area: Rect) -> [Rect; 4] {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);
    [chunks[0], chunks[1], chunks[2], chunks[3]]
}

// ── Screen: SelectImage ───────────────────────────────────────────────────────

fn render_browse_image(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
    theme_name: &str,
) {
    let [hdr, bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "OS Image Writer", theme_name, pal);
    render_breadcrumbs(frame, bc, 1, pal);
    render_footer(
        frame,
        ftr,
        &[
            ("\u{2191}/\u{2193}/j/k", "Navigate"),
            ("Enter", "Open / Select"),
            ("\u{2190}/Bksp", "Go up"),
            ("/", "Search"),
            ("y/x/p/d", "Copy/Cut/Paste/Del"),
            ("Ctrl+T", "Cycle theme"),
            ("Shift+T", "Theme panel"),
            ("Esc", "Back"),
        ],
        pal,
    );

    let theme = app.current_explorer_theme().clone();
    render_themed(&mut app.file_explorer, frame, body, &theme);

    match &app.file_op_mode {
        FileOpMode::ConfirmDelete(path) => {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            render_file_op_modal(
                frame,
                " \u{26a0}  Confirm Delete ",
                &format!("Delete \"{}\"?", name),
                area,
                pal,
            );
        }
        FileOpMode::ConfirmOverwrite { dst, .. } => {
            let name = dst
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            render_file_op_modal(
                frame,
                " \u{26a0}  Confirm Overwrite ",
                &format!("\"{}\" already exists. Overwrite?", name),
                area,
                pal,
            );
        }
        FileOpMode::Normal => {
            if !app.file_op_status.is_empty() || app.file_clipboard.is_some() {
                render_file_op_status(app, frame, body, pal);
            }
        }
    }
}

/// Overlay the global theme-picker panel on the right side of `area`.
///
/// The panel is drawn on top of whatever screen is currently active.
/// Navigation: ↑/↓ or j/k to move cursor, Enter to apply, Esc/T to close.
fn render_app_theme_panel(app: &App, frame: &mut Frame, area: Rect, pal: &TuiPalette) {
    // Panel width: wide enough for theme names + decorations.
    const PANEL_W: u16 = 36;
    let panel_w = PANEL_W.min(area.width);
    let panel_area = Rect {
        x: area.x + area.width.saturating_sub(panel_w),
        y: area.y,
        width: panel_w,
        height: area.height,
    };

    // Split into [list | current-name footer]
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(4)])
        .split(panel_area);

    // Scroll so the cursor row is always visible.
    let inner_h = split[0].height.saturating_sub(2) as usize; // subtract borders
                                                              // +2 accounts for the two header lines inside the list block
    let row = app.app_theme_panel_cursor + 2;
    let scroll_y = (row + 1).saturating_sub(inner_h) as u16;

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  ↑/k prev   ↓/j next",
            Style::default().fg(pal.dim),
        )),
        Line::from(vec![]),
    ];
    for (i, (name, _)) in app.explorer_themes.iter().enumerate() {
        let is_active = i == app.explorer_theme_idx;
        let is_cursor = i == app.app_theme_panel_cursor;

        let indicator = if is_cursor { " \u{25b6} " } else { "   " };
        let style = if is_cursor && is_active {
            Style::default()
                .fg(pal.brand)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else if is_cursor {
            Style::default()
                .fg(pal.accent)
                .add_modifier(Modifier::REVERSED)
        } else if is_active {
            Style::default().fg(pal.brand).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(pal.dim)
        };

        lines.push(Line::from(vec![
            Span::styled(indicator, style),
            Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(pal.accent)),
            Span::styled(name.clone(), style),
        ]));
    }

    frame.render_widget(Clear, split[0]);
    let panel = Paragraph::new(lines).scroll((scroll_y, 0)).block(
        Block::default()
            .title(Span::styled(
                " \u{1f3a8} Themes ",
                Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.accent)),
    );
    frame.render_widget(panel, split[0]);

    // Footer: shows active theme name + hint
    let active_name = &app.explorer_themes[app.explorer_theme_idx].0;
    let cursor_name = &app.explorer_themes[app.app_theme_panel_cursor].0;
    let footer_lines = vec![
        Line::from(Span::styled(
            format!("  \u{25cf} {cursor_name}"),
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  Active: ", Style::default().fg(pal.dim)),
            Span::styled(
                active_name.clone(),
                Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "  [Enter] apply  [T/Esc] close",
            Style::default().fg(pal.dim),
        )),
    ];
    frame.render_widget(Clear, split[1]);
    let footer = Paragraph::new(footer_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.dim)),
    );
    frame.render_widget(footer, split[1]);
}

fn render_file_op_modal(frame: &mut Frame, title: &str, body: &str, area: Rect, pal: &TuiPalette) {
    let w = 60u16;
    let h = 7u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let modal = Rect {
        x,
        y,
        width: w.min(area.width),
        height: h.min(area.height),
    };
    frame.render_widget(Clear, modal);
    let lines = vec![
        Line::from(vec![]),
        Line::from(Span::styled(
            format!("  {body}"),
            Style::default().fg(pal.fg),
        )),
        Line::from(vec![]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                " y ",
                Style::default()
                    .fg(pal.success)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            ),
            Span::styled(" Yes       ", Style::default().fg(pal.success)),
            Span::styled(
                " n / Esc ",
                Style::default()
                    .fg(pal.dim)
                    .add_modifier(Modifier::REVERSED),
            ),
            Span::styled(" No", Style::default().fg(pal.dim)),
        ]),
    ];
    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(
                title,
                Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.brand)),
    );
    frame.render_widget(popup, modal);
}

fn render_file_op_status(app: &App, frame: &mut Frame, area: Rect, pal: &TuiPalette) {
    use crate::tui::app::ClipOp;
    let text = if !app.file_op_status.is_empty() {
        app.file_op_status.clone()
    } else if let Some(clip) = &app.file_clipboard {
        let op = match clip.op {
            ClipOp::Copy => "Copy",
            ClipOp::Cut => "Cut",
        };
        format!(
            "{op}: {}",
            clip.path.file_name().unwrap_or_default().to_string_lossy()
        )
    } else {
        return;
    };
    let bar_w = (text.len() as u16 + 4).min(area.width);
    let bar = Rect {
        x: area.x + area.width.saturating_sub(bar_w),
        y: area.y + area.height.saturating_sub(4),
        width: bar_w,
        height: 3,
    };
    frame.render_widget(Clear, bar);
    let p = Paragraph::new(Span::styled(
        format!(" {text} "),
        Style::default().fg(pal.success),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.accent)),
    );
    frame.render_widget(p, bar);
}

fn render_select_image(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
    theme_name: &str,
) {
    let [hdr, bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "OS Image Writer", theme_name, pal);
    render_breadcrumbs(frame, bc, 1, pal);
    render_footer(
        frame,
        ftr,
        &[
            ("Enter", "Confirm path"),
            ("Tab", "Browse files"),
            ("Ctrl+T", "Cycle theme"),
            ("Shift+T", "Theme panel"),
            ("←/→", "Move cursor"),
            ("Ctrl-C", "Quit"),
        ],
        pal,
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(9),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(body);

    // Instruction panel
    let instr = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Enter the full path to an ", Style::default().fg(pal.dim)),
            Span::styled(
                ".iso / .img",
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " file to flash onto your USB drive.",
                Style::default().fg(pal.dim),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Example: ", Style::default().fg(pal.dim)),
            Span::styled(
                "/home/user/Downloads/ubuntu-24.04-desktop-amd64.iso",
                Style::default().fg(pal.dim),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(pal.dim)),
            Span::styled(
                "Tab",
                Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to open the interactive ", Style::default().fg(pal.dim)),
            Span::styled(
                "file browser",
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" instead.", Style::default().fg(pal.dim)),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title(Span::styled(
                " 📁  Select OS Image ",
                Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.accent))
            .padding(Padding::uniform(1)),
    );
    frame.render_widget(instr, rows[1]);

    // Text input field
    let is_editing = app.input_mode == InputMode::Editing;
    let border_color = if is_editing { pal.brand } else { pal.dim };
    let mode_label = if is_editing {
        " EDITING "
    } else {
        " PRESS i TO EDIT "
    };

    // Build display string with cursor marker
    let display: String = {
        let chars: Vec<char> = app.image_input.chars().collect();
        let mut s = String::new();
        for (i, &c) in chars.iter().enumerate() {
            if i == app.image_cursor && is_editing {
                s.push('│');
            }
            s.push(c);
        }
        if app.image_cursor == chars.len() && is_editing {
            s.push('│');
        }
        s
    };

    let input_para = Paragraph::new(Span::raw(display))
        .style(Style::default().fg(pal.fg))
        .block(
            Block::default()
                .title(Span::styled(
                    mode_label,
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        );
    frame.render_widget(input_para, rows[2]);
}

// ── Screen: SelectDrive ───────────────────────────────────────────────────────

fn render_select_drive(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
    theme_name: &str,
) {
    let [hdr, bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "OS Image Writer", theme_name, pal);
    render_breadcrumbs(frame, bc, 2, pal);
    render_footer(
        frame,
        ftr,
        &[
            ("↑/↓", "Navigate"),
            ("Enter / Space", "Select"),
            ("Ctrl+T", "Cycle theme"),
            ("Shift+T", "Theme panel"),
            ("R / F5", "Refresh"),
            ("B / Esc", "Back"),
        ],
        pal,
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(body);

    // ── Drive list — each entry rendered as a tui-checkbox ───────────────────
    let drives = &app.available_drives;

    let (title_text, items): (String, Vec<ListItem>) = if app.drives_loading {
        (
            " ⟳  Scanning for drives… ".to_string(),
            vec![ListItem::new(Line::from(Span::styled(
                "  Detecting USB drives…",
                Style::default().fg(pal.dim),
            )))],
        )
    } else if drives.is_empty() {
        (
            " 💾  No drives found ".to_string(),
            vec![ListItem::new(Line::from(Span::styled(
                "  No removable drives detected. Press [R] to refresh.",
                Style::default().fg(pal.warn),
            )))],
        )
    } else {
        let items: Vec<ListItem> = drives
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let selected = i == app.drive_cursor;
                let is_selected_drive = app.selected_drive.as_ref() == Some(d);

                // tui-checkbox: checked if this is the actively selected drive,
                // styled differently if it is the highlighted cursor row.
                let cb_style = if d.is_system || d.is_read_only {
                    Style::default().fg(pal.dim)
                } else if selected {
                    Style::default()
                        .fg(pal.brand)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default().fg(pal.fg)
                };

                let size_str = if d.size_gb >= 1.0 {
                    format!("{:.1} GB", d.size_gb)
                } else {
                    format!("{:.0} MB", d.size_gb * 1024.0)
                };

                let status_icon = if d.is_system {
                    "🔒"
                } else if d.is_read_only {
                    "🚫"
                } else {
                    "💾"
                };

                let label = format!(" {} {}  ({})", status_icon, d.name, size_str);

                // Build a one-line representation using Checkbox rendering logic.
                // We render it as text because ListItem needs Lines, not widgets.
                // The checkbox symbol gives the visual tick/untick state.
                let checked_sym = if is_selected_drive { "☑ " } else { "☐ " };
                let prefix = if selected { " ▶ " } else { "   " };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(pal.accent)),
                    Span::styled(checked_sym, cb_style.add_modifier(Modifier::BOLD)),
                    Span::styled(label, cb_style),
                ]))
            })
            .collect();

        (format!(" 💾  USB Drives ({}) ", drives.len()), items)
    };

    let mut list_state = ListState::default();
    if !drives.is_empty() {
        list_state.select(Some(app.drive_cursor));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    title_text,
                    Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(pal.accent)),
        )
        .highlight_style(Style::default().fg(pal.brand).add_modifier(Modifier::BOLD));

    frame.render_stateful_widget(list, cols[0], &mut list_state);

    // ── Drive detail panel ────────────────────────────────────────────────────
    let detail_lines: Vec<Line> = if let Some(d) = drives.get(app.drive_cursor) {
        let status_spans = if d.is_system {
            vec![Span::styled(
                "⚠ System drive — cannot flash",
                Style::default().fg(pal.err),
            )]
        } else if d.is_read_only {
            vec![Span::styled(
                "⚠ Read-only — cannot flash",
                Style::default().fg(pal.warn),
            )]
        } else {
            vec![Span::styled(
                "✓ Available for flashing",
                Style::default().fg(pal.success),
            )]
        };

        let size_str = if d.size_gb >= 1.0 {
            format!("{:.2} GB", d.size_gb)
        } else {
            format!("{:.0} MB", d.size_gb * 1024.0)
        };

        vec![
            Line::from(vec![
                Span::styled("Name:    ", Style::default().fg(pal.dim)),
                Span::styled(
                    d.name.clone(),
                    Style::default().fg(pal.fg).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Device:  ", Style::default().fg(pal.dim)),
                Span::styled(d.device_path.clone(), Style::default().fg(pal.accent)),
            ]),
            Line::from(vec![
                Span::styled("Mount:   ", Style::default().fg(pal.dim)),
                Span::styled(d.mount_point.clone(), Style::default().fg(pal.dim)),
            ]),
            Line::from(vec![
                Span::styled("Size:    ", Style::default().fg(pal.dim)),
                Span::styled(size_str, Style::default().fg(pal.fg)),
            ]),
            Line::from(""),
            Line::from(status_spans),
        ]
    } else {
        vec![Line::from(Span::styled(
            "No drive selected",
            Style::default().fg(pal.dim),
        ))]
    };

    let detail = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .title(Span::styled(
                    " Drive Details ",
                    Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(pal.dim))
                .padding(Padding::uniform(1)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(detail, cols[1]);
}

// ── Screen: DriveInfo ─────────────────────────────────────────────────────────

fn render_drive_info(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
    theme_name: &str,
) {
    let [hdr, bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "Drive Storage Overview", theme_name, pal);
    render_breadcrumbs(frame, bc, 2, pal);
    render_footer(
        frame,
        ftr,
        &[
            ("Enter / F", "Continue to confirm"),
            ("Ctrl+T", "Cycle theme"),
            ("Shift+T", "Theme panel"),
            ("B / Esc", "Back"),
        ],
        pal,
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body);

    // ── Left: tui-piechart — image vs free space ──────────────────────────────
    let drive_bytes = app.drive_size_bytes();
    let image_bytes = app.image_size_bytes();

    let (image_pct, free_pct) = if drive_bytes > 0 {
        let ip = (image_bytes as f64 / drive_bytes as f64 * 100.0).min(100.0);
        (ip, (100.0 - ip).max(0.0))
    } else {
        (0.0, 100.0)
    };

    let slices = vec![
        PieSlice::new("Image", image_pct, pal.brand),
        PieSlice::new("Free", free_pct, pal.accent),
    ];

    let pie = PieChart::new(slices)
        .show_legend(true)
        .show_percentages(true)
        .legend_position(LegendPosition::Right)
        .legend_layout(LegendLayout::Vertical)
        .high_resolution(true)
        .block(
            Block::default()
                .title(Span::styled(
                    " 🥧  Drive Storage Layout ",
                    Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(pal.accent)),
        );

    frame.render_widget(pie, cols[0]);

    // ── Right: numeric details ────────────────────────────────────────────────
    let fmt_bytes = |b: u64| -> String {
        if b >= 1_000_000_000 {
            format!("{:.2} GB", b as f64 / 1_000_000_000.0)
        } else if b >= 1_000_000 {
            format!("{:.1} MB", b as f64 / 1_000_000.0)
        } else {
            format!("{} KB", b / 1_000)
        }
    };

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Image Details",
            Style::default()
                .fg(pal.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        Line::from(""),
    ];

    if let Some(img) = &app.selected_image {
        lines.push(Line::from(vec![
            Span::styled("File:   ", Style::default().fg(pal.dim)),
            Span::styled(
                img.name.clone(),
                Style::default().fg(pal.fg).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Size:   ", Style::default().fg(pal.dim)),
            Span::styled(fmt_bytes(image_bytes), Style::default().fg(pal.brand)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Drive Details",
        Style::default()
            .fg(pal.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(Line::from(""));

    if let Some(d) = &app.selected_drive {
        lines.push(Line::from(vec![
            Span::styled("Name:   ", Style::default().fg(pal.dim)),
            Span::styled(
                d.name.clone(),
                Style::default().fg(pal.fg).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Device: ", Style::default().fg(pal.dim)),
            Span::styled(d.device_path.clone(), Style::default().fg(pal.accent)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Total:  ", Style::default().fg(pal.dim)),
            Span::styled(fmt_bytes(drive_bytes), Style::default().fg(pal.fg)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Image:  ", Style::default().fg(pal.dim)),
            Span::styled(
                format!("{} ({:.1}%)", fmt_bytes(image_bytes), image_pct),
                Style::default().fg(pal.brand),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Free:   ", Style::default().fg(pal.dim)),
            Span::styled(
                format!(
                    "{} ({:.1}%)",
                    fmt_bytes(drive_bytes.saturating_sub(image_bytes)),
                    free_pct
                ),
                Style::default().fg(pal.accent),
            ),
        ]));

        if image_bytes > drive_bytes && drive_bytes > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "⚠ Image is larger than the drive!",
                Style::default().fg(pal.err).add_modifier(Modifier::BOLD),
            )));
        }
    }

    let detail = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(
                    " Storage Info ",
                    Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(pal.dim))
                .padding(Padding::uniform(1)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(detail, cols[1]);
}

// ── Screen: ConfirmFlash ──────────────────────────────────────────────────────

fn render_confirm_flash(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
    theme_name: &str,
) {
    let [hdr, bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "Confirm Flash Operation", theme_name, pal);
    render_breadcrumbs(frame, bc, 3, pal);
    render_footer(
        frame,
        ftr,
        &[
            ("Y / Enter", "Flash now"),
            ("Ctrl+T", "Cycle theme"),
            ("Shift+T", "Theme panel"),
            ("N / Esc / B", "Go back"),
        ],
        pal,
    );

    // Centre a dialog box
    let dialog = centred_rect(body, 64, 18);
    frame.render_widget(Clear, dialog);

    let image_name = app
        .selected_image
        .as_ref()
        .map(|i| i.name.as_str())
        .unwrap_or("—");
    let drive_desc = app
        .selected_drive
        .as_ref()
        .map(|d| format!("{} ({})", d.name, d.device_path))
        .unwrap_or_else(|| "—".to_string());
    let image_size = app
        .selected_image
        .as_ref()
        .map(|i| format!("{:.2} MB", i.size_mb))
        .unwrap_or_default();

    // Split dialog into text area + checkbox confirmation area
    let dialog_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(5), // tui-checkbox confirmation area
        ])
        .split(dialog);

    // Main warning text
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  ⚠  ",
                Style::default().fg(pal.warn).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "ALL DATA ON THE TARGET DRIVE WILL BE ERASED",
                Style::default().fg(pal.warn).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Image:   ", Style::default().fg(pal.dim)),
            Span::styled(
                image_name,
                Style::default().fg(pal.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  ({})", image_size), Style::default().fg(pal.dim)),
        ]),
        Line::from(vec![
            Span::styled("  Target:  ", Style::default().fg(pal.dim)),
            Span::styled(
                drive_desc,
                Style::default().fg(pal.err).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(pal.dim)),
            Span::styled(
                "[Y / Enter]",
                Style::default()
                    .fg(pal.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to flash  or  ", Style::default().fg(pal.dim)),
            Span::styled(
                "[N / Esc]",
                Style::default().fg(pal.err).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to cancel.", Style::default().fg(pal.dim)),
        ]),
    ];

    let para = Paragraph::new(text)
        .block(
            Block::default()
                .title(Span::styled(
                    " ⚡  Ready to Flash ",
                    Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(pal.warn)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(para, dialog_rows[0]);

    // ── tui-checkbox confirmation checklist ───────────────────────────────────
    // Three visual checkboxes rendered in a horizontal row to confirm the
    // three key facts the user should have acknowledged.
    let cb_area = dialog_rows[1];
    let cb_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(cb_area);

    // We treat all three as "checked" (confirmed) since the user arrived here
    // by actively choosing image + drive; these are read-only acknowledgement
    // indicators, styled green to signal everything is configured.
    let cb_image = Checkbox::new(
        format!("Image ready: {image_name}"),
        app.selected_image.is_some(),
    )
    .checkbox_style(
        Style::default()
            .fg(pal.success)
            .add_modifier(Modifier::BOLD),
    )
    .label_style(Style::default().fg(pal.dim))
    .checked_symbol("☑ ")
    .unchecked_symbol("☐ ");

    let drive_ready = app
        .selected_drive
        .as_ref()
        .is_some_and(|d| !d.is_system && !d.is_read_only);

    let cb_drive = Checkbox::new(
        format!(
            "Drive selected: {}",
            app.selected_drive
                .as_ref()
                .map(|d| d.device_path.as_str())
                .unwrap_or("—")
        ),
        drive_ready,
    )
    .checkbox_style(
        Style::default()
            .fg(if drive_ready { pal.success } else { pal.err })
            .add_modifier(Modifier::BOLD),
    )
    .label_style(Style::default().fg(pal.dim))
    .checked_symbol("☑ ")
    .unchecked_symbol("☐ ");

    let cb_warn = Checkbox::new("Data loss understood", true)
        .checkbox_style(Style::default().fg(pal.warn).add_modifier(Modifier::BOLD))
        .label_style(Style::default().fg(pal.dim))
        .checked_symbol("☑ ")
        .unchecked_symbol("☐ ");

    frame.render_widget(cb_image, cb_cols[0]);
    frame.render_widget(cb_drive, cb_cols[1]);
    frame.render_widget(cb_warn, cb_cols[2]);
}

// ── Screen: Flashing ──────────────────────────────────────────────────────────

fn render_flashing(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
    theme_name: &str,
) {
    let [hdr, _bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "Flashing…", theme_name, pal);
    render_footer(frame, ftr, &[("C / Esc", "Cancel flash")], pal);

    let is_verifying = app.verify_progress.is_some();

    // When verifying we need an extra block for the verify panel.
    let rows = if is_verifying {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // stage label
                Constraint::Length(5), // overall progress slider
                Constraint::Length(7), // verify panel (two sub-bars)
                Constraint::Length(8), // stats + log
                Constraint::Min(0),
            ])
            .split(body)
    } else {
        // Pad with a dummy last segment so indexing stays consistent below.
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // stage label
                Constraint::Length(5), // overall progress slider
                Constraint::Length(0), // (hidden verify panel)
                Constraint::Length(8), // stats + log
                Constraint::Min(0),
            ])
            .split(body)
    };

    // ── Stage label ───────────────────────────────────────────────────────────
    let stage_label = app.flash_stage.trim().to_string();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Stage: ", Style::default().fg(pal.dim)),
            Span::styled(
                stage_label,
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ),
        ]))
        .alignment(Alignment::Center),
        rows[1],
    );

    // ── Overall progress slider ───────────────────────────────────────────────
    let pct = app.flash_progress;
    let pct_label = format!("{:.1}%", pct * 100.0);

    let slider_state = SliderState::new((pct * 100.0) as f64, 0.0, 100.0);

    let slider_outer = Block::default()
        .title(Span::styled(
            format!(" ⚡  Flashing  {pct_label} "),
            Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(pal.accent));

    let slider_inner = slider_outer.inner(rows[2]);
    frame.render_widget(slider_outer, rows[2]);

    let slider = Slider::from_state(&slider_state)
        .orientation(SliderOrientation::Horizontal)
        .show_value(true)
        .show_handle(false)
        .filled_symbol("━")
        .empty_symbol("─")
        .filled_color(pal.brand)
        .empty_color(pal.dim);

    frame.render_widget(slider, slider_inner);

    // ── Verification panel ────────────────────────────────────────────────────
    // Shown only while the verify stage is active. Contains two sub-bars:
    // one for the image-hash pass and one for the device read-back pass.
    if is_verifying && rows[3].height > 0 {
        let v_overall = app.verify_progress.unwrap_or(0.0);

        // image pass: overall 0.0–0.5 maps to 0–100 %
        let image_pct: f64 = if app.verify_phase == "image" {
            (v_overall * 2.0).clamp(0.0, 1.0) as f64 * 100.0
        } else {
            // image pass finished
            100.0
        };

        // device pass: overall 0.5–1.0 maps to 0–100 %
        let device_pct: f64 = if app.verify_phase == "device" {
            ((v_overall - 0.5) * 2.0).clamp(0.0, 1.0) as f64 * 100.0
        } else if app.verify_phase == "image" {
            0.0
        } else {
            100.0
        };

        let verify_speed_label = if app.verify_speed > 0.0 {
            format!(" {:.1} MB/s", app.verify_speed)
        } else {
            String::new()
        };

        let verify_outer = Block::default()
            .title(Span::styled(
                format!(
                    " 🔍  Verifying  {:.1}%{} ",
                    v_overall * 100.0,
                    verify_speed_label
                ),
                Style::default()
                    .fg(pal.success)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.success));

        let verify_inner = verify_outer.inner(rows[3]);
        frame.render_widget(verify_outer, rows[3]);

        // Split inner area into two rows: image bar and device bar.
        let sub_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
            .split(verify_inner);

        // Image hash sub-bar
        let img_state = SliderState::new(image_pct, 0.0, 100.0);
        let img_label = format!(" Image hash  {image_pct:.1}% ");
        let img_block = Block::default()
            .title(Span::styled(img_label, Style::default().fg(pal.dim)))
            .borders(Borders::NONE);
        let img_inner = img_block.inner(sub_rows[0]);
        frame.render_widget(img_block, sub_rows[0]);
        frame.render_widget(
            Slider::from_state(&img_state)
                .orientation(SliderOrientation::Horizontal)
                .show_value(false)
                .show_handle(false)
                .filled_symbol("─")
                .empty_symbol("─")
                .filled_color(pal.success)
                .empty_color(pal.dim),
            img_inner,
        );

        // Device read-back sub-bar
        let dev_state = SliderState::new(device_pct, 0.0, 100.0);
        let dev_label = format!(" Device read  {device_pct:.1}% ");
        let dev_block = Block::default()
            .title(Span::styled(dev_label, Style::default().fg(pal.dim)))
            .borders(Borders::NONE);
        let dev_inner = dev_block.inner(sub_rows[1]);
        frame.render_widget(dev_block, sub_rows[1]);
        frame.render_widget(
            Slider::from_state(&dev_state)
                .orientation(SliderOrientation::Horizontal)
                .show_value(false)
                .show_handle(false)
                .filled_symbol("─")
                .empty_symbol("─")
                .filled_color(pal.accent)
                .empty_color(pal.dim),
            dev_inner,
        );
    }

    // ── Stats + log ───────────────────────────────────────────────────────────
    let stats_log_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(rows[4]);

    let fmt_bytes = |b: u64| -> String {
        if b >= 1_000_000_000 {
            format!("{:.2} GB", b as f64 / 1_000_000_000.0)
        } else {
            format!("{:.1} MB", b as f64 / 1_000_000.0)
        }
    };

    let total = app.image_size_bytes();

    // During verification, swap "Speed" to show the verify read speed.
    let speed_label = if is_verifying && app.verify_speed > 0.0 {
        format!("{:.1} MB/s", app.verify_speed)
    } else {
        format!("{:.1} MB/s", app.flash_speed)
    };

    let stats_lines = vec![
        Line::from(vec![
            Span::styled("Written:  ", Style::default().fg(pal.dim)),
            Span::styled(
                fmt_bytes(app.flash_bytes),
                Style::default().fg(pal.fg).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total:    ", Style::default().fg(pal.dim)),
            Span::styled(fmt_bytes(total), Style::default().fg(pal.dim)),
        ]),
        Line::from(vec![
            Span::styled("Speed:    ", Style::default().fg(pal.dim)),
            Span::styled(
                speed_label,
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Progress: ", Style::default().fg(pal.dim)),
            Span::styled(
                pct_label,
                Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let stats = Paragraph::new(stats_lines).block(
        Block::default()
            .title(Span::styled(
                " Statistics ",
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.dim))
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(stats, stats_log_cols[0]);

    // Log panel — tail of the flash log
    let log_height = stats_log_cols[1].height.saturating_sub(2) as usize;
    let log_lines: Vec<Line> = app
        .flash_log
        .iter()
        .rev()
        .take(log_height)
        .rev()
        .map(|l| {
            let style = if l.to_lowercase().contains("error") {
                Style::default().fg(pal.err)
            } else if l.to_lowercase().contains("verification passed")
                || l.to_lowercase().contains("complete")
                || l.to_lowercase().contains("done")
            {
                Style::default().fg(pal.success)
            } else if l.to_lowercase().contains("verif") {
                Style::default().fg(pal.success)
            } else if l.to_uppercase() == *l && !l.is_empty() {
                Style::default().fg(pal.accent)
            } else {
                Style::default().fg(pal.dim)
            };
            Line::from(Span::styled(l.as_str(), style))
        })
        .collect();

    let log = Paragraph::new(log_lines).block(
        Block::default()
            .title(Span::styled(
                " Log ",
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.dim))
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(log, stats_log_cols[1]);
}

// ── Screen: Complete ──────────────────────────────────────────────────────────

fn render_complete(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
    theme_name: &str,
) {
    let [hdr, _bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "Flash Complete!", theme_name, pal);
    render_footer(
        frame,
        ftr,
        &[
            ("↑/↓", "Scroll contents"),
            ("Ctrl+T", "Cycle theme"),
            ("Shift+T", "Theme panel"),
            ("R", "Flash again"),
            ("Q / Esc", "Quit"),
        ],
        pal,
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(body);

    // ── Success banner ────────────────────────────────────────────────────────
    let drive_name = app
        .selected_drive
        .as_ref()
        .map(|d| format!("  Your USB drive ({}) is ready.", d.name))
        .unwrap_or_default();

    let banner = Paragraph::new(Line::from(vec![
        Span::styled(
            "  ✓  Flash completed successfully!",
            Style::default()
                .fg(pal.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(drive_name, Style::default().fg(pal.dim)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.success)),
    );
    frame.render_widget(banner, rows[0]);

    // ── Main split: USB tree (left) + piechart (right) ────────────────────────
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(rows[1]);

    render_usb_contents(app, frame, cols[0], pal);
    render_contents_piechart(app, frame, cols[1], pal);
}

fn render_usb_contents(app: &App, frame: &mut Frame, area: Rect, pal: &TuiPalette) {
    let inner_h = area.height.saturating_sub(2) as usize;
    let entries = &app.usb_contents;

    let items: Vec<ListItem> = if entries.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  (no contents to display)",
            Style::default().fg(pal.dim),
        )))]
    } else {
        entries
            .iter()
            .skip(app.contents_scroll)
            .take(inner_h)
            .map(|e| {
                let indent = "  ".repeat(e.depth);
                let icon = if e.is_dir { "📁" } else { file_icon(&e.name) };
                let size_str = if e.size_bytes > 0 {
                    format!("  {}", fmt_size(e.size_bytes))
                } else {
                    String::new()
                };
                let name_style = if e.is_dir {
                    Style::default().fg(pal.dir).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(pal.fg)
                };
                ListItem::new(Line::from(vec![
                    Span::raw(indent),
                    Span::raw(icon),
                    Span::raw(" "),
                    Span::styled(e.name.clone(), name_style),
                    Span::styled(size_str, Style::default().fg(pal.dim)),
                ]))
            })
            .collect()
    };

    let scroll_info = if entries.len() > inner_h {
        format!(
            " ({}/{}) ",
            app.contents_scroll.min(entries.len()),
            entries.len()
        )
    } else {
        String::new()
    };

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                format!(" 📋  USB Contents{scroll_info}"),
                Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(pal.success)),
    );

    frame.render_widget(list, area);
}

fn render_contents_piechart(app: &App, frame: &mut Frame, area: Rect, pal: &TuiPalette) {
    let (slices, legend_lines) = build_filetype_piechart(&app.usb_contents);

    if slices.is_empty() {
        let placeholder = Paragraph::new(Span::styled(
            "No files found on drive",
            Style::default().fg(pal.dim),
        ))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(Span::styled(
                    " 🥧  Contents Breakdown ",
                    Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(pal.dim)),
        );
        frame.render_widget(placeholder, area);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    // tui-piechart — file-type breakdown
    let pie = PieChart::new(slices)
        .show_legend(true)
        .show_percentages(true)
        .legend_position(LegendPosition::Right)
        .legend_layout(LegendLayout::Vertical)
        .high_resolution(true)
        .block(
            Block::default()
                .title(Span::styled(
                    " 🥧  Contents Breakdown ",
                    Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(pal.success)),
        );

    frame.render_widget(pie, rows[0]);

    // ── tui-checkbox legend — one checkbox per file-type category ────────────
    // Each checkbox is "checked" (it's a read-only legend indicator showing
    // which file types were found), styled in the slice's colour.
    let cb_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat_n(
                Constraint::Length(1),
                legend_lines.len().min(rows[1].height as usize),
            )
            .collect::<Vec<_>>(),
        )
        .split(rows[1]);

    for (i, (label, count, color)) in legend_lines.iter().enumerate() {
        if i >= cb_rows.len() {
            break;
        }
        let cb = Checkbox::new(format!("{:<18} — {} file(s)", label, count), true)
            .checkbox_style(Style::default().fg(*color).add_modifier(Modifier::BOLD))
            .label_style(Style::default().fg(pal.dim))
            .checked_symbol("■ ")
            .unchecked_symbol("□ ");

        frame.render_widget(cb, cb_rows[i]);
    }
}

// ── Screen: Error ─────────────────────────────────────────────────────────────

fn render_error(app: &mut App, frame: &mut Frame, area: Rect, pal: &TuiPalette, theme_name: &str) {
    let [hdr, _bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "Error", theme_name, pal);
    render_footer(
        frame,
        ftr,
        &[
            ("R / Enter", "Try again"),
            ("Ctrl+T", "Cycle theme"),
            ("Shift+T", "Theme panel"),
            ("Q / Esc", "Quit"),
        ],
        pal,
    );

    let dialog = centred_rect(body, 62, 10);
    frame.render_widget(Clear, dialog);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ✕  An error occurred:",
            Style::default().fg(pal.err).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", app.error_message),
            Style::default().fg(pal.fg),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(pal.dim)),
            Span::styled(
                "[R / Enter]",
                Style::default()
                    .fg(pal.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to start over  or  ", Style::default().fg(pal.dim)),
            Span::styled(
                "[Q / Esc]",
                Style::default().fg(pal.err).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to quit.", Style::default().fg(pal.dim)),
        ]),
    ];

    let para = Paragraph::new(text)
        .block(
            Block::default()
                .title(Span::styled(
                    " ✕  FlashKraft Error ",
                    Style::default().fg(pal.err).add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(pal.err)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(para, dialog);
}

// ── Layout helpers ────────────────────────────────────────────────────────────

/// Centre a `width × height` rect inside `r`.
fn centred_rect(r: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: r.x + r.width.saturating_sub(width) / 2,
        y: r.y + r.height.saturating_sub(height) / 2,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

// ── File-type classification ──────────────────────────────────────────────────

fn classify_ext(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "iso" | "img" | "bin" | "dmg" | "vhd" | "vmdk" => "Disk Images",
        "exe" | "msi" | "deb" | "rpm" | "apk" | "appimage" => "Executables",
        "sh" | "bat" | "cmd" | "ps1" | "py" | "rb" | "pl" => "Scripts",
        "txt" | "md" | "rst" | "log" | "cfg" | "conf" | "ini" | "toml" | "yaml" | "yml"
        | "json" | "xml" => "Text / Config",
        "jpg" | "jpeg" | "png" | "gif" | "svg" | "bmp" | "ico" | "webp" => "Images",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "Video",
        "mp3" | "flac" | "ogg" | "wav" | "aac" | "m4a" => "Audio",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" => "Archives",
        "efi" | "sys" | "ko" | "so" | "dll" | "o" | "a" | "lib" => "System / Libs",
        _ => "Other",
    }
}

fn file_icon(name: &str) -> &'static str {
    match classify_ext(name) {
        "Disk Images" => "💿",
        "Executables" => "⚙",
        "Scripts" => "📜",
        "Text / Config" => "📄",
        "Images" => "🖼",
        "Video" => "🎬",
        "Audio" => "🎵",
        "Archives" => "📦",
        "System / Libs" => "🔧",
        _ => "📄",
    }
}

/// Build `PieSlice`s and a legend from a list of USB entries.
///
/// Returns `(slices, legend)` where each legend entry is `(label, count, color)`.
fn build_filetype_piechart(
    entries: &[UsbEntry],
) -> (Vec<PieSlice<'_>>, Vec<(String, usize, Color)>) {
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for e in entries {
        if !e.is_dir {
            *counts.entry(classify_ext(&e.name)).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        return (vec![], vec![]);
    }

    let total: usize = counts.values().sum();
    let mut slices = Vec::new();
    let mut legend = Vec::new();

    for (i, (label, count)) in counts.iter().enumerate() {
        let pct = *count as f64 / total as f64 * 100.0;
        let color = slice_color(i);
        slices.push(PieSlice::new(label, pct, color));
        legend.push((label.to_string(), *count, color));
    }

    (slices, legend)
}

// ── Size formatting ───────────────────────────────────────────────────────────

fn fmt_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1}G", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1}M", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1}K", bytes as f64 / 1_024.0)
    } else {
        format!("{bytes}B")
    }
}
