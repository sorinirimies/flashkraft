use super::super::*;

pub(in crate::ui) fn render_flashing(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    pal: &TuiPalette,
    theme_name: &str,
) {
    let [hdr, _bc, body, ftr] = chrome_layout(area);

    render_header(frame, hdr, "Flashing…", theme_name, pal);
    let log_hint = if app.log_focused {
        "↑/↓ j/k scroll • PgUp/PgDn page • Home/End jump • Tab unfocus"
    } else {
        "Tab focus Log to scroll"
    };
    render_footer(
        frame,
        ftr,
        &[("C / Esc", "Cancel flash"), ("Tab", log_hint)],
        pal,
    );

    let is_verifying = app.verify_progress.is_some();

    // Layout grows the Statistics/Log row (`Min`) to absorb any extra
    // vertical space when the terminal is resized taller, instead of that
    // space becoming blank padding.
    let rows = if is_verifying {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // stage label
                Constraint::Length(5), // overall progress slider
                Constraint::Length(7), // verify panel (two sub-bars)
                Constraint::Min(8),    // stats + log (grows with the window)
            ])
            .split(body)
    } else {
        // Pad with a dummy segment so indexing stays consistent below.
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // stage label
                Constraint::Length(5), // overall progress slider
                Constraint::Length(0), // (hidden verify panel)
                Constraint::Min(8),    // stats + log (grows with the window)
            ])
            .split(body)
    };

    // ── Stage label ───────────────────────────────────────────────────────────
    let stage_label = app.flash_stage.trim().to_string();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Stage: ", Style::default().fg(pal.dim)),
            Span::styled(
                stage_label,
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ),
        ]))
        .alignment(Alignment::Center),
        rows[0],
    );

    // ── Overall progress slider ───────────────────────────────────────────────
    let pct = app.flash_progress;

    let slider_state = SliderState::new((pct * 100.0) as f64, 0.0, 100.0);

    let slider_outer = themed_block!(" \u{26a1}  Flashing ", pal.brand, pal.accent)
        .title_alignment(Alignment::Center);

    let slider_inner = slider_outer.inner(rows[1]);
    frame.render_widget(slider_outer, rows[1]);

    let slider = Slider::from_state(&slider_state)
        .orientation(SliderOrientation::Horizontal)
        .show_value(true)
        .show_handle(false)
        .filled_symbol("\u{2501}")
        .empty_symbol("\u{2500}")
        .filled_color(pal.brand)
        .empty_color(pal.dim);

    frame.render_widget(slider, slider_inner);

    // ── Verification panel ────────────────────────────────────────────────────
    // Shown only while the verify stage is active. Contains two sub-bars:
    // one for the image-hash pass and one for the device read-back pass.
    if is_verifying && rows[2].height > 0 {
        let v_overall = app.verify_progress.unwrap_or(0.0);

        // image pass: overall 0.0–0.5 maps to 0–100 %
        let image_pct: f64 = if app.verify_phase == "image" {
            (v_overall * 2.0).clamp(0.0, 1.0) as f64 * 100.0
        } else {
            // image pass finished
            100.0
        };

        // device pass: overall 0.5–1.0 maps to 0–100 %
        let device_pct: f64 = if app.verify_phase == "device" {
            ((v_overall - 0.5) * 2.0).clamp(0.0, 1.0) as f64 * 100.0
        } else if app.verify_phase == "image" {
            0.0
        } else {
            100.0
        };

        let verify_speed_label = if app.verify_speed > 0.0 {
            format!(" {:.1} MB/s", app.verify_speed)
        } else {
            String::new()
        };

        let verify_outer = themed_block!(
            format!(" \u{1f50d}  Verifying{} ", verify_speed_label),
            pal.success,
            pal.success
        )
        .title_alignment(Alignment::Center);

        let verify_inner = verify_outer.inner(rows[2]);
        frame.render_widget(verify_outer, rows[2]);

        // Split inner area into two rows: image bar and device bar.
        let sub_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
            .split(verify_inner);

        // Image hash sub-bar
        let img_state = SliderState::new(image_pct, 0.0, 100.0);
        let img_label = format!(" Image hash  {image_pct:.1}% ");
        let img_block = Block::default()
            .title(Span::styled(img_label, Style::default().fg(pal.dim)))
            .borders(Borders::NONE);
        let img_inner = img_block.inner(sub_rows[0]);
        frame.render_widget(img_block, sub_rows[0]);
        frame.render_widget(
            Slider::from_state(&img_state)
                .orientation(SliderOrientation::Horizontal)
                .show_value(false)
                .show_handle(false)
                .filled_symbol("\u{2500}")
                .empty_symbol("\u{2500}")
                .filled_color(pal.success)
                .empty_color(pal.dim),
            img_inner,
        );

        // Device read-back sub-bar
        let dev_state = SliderState::new(device_pct, 0.0, 100.0);
        let dev_label = format!(" Device read  {device_pct:.1}% ");
        let dev_block = Block::default()
            .title(Span::styled(dev_label, Style::default().fg(pal.dim)))
            .borders(Borders::NONE);
        let dev_inner = dev_block.inner(sub_rows[1]);
        frame.render_widget(dev_block, sub_rows[1]);
        frame.render_widget(
            Slider::from_state(&dev_state)
                .orientation(SliderOrientation::Horizontal)
                .show_value(false)
                .show_handle(false)
                .filled_symbol("\u{2500}")
                .empty_symbol("\u{2500}")
                .filled_color(pal.accent)
                .empty_color(pal.dim),
            dev_inner,
        );
    }

    // ── Stats + log ───────────────────────────────────────────────────────────
    let stats_log_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(rows[3]);

    let fmt_bytes = |b: u64| -> String {
        if b >= 1_000_000_000 {
            format!("{:.2} GB", b as f64 / 1_000_000_000.0)
        } else {
            format!("{:.1} MB", b as f64 / 1_000_000.0)
        }
    };

    let total = app.image_size_bytes();

    // During verification, swap "Speed" to show the verify read speed.
    let speed_label = if is_verifying && app.verify_speed > 0.0 {
        format!("{:.1} MB/s", app.verify_speed)
    } else {
        format!("{:.1} MB/s", app.flash_speed)
    };

    let stats_lines = vec![
        kv_line!("Written:  ", fmt_bytes(app.flash_bytes), pal, bold pal.fg),
        kv_line!("Total:    ", fmt_bytes(total), pal, pal.dim),
        kv_line!("Speed:    ", speed_label, pal, bold pal.accent),
        kv_line!("Progress: ", format!("{:.1}%", pct * 100.0), pal, bold pal.brand),
    ];

    let stats = Paragraph::new(stats_lines)
        .block(themed_block!(" Statistics ", pal.accent, pal.dim).padding(Padding::horizontal(1)));
    frame.render_widget(stats, stats_log_cols[0]);

    // ── Log panel + Zed-style spinner ────────────────────────────────────────
    //
    // The log panel is split into two columns:
    //   • left  : the scrolling log text  (fills available width)
    //   • right : a 1-cell wide spinner column (bouncing braille dot)
    //
    // Uses tui-spinner's LinearSpinner (vertical bounce) for the Zed/Copilot look.

    // The log block — rendered first so we can measure the inner height.
    // Border brightens to `pal.brand` while focused so it's obvious Tab/arrow
    // keys are about to scroll this panel.
    let log_border_color = if app.log_focused { pal.brand } else { pal.dim };
    let log_title = match (app.log_focused, app.log_scroll) {
        (true, 0) => " Log (focused) ".to_string(),
        (true, n) => format!(" Log ↑{n} (focused) "),
        (false, 0) => " Log ".to_string(),
        (false, n) => format!(" Log ↑{n} "),
    };
    let log_block =
        themed_block!(log_title, pal.accent, log_border_color).padding(Padding::horizontal(1));

    let log_inner = log_block.inner(stats_log_cols[1]);

    // Split the inner area: log text on the left, 1-cell spinner on the right.
    let log_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(log_inner);

    let log_height = log_cols[0].height as usize;

    // Remember the rendered viewport height (used by key handling for
    // page-scroll sizing and clamping) and clamp any stale scroll offset
    // left over from a resize or from the log being cleared.
    app.log_view_height = log_height;
    let max_scroll = app.flash_log.len().saturating_sub(log_height);
    if app.log_scroll > max_scroll {
        app.log_scroll = max_scroll;
    }

    let log_lines: Vec<Line> = {
        let end = app.flash_log.len().saturating_sub(app.log_scroll);
        let start = end.saturating_sub(log_height);
        let mut lines: Vec<Line> = app.flash_log[start..end]
            .iter()
            .map(|l| {
                let style = if l.to_lowercase().contains("error") {
                    Style::default().fg(pal.err)
                } else if l.to_lowercase().contains("verif")
                    || l.to_lowercase().contains("complete")
                    || l.to_lowercase().contains("done")
                {
                    Style::default().fg(pal.success)
                } else if l.to_uppercase() == *l && !l.is_empty() {
                    Style::default().fg(pal.accent)
                } else {
                    Style::default().fg(pal.dim)
                };
                Line::from(Span::styled(l.as_str(), style))
            })
            .collect();
        // Pad top with blank lines so text stays pinned to the bottom.
        while lines.len() < log_height {
            lines.insert(0, Line::from(""));
        }
        lines
    };

    // Render the vertical flux spinner — single-glyph column cycling through
    // braille frames with a travelling-wave phase offset between rows.
    let spinner = FluxSpinner::new(app.tick_count)
        .frames(FluxFrames::BRAILLE)
        .width(1)
        .height(log_cols[1].height as usize)
        .phase_step(1)
        .spin(SpinDir::Clockwise)
        .color(pal.brand)
        .ticks_per_step(2);

    frame.render_widget(log_block, stats_log_cols[1]);
    frame.render_widget(Paragraph::new(log_lines), log_cols[0]);
    frame.render_widget(spinner, log_cols[1]);
}
