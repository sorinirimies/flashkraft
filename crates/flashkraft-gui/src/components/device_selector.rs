//! Device Selector Component
//!
//! This module contains the device selector overlay that allows users
//! to choose a target drive for flashing.

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length};

use crate::core::message::Message;
use crate::domain::{constraints, DriveInfo, ImageInfo};
use crate::utils::icons_bootstrap_mapper as icons;
use iced_fonts::bootstrap;

/// Build an [`iced::widget::Text`] that is dimmed when disabled.
macro_rules! styled_text {
    ($content:expr, $size:expr, $disabled:expr) => {{
        let t = text($content).size($size);
        if $disabled {
            t.color(Color::from_rgb(0.5, 0.5, 0.5))
        } else {
            t
        }
    }};
}

/// Return (colour, icon-builder) for a compatibility status type.
///
/// The second element is a pre-configured `Text` widget from the
/// `iced_fonts::bootstrap` module (iced_fonts 0.3 API).
fn status_style(ty: &constraints::CompatibilityStatusType) -> (Color, iced::widget::Text<'static>) {
    match ty {
        constraints::CompatibilityStatusType::Error => {
            (Color::from_rgb(0.9, 0.3, 0.3), bootstrap::x_circle())
        }
        constraints::CompatibilityStatusType::Warning => (
            Color::from_rgb(0.9, 0.7, 0.2),
            bootstrap::exclamation_triangle(),
        ),
    }
}

/// Device selector overlay
pub fn view_device_selector<'a>(
    available_drives: &'a [DriveInfo],
    selected_target: &'a Option<DriveInfo>,
    selected_image: &'a Option<ImageInfo>,
) -> Element<'a, Message> {
    let title = text("Select Target Drive").size(24);

    let drives_list: Element<'_, Message> = if available_drives.is_empty() {
        column![text("No drives detected").size(16)]
            .spacing(10)
            .align_x(Alignment::Center)
            .into()
    } else {
        let drive_rows: Vec<Element<'_, Message>> = available_drives
            .iter()
            .map(|drive| view_device_row(drive, selected_target.as_ref(), selected_image.as_ref()))
            .collect();

        scrollable(column(drive_rows).spacing(10))
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
    .padding(10);

    let cancel_button = button(text("Cancel").size(14))
        .on_press(Message::CloseDeviceSelection)
        .padding(10);

    let content = column![
        title,
        Space::new().height(20),
        drives_list,
        Space::new().height(20),
        row![refresh_button, Space::new().width(10), cancel_button].align_y(Alignment::Center)
    ]
    .spacing(10)
    .padding(30)
    .align_x(Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// Single drive row in device selector
fn view_device_row<'a>(
    drive: &'a DriveInfo,
    selected: Option<&'a DriveInfo>,
    image: Option<&'a ImageInfo>,
) -> Element<'a, Message> {
    let is_selected = selected.is_some_and(|s| s.device_path == drive.device_path);

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
        icons::icon(bootstrap::x_circle_fill(), 40.0)
    } else if has_warnings {
        icons::icon(bootstrap::exclamation_triangle_fill(), 40.0)
    } else if is_selected {
        icons::icon(bootstrap::check_circle_fill(), 40.0)
    } else {
        icons::icon(bootstrap::device_hdd(), 40.0)
    };

    // Apply grayed-out styling for disabled drives
    let name_text = styled_text!(&drive.name, 18, is_disabled);
    let size_text = styled_text!(
        flashkraft_core::fmt_bytes((drive.size_gb * 1_073_741_824.0) as u64),
        14,
        is_disabled
    );
    let mount_text = styled_text!(&drive.mount_point, 12, is_disabled);
    let device_text = styled_text!(&drive.device_path, 12, is_disabled);

    // Add warning/error badges if present
    let mut info_column = column![
        name_text,
        row![size_text, text(" • ").size(14), mount_text].spacing(5),
        device_text
    ]
    .spacing(5);

    // Add status messages
    for status in statuses {
        let (status_color, status_icon) = status_style(&status.status_type);

        info_column = info_column.push(
            row![
                icons::icon(status_icon, 12.0),
                text(status.message).size(11).color(status_color)
            ]
            .spacing(5),
        );
    }

    let info = info_column;

    let row_content =
        container(row![icon, info].spacing(20).align_y(Alignment::Center)).width(Length::Fill);

    // Only allow clicking if not disabled
    let drive_button = if is_disabled {
        button(row_content).padding(5)
    } else {
        button(row_content)
            .on_press(Message::TargetDriveClicked(drive.clone()))
            .padding(5)
    };

    drive_button.into()
}
