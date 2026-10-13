//! Colour themes: the core's 43 palettes as CSS custom properties.
//!
//! `/style.css` is written against `var(--bg)`, `var(--accent)` … with the
//! Default theme as fallback; a `<style>` block from [`css`] overrides them.

use flashkraft_core::{
    AppTheme, Rgb, THEME_COUNT, THEME_NAMES, theme_by_index, theme_index_by_name,
};

/// Uppercase `#rrggbb`.
pub fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// CSS custom properties for the theme at `index` (clamped) as one `:root{…}` rule.
pub fn css(index: usize) -> String {
    let t = theme_by_index(index.min(THEME_COUNT - 1));
    let vars: [(&str, Rgb); 11] = [
        ("bg", t.background),
        ("panel", t.surface),
        ("line", t.border),
        ("sel", t.selection),
        ("fg", t.text_primary),
        ("fg2", t.text_secondary),
        ("dim", t.text_muted),
        ("accent", t.accent),
        ("ok", t.success),
        ("warn", t.warning),
        ("err", t.error),
    ];
    let body: String = vars
        .iter()
        .map(|(k, c)| format!("--{k}:{};", hex(*c)))
        .collect();
    format!(
        ":root{{color-scheme:{};{body}}}",
        if t.is_dark { "dark" } else { "light" }
    )
}

/// Index of `name` (exact, then case-insensitive).
pub fn find(name: &str) -> Option<usize> {
    theme_index_by_name(name).or_else(|| {
        THEME_NAMES
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name.trim()))
    })
}

/// Pick the theme: an explicit, valid `requested` name wins, otherwise the
/// TUI's saved theme, otherwise the first (Default).
pub fn resolve(requested: Option<&str>, saved: Option<&str>) -> usize {
    requested
        .and_then(find)
        .or_else(|| saved.and_then(find))
        .unwrap_or(0)
}

/// Theme names in canonical order (for the select).
pub fn names() -> &'static [&'static str] {
    &THEME_NAMES
}

fn palette_json(t: &AppTheme, name: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "dark": t.is_dark,
        "background": hex(t.background),
        "surface": hex(t.surface),
        "border": hex(t.border),
        "selection": hex(t.selection),
        "text": hex(t.text_primary),
        "text_secondary": hex(t.text_secondary),
        "text_muted": hex(t.text_muted),
        "accent": hex(t.accent),
        "success": hex(t.success),
        "warning": hex(t.warning),
        "error": hex(t.error),
    })
}

/// Every theme with its colours, for `/api/themes`.
pub fn catalogue() -> serde_json::Value {
    (0..THEME_COUNT)
        .map(|i| palette_json(&theme_by_index(i), THEME_NAMES[i]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_yields_a_complete_rule() {
        for i in 0..THEME_COUNT {
            let c = css(i);
            for v in [
                "--bg:",
                "--panel:",
                "--line:",
                "--fg:",
                "--accent:",
                "--ok:",
                "--warn:",
                "--err:",
            ] {
                assert!(c.contains(v), "theme {i} lacks {v}");
            }
            assert!(c.starts_with(":root{") && c.ends_with('}'));
        }
    }

    #[test]
    fn css_is_safe_to_inline_in_a_style_element() {
        for i in 0..THEME_COUNT {
            assert!(
                css(i)
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || "#:;{}-,. ".contains(ch)),
                "theme {i}"
            );
        }
    }

    #[test]
    fn light_and_dark_set_the_matching_color_scheme() {
        assert!(css(0).contains("color-scheme:dark"));
        let light = (0..THEME_COUNT)
            .find(|i| !theme_by_index(*i).is_dark)
            .unwrap();
        assert!(css(light).contains("color-scheme:light"));
    }

    #[test]
    fn out_of_range_index_is_clamped() {
        assert_eq!(css(usize::MAX), css(THEME_COUNT - 1));
    }

    #[test]
    fn resolve_prefers_valid_request_then_saved_then_default() {
        let nord = find("Nord").unwrap();
        assert_eq!(resolve(Some("nord"), Some("Dracula")), nord);
        assert_eq!(resolve(Some("nope"), Some("Nord")), nord);
        assert_eq!(resolve(None, Some("garbage")), 0);
        assert_eq!(resolve(None, None), 0);
    }

    #[test]
    fn catalogue_lists_every_theme() {
        let c = catalogue();
        assert_eq!(c.as_array().unwrap().len(), THEME_COUNT);
        assert_eq!(c[0]["name"], "Default");
        assert!(c[0]["accent"].as_str().unwrap().starts_with('#'));
    }
}
