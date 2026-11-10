//! Device Selector Component
//!
//! This module contains the device selector overlay that allows users
//! to choose a target drive for flashing.

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Theme};

use crate::core::message::Message;
use crate::domain::{DriveInfo, ImageInfo, constraints};
use crate::ui::theme as style;
use crate::utils::icons;
use iced_fonts::bootstrap;

/// Build an [`iced::widget::Text`] that is themed-dim when disabled, instead
/// of a hardcoded grey that ignores the active palette.
macro_rules! styled_text {
    ($content:expr, $size:expr, $disabled:expr) => {{
        text($content).size($size).style(move |t: &Theme| {
            if $disabled {
                style::disabled_text(t)
            } else {
                iced::widget::text::Style::default()
            }
        })
    }};
}

/// Icon for a compatibility status type (error -> x-circle, warning ->
/// exclamation-triangle).
fn status_icon(is_error: bool) -> iced::widget::Text<'static> {
    if is_error {
        bootstrap::x_circle()
    } else {
        bootstrap::exclamation_triangle()
    }
}

/// Device selector overlay
pub fn view_device_selector<'a>(
    available_drives: &'a [DriveInfo],
    selected_target: &'a Option<DriveInfo>,
    selected_image: &'a Option<ImageInfo>,
    marked: &'a [DriveInfo],
) -> Element<'a, Message> {
    let title = text("Select Target Drive").size(22);
    let subtitle = text("Click a drive to flash it, or tick several to flash them all at once")
        .size(13)
        .style(style::muted_text);

    let drives_list: Element<'_, Message> = if available_drives.is_empty() {
        column![
            text("No drives detected").size(16),
            text("Plug in a USB drive, then hit Refresh")
                .size(12)
                .style(style::muted_text),
        ]
        .spacing(style::SPACE_XS)
        .align_x(Alignment::Center)
        .into()
    } else {
        let drive_rows: Vec<Element<'_, Message>> = available_drives
            .iter()
            .map(|drive| {
                view_device_row(
                    drive,
                    selected_target.as_ref(),
                    selected_image.as_ref(),
                    marked,
                )
            })
            .collect();

        scrollable(column(drive_rows).spacing(style::SPACE_SM))
            .height(Length::Fill)
            .into()
    };

    let refresh_button = button(
        row![
            icons::icon(bootstrap::arrow_clockwise(), 16.0),
            text("Refresh").size(14)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(Message::RefreshDrivesClicked)
    .padding(10)
    .style(style::ghost_button);

    let cancel_button = button(text("Cancel").size(14))
        .on_press(Message::CloseDeviceSelection)
        .padding(10)
        .style(style::ghost_button);

    let done_button = (!marked.is_empty()).then(|| {
        button(text(format!("Use {} marked", marked.len())).size(14))
            .on_press(Message::CloseDeviceSelection)
            .padding(10)
            .style(style::primary_button)
    });
    let mut actions = row![
        refresh_button,
        Space::new().width(style::SPACE_SM),
        cancel_button
    ]
    .align_y(Alignment::Center);
    if let Some(done) = done_button {
        actions = actions.push(Space::new().width(style::SPACE_SM)).push(done);
    }

    let content = column![
        title,
        subtitle,
        Space::new().height(style::SPACE_MD),
        drives_list,
        Space::new().height(style::SPACE_MD),
        actions
    ]
    .spacing(style::SPACE_SM)
    .padding(30)
    .width(Length::Fixed(620.0))
    .align_x(Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// Single drive row in device selector
pub(crate) fn view_device_row<'a>(
    drive: &'a DriveInfo,
    selected: Option<&'a DriveInfo>,
    image: Option<&'a ImageInfo>,
    marked: &'a [DriveInfo],
) -> Element<'a, Message> {
    let is_marked = marked.iter().any(|m| m.device_path == drive.device_path);
    let is_selected = selected.is_some_and(|s| s.device_path == drive.device_path) || is_marked;

    // Get compatibility statuses
    let statuses = constraints::get_drive_image_compatibility_statuses(drive, image);
    let has_errors = statuses
        .iter()
        .any(|s| s.status_type == constraints::CompatibilityStatusType::Error);
    let has_warnings = statuses
        .iter()
        .any(|s| s.status_type == constraints::CompatibilityStatusType::Warning);

    // Determine if drive should be disabled
    let is_disabled = drive.disabled || has_errors;

    // Choose icon based on state
    let icon = if is_disabled {
        bootstrap::x_circle_fill()
    } else if has_warnings {
        bootstrap::exclamation_triangle_fill()
    } else if is_selected {
        bootstrap::check_circle_fill()
    } else {
        bootstrap::device_hdd()
    };
    let badge = container(icon.size(20.0))
        .width(40.0)
        .height(40.0)
        .center_x(40.0)
        .center_y(40.0)
        .style(move |t: &Theme| style::icon_badge(t, is_selected && !is_disabled));

    // Apply grayed-out styling for disabled drives
    let name_text = styled_text!(&drive.name, 16, is_disabled);
    let size_text = styled_text!(
        flashkraft_core::fmt_bytes(drive.size_bytes()),
        13,
        is_disabled
    );
    let mount_text = text(&drive.mount_point).size(11).style(style::muted_text);
    let device_text = text(&drive.device_path).size(11).style(style::muted_text);

    // Add warning/error badges if present
    let mut info_column = column![
        name_text,
        row![
            size_text,
            text(" • ").size(13).style(style::muted_text),
            mount_text
        ]
        .spacing(5),
        device_text
    ]
    .spacing(4);

    // Add status messages
    for status in statuses {
        let is_error = status.status_type == constraints::CompatibilityStatusType::Error;
        let color_style = move |t: &Theme| iced::widget::text::Style {
            color: Some(style::status_color(t, is_error)),
        };
        info_column = info_column.push(
            row![
                status_icon(is_error).size(12.0).style(color_style),
                text(status.message).size(11).style(color_style)
            ]
            .spacing(5),
        );
    }

    let info = info_column;

    let row_content = container(
        row![badge, info]
            .spacing(style::SPACE_MD)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill);

    // Only allow clicking if not disabled
    let drive_button = if is_disabled {
        button(row_content).padding(12)
    } else {
        button(row_content)
            .on_press(Message::TargetDriveClicked(drive.clone()))
            .padding(12)
    }
    .width(Length::Fill)
    .style(move |t, s| style::card_button(t, s, is_selected));

    let mark_button = button(
        text(if is_marked {
            "\u{2713} Marked"
        } else {
            "+ Add"
        })
        .size(13),
    )
    .padding([8, 12])
    .style(move |t, s| {
        if is_marked {
            style::primary_button(t, s)
        } else {
            style::ghost_button(t, s)
        }
    });
    let mark_button = if is_disabled {
        mark_button
    } else {
        mark_button.on_press(Message::ToggleMarkTarget(drive.clone()))
    };

    row![drive_button, mark_button]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center)
        .into()
}
