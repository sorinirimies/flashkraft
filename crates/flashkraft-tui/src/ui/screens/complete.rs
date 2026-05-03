use super::super::*;

pub(in crate::ui) fn render_complete(
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
            ("\u{2191}/\u{2193}", "Scroll contents"),
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
            "  \u{2713}  Flash completed successfully!",
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

pub(in crate::ui) fn render_usb_contents(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
) {
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
                let icon = if e.is_dir {
                    "\u{1f4c1}"
                } else {
                    file_icon(&e.name)
                };
                let size_str = if e.size_bytes > 0 {
                    format!("  {}", flashkraft_core::fmt_bytes(e.size_bytes))
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

    let list = List::new(items).block(themed_block!(
        format!(" \u{1f4cb}  USB Contents{scroll_info}"),
        pal.brand,
        pal.success
    ));

    frame.render_widget(list, area);
}

pub(in crate::ui) fn render_contents_piechart(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
) {
    let (slices, legend_lines) = build_filetype_piechart(&app.usb_contents);

    if slices.is_empty() {
        let placeholder = Paragraph::new(Span::styled(
            "No files found on drive",
            Style::default().fg(pal.dim),
        ))
        .alignment(Alignment::Center)
        .block(themed_block!(
            " \u{1f967}  Contents Breakdown ",
            pal.brand,
            pal.dim
        ));
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
        .block(themed_block!(
            " \u{1f967}  Contents Breakdown ",
            pal.brand,
            pal.success
        ));

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
        let cb = themed_checkbox!(
            format!("{:<18} \u{2014} {} file(s)", label, count),
            true,
            *color,
            pal,
            "\u{25a0} ",
            "\u{25a1} "
        );

        frame.render_widget(cb, cb_rows[i]);
    }
}
