//! Platform-agnostic theme colour types.

/// A single RGB colour, platform-agnostic (0–255 per channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Create a new `Rgb` value. Usable in `const` contexts.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// A complete UI colour palette — every semantic slot used by both frontends.
///
/// Defined once in core; each frontend converts [`Rgb`] values to its own
/// framework colour type (e.g. `iced::Color`, `ratatui::style::Color`).
#[derive(Debug, Clone)]
pub struct AppTheme {
    /// `true` for dark themes, `false` for light themes.
    pub is_dark: bool,

    // ── Structural ───────────────────────────────────────────────────────────
    /// Main window / terminal background.
    pub background: Rgb,
    /// Slightly elevated surface (panels, cards).
    pub surface: Rgb,
    /// Borders and dividers.
    pub border: Rgb,
    /// Background for selected / highlighted rows.
    pub selection: Rgb,

    // ── Text ────────────────────────────────────────────────────────────────
    /// Primary body text.
    pub text_primary: Rgb,
    /// Secondary / less prominent text.
    pub text_secondary: Rgb,
    /// Muted / disabled text.
    pub text_muted: Rgb,

    // ── Semantic ────────────────────────────────────────────────────────────
    /// Accent / primary action colour.
    pub accent: Rgb,
    /// Success state.
    pub success: Rgb,
    /// Warning state.
    pub warning: Rgb,
    /// Error / danger state.
    pub error: Rgb,
}
