//! Bootstrap Icons Mapper Module
//!
//! This utility module provides helper functions for mapping Bootstrap Icons
//! to Iced elements in the FlashKraft application. It uses Bootstrap Icons
//! from the iced_fonts crate.
//!
//! In iced_fonts 0.3+, the `Bootstrap` enum was replaced by a `bootstrap`
//! module containing one function per icon that returns a pre-configured
//! `Text` widget.  This helper adds a `.size()` call and converts to
//! `Element`.

use iced::widget::Text;
use iced::{Element, Renderer, Theme};

use crate::core::message::Message;

/// Finish an icon [`Text`] widget by applying a pixel size and converting to
/// [`Element`].
///
/// # Usage
///
/// ```ignore
/// use iced_fonts::bootstrap;
/// icon(bootstrap::image(), 32.0)
/// ```
pub fn icon<'a>(base: Text<'a, Theme, Renderer>, size: f32) -> Element<'a, Message> {
    base.size(size).into()
}
