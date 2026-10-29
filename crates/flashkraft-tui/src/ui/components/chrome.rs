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
    pal: &tui_nuances::Theme,
) {
    if area.height == 0 {
        return;
    }
    // Full header has padding rows; shorter terminals drop them progressively.
    let (line, rule) = match area.height {
        1 => (area, Rect { height: 0, ..area }),
        2 | 3 => {
            let [line, rule] =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
            (line, rule)
        }
        _ => {
            let [_, line, _, rule] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);
            (line, rule)
        }
    };

    let narrow = area.width < crate::ui::responsive::COMPACT_BELOW;
    let tiny = area.width < 45;
    let mut left = vec![
        Span::styled(
            " ⚡ Flash",
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Kraft ",
            Style::default()
                .fg(pal.foreground)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if !narrow {
        left.push(Span::styled(
            format!("v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(pal.muted),
        ));
    }
    left.push(Span::raw(" "));
    // Left brand/title gets what the right-aligned theme chip leaves over.
    let theme_w = theme_name.chars().count() + 3 + if tiny { 0 } else { 6 };
    let left_budget = (area.width as usize).saturating_sub(theme_w + 1);
    let brand_w: usize = left.iter().map(|s| s.content.chars().count()).sum();
    if brand_w + subtitle.chars().count() + 2 <= left_budget {
        left.push(pill(
            subtitle,
            pal.background.unwrap_or_default(),
            pal.accent,
        ));
    }
    let left_area = Rect {
        width: (left_budget as u16).min(line.width),
        ..line
    };
    frame.render_widget(Paragraph::new(Line::from(left)), left_area);

    let mut right = Vec::new();
    if !tiny {
        right.push(Span::styled("theme ", Style::default().fg(pal.muted)));
    }
    right.push(pill(theme_name, pal.foreground, pal.selection_bg));
    right.push(Span::raw(" "));
    frame.render_widget(
        Paragraph::new(Line::from(right)).alignment(Alignment::Right),
        line,
    );

    if rule.height == 0 {
        return;
    }
    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(pal.selection_bg)),
        rule,
    );
}

/// Key hints as `key` chips over a solid accent status bar.
pub(in crate::ui) fn render_footer(
    frame: &mut Frame,
    area: Rect,
    hints: &[(&str, &str)],
    pal: &tui_nuances::Theme,
) {
    if area.height == 0 {
        return;
    }
    // 3+ rows: spacer + hints + status bar; 2: hints + bar; 1: hints only.
    let (keys, bar) = match area.height {
        1 => (area, Rect { height: 0, ..area }),
        2 => {
            let [k, b] =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
            (k, b)
        }
        _ => {
            let [_, k, b] = Layout::vertical([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);
            (k, b)
        }
    };

    // Drop trailing hints that do not fit the width (key pill + desc + gap).
    let widths: Vec<usize> = hints
        .iter()
        .enumerate()
        .map(|(i, (k, d))| {
            let gap = if i > 0 { 2 } else { 0 };
            gap + k.chars().count() + 2 + 1 + d.chars().count()
        })
        .collect();
    let keep = crate::ui::responsive::fit_count(&widths, (area.width as usize).saturating_sub(2));
    let hints = &hints[..keep];

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(pill(*key, pal.foreground, pal.selection_bg));
        spans.push(Span::styled(
            format!(" {desc}"),
            Style::default().fg(pal.muted),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), keys);

    if bar.height == 0 {
        return;
    }
    let bar_style = Style::default()
        .fg(pal.background.unwrap_or_default())
        .bg(pal.accent);
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
    pal: &tui_nuances::Theme,
) {
    // Narrow terminals show only the active step's label.
    let compact = area.width < 56;
    let steps: &[(usize, &str)] = &[(1, "Select Image"), (2, "Select Drive"), (3, "Flash")];

    let mut spans: Vec<Span> = Vec::new();
    for (i, (num, label)) in steps.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ▸ ", Style::default().fg(pal.muted)));
        }
        let label = if compact && *num != active { "" } else { label };
        let span = if *num == active {
            pill(
                format!("{num} {label}"),
                pal.background.unwrap_or_default(),
                pal.accent,
            )
        } else if *num < active {
            Span::styled(
                format!(" ✓ {label} "),
                Style::default()
                    .fg(pal.success)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!(" {num} {label} "), Style::default().fg(pal.muted))
        };
        spans.push(span);
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

/// Split `area` into [header, breadcrumbs, body, footer].
///
/// Chrome shrinks with the terminal height so the body keeps usable room.
pub(in crate::ui) fn chrome_layout(area: Rect) -> [Rect; 4] {
    let (h, b, f) = match area.height {
        26.. => (4, 2, 3),
        16..=25 => (2, 1, 2),
        _ => (2, 0, 1),
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(h),
            Constraint::Length(b),
            Constraint::Min(0),
            Constraint::Length(f),
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
        let pal = tui_nuances::Theme::default();
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
        let pal = tui_nuances::Theme::default();
        let out = draw(50, 4, |f, a| render_header(f, a, "Flash", "Default", &pal));
        assert!(out.contains("FlashKraft"));
        assert!(!out.contains(&format!("v{}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn footer_lists_every_hint() {
        let pal = tui_nuances::Theme::default();
        let out = draw(100, 3, |f, a| {
            render_footer(f, a, &[("q", "quit"), ("Enter", "select")], &pal)
        });
        for needle in ["q", "quit", "Enter", "select"] {
            assert!(out.contains(needle), "missing {needle}: {out}");
        }
    }

    #[test]
    fn breadcrumbs_mark_done_active_and_pending_steps() {
        let pal = tui_nuances::Theme::default();
        let out = draw(80, 1, |f, a| render_breadcrumbs(f, a, 2, &pal));
        assert!(out.contains("✓ Select Image"), "{out}");
        assert!(out.contains("2 Select Drive"));
        assert!(out.contains("3 Flash"));
    }

    #[test]
    fn zero_height_areas_do_not_panic() {
        let pal = tui_nuances::Theme::default();
        draw(10, 1, |f, _| {
            let empty = Rect::new(0, 0, 10, 0);
            render_header(f, empty, "x", "y", &pal);
            render_footer(f, empty, &[("a", "b")], &pal);
        });
    }

    #[test]
    fn chrome_layout_always_fits_and_keeps_body_room() {
        for h in 0..120u16 {
            let [a, b, body, d] = chrome_layout(Rect::new(0, 0, 80, h));
            assert!(a.height + b.height + body.height + d.height <= h);
            if h >= 10 {
                assert!(body.height >= 5, "h={h} body={}", body.height);
            }
        }
    }

    #[test]
    fn header_footer_breadcrumbs_survive_every_small_size() {
        let pal = tui_nuances::Theme::default();
        for w in 0..90u16 {
            for h in 0..6u16 {
                let _ = draw(w, h, |f, a| {
                    render_header(f, a, "OS Image Writer", "Tokyo Night", &pal);
                    render_footer(f, a, &[("q", "quit"), ("Enter", "select")], &pal);
                    render_breadcrumbs(f, a, 2, &pal);
                });
            }
        }
    }
}
