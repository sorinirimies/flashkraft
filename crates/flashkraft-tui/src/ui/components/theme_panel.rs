use super::super::*;

/// Overlay the global theme-picker panel on the right side of `area`.
///
/// The panel is drawn on top of whatever screen is currently active.
/// Navigation: ↑/↓ or j/k to move cursor, Enter to apply, Esc/T to close.
pub(in crate::ui) fn render_app_theme_panel(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
) {
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
            "  \u{2191}/k prev   \u{2193}/j next",
            Style::default().fg(pal.dim),
        )),
        Line::from(vec![]),
    ];
    for (i, (name, _)) in app.app_themes.iter().enumerate() {
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
        themed_block!(" \u{1f3a8} Themes ", pal.brand, pal.accent)
            .title_alignment(Alignment::Center),
    );
    frame.render_widget(panel, split[0]);

    // Footer: shows active theme name + hint
    let active_name = &app.app_themes[app.explorer_theme_idx].0;
    let cursor_name = &app.app_themes[app.app_theme_panel_cursor].0;
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
