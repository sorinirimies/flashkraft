//! Selection Panels Component
//!
//! This module contains the three main selection panels:
//! - Image selection panel
//! - Target drive selection panel
//! - Flash button panel

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::core::message::Message;
use crate::domain::{DriveInfo, ImageInfo};
use crate::utils::icons_bootstrap_mapper as icons;
use iced_fonts::bootstrap;

/// Three buttons section
pub fn view_buttons<'a>(
    selected_image: &'a Option<ImageInfo>,
    selected_target: &'a Option<DriveInfo>,
    is_ready_to_flash: bool,
) -> Element<'a, Message> {
    let image_button = view_image_section(selected_image);
    let target_button = view_target_section(selected_target);
    let flash_button = view_flash_section(is_ready_to_flash);

    let buttons_row = row![image_button, target_button, flash_button]
        .spacing(40)
        .align_y(Alignment::Start);

    let mut content = column![buttons_row].spacing(20).align_x(Alignment::Center);

    // Show cancel button if something is selected
    if selected_image.is_some() || selected_target.is_some() {
        content = content.push(
            button(text("Cancel").size(14))
                .on_press(Message::CancelClicked)
                .padding(10),
        );
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// Image selection panel
fn view_image_section(image: &Option<ImageInfo>) -> Element<'_, Message> {
    let content = if let Some(img) = image {
        column![
            icons::icon(bootstrap::check_circle(), 50.0),
            text(&img.name).size(16),
            text(flashkraft_core::fmt_bytes(
                (img.size_mb * 1_048_576.0) as u64
            ))
            .size(12),
        ]
    } else {
        column![
            icons::icon(bootstrap::plus_circle(), 50.0),
            text("Flash from file").size(16),
            text("Click to select").size(12),
        ]
    };

    button(
        container(content.spacing(10).align_x(Alignment::Center))
            .width(220)
            .height(90)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .on_press(Message::SelectImageClicked)
    .padding(10)
    .into()
}

/// Target drive selection panel
fn view_target_section(selected: &Option<DriveInfo>) -> Element<'_, Message> {
    let content = if let Some(target) = selected {
        column![
            icons::icon(bootstrap::check_circle(), 50.0),
            text(&target.name).size(16),
            text(flashkraft_core::fmt_bytes(
                (target.size_gb * 1_073_741_824.0) as u64
            ))
            .size(12),
            text(&target.mount_point).size(10),
        ]
    } else {
        column![
            icons::icon(bootstrap::device_hdd(), 50.0),
            text("Select target").size(16),
            text("Click to choose").size(12),
        ]
    };

    button(
        container(content.spacing(10).align_x(Alignment::Center))
            .width(220)
            .height(90)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .on_press(Message::OpenDeviceSelection)
    .padding(10)
    .into()
}

/// Flash button panel
fn view_flash_section(is_ready: bool) -> Element<'static, Message> {
    let content = column![
        icons::icon(bootstrap::lightning_fill(), 50.0),
        text("Flash!").size(16),
        text(if is_ready {
            "Ready to flash"
        } else {
            "Select image & target"
        })
        .size(12),
    ];

    let btn = button(
        container(content.spacing(10).align_x(Alignment::Center))
            .width(220)
            .height(90)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .padding(10);

    if is_ready {
        btn.on_press(Message::FlashClicked).into()
    } else {
        btn.into()
    }
}
