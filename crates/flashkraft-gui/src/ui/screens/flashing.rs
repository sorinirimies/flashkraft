//! Flashing Progress Screen
//!
//! Shows the animated progress bar, speed, ETA, and cancel button
//! while the flash (and optional verification) pipeline is running.

use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element};

use crate::core::message::Message;
use crate::core::FlashKraft;
use crate::utils::icons;
use iced_fonts::bootstrap;

/// Flashing progress view
pub fn view_flashing(state: &FlashKraft) -> Element<'_, Message> {
    if let Some(batch) = &state.batch {
        return view_batch(state, batch);
    }
    let progress = state.flash_progress.unwrap_or(0.0);
    let progress_percent = (progress * 100.0) as u32;
    let speed_mb_s = state.flash_speed_mb_s;
    let is_verifying = state.verify_progress.is_some();

    // ── Icon & headline ───────────────────────────────────────────────────────
    let (headline_icon, headline_text) = if is_verifying {
        (
            bootstrap::shield_fill_check(),
            format!(
                "Verifying… {}%",
                (state.verify_progress.unwrap_or(0.0) * 100.0) as u32
            ),
        )
    } else {
        (
            bootstrap::lightning_fill(),
            format!("Flashing… {}%", progress_percent),
        )
    };

    // ── Stage label ───────────────────────────────────────────────────────────
    let stage_label = if is_verifying {
        match state.verify_phase {
            "image" => "Hashing source image…".to_string(),
            "device" => "Reading back device…".to_string(),
            _ => "Verifying written data…".to_string(),
        }
    } else if state.flash_stage.is_empty() {
        "Starting…".to_string()
    } else {
        state.flash_stage.clone()
    };

    // Animated spinner glyph — cycles through tui-spinner frame presets
    let frames: &[char] = if is_verifying {
        tui_spinner::FluxFrames::CIRCLE_FILL
    } else {
        tui_spinner::FluxFrames::BRAILLE
    };
    let spinner_idx = (state.animation_time * 10.0) as usize % frames.len();
    let stage_label = format!("{} {}", frames[spinner_idx], stage_label);

    // ── Speed / ETA ───────────────────────────────────────────────────────────
    let (speed_text, eta_text) = if is_verifying {
        let spd = if state.verify_speed_mb_s > 0.0 {
            format!("{:.1} MB/s", state.verify_speed_mb_s)
        } else {
            "-- MB/s".to_string()
        };
        let v = state.verify_progress.unwrap_or(0.0);
        let pass_label = if state.verify_phase == "image" {
            let pct = (v * 200.0).clamp(0.0, 100.0) as u32;
            format!("Image hash: {}%", pct)
        } else {
            let pct = ((v - 0.5) * 200.0).clamp(0.0, 100.0) as u32;
            format!("Device read-back: {}%", pct)
        };
        (spd, pass_label)
    } else {
        let is_writing = progress < 0.80
            || state.flash_stage.is_empty()
            || state.flash_stage == "Writing image to device…"
            || state.flash_stage == "Unmounting partitions…";

        let spd = if speed_mb_s > 0.0 {
            format!("{:.1} MB/s", speed_mb_s)
        } else {
            "-- MB/s".to_string()
        };
        let eta = if is_writing && speed_mb_s > 0.0 && state.flash_bytes_written > 0 {
            let total_bytes = state
                .selected_image
                .as_ref()
                .map(|img| (img.size_mb * 1024.0 * 1024.0) as u64)
                .unwrap_or(0);
            let bytes_remaining = total_bytes.saturating_sub(state.flash_bytes_written);
            let speed_bytes_s = speed_mb_s * 1024.0 * 1024.0;
            let eta_seconds = (bytes_remaining as f32 / speed_bytes_s) as u64;
            format!("ETA: {}m{}s", eta_seconds / 60, eta_seconds % 60)
        } else if !is_writing && !state.flash_stage.is_empty() {
            state.flash_stage.clone()
        } else {
            "ETA: calculating...".to_string()
        };
        (spd, eta)
    };

    // ── Progress bar(s) ───────────────────────────────────────────────────────
    // Always show the main (themed) bar.
    // During verification also show the green verify bar underneath it.
    let main_bar = state
        .animated_progress
        .view::<Message>()
        .map(|_| Message::AnimationTick);

    let mut progress_content = column![
        icons::icon(headline_icon, 80.0),
        text(headline_text).size(32),
        text(stage_label).size(14),
        Space::new().height(20),
        main_bar,
    ]
    .spacing(10)
    .align_x(Alignment::Center)
    .padding(40);

    // Green verification bar — shown only during the verify stage.
    if is_verifying {
        let v_overall = state.verify_progress.unwrap_or(0.0);
        let image_pct = if state.verify_phase == "image" {
            (v_overall * 200.0).clamp(0.0, 100.0) as u32
        } else {
            100
        };
        let device_pct = if state.verify_phase == "device" {
            ((v_overall - 0.5) * 200.0).clamp(0.0, 100.0) as u32
        } else if state.verify_phase == "image" {
            0
        } else {
            100
        };

        progress_content = progress_content
            .push(Space::new().height(4))
            .push(
                state
                    .verify_animated_progress
                    .view::<Message>()
                    .map(|_| Message::AnimationTick),
            )
            .push(Space::new().height(6))
            .push(
                row![
                    text(format!("✓ Image hash {}%", image_pct)).size(13),
                    Space::new().width(24),
                    text(format!("✓ Device read-back {}%", device_pct)).size(13),
                ]
                .align_y(Alignment::Center),
            );
    }

    progress_content = progress_content
        .push(Space::new().height(15))
        .push(
            row![
                text(speed_text).size(16),
                Space::new().width(40),
                text(eta_text).size(16),
            ]
            .align_y(Alignment::Center),
        )
        .push(Space::new().height(20))
        .push(
            button(text("Cancel").size(14))
                .on_press(Message::CancelFlash)
                .padding(10),
        );

    super::status_page(state, progress_content.into())
}

/// Multi-device flash: a card per drive with its own bar.
fn view_batch<'a>(
    state: &'a FlashKraft,
    batch: &'a flashkraft_core::MultiState,
) -> Element<'a, Message> {
    use flashkraft_core::multi_flash::TargetStatus;
    use iced::widget::{container, progress_bar, scrollable};
    use iced::{Length, Theme};

    let headline = text(format!(
        "Flashing {} drives… {}%",
        batch.targets.len(),
        (batch.overall() * 100.0) as u32
    ))
    .size(28);
    let sub = text(format!(
        "{} running · {} done · {} failed · {:.1} MB/s total",
        batch.running(),
        batch.done(),
        batch.failed(),
        batch.total_speed_mb_s()
    ))
    .size(13);

    let rows: Vec<Element<'a, Message>> = batch
        .targets
        .iter()
        .map(|t| {
            let (icon, note) = match &t.status {
                TargetStatus::Queued => (bootstrap::hourglass_split(), "queued".to_string()),
                TargetStatus::Running => (
                    bootstrap::lightning_fill(),
                    format!("{:.1} MB/s  {}", t.speed_mb_s, t.message),
                ),
                TargetStatus::Done => (bootstrap::check_circle_fill(), "verified".to_string()),
                TargetStatus::Failed(e) => (bootstrap::x_circle_fill(), e.clone()),
            };
            let failed = matches!(t.status, TargetStatus::Failed(_));
            let done = t.status == TargetStatus::Done;
            container(
                column![
                    row![
                        icons::icon(icon, 18.0),
                        text(&t.device).size(15),
                        Space::new().width(Length::Fill),
                        text(format!("{}%", (t.overall() * 100.0) as u32)).size(13),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    progress_bar(0.0..=1.0, t.overall())
                        .girth(8)
                        .style(move |theme: &Theme| {
                            let mut s = progress_bar::primary(theme);
                            let p = theme.extended_palette();
                            if failed {
                                s.bar = p.danger.base.color.into();
                            } else if done {
                                s.bar = p.success.base.color.into();
                            }
                            s
                        }),
                    text(note).size(11),
                ]
                .spacing(6),
            )
            .padding(14)
            .width(Length::Fill)
            .style(move |theme: &Theme| crate::ui::theme::card(theme, done))
            .into()
        })
        .collect();

    let cancel = button(text("Cancel all").size(14))
        .on_press(Message::CancelFlash)
        .padding(10)
        .style(crate::ui::theme::ghost_button);

    let content = column![
        headline,
        sub,
        Space::new().height(10),
        scrollable(column(rows).spacing(10)).height(Length::Fill),
        cancel,
    ]
    .spacing(8)
    .align_x(Alignment::Center)
    .padding(30)
    .max_width(760);

    super::status_page(state, content.into())
}
