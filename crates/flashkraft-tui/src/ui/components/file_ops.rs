use super::super::*;

pub(in crate::ui) fn render_file_op_modal(
    frame: &mut Frame,
    title: &str,
    body: &str,
    area: Rect,
    pal: &TuiPalette,
) {
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
    let popup = Paragraph::new(lines).block(themed_block!(title, pal.brand, pal.brand));
    frame.render_widget(popup, modal);
}

pub(in crate::ui) fn render_file_op_status(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
) {
    use crate::core::message::ClipOp;
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
