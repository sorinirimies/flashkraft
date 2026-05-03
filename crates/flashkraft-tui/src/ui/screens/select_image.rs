use super::super::*;

pub(in crate::ui) fn render_select_image(
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
            ("\u{2190}/\u{2192}", "Move cursor"),
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
        themed_block!(" \u{1f4c1}  Select OS Image ", pal.brand, pal.accent)
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
                s.push('\u{2502}');
            }
            s.push(c);
        }
        if app.image_cursor == chars.len() && is_editing {
            s.push('\u{2502}');
        }
        s
    };

    let input_para = Paragraph::new(Span::raw(display))
        .style(Style::default().fg(pal.fg))
        .block(themed_block!(mode_label, border_color, border_color));
    frame.render_widget(input_para, rows[2]);
}

pub(in crate::ui) fn render_browse_image(
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
            ("\u{2191}\u{2193}/j/k", "Navigate"),
            ("\u{2192}/l/Enter", "Open"),
            ("\u{2190}/h/Bksp", "Go up"),
            ("/", "Search"),
            ("s", "Sort"),
            (".", "Hidden"),
            ("n", "Mkdir"),
            ("N", "Touch"),
            ("r", "Rename"),
            ("Spc", "Mark"),
            ("y/x/p/d", "Copy/Cut/Paste/Del"),
            ("Ctrl+T", "Cycle theme"),
            ("Shift+T", "Theme panel"),
            ("Esc", "Back"),
        ],
        pal,
    );

    let theme = *app.current_explorer_theme();
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
