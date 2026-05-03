use super::super::*;

// ── Shared chrome ─────────────────────────────────────────────────────────────

pub(in crate::ui) fn render_header(
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

pub(in crate::ui) fn render_footer(
    frame: &mut Frame,
    area: Rect,
    hints: &[(&str, &str)],
    pal: &TuiPalette,
) {
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

pub(in crate::ui) fn render_breadcrumbs(
    frame: &mut Frame,
    area: Rect,
    active: usize,
    pal: &TuiPalette,
) {
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
pub(in crate::ui) fn chrome_layout(area: Rect) -> [Rect; 4] {
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
