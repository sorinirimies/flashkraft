use super::super::*;

pub(in crate::ui) fn render_confirm_flash(
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

    // Centre a dialog box — use most of the available width so long image
    // names and drive descriptions never wrap.
    let dialog_w = body.width.saturating_sub(8).max(60);
    let dialog_h = 22u16.min(body.height.saturating_sub(4));
    let dialog = centred_rect(body, dialog_w, dialog_h);
    frame.render_widget(Clear, dialog);

    let image_name = app
        .selected_image
        .as_ref()
        .map(|i| i.name.as_str())
        .unwrap_or("\u{2014}");
    let drive_desc = app
        .selected_drive
        .as_ref()
        .map(|d| format!("{} ({})", d.name, d.device_path))
        .unwrap_or_else(|| "\u{2014}".to_string());
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
            Constraint::Length(7), // tui-checkbox confirmation area (3 rows + padding)
        ])
        .split(dialog);

    // Main warning text — image name and size on separate lines so long
    // filenames never cause wrapping.
    let text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  \u{26a0}   ALL DATA ON THE TARGET DRIVE WILL BE ERASED",
            Style::default().fg(pal.warn).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Image:   ", Style::default().fg(pal.dim)),
            Span::styled(
                image_name,
                Style::default().fg(pal.fg).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Size:    ", Style::default().fg(pal.dim)),
            Span::styled(image_size, Style::default().fg(pal.dim)),
        ]),
        Line::from(""),
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
                    " \u{26a1}  Ready to Flash ",
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
    // Three checkboxes stacked vertically — one per row — so the labels are
    // never truncated regardless of terminal width.
    let cb_area = dialog_rows[1];
    let cb_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top padding
            Constraint::Length(1), // checkbox 1
            Constraint::Length(1), // checkbox 2
            Constraint::Length(1), // checkbox 3
            Constraint::Min(0),    // bottom padding
        ])
        .split(cb_area);

    // Indent the checkboxes to align with the text above.
    let indent = |area: Rect| -> Rect {
        Rect {
            x: area.x + 2,
            width: area.width.saturating_sub(2),
            ..area
        }
    };

    let drive_ready = app
        .selected_drive
        .as_ref()
        .is_some_and(|d| !d.is_system && !d.is_read_only);

    let cb_image = themed_checkbox!(
        format!("Image ready: {image_name}"),
        app.selected_image.is_some(),
        pal.success,
        pal
    );

    let cb_drive = themed_checkbox!(
        format!(
            "Drive selected: {}",
            app.selected_drive
                .as_ref()
                .map(|d| d.device_path.as_str())
                .unwrap_or("\u{2014}")
        ),
        drive_ready,
        if drive_ready { pal.success } else { pal.err },
        pal
    );

    let cb_warn = themed_checkbox!("Data loss understood", true, pal.warn, pal);

    frame.render_widget(cb_image, indent(cb_rows[1]));
    frame.render_widget(cb_drive, indent(cb_rows[2]));
    frame.render_widget(cb_warn, indent(cb_rows[3]));
}
