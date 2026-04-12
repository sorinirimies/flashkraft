//! TUI Application Colour Palette
//!
//! This module bridges the [`tui_file_explorer::Theme`] preset catalogue with
//! the colour needs of the FlashKraft TUI.  Every named preset from the file
//! explorer is represented here as a [`TuiPalette`] that additionally carries
//! a background colour (`bg`) plus semantic `warn` and `err` colours that are
//! not part of the explorer's theme model.
//!
//! # Usage
//!
//! ```no_run
//! use flashkraft_tui::tui::theme::all_app_themes;
//! let themes = all_app_themes();           // Vec<(String, TuiPalette)>
//! let pal    = &themes[0].1;
//! // then pass `pal` into every render_* function
//! ```

use ratatui::style::Color;
use tui_file_explorer::Theme;

// ── Palette struct ────────────────────────────────────────────────────────────

/// A complete colour palette for the FlashKraft TUI.
///
/// Fields map directly to the semantic roles used throughout `ui.rs`.
#[derive(Debug, Clone)]
pub struct TuiPalette {
    /// Brand / primary accent (titles, active elements).
    pub brand: Color,
    /// Secondary accent (borders, highlights).
    pub accent: Color,
    /// Positive / success state.
    pub success: Color,
    /// Warning / caution state.
    pub warn: Color,
    /// Error / destructive state.
    pub err: Color,
    /// Dimmed / secondary text.
    pub dim: Color,
    /// Default foreground.
    pub fg: Color,
    /// Terminal background fill.
    pub bg: Color,
    /// Selected-row background (list highlight).
    pub sel_bg: Color,
    /// Directory names in the file explorer.
    pub dir: Color,
}

impl Default for TuiPalette {
    /// The original FlashKraft palette — orange brand, sky-blue accent.
    fn default() -> Self {
        Self {
            brand: Color::Rgb(255, 100, 30),
            accent: Color::Rgb(80, 200, 255),
            success: Color::Rgb(80, 220, 120),
            warn: Color::Rgb(255, 200, 50),
            err: Color::Rgb(255, 80, 80),
            dim: Color::Rgb(120, 120, 130),
            fg: Color::White,
            bg: Color::Rgb(18, 18, 26),
            sel_bg: Color::Rgb(40, 60, 80),
            dir: Color::Rgb(255, 210, 80),
        }
    }
}

// ── Catalogue ─────────────────────────────────────────────────────────────────

/// Build the full list of named app themes.
///
/// The order mirrors [`tui_file_explorer::Theme::all_presets`] so that
/// `explorer_theme_idx` can serve as a shared index into both lists.
pub fn all_app_themes() -> Vec<(String, TuiPalette)> {
    Theme::all_presets()
        .into_iter()
        .map(|(name, _, t)| (name.to_string(), palette_from_preset(name, &t)))
        .collect()
}

// ── Internal mapping ──────────────────────────────────────────────────────────

/// Derive a [`TuiPalette`] from a file-explorer [`Theme`] preset.
///
/// The explorer theme already provides `brand`, `accent`, `success`, `dim`,
/// `fg`, `sel_bg`, and `dir`.  We add `bg`, `warn`, and `err` from a
/// hard-coded per-preset table that matches the visual intent of each scheme.
fn palette_from_preset(name: &str, t: &Theme) -> TuiPalette {
    let (bg, warn, err) = extras(name);
    TuiPalette {
        brand: t.brand,
        accent: t.accent,
        success: t.success,
        warn,
        err,
        dim: t.dim,
        fg: t.fg,
        bg,
        sel_bg: t.sel_bg,
        dir: t.dir,
    }
}

/// Per-theme background, warn, and error colours.
///
/// Returns `(bg, warn, err)`.
macro_rules! theme_extras {
    ( $( $name:expr => bg($br:expr,$bg:expr,$bb:expr) warn($wr:expr,$wg:expr,$wb:expr) err($er:expr,$eg:expr,$eb:expr) );+ $(;)? ) => {
        fn extras(name: &str) -> (Color, Color, Color) {
            match name {
                $( $name => (
                    Color::Rgb($br, $bg, $bb),
                    Color::Rgb($wr, $wg, $wb),
                    Color::Rgb($er, $eg, $eb),
                ), )+
                _ => (
                    Color::Rgb(18, 18, 26),
                    Color::Rgb(255, 200, 50),
                    Color::Rgb(255, 80, 80),
                ),
            }
        }
    };
}

theme_extras! {
    // ── Built-in ─────────────────────────────────────────────────────────
    "Default"              => bg(18,18,26)     warn(255,200,50)   err(255,80,80);
    // ── Decorative ───────────────────────────────────────────────────────
    "Grape"                => bg(18,12,30)     warn(210,170,255)  err(255,80,150);
    "Ocean"                => bg(0,20,35)      warn(255,220,80)   err(255,100,100);
    "Sunset"               => bg(22,8,6)       warn(255,230,80)   err(255,50,50);
    "Forest"               => bg(8,18,8)       warn(220,200,80)   err(210,80,80);
    "Rose"                 => bg(28,6,16)      warn(255,220,180)  err(220,60,100);
    "Mono"                 => bg(8,8,10)       warn(200,200,200)  err(160,160,160);
    "Neon"                 => bg(6,0,14)       warn(255,220,0)    err(255,30,80);
    // ── Editor / terminal presets ────────────────────────────────────────
    "Dracula"              => bg(40,42,54)     warn(241,250,140)  err(255,85,85);
    "Nord"                 => bg(29,35,42)     warn(235,203,139)  err(191,97,106);
    "Solarized Dark"       => bg(0,43,54)      warn(181,137,0)    err(220,50,47);
    "Solarized Light"      => bg(253,246,227)  warn(181,137,0)    err(220,50,47);
    "Gruvbox Dark"         => bg(29,28,27)     warn(250,189,47)   err(251,73,52);
    "Gruvbox Light"        => bg(251,241,199)  warn(215,153,33)   err(214,93,14);
    "Catppuccin Latte"     => bg(239,241,245)  warn(223,142,29)   err(210,15,57);
    "Catppuccin Frappé"    => bg(48,52,70)     warn(229,200,144)  err(231,130,132);
    "Catppuccin Macchiato" => bg(36,39,58)     warn(238,212,159)  err(237,135,150);
    "Catppuccin Mocha"     => bg(30,30,46)     warn(249,226,175)  err(243,139,168);
    "Tokyo Night"          => bg(26,27,38)     warn(224,175,104)  err(247,118,142);
    "Tokyo Night Storm"    => bg(36,40,59)     warn(224,175,104)  err(247,118,142);
    "Tokyo Night Light"    => bg(213,214,219)  warn(140,108,62)   err(210,15,57);
    "Kanagawa Wave"        => bg(22,22,30)     warn(220,165,97)   err(210,126,153);
    "Kanagawa Dragon"      => bg(20,20,20)     warn(200,170,109)  err(210,126,153);
    "Kanagawa Lotus"       => bg(246,243,228)  warn(119,113,63)   err(192,71,71);
    "Moonfly"              => bg(8,8,8)        warn(226,164,120)  err(255,115,131);
    "Nightfly"             => bg(1,22,38)      warn(243,218,11)   err(252,87,73);
    "Oxocarbon"            => bg(22,22,22)     warn(250,204,55)   err(255,97,101)
}
