use super::super::*;

pub(in crate::ui) fn render_error(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &tui_nuances::Theme,
    theme_name: &str,
) {
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
            "  \u{2715}  An error occurred:",
            Style::default().fg(pal.error).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", app.error_message),
            Style::default().fg(pal.foreground),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(pal.muted)),
            Span::styled(
                "[R / Enter]",
                Style::default()
                    .fg(pal.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to start over  or  ", Style::default().fg(pal.muted)),
            Span::styled(
                "[Q / Esc]",
                Style::default().fg(pal.error).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to quit.", Style::default().fg(pal.muted)),
        ]),
    ];

    let para = Paragraph::new(text)
        .block(
            Block::default()
                .title(Span::styled(
                    " \u{2715}  FlashKraft Error ",
                    Style::default().fg(pal.error).add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(pal.error)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(para, dialog);
}
