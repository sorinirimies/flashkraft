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
use iced::{Background, Border, Color, Shadow, Theme, Vector};

/// Corner radius of cards and buttons.
pub const RADIUS: f32 = 10.0;

/// Spacing scale shared by every screen, so paddings/gaps stay consistent
/// instead of each view picking its own magic numbers.
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 16.0;
pub const SPACE_LG: f32 = 24.0;
pub const SPACE_XL: f32 = 40.0;

/// Soft drop shadow for raised surfaces (cards, the primary CTA). Kept subtle
/// so it reads as "elevated" without a heavy skeuomorphic look.
fn elevation(alpha: f32, offset_y: f32, blur: f32) -> Shadow {
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, alpha),
        offset: Vector::new(0.0, offset_y),
        blur_radius: blur,
    }
}

/// Coloured "glow" shadow (shadow tinted with the given colour instead of
/// plain black) — used on hover so interactive elements get a clear,
/// strongly-coloured highlight instead of just a slightly bigger grey blur.
fn glow(color: Color, alpha: f32, offset_y: f32, blur: f32) -> Shadow {
    Shadow {
        color: Color { a: alpha, ..color },
        offset: Vector::new(0.0, offset_y),
        blur_radius: blur,
    }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}

/// Card surface: `active` paints the border in the accent colour and lifts
/// the shadow slightly, so "selected" reads at a glance, not just on hover.
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
        shadow: elevation(if active { 0.22 } else { 0.12 }, 1.0, 6.0),
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

/// Dimmed full-screen backdrop behind a modal (About / Settings panels).
pub fn backdrop(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(Color {
            a: 0.55,
            ..p.background.strongest.color
        })),
        ..container::Style::default()
    }
}

/// Round badge behind an icon glyph — `active` fills it with the accent
/// colour instead of a neutral tint, so a completed/selected step reads at
/// a glance without a separate indicator row duplicating it.
pub fn icon_badge(theme: &Theme, active: bool) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(if active {
            p.primary.base.color
        } else {
            p.background.strong.color
        })),
        text_color: Some(if active {
            p.primary.base.text
        } else {
            p.background.base.text
        }),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// De-emphasised caption/metadata text (65% opacity over the normal text
/// colour) — one place to tweak instead of every screen hand-rolling its own
/// `scale_alpha` call.
pub fn muted_text(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(
            theme
                .extended_palette()
                .background
                .base
                .text
                .scale_alpha(0.65),
        ),
    }
}

/// Text dimmed further for a *disabled* row/item — distinct from
/// [`muted_text`] (de-emphasised but enabled) so the two states don't look
/// identical.
pub fn disabled_text(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(
            theme
                .extended_palette()
                .background
                .base
                .text
                .scale_alpha(0.4),
        ),
    }
}

/// Theme colour for a compatibility status (error -> danger, warning ->
/// warning) instead of hardcoded RGB that ignores the active theme/palette.
pub fn status_color(theme: &Theme, is_error: bool) -> Color {
    let p = theme.extended_palette();
    if is_error {
        p.danger.base.color
    } else {
        p.warning.base.color
    }
}

/// Tone of a [`badge`] — the big round icon on status screens (flashing,
/// complete, error) and small role badges elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Normal in-progress / brand accent.
    Accent,
    /// Success (flash complete).
    Success,
    /// Failure (error screen).
    Danger,
}

/// Round, filled badge in the given [`Tone`] — used for the big "hero icon"
/// on status screens so success/failure read from colour, not just the
/// glyph, and stay legible (readable fg/bg pair) across every theme.
pub fn badge(theme: &Theme, tone: Tone) -> container::Style {
    let p = theme.extended_palette();
    let (bg, fg) = match tone {
        Tone::Accent => (p.primary.base.color, p.primary.base.text),
        Tone::Success => (p.success.base.color, p.success.base.text),
        Tone::Danger => (p.danger.base.color, p.danger.base.text),
    };
    container::Style {
        background: Some(Background::Color(bg)),
        text_color: Some(fg),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        shadow: elevation(0.25, 2.0, 12.0),
        ..container::Style::default()
    }
}

/// Card variant for a block of monospace detail (a shell command, a raw
/// error dump): same radius/border language as [`card`] but with a flat,
/// slightly recessed background instead of a manually darkened colour that
/// only looked right on dark themes.
pub fn code_card(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(p.background.strong.color)),
        text_color: Some(p.background.strong.text),
        border: Border {
            color: p.background.strong.color,
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..container::Style::default()
    }
}

/// Clickable card: looks like [`card`], glows strongly on hover, visibly
/// "sinks" (flattened shadow, deeper tint) while pressed, dims when
/// disabled. Hover and press are deliberately distinct states, not just two
/// names for the same look.
pub fn card_button(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let p = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered);
    let pressed = matches!(status, button::Status::Pressed);
    let disabled = matches!(status, button::Status::Disabled);

    let bg = if pressed {
        mix(p.background.weak.color, p.primary.base.color, 0.32)
    } else if hovered {
        mix(p.background.weak.color, p.primary.base.color, 0.20)
    } else {
        p.background.weak.color
    };
    let border = if active || hovered || pressed {
        p.primary.strong.color
    } else {
        p.background.strong.color
    };
    let text = if disabled {
        mix(p.background.base.text, p.background.base.color, 0.55)
    } else {
        p.background.base.text
    };
    let shadow = if pressed {
        // Flattened almost completely — reads as "pushed in", not just a
        // dimmer version of the hover glow.
        elevation(0.10, 0.0, 3.0)
    } else if hovered {
        // Big, accent-tinted glow: an unmistakable "this is interactive" cue.
        glow(p.primary.base.color, 0.38, 3.0, 18.0)
    } else {
        elevation(0.12, 1.0, 6.0)
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: Border {
            color: border,
            width: if hovered || pressed {
                2.0
            } else if active {
                1.5
            } else {
                1.0
            },
            radius: RADIUS.into(),
        },
        shadow,
        ..button::Style::default()
    }
}

/// Filled call-to-action button (Flash!). Dim while disabled; hover gets a
/// bright highlighting ring + glow, pressed visibly darkens and flattens.
pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let (bg, fg) = match status {
        button::Status::Disabled => (p.background.strong.color, p.background.strong.text),
        button::Status::Pressed => (
            mix(p.primary.strong.color, Color::BLACK, 0.2),
            p.primary.strong.text,
        ),
        button::Status::Hovered => (p.primary.strong.color, p.primary.strong.text),
        button::Status::Active => (p.primary.base.color, p.primary.base.text),
    };
    let shadow = match status {
        button::Status::Disabled => Shadow::default(),
        button::Status::Pressed => elevation(0.15, 0.0, 4.0),
        button::Status::Hovered => glow(p.primary.base.color, 0.5, 4.0, 24.0),
        button::Status::Active => elevation(0.18, 1.5, 8.0),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if matches!(status, button::Status::Disabled) {
            mix(fg, bg, 0.5)
        } else {
            fg
        },
        border: Border {
            color: p.primary.strong.color.scale_alpha(0.9),
            width: if matches!(status, button::Status::Hovered) {
                2.0
            } else {
                0.0
            },
            radius: RADIUS.into(),
        },
        shadow,
        ..button::Style::default()
    }
}

/// Low-key button (Cancel, Refresh): transparent until interacted with, then
/// a clear accent-tinted highlight on hover and a deeper tint while pressed.
pub fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered);
    let pressed = matches!(status, button::Status::Pressed);

    let bg = if pressed {
        Some(Background::Color(mix(
            p.background.strong.color,
            p.primary.base.color,
            0.28,
        )))
    } else if hovered {
        Some(Background::Color(mix(
            p.background.strong.color,
            p.primary.base.color,
            0.14,
        )))
    } else {
        None
    };
    button::Style {
        background: bg,
        text_color: if hovered || pressed {
            p.primary.strong.color
        } else {
            p.background.base.text
        },
        border: Border {
            color: if hovered || pressed {
                p.primary.strong.color
            } else {
                p.background.strong.color
            },
            width: if hovered || pressed { 1.5 } else { 1.0 },
            radius: RADIUS.into(),
        },
        shadow: if hovered {
            glow(p.primary.base.color, 0.28, 2.0, 12.0)
        } else {
            Shadow::default()
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
    fn card_button_pressed_is_visually_distinct_from_hovered() {
        let t = Theme::Dark;
        let hover = card_button(&t, button::Status::Hovered, false);
        let press = card_button(&t, button::Status::Pressed, false);
        assert_ne!(
            hover.background, press.background,
            "pressed should look different from hovered, not identical"
        );
        assert_ne!(hover.shadow.blur_radius, press.shadow.blur_radius);
    }

    #[test]
    fn primary_button_pressed_is_visually_distinct_from_hovered() {
        let t = Theme::Dark;
        let hover = primary_button(&t, button::Status::Hovered);
        let press = primary_button(&t, button::Status::Pressed);
        assert_ne!(hover.background, press.background);
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
