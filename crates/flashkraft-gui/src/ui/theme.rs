//! Visual language shared with the TUI and the web dashboard.
//!
//! * **Cards** — a raised surface, 1 px border, 10 px radius. The border turns
//!   to the accent colour when the card is *active* (selected / done).
//! * **Chips** — small rounded pills for metadata (versions, sizes, badges).
//! * **Brand** — `⚡ Flash` in the accent colour, `Kraft` in the text colour.
//!
//! All colours come from the active [`iced::Theme`] palette, so every theme
//! in the picker restyles the whole window.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};

/// Corner radius of cards and buttons.
pub const RADIUS: f32 = 10.0;

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}

/// Card surface: `active` paints the border in the accent colour.
pub fn card(theme: &Theme, active: bool) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(p.background.weak.color)),
        text_color: Some(p.background.base.text),
        border: Border {
            color: if active {
                p.primary.strong.color
            } else {
                p.background.strong.color
            },
            width: if active { 1.5 } else { 1.0 },
            radius: RADIUS.into(),
        },
        ..container::Style::default()
    }
}

/// Small rounded pill (version, size, status).
pub fn chip(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(p.background.strong.color)),
        text_color: Some(p.background.strong.text),
        border: Border {
            radius: 20.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Clickable card: looks like [`card`], lifts on hover, dims when disabled.
pub fn card_button(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let p = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let disabled = matches!(status, button::Status::Disabled);

    let bg = if hovered {
        mix(p.background.weak.color, p.primary.base.color, 0.12)
    } else {
        p.background.weak.color
    };
    let border = if active || hovered {
        p.primary.strong.color
    } else {
        p.background.strong.color
    };
    let text = if disabled {
        mix(p.background.base.text, p.background.base.color, 0.55)
    } else {
        p.background.base.text
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: Border {
            color: border,
            width: if active || hovered { 1.5 } else { 1.0 },
            radius: RADIUS.into(),
        },
        ..button::Style::default()
    }
}

/// Filled call-to-action button (Flash!). Dim while disabled.
pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let (bg, fg) = match status {
        button::Status::Disabled => (p.background.strong.color, p.background.strong.text),
        button::Status::Hovered | button::Status::Pressed => {
            (p.primary.strong.color, p.primary.strong.text)
        }
        button::Status::Active => (p.primary.base.color, p.primary.base.text),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if matches!(status, button::Status::Disabled) {
            mix(fg, bg, 0.5)
        } else {
            fg
        },
        border: Border {
            radius: RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Low-key button (Cancel, Refresh).
pub fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: hovered.then_some(Background::Color(p.background.strong.color)),
        text_color: p.background.base.text,
        border: Border {
            color: p.background.strong.color,
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_cards_use_the_accent_border() {
        let t = Theme::TokyoNight;
        let p = t.extended_palette();
        assert_eq!(card(&t, true).border.color, p.primary.strong.color);
        assert_eq!(card(&t, false).border.color, p.background.strong.color);
        assert_eq!(card(&t, false).border.radius, RADIUS.into());
    }

    #[test]
    fn card_button_highlights_on_hover_only_when_enabled_states_differ() {
        let t = Theme::Dark;
        let idle = card_button(&t, button::Status::Active, false);
        let hover = card_button(&t, button::Status::Hovered, false);
        assert_ne!(idle.border.color, hover.border.color);
        assert_ne!(idle.background, hover.background);
    }

    #[test]
    fn disabled_text_is_dimmer_than_enabled() {
        let t = Theme::Dark;
        let on = card_button(&t, button::Status::Active, false).text_color;
        let off = card_button(&t, button::Status::Disabled, false).text_color;
        assert_ne!(on, off);
    }

    #[test]
    fn every_builtin_theme_yields_distinct_primary_button_states() {
        for t in Theme::ALL {
            let a = primary_button(t, button::Status::Active);
            let d = primary_button(t, button::Status::Disabled);
            assert_ne!(a.background, d.background, "{t}");
        }
    }

    #[test]
    fn mix_endpoints() {
        let (a, b) = (Color::BLACK, Color::WHITE);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }
}
