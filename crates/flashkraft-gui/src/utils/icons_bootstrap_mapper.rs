//! Bootstrap Icons Mapper Module
//!
//! This utility module provides helper functions for mapping Bootstrap Icons
//! to Iced elements in the FlashKraft application. It defines a local `Icon`
//! enum with Unicode codepoints from the Bootstrap Icons font, since
//! `iced_fonts` 0.3 no longer exposes a `Bootstrap` enum.

use iced::widget::text;
use iced::Element;

use crate::core::message::Message;

/// Bootstrap icon variants used throughout the application.
///
/// Each variant maps to a Unicode codepoint in the Bootstrap Icons font.
// TODO: Verify codepoints against https://icons.getbootstrap.com
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    ArrowClockwise,
    CheckCircle,
    CheckCircleFill,
    DeviceHdd,
    ExclamationTriangle,
    ExclamationTriangleFill,
    Image,
    LightningFill,
    PlusCircle,
    ShieldFillCheck,
    XCircle,
    XCircleFill,
}

impl From<Icon> for char {
    fn from(icon: Icon) -> Self {
        // TODO: Verify these codepoints against the actual Bootstrap Icons font
        match icon {
            Icon::ArrowClockwise => '\u{f130}',
            Icon::CheckCircle => '\u{f26e}',
            Icon::CheckCircleFill => '\u{f26f}',
            Icon::DeviceHdd => '\u{f41e}',
            Icon::ExclamationTriangle => '\u{f334}',
            Icon::ExclamationTriangleFill => '\u{f335}',
            Icon::Image => '\u{f40a}',
            Icon::LightningFill => '\u{f44d}',
            Icon::PlusCircle => '\u{f4fe}',
            Icon::ShieldFillCheck => '\u{f545}',
            Icon::XCircle => '\u{f62a}',
            Icon::XCircleFill => '\u{f62b}',
        }
    }
}

/// Create an icon element from a Bootstrap icon
///
/// # Arguments
///
/// * `icon` - The icon variant to display
/// * `size` - The size of the icon in pixels
///
/// # Returns
///
/// An Element that displays the icon
pub fn icon<'a>(icon: Icon, size: f32) -> Element<'a, Message> {
    text(char::from(icon))
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(size)
        .into()
}
