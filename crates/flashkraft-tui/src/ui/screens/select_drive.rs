use super::super::*;

pub(in crate::ui) fn render_select_drive(
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
            ("\u{2191}/\u{2193}", "Navigate"),
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
            " \u{27f3}  Scanning for drives\u{2026} ".to_string(),
            vec![ListItem::new(Line::from(Span::styled(
                "  Detecting USB drives\u{2026}",
                Style::default().fg(pal.dim),
            )))],
        )
    } else if drives.is_empty() {
        (
            " \u{1f4be}  No drives found ".to_string(),
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
                    "\u{1f512}"
                } else if d.is_read_only {
                    "\u{1f6ab}"
                } else {
                    "\u{1f4be}"
                };

                let label = format!(" {} {}  ({})", status_icon, d.name, size_str);

                // Build a one-line representation using Checkbox rendering logic.
                // We render it as text because ListItem needs Lines, not widgets.
                // The checkbox symbol gives the visual tick/untick state.
                let checked_sym = if is_selected_drive {
                    "\u{2611} "
                } else {
                    "\u{2610} "
                };
                let prefix = if selected { " \u{25b6} " } else { "   " };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(pal.accent)),
                    Span::styled(checked_sym, cb_style.add_modifier(Modifier::BOLD)),
                    Span::styled(label, cb_style),
                ]))
            })
            .collect();

        (format!(" \u{1f4be}  USB Drives ({}) ", drives.len()), items)
    };

    let mut list_state = ListState::default();
    if !drives.is_empty() {
        list_state.select(Some(app.drive_cursor));
    }

    let list = List::new(items)
        .block(themed_block!(title_text, pal.accent, pal.accent))
        .highlight_style(Style::default().fg(pal.brand).add_modifier(Modifier::BOLD));

    // When scanning, split the list area vertically to fit a bar spinner at the bottom.
    if app.drives_loading {
        let list_spinner_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(cols[0]);

        frame.render_stateful_widget(list, list_spinner_rows[0], &mut list_state);

        let scan_spinner = BarSpinner::new(app.tick_count)
            .arc_color(pal.brand)
            .dim_color(pal.dim);
        frame.render_widget(scan_spinner, list_spinner_rows[1]);
    } else {
        frame.render_stateful_widget(list, cols[0], &mut list_state);
    }

    // ── Drive detail panel ────────────────────────────────────────────────────
    let detail_lines: Vec<Line> = if let Some(d) = drives.get(app.drive_cursor) {
        let status_spans = if d.is_system {
            vec![Span::styled(
                "\u{26a0} System drive \u{2014} cannot flash",
                Style::default().fg(pal.err),
            )]
        } else if d.is_read_only {
            vec![Span::styled(
                "\u{26a0} Read-only \u{2014} cannot flash",
                Style::default().fg(pal.warn),
            )]
        } else {
            vec![Span::styled(
                "\u{2713} Available for flashing",
                Style::default().fg(pal.success),
            )]
        };

        let size_str = if d.size_gb >= 1.0 {
            format!("{:.2} GB", d.size_gb)
        } else {
            format!("{:.0} MB", d.size_gb * 1024.0)
        };

        vec![
            kv_line!("Name:    ", d.name.clone(), pal, bold pal.fg),
            Line::from(""),
            kv_line!("Device:  ", d.device_path.clone(), pal, pal.accent),
            kv_line!("Mount:   ", d.mount_point.clone(), pal, pal.dim),
            kv_line!("Size:    ", size_str, pal),
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
        .block(themed_block!(" Drive Details ", pal.brand, pal.dim).padding(Padding::uniform(1)))
        .wrap(Wrap { trim: true });

    frame.render_widget(detail, cols[1]);
}

pub(in crate::ui) fn render_drive_info(
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
        .block(themed_block!(
            " \u{1f967}  Drive Storage Layout ",
            pal.brand,
            pal.accent
        ));

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
        lines.push(kv_line!("File:   ", img.name.clone(), pal, bold pal.fg));
        lines.push(kv_line!("Size:   ", fmt_bytes(image_bytes), pal, pal.brand));
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
        lines.push(kv_line!("Name:   ", d.name.clone(), pal, bold pal.fg));
        lines.push(kv_line!("Device: ", d.device_path.clone(), pal, pal.accent));
        lines.push(kv_line!("Total:  ", fmt_bytes(drive_bytes), pal));
        lines.push(kv_line!(
            "Image:  ",
            format!("{} ({:.1}%)", fmt_bytes(image_bytes), image_pct),
            pal,
            pal.brand
        ));
        lines.push(kv_line!(
            "Free:   ",
            format!(
                "{} ({:.1}%)",
                fmt_bytes(drive_bytes.saturating_sub(image_bytes)),
                free_pct
            ),
            pal,
            pal.accent
        ));

        if image_bytes > drive_bytes && drive_bytes > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "\u{26a0} Image is larger than the drive!",
                Style::default().fg(pal.err).add_modifier(Modifier::BOLD),
            )));
        }
    }

    let detail = Paragraph::new(lines)
        .block(themed_block!(" Storage Info ", pal.brand, pal.dim).padding(Padding::uniform(1)))
        .wrap(Wrap { trim: true });

    frame.render_widget(detail, cols[1]);
}
