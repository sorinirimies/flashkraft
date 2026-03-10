//! Status Views Component
//!
//! This module contains the various status views for the application:
//! - Flashing progress view
//! - Error view
//! - Flash complete view

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Border, Color, Element, Length, Theme};

use crate::components::theme_selector;
use crate::core::message::Message;
use crate::core::FlashKraft;
use crate::utils::icons_bootstrap_mapper as icons;
use iced_fonts::Bootstrap;

/// Flashing progress view
pub fn view_flashing(state: &FlashKraft) -> Element<'_, Message> {
    let progress = state.flash_progress.unwrap_or(0.0);
    let progress_percent = (progress * 100.0) as u32;
    let speed_mb_s = state.flash_speed_mb_s;
    let is_verifying = state.verify_progress.is_some();

    // ── Icon & headline ───────────────────────────────────────────────────────
    let (headline_icon, headline_text) = if is_verifying {
        (
            Bootstrap::ShieldFillCheck,
            format!(
                "Verifying… {}%",
                (state.verify_progress.unwrap_or(0.0) * 100.0) as u32
            ),
        )
    } else {
        (
            Bootstrap::LightningFill,
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
        Space::with_height(20),
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
            .push(Space::with_height(4))
            .push(
                state
                    .verify_animated_progress
                    .view::<Message>()
                    .map(|_| Message::AnimationTick),
            )
            .push(Space::with_height(6))
            .push(
                row![
                    text(format!("✓ Image hash {}%", image_pct)).size(13),
                    Space::with_width(24),
                    text(format!("✓ Device read-back {}%", device_pct)).size(13),
                ]
                .align_y(Alignment::Center),
            );
    }

    progress_content = progress_content
        .push(Space::with_height(15))
        .push(
            row![
                text(speed_text).size(16),
                Space::with_width(40),
                text(eta_text).size(16),
            ]
            .align_y(Alignment::Center),
        )
        .push(Space::with_height(20))
        .push(
            button(text("Cancel").size(14))
                .on_press(Message::CancelFlash)
                .padding(10),
        );

    let content = column![
        theme_selector::theme_selector_right(&state.theme),
        container(progress_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    ];

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Split a raw error string into a short headline and optional detail lines.
///
/// The first sentence (up to the first `\n`) becomes the headline.
/// Remaining lines become detail entries, with blank lines dropped.
fn split_error(error: &str) -> (&str, Vec<&str>) {
    let mut lines = error.splitn(2, '\n');
    let headline = lines.next().unwrap_or(error).trim();
    let detail: Vec<&str> = lines
        .next()
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    (headline, detail)
}

/// Return true when a detail line looks like a shell command (starts with `sudo`
/// or a path separator, or the word `Install`), so we can render it in the code card.
fn is_command_line(line: &str) -> bool {
    line.starts_with("sudo ") || line.starts_with('/')
}

/// Collapse runs of internal whitespace to a single space so that padding
/// used for source-code alignment (e.g. `u+s      /usr/bin/…`) is not
/// preserved verbatim in the rendered output.
fn normalise_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Error view
pub fn view_error<'a>(state: &'a crate::core::FlashKraft, error: &'a str) -> Element<'a, Message> {
    let (headline, detail_lines) = split_error(error);

    // ── Headline ──────────────────────────────────────────────────────────────
    let mut body: iced::widget::Column<'a, Message> = column![
        icons::icon(Bootstrap::ExclamationTriangleFill, 64.0),
        Space::with_height(4),
        text("Error").size(28),
        Space::with_height(12),
        container(text(headline).size(15)).max_width(520),
    ]
    .spacing(0)
    .align_x(Alignment::Center);

    // ── Detail / command card ─────────────────────────────────────────────────
    // Prose lines (explanatory text) go above the card; command lines go inside.
    let prose: Vec<&str> = detail_lines
        .iter()
        .copied()
        .filter(|l| !is_command_line(l))
        .collect();
    let commands: Vec<&str> = detail_lines
        .iter()
        .copied()
        .filter(|l| is_command_line(l))
        .collect();

    for line in &prose {
        body = body.push(Space::with_height(6));
        body = body.push(container(text(*line).size(13)).max_width(520));
    }

    if !commands.is_empty() {
        let mut card_col: iced::widget::Column<'a, Message> =
            column![].spacing(4).padding(14).align_x(Alignment::Start);
        for cmd in &commands {
            card_col = card_col.push(
                text(normalise_spaces(cmd))
                    .size(13)
                    .font(iced::Font::MONOSPACE),
            );
        }

        let card = container(card_col)
            .style(|theme: &Theme| {
                let base = theme.palette().background;
                let darkened = Color {
                    r: (base.r * 0.6).clamp(0.0, 1.0),
                    g: (base.g * 0.6).clamp(0.0, 1.0),
                    b: (base.b * 0.6).clamp(0.0, 1.0),
                    a: 1.0,
                };
                container::Style {
                    background: Some(iced::Background::Color(darkened)),
                    border: Border {
                        color: Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.15,
                        },
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }
            })
            .width(Length::Fixed(480.0));

        body = body.push(Space::with_height(16));
        body = body.push(container(card).width(Length::Fill).center_x(Length::Fill));
    }

    // ── Action buttons ────────────────────────────────────────────────────────
    body = body.push(Space::with_height(24));
    body = body.push(
        row![
            button(text("Go Back").size(14))
                .on_press(Message::CancelClicked)
                .padding([8, 20]),
            Space::with_width(12),
            button(text("Try Again").size(14))
                .on_press(Message::ResetClicked)
                .padding([8, 20]),
        ]
        .align_y(Alignment::Center),
    );

    let error_content = container(body.padding(40))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    let content = column![
        theme_selector::theme_selector_right(&state.theme),
        error_content,
    ];

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Flash complete view
pub fn view_complete(state: &crate::core::FlashKraft) -> Element<'_, Message> {
    let complete_content = column![
        icons::icon(Bootstrap::CheckCircleFill, 80.0),
        text("Flash Complete!").size(32),
        Space::with_height(20),
        text("Your device is ready to use").size(16),
        Space::with_height(20),
        button(text("Flash Another").size(14))
            .on_press(Message::ResetClicked)
            .padding(10),
    ]
    .spacing(10)
    .align_x(Alignment::Center)
    .padding(40);

    let content = column![
        theme_selector::theme_selector_right(&state.theme),
        container(complete_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    ];

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── split_error ──────────────────────────────────────────────────────────

    #[test]
    fn test_split_error_single_line() {
        let (headline, detail) = split_error("Something went wrong");
        assert_eq!(headline, "Something went wrong");
        assert!(detail.is_empty());
    }

    #[test]
    fn test_split_error_two_lines() {
        let (headline, detail) = split_error("Headline\nDetail line");
        assert_eq!(headline, "Headline");
        assert_eq!(detail, vec!["Detail line"]);
    }

    #[test]
    fn test_split_error_trims_headline_whitespace() {
        let (headline, _) = split_error("  Headline  \nDetail");
        assert_eq!(headline, "Headline");
    }

    #[test]
    fn test_split_error_drops_blank_detail_lines() {
        let (_, detail) = split_error("Headline\n\nLine A\n\nLine B\n");
        assert_eq!(detail, vec!["Line A", "Line B"]);
    }

    #[test]
    fn test_split_error_trims_detail_lines() {
        let (_, detail) = split_error("Headline\n   indented line   ");
        assert_eq!(detail, vec!["indented line"]);
    }

    /// Simulates the exact permission-denied message produced by open_device_for_writing.
    #[test]
    fn test_split_error_permission_denied_shape() {
        let msg = "Permission denied opening '/dev/sdb'.\n\
                   FlashKraft needs root access to write to block devices.\n\
                   Install setuid-root so it can escalate automatically:\n\
                   sudo chown root:root /usr/bin/flashkraft\n\
                   sudo chmod u+s /usr/bin/flashkraft";

        let (headline, detail) = split_error(msg);

        assert_eq!(headline, "Permission denied opening '/dev/sdb'.");
        assert_eq!(detail.len(), 4);
        assert_eq!(
            detail[0],
            "FlashKraft needs root access to write to block devices."
        );
        assert_eq!(
            detail[1],
            "Install setuid-root so it can escalate automatically:"
        );
        assert!(detail[2].contains("chown"));
        assert!(detail[3].contains("chmod"));
    }

    // ── is_command_line ──────────────────────────────────────────────────────

    #[test]
    fn test_is_command_line_sudo() {
        assert!(is_command_line("sudo chown root:root /usr/bin/flashkraft"));
        assert!(is_command_line("sudo chmod u+s /usr/bin/flashkraft"));
    }

    #[test]
    fn test_is_command_line_absolute_path() {
        assert!(is_command_line("/usr/bin/flashkraft"));
        assert!(is_command_line("/dev/sdb"));
    }

    #[test]
    fn test_is_command_line_prose_is_not_command() {
        assert!(!is_command_line("Install with:"));
        assert!(!is_command_line("The binary is not installed setuid-root."));
        assert!(!is_command_line("Permission denied opening the device."));
        assert!(!is_command_line(""));
    }

    #[test]
    fn test_normalise_spaces_collapses_internal_padding() {
        assert_eq!(
            normalise_spaces("sudo chmod u+s      /usr/bin/flashkraft"),
            "sudo chmod u+s /usr/bin/flashkraft"
        );
        assert_eq!(
            normalise_spaces("sudo chown root:root /usr/bin/flashkraft"),
            "sudo chown root:root /usr/bin/flashkraft"
        );
        assert_eq!(
            normalise_spaces("  leading and trailing  "),
            "leading and trailing"
        );
        assert_eq!(normalise_spaces("no  extra  spaces"), "no extra spaces");
    }

    // ── prose vs command partitioning ────────────────────────────────────────

    #[test]
    fn test_permission_denied_partitions_correctly() {
        let msg = "Permission denied opening '/dev/sdb'.\n\
                   FlashKraft needs root access to write to block devices.\n\
                   Install setuid-root so it can escalate automatically:\n\
                   sudo chown root:root /usr/bin/flashkraft\n\
                   sudo chmod u+s /usr/bin/flashkraft";

        let (_, detail) = split_error(msg);

        let prose: Vec<&str> = detail
            .iter()
            .copied()
            .filter(|l| !is_command_line(l))
            .collect();
        let commands: Vec<&str> = detail
            .iter()
            .copied()
            .filter(|l| is_command_line(l))
            .collect();

        // Prose lines: explanatory text and label lines (not sudo/path commands)
        assert_eq!(
            prose,
            vec![
                "FlashKraft needs root access to write to block devices.",
                "Install setuid-root so it can escalate automatically:",
            ]
        );
        assert_eq!(commands.len(), 2);
        assert!(commands[0].contains("chown"));
        assert!(commands[1].contains("chmod"));
    }
}
