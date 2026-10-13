use super::super::*;

// ── Shared chrome ─────────────────────────────────────────────────────────────

/// A filled pill: ` text ` on `bg` with `fg` text.
fn pill(text: impl Into<String>, fg: Color, bg: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", text.into()),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

/// Title bar — the same layout as the TokenBurn apps: brand, version, a chip
/// for the current screen, and the theme chip on the right, over a divider.
pub(in crate::ui) fn render_header(
    frame: &mut Frame,
    area: Rect,
    subtitle: &str,
    theme_name: &str,
    pal: &TuiPalette,
) {
    if area.height == 0 {
        return;
    }
    let [_, line, _, rule] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let narrow = area.width < 70;
    let mut left = vec![
        Span::styled(
            " ⚡ Flash",
            Style::default().fg(pal.brand).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Kraft ",
            Style::default().fg(pal.fg).add_modifier(Modifier::BOLD),
        ),
    ];
    if !narrow {
        left.push(Span::styled(
            format!("v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(pal.dim),
        ));
    }
    left.push(Span::raw(" "));
    left.push(pill(subtitle, pal.bg, pal.brand));
    frame.render_widget(Paragraph::new(Line::from(left)), line);

    let right = Line::from(vec![
        Span::styled("theme ", Style::default().fg(pal.dim)),
        pill(theme_name, pal.fg, pal.sel_bg),
        Span::raw(" "),
    ]);
    frame.render_widget(Paragraph::new(right).alignment(Alignment::Right), line);

    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(pal.sel_bg)),
        rule,
    );
}

/// Key hints as `key` chips over a solid accent status bar.
pub(in crate::ui) fn render_footer(
    frame: &mut Frame,
    area: Rect,
    hints: &[(&str, &str)],
    pal: &TuiPalette,
) {
    if area.height == 0 {
        return;
    }
    let [_, keys, bar] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(pill(*key, pal.fg, pal.sel_bg));
        spans.push(Span::styled(
            format!(" {desc}"),
            Style::default().fg(pal.dim),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), keys);

    let bar_style = Style::default().fg(pal.bg).bg(pal.brand);
    let demo = if flashkraft_core::commands::demo::is_demo() {
        "  demo mode — no real drives  "
    } else {
        ""
    };
    frame.render_widget(
        Paragraph::new(format!(" ⚡ FlashKraft{demo}"))
            .style(bar_style.add_modifier(Modifier::BOLD)),
        bar,
    );
    frame.render_widget(
        Paragraph::new(format!("{} ", env!("CARGO_PKG_NAME")))
            .alignment(Alignment::Right)
            .style(bar_style),
        bar,
    );
}

/// Step pills: the active step is filled, finished steps are green ticks.
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
            spans.push(Span::styled(" ▸ ", Style::default().fg(pal.dim)));
        }
        let span = if *num == active {
            pill(format!("{num} {label}"), pal.bg, pal.brand)
        } else if *num < active {
            Span::styled(
                format!(" ✓ {label} "),
                Style::default()
                    .fg(pal.success)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!(" {num} {label} "), Style::default().fg(pal.dim))
        };
        spans.push(span);
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
            Constraint::Length(4), // header (1 padding + 1 title + 1 border)
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);
    [chunks[0], chunks[1], chunks[2], chunks[3]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn draw(w: u16, h: u16, f: impl FnOnce(&mut Frame, Rect)) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|fr| {
            let area = fr.area();
            f(fr, area)
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn header_shows_brand_subtitle_and_theme() {
        let pal = TuiPalette::default();
        let out = draw(100, 4, |f, a| {
            render_header(f, a, "OS Image Writer", "Tokyo Night", &pal)
        });
        assert!(out.contains("FlashKraft"), "{out}");
        assert!(out.contains("OS Image Writer"));
        assert!(out.contains("Tokyo Night"));
        assert!(out.contains(&format!("v{}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn narrow_header_drops_the_version_but_keeps_the_brand() {
        let pal = TuiPalette::default();
        let out = draw(50, 4, |f, a| render_header(f, a, "Flash", "Default", &pal));
        assert!(out.contains("FlashKraft"));
        assert!(!out.contains(&format!("v{}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn footer_lists_every_hint() {
        let pal = TuiPalette::default();
        let out = draw(100, 3, |f, a| {
            render_footer(f, a, &[("q", "quit"), ("Enter", "select")], &pal)
        });
        for needle in ["q", "quit", "Enter", "select"] {
            assert!(out.contains(needle), "missing {needle}: {out}");
        }
    }

    #[test]
    fn breadcrumbs_mark_done_active_and_pending_steps() {
        let pal = TuiPalette::default();
        let out = draw(80, 1, |f, a| render_breadcrumbs(f, a, 2, &pal));
        assert!(out.contains("✓ Select Image"), "{out}");
        assert!(out.contains("2 Select Drive"));
        assert!(out.contains("3 Flash"));
    }

    #[test]
    fn zero_height_areas_do_not_panic() {
        let pal = TuiPalette::default();
        draw(10, 1, |f, _| {
            let empty = Rect::new(0, 0, 10, 0);
            render_header(f, empty, "x", "y", &pal);
            render_footer(f, empty, &[("a", "b")], &pal);
        });
    }
}
