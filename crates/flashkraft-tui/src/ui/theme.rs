//! TUI Application Colour Palette
//!
//! Derives [`TuiPalette`] from the platform-agnostic [`flashkraft_core::AppTheme`]
//! presets, giving the TUI access to all 28 themes including Cyberpunk.

use flashkraft_core::{theme_by_index, AppTheme, THEME_COUNT, THEME_NAMES};
use ratatui::style::Color;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Convert a core [`flashkraft_core::Rgb`] to a ratatui [`Color`].
fn rgb(c: flashkraft_core::Rgb) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

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
    /// The FlashKraft palette — sky-blue brand, matching the GUI default.
    fn default() -> Self {
        Self {
            brand: Color::Rgb(80, 200, 255), // sky-blue — primary brand
            accent: Color::Rgb(60, 80, 100), // steel-blue — borders, badges, hints
            success: Color::Rgb(80, 220, 120),
            warn: Color::Rgb(255, 200, 50),
            err: Color::Rgb(255, 80, 80),
            dim: Color::Rgb(120, 120, 130),
            fg: Color::White,
            bg: Color::Rgb(18, 18, 26),
            sel_bg: Color::Rgb(40, 60, 80),
            dir: Color::Rgb(255, 100, 30), // orange — directory names
        }
    }
}

// ── Catalogue ─────────────────────────────────────────────────────────────────

/// Build the full list of named app themes from core presets.
///
/// Returns one entry per theme in `flashkraft_core::THEME_NAMES` order,
/// covering all 28 themes including Cyberpunk.
pub fn all_app_themes() -> Vec<(String, TuiPalette)> {
    (0..THEME_COUNT)
        .map(|i| (THEME_NAMES[i].to_string(), palette_from_theme(i)))
        .collect()
}

// ── Conversion ────────────────────────────────────────────────────────────────

/// Convert a core [`AppTheme`] index into a [`TuiPalette`].
pub fn palette_from_theme(index: usize) -> TuiPalette {
    let t = theme_by_index(index);
    palette_from_app_theme(&t)
}

/// Convert a core [`AppTheme`] into a [`TuiPalette`].
pub fn palette_from_app_theme(t: &AppTheme) -> TuiPalette {
    TuiPalette {
        brand: rgb(t.accent),  // primary brand colour (bold, titles, active elements)
        accent: rgb(t.border), // secondary accent (softer, borders, badges, hints)
        success: rgb(t.success),
        warn: rgb(t.warning),
        err: rgb(t.error),
        dim: rgb(t.text_muted),
        fg: rgb(t.text_primary),
        bg: rgb(t.background),
        sel_bg: rgb(t.selection),
        dir: rgb(t.text_secondary), // directory names — secondary colour
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_app_themes_non_empty() {
        let themes = all_app_themes();
        assert!(
            !themes.is_empty(),
            "all_app_themes() must return at least one theme"
        );
    }

    #[test]
    fn all_app_themes_returns_43_themes() {
        assert_eq!(all_app_themes().len(), 43, "expected exactly 43 themes");
    }

    #[test]
    fn cyberpunk_is_present() {
        let names: Vec<String> = all_app_themes().into_iter().map(|(n, _)| n).collect();
        assert!(
            names.iter().any(|n| n == "Cyberpunk"),
            "Cyberpunk theme must be in the catalogue"
        );
    }

    #[test]
    fn cyberpunk_is_dark() {
        let themes = all_app_themes();
        let cyberpunk = themes.iter().find(|(n, _)| n == "Cyberpunk").unwrap();
        // Dark theme — bg should have low luminance (sum of channels < 100)
        let bg_sum = match cyberpunk.1.bg {
            Color::Rgb(r, g, b) => r as u32 + g as u32 + b as u32,
            _ => 999,
        };
        assert!(
            bg_sum < 100,
            "Cyberpunk background should be dark, got bg sum {bg_sum}"
        );
    }

    #[test]
    fn all_app_themes_have_names() {
        for (name, _palette) in all_app_themes() {
            assert!(!name.is_empty(), "theme name must not be empty");
        }
    }

    #[test]
    fn all_app_themes_palettes_have_distinct_fg_bg() {
        for (name, pal) in all_app_themes() {
            // fg and bg should not both be the zero color (black)
            let both_black = matches!((pal.fg, pal.bg), (Color::Rgb(0, 0, 0), Color::Rgb(0, 0, 0)));
            assert!(!both_black, "theme {name:?} has both fg and bg as (0,0,0)",);
        }
    }

    #[test]
    fn all_app_themes_palettes_fg_differs_from_bg() {
        for (name, pal) in all_app_themes() {
            assert_ne!(
                pal.fg, pal.bg,
                "theme {name:?} has identical fg and bg — text would be invisible",
            );
        }
    }

    #[test]
    fn default_palette_is_valid() {
        let pal = TuiPalette::default();
        // Brand colour should be sky-blue (matches GUI default)
        assert_eq!(pal.brand, Color::Rgb(80, 200, 255));
        // fg should be white
        assert_eq!(pal.fg, Color::White);
        // bg should be the dark blue-ish
        assert_eq!(pal.bg, Color::Rgb(18, 18, 26));
        // fg != bg
        assert_ne!(pal.fg, pal.bg);
    }

    // -- Additional tests -----------------------------------------------------

    #[test]
    fn all_theme_names_are_unique() {
        let themes = all_app_themes();
        let mut seen = std::collections::HashSet::new();
        for (name, _) in &themes {
            assert!(seen.insert(name.clone()), "duplicate theme name: {name:?}");
        }
    }

    #[test]
    fn no_theme_has_pure_black_bg_unless_intended() {
        // Pure black (0,0,0) as bg is unusual — only allow it if the theme
        // name explicitly suggests it.  Currently none of our themes use it.
        let allow_black: &[&str] = &[]; // add names here if a theme intentionally uses #000
        for (name, pal) in all_app_themes() {
            if allow_black.contains(&name.as_str()) {
                continue;
            }
            let is_pure_black = matches!(pal.bg, Color::Rgb(0, 0, 0));
            assert!(
                !is_pure_black,
                "theme {name:?} has pure-black bg (0,0,0) — is this intentional?"
            );
        }
    }

    #[test]
    fn default_palette_has_reasonable_field_values() {
        let pal = TuiPalette::default();
        // accent should be set (not black)
        assert_ne!(pal.accent, Color::Rgb(0, 0, 0));
        // success should be set
        assert_ne!(pal.success, Color::Rgb(0, 0, 0));
        // warn and err should be set
        assert_ne!(pal.warn, Color::Rgb(0, 0, 0));
        assert_ne!(pal.err, Color::Rgb(0, 0, 0));
        // dim should be set
        assert_ne!(pal.dim, Color::Rgb(0, 0, 0));
        // sel_bg should be set
        assert_ne!(pal.sel_bg, Color::Rgb(0, 0, 0));
        // dir should be set
        assert_ne!(pal.dir, Color::Rgb(0, 0, 0));
    }

    #[test]
    fn default_palette_semantic_colors_are_distinct() {
        let pal = TuiPalette::default();
        let semantic = [pal.brand, pal.accent, pal.warn, pal.err];
        // All four should be different from each other.
        for i in 0..semantic.len() {
            for j in (i + 1)..semantic.len() {
                assert_ne!(
                    semantic[i], semantic[j],
                    "default palette: semantic color at index {i} equals color at index {j}"
                );
            }
        }
    }

    #[test]
    fn every_palette_has_distinct_brand_accent_warn_err() {
        // Some themes legitimately share semantic colours.
        // These overlaps are documented here so we still catch regressions in
        // all other themes.
        let known_overlaps: &[(&str, &str, &str)] = &[
            // Mono themes use the same greyscale for multiple semantic slots
            ("Mono", "brand", "accent"),
            ("Mono", "brand", "warn"),
            ("Mono", "brand", "err"),
            ("Mono", "accent", "warn"),
            ("Mono", "accent", "err"),
            ("Mono", "warn", "err"),
            // Solarized Dark/Light: accent=brand (both blue)
            ("Solarized Dark", "brand", "accent"),
            ("Solarized Light", "brand", "accent"),
            // Gruvbox: accent from text_secondary matches warning slot
            ("Gruvbox Dark", "accent", "warn"),
            // Gruvbox Light: brand (accent) and error use the same orange
            ("Gruvbox Light", "brand", "err"),
            ("Gruvbox Light", "accent", "warn"),
            // Kanagawa: brand == error (both use the pink/sakura)
            ("Kanagawa Wave", "brand", "err"),
            ("Kanagawa Dragon", "brand", "err"),
            // Tokyo Night variants share the same purple accent
            ("Tokyo Night", "brand", "accent"),
            ("Tokyo Night Storm", "brand", "accent"),
            // Synthwave: accent and error are both the hot-pink
            ("Synthwave", "brand", "err"),
            // Everforest Dark: accent and success are both sage-green
            ("Everforest Dark", "brand", "success"),
            // Andromeda: accent and success are both the green
            ("Andromeda", "brand", "success"),
            // Poimandres: accent and success are both the teal
            ("Poimandres", "brand", "success"),
            // Ayu Mirage: accent == text_secondary (both cyan)
            ("Ayu Mirage", "brand", "accent"),
        ];

        for (name, pal) in all_app_themes() {
            let colors = [pal.brand, pal.accent, pal.warn, pal.err];
            let labels = ["brand", "accent", "warn", "err"];
            for i in 0..colors.len() {
                for j in (i + 1)..colors.len() {
                    let is_known = known_overlaps.iter().any(|(n, a, b)| {
                        *n == name
                            && ((*a == labels[i] && *b == labels[j])
                                || (*a == labels[j] && *b == labels[i]))
                    });
                    if is_known {
                        continue;
                    }
                    assert_ne!(
                        colors[i],
                        colors[j],
                        "theme {name:?}: {l1} and {l2} are identical",
                        l1 = labels[i],
                        l2 = labels[j],
                    );
                }
            }
        }
    }

    #[test]
    fn palette_clone_produces_equal_values() {
        let pal = TuiPalette::default();
        let cloned = pal.clone();
        assert_eq!(cloned.brand, pal.brand);
        assert_eq!(cloned.bg, pal.bg);
        assert_eq!(cloned.fg, pal.fg);
        assert_eq!(cloned.accent, pal.accent);
        assert_eq!(cloned.warn, pal.warn);
        assert_eq!(cloned.err, pal.err);
    }
}
