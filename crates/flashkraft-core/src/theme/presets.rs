//! Named [`AppTheme`] presets — single source of truth for every theme used
//! by all FlashKraft frontends.
//!
//! The first 27 entries mirror the `tui_file_explorer::Theme::all_presets()`
//! catalogue in the same order so that saved theme indices remain compatible.
//! The 28th entry ("Cyberpunk") is an addition not present in the file-explorer library.

use super::types::{AppTheme, Rgb};

// ── Catalogue metadata ────────────────────────────────────────────────────────

/// Total number of named presets.
pub const THEME_COUNT: usize = 43;

/// Display names for all presets, in catalogue order.
pub const THEME_NAMES: [&str; THEME_COUNT] = [
    // ── Built-in ──────────────────────────────────────────────────────────────
    "Default",
    "Default Light",
    // ── Decorative ────────────────────────────────────────────────────────────
    "Grape",
    "Ocean",
    "Sunset",
    "Forest",
    "Rose",
    "Mono",
    "Neon",
    // ── Editor / terminal ─────────────────────────────────────────────────────
    "Dracula",
    "Nord",
    "Solarized Dark",
    "Solarized Light",
    "Gruvbox Dark",
    "Gruvbox Light",
    "Catppuccin Latte",
    "Catppuccin Frappé",
    "Catppuccin Macchiato",
    "Catppuccin Mocha",
    "Tokyo Night",
    "Tokyo Night Storm",
    "Tokyo Night Light",
    "Kanagawa Wave",
    "Kanagawa Dragon",
    "Kanagawa Lotus",
    "Moonfly",
    "Nightfly",
    "Oxocarbon",
    // ── FlashKraft exclusive ───────────────────────────────────────────────────
    "Cyberpunk",
    // ── From Ghostty ──────────────────────────────────────────────────────────
    "Rose Pine",
    "Rose Pine Moon",
    "Rose Pine Dawn",
    "Ayu Mirage",
    "Everforest Dark",
    "Atom One Dark",
    "Atom One Light",
    "Night Owl",
    "Poimandres",
    "Flexoki Dark",
    "Flexoki Light",
    "Carbonfox",
    "Andromeda",
    "Synthwave",
];

// ── Public API ────────────────────────────────────────────────────────────────

/// Look up a preset by index (wraps if out of range).
pub fn theme_by_index(index: usize) -> AppTheme {
    match index % THEME_COUNT {
        0 => default(),
        1 => default_light(),
        2 => grape(),
        3 => ocean(),
        4 => sunset(),
        5 => forest(),
        6 => rose(),
        7 => mono(),
        8 => neon(),
        9 => dracula(),
        10 => nord(),
        11 => solarized_dark(),
        12 => solarized_light(),
        13 => gruvbox_dark(),
        14 => gruvbox_light(),
        15 => catppuccin_latte(),
        16 => catppuccin_frappe(),
        17 => catppuccin_macchiato(),
        18 => catppuccin_mocha(),
        19 => tokyo_night(),
        20 => tokyo_night_storm(),
        21 => tokyo_night_light(),
        22 => kanagawa_wave(),
        23 => kanagawa_dragon(),
        24 => kanagawa_lotus(),
        25 => moonfly(),
        26 => nightfly(),
        27 => oxocarbon(),
        28 => cyberpunk(),
        29 => rose_pine(),
        30 => rose_pine_moon(),
        31 => rose_pine_dawn(),
        32 => ayu_mirage(),
        33 => everforest_dark(),
        34 => atom_one_dark(),
        35 => atom_one_light(),
        36 => night_owl(),
        37 => poimandres(),
        38 => flexoki_dark(),
        39 => flexoki_light(),
        40 => carbonfox(),
        41 => andromeda(),
        42 => synthwave(),
        _ => default(),
    }
}

/// Look up the index of a preset by display name. Returns `None` if not found.
pub fn theme_index_by_name(name: &str) -> Option<usize> {
    THEME_NAMES.iter().position(|n| *n == name)
}

// ── Individual theme constructors ─────────────────────────────────────────────

fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}

pub fn default() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(18, 18, 26),
        surface: rgb(28, 28, 40),
        border: rgb(60, 80, 100),
        selection: rgb(40, 60, 80),
        text_primary: rgb(255, 255, 255),
        text_secondary: rgb(255, 100, 30), // orange — secondary / accent borders
        text_muted: rgb(120, 120, 130),
        accent: rgb(80, 200, 255), // sky-blue — primary brand (matches GUI default)
        success: rgb(80, 220, 120),
        warning: rgb(255, 200, 50),
        error: rgb(255, 80, 80),
    }
}

/// Default Light — the light companion to Default: same sky-blue brand on a
/// clean off-white background with soft grey borders.
pub fn default_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(250, 250, 255), // near-white with a hint of blue
        surface: rgb(235, 238, 248),    // soft lavender-grey panels
        border: rgb(180, 195, 220),     // muted blue-grey borders
        selection: rgb(200, 220, 255),  // soft blue selection
        text_primary: rgb(30, 35, 50),  // near-black with blue tint
        text_secondary: rgb(255, 100, 30), // orange accent (same as dark)
        text_muted: rgb(130, 140, 160), // muted blue-grey
        accent: rgb(0, 140, 220),       // darker sky-blue for contrast on light bg
        success: rgb(30, 160, 80),      // darker green for light bg
        warning: rgb(200, 140, 0),      // darker amber
        error: rgb(210, 50, 50),        // darker red
    }
}

pub fn grape() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(18, 12, 30),
        surface: rgb(30, 20, 50),
        border: rgb(100, 70, 150),
        selection: rgb(50, 35, 80),
        text_primary: rgb(230, 220, 255),
        text_secondary: rgb(130, 180, 255),
        text_muted: rgb(110, 100, 130),
        accent: rgb(200, 120, 255),
        success: rgb(160, 110, 255),
        warning: rgb(210, 170, 255),
        error: rgb(255, 80, 150),
    }
}

pub fn ocean() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(0, 20, 35),
        surface: rgb(0, 35, 55),
        border: rgb(0, 100, 130),
        selection: rgb(0, 50, 70),
        text_primary: rgb(200, 240, 245),
        text_secondary: rgb(0, 175, 210),
        text_muted: rgb(80, 120, 130),
        accent: rgb(0, 200, 180),
        success: rgb(80, 230, 200),
        warning: rgb(255, 220, 80),
        error: rgb(255, 100, 100),
    }
}

pub fn sunset() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(22, 8, 6),
        surface: rgb(40, 15, 10),
        border: rgb(130, 60, 30),
        selection: rgb(80, 30, 20),
        text_primary: rgb(255, 235, 210),
        text_secondary: rgb(255, 150, 50),
        text_muted: rgb(140, 100, 80),
        accent: rgb(255, 80, 80),
        success: rgb(255, 180, 80),
        warning: rgb(255, 230, 80),
        error: rgb(255, 50, 50),
    }
}

pub fn forest() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(8, 18, 8),
        surface: rgb(15, 30, 15),
        border: rgb(50, 100, 50),
        selection: rgb(20, 50, 20),
        text_primary: rgb(210, 235, 200),
        text_secondary: rgb(80, 160, 80),
        text_muted: rgb(90, 120, 80),
        accent: rgb(100, 200, 80),
        success: rgb(120, 210, 90),
        warning: rgb(220, 200, 80),
        error: rgb(210, 80, 80),
    }
}

pub fn rose() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(28, 6, 16),
        surface: rgb(50, 12, 30),
        border: rgb(140, 60, 100),
        selection: rgb(80, 20, 40),
        text_primary: rgb(255, 230, 235),
        text_secondary: rgb(255, 140, 180),
        text_muted: rgb(140, 90, 110),
        accent: rgb(255, 100, 150),
        success: rgb(255, 160, 190),
        warning: rgb(255, 220, 180),
        error: rgb(220, 60, 100),
    }
}

pub fn mono() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(8, 8, 10),
        surface: rgb(20, 20, 22),
        border: rgb(80, 80, 85),
        selection: rgb(50, 50, 55),
        text_primary: rgb(210, 210, 210),
        text_secondary: rgb(180, 180, 180),
        text_muted: rgb(110, 110, 115),
        accent: rgb(200, 200, 200),
        success: rgb(200, 200, 200),
        warning: rgb(200, 200, 200),
        error: rgb(160, 160, 160),
    }
}

pub fn neon() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(6, 0, 14),
        surface: rgb(15, 0, 30),
        border: rgb(100, 0, 140),
        selection: rgb(30, 0, 50),
        text_primary: rgb(230, 230, 255),
        text_secondary: rgb(0, 255, 200),
        text_muted: rgb(100, 80, 120),
        accent: rgb(255, 0, 200),
        success: rgb(0, 255, 130),
        warning: rgb(255, 220, 0),
        error: rgb(255, 30, 80),
    }
}

pub fn dracula() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(40, 42, 54),
        surface: rgb(68, 71, 90),
        border: rgb(98, 114, 164),
        selection: rgb(68, 71, 90),
        text_primary: rgb(248, 248, 242),
        text_secondary: rgb(139, 233, 253),
        text_muted: rgb(98, 114, 164),
        accent: rgb(255, 121, 198),
        success: rgb(80, 250, 123),
        warning: rgb(241, 250, 140),
        error: rgb(255, 85, 85),
    }
}

pub fn nord() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(29, 35, 42),
        surface: rgb(59, 66, 82),
        border: rgb(76, 86, 106),
        selection: rgb(59, 66, 82),
        text_primary: rgb(216, 222, 233),
        text_secondary: rgb(129, 161, 193),
        text_muted: rgb(76, 86, 106),
        accent: rgb(136, 192, 208),
        success: rgb(163, 190, 140),
        warning: rgb(235, 203, 139),
        error: rgb(191, 97, 106),
    }
}

pub fn solarized_dark() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(0, 43, 54),
        surface: rgb(7, 54, 66),
        border: rgb(88, 110, 117),
        selection: rgb(7, 54, 66),
        text_primary: rgb(131, 148, 150),
        text_secondary: rgb(42, 161, 152),
        text_muted: rgb(88, 110, 117),
        accent: rgb(38, 139, 210),
        success: rgb(133, 153, 0),
        warning: rgb(181, 137, 0),
        error: rgb(220, 50, 47),
    }
}

pub fn solarized_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(253, 246, 227),
        surface: rgb(238, 232, 213),
        border: rgb(147, 161, 161),
        selection: rgb(238, 232, 213),
        text_primary: rgb(101, 123, 131),
        text_secondary: rgb(42, 161, 152),
        text_muted: rgb(147, 161, 161),
        accent: rgb(38, 139, 210),
        success: rgb(133, 153, 0),
        warning: rgb(181, 137, 0),
        error: rgb(220, 50, 47),
    }
}

pub fn gruvbox_dark() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(29, 28, 27),
        surface: rgb(60, 56, 54),
        border: rgb(146, 131, 116),
        selection: rgb(60, 56, 54),
        text_primary: rgb(235, 219, 178),
        text_secondary: rgb(250, 189, 47),
        text_muted: rgb(146, 131, 116),
        accent: rgb(254, 128, 25),
        success: rgb(184, 187, 38),
        warning: rgb(250, 189, 47),
        error: rgb(251, 73, 52),
    }
}

pub fn gruvbox_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(251, 241, 199),
        surface: rgb(213, 196, 161),
        border: rgb(146, 131, 116),
        selection: rgb(213, 196, 161),
        text_primary: rgb(60, 56, 54),
        text_secondary: rgb(215, 153, 33),
        text_muted: rgb(146, 131, 116),
        accent: rgb(214, 93, 14),
        success: rgb(121, 116, 14),
        warning: rgb(215, 153, 33),
        error: rgb(214, 93, 14),
    }
}

pub fn catppuccin_latte() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(239, 241, 245),
        surface: rgb(204, 208, 218),
        border: rgb(156, 160, 176),
        selection: rgb(204, 208, 218),
        text_primary: rgb(76, 79, 105),
        text_secondary: rgb(30, 102, 245),
        text_muted: rgb(156, 160, 176),
        accent: rgb(136, 57, 239),
        success: rgb(64, 160, 43),
        warning: rgb(223, 142, 29),
        error: rgb(210, 15, 57),
    }
}

pub fn catppuccin_frappe() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(48, 52, 70),
        surface: rgb(65, 69, 89),
        border: rgb(115, 121, 148),
        selection: rgb(65, 69, 89),
        text_primary: rgb(198, 208, 245),
        text_secondary: rgb(140, 170, 238),
        text_muted: rgb(115, 121, 148),
        accent: rgb(202, 158, 230),
        success: rgb(166, 209, 137),
        warning: rgb(229, 200, 144),
        error: rgb(231, 130, 132),
    }
}

pub fn catppuccin_macchiato() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(36, 39, 58),
        surface: rgb(54, 58, 79),
        border: rgb(110, 115, 141),
        selection: rgb(54, 58, 79),
        text_primary: rgb(202, 211, 245),
        text_secondary: rgb(138, 173, 244),
        text_muted: rgb(110, 115, 141),
        accent: rgb(198, 160, 246),
        success: rgb(166, 218, 149),
        warning: rgb(238, 212, 159),
        error: rgb(237, 135, 150),
    }
}

pub fn catppuccin_mocha() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(30, 30, 46),
        surface: rgb(49, 50, 68),
        border: rgb(108, 112, 134),
        selection: rgb(49, 50, 68),
        text_primary: rgb(205, 214, 244),
        text_secondary: rgb(137, 180, 250),
        text_muted: rgb(108, 112, 134),
        accent: rgb(203, 166, 247),
        success: rgb(166, 227, 161),
        warning: rgb(249, 226, 175),
        error: rgb(243, 139, 168),
    }
}

pub fn tokyo_night() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(26, 27, 38),
        surface: rgb(41, 46, 66),
        border: rgb(86, 95, 137),
        selection: rgb(41, 46, 66),
        text_primary: rgb(192, 202, 245),
        text_secondary: rgb(122, 162, 247),
        text_muted: rgb(86, 95, 137),
        accent: rgb(187, 154, 247),
        success: rgb(158, 206, 106),
        warning: rgb(224, 175, 104),
        error: rgb(247, 118, 142),
    }
}

pub fn tokyo_night_storm() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(36, 40, 59),
        surface: rgb(45, 49, 75),
        border: rgb(86, 95, 137),
        selection: rgb(45, 49, 75),
        text_primary: rgb(192, 202, 245),
        text_secondary: rgb(122, 162, 247),
        text_muted: rgb(86, 95, 137),
        accent: rgb(187, 154, 247),
        success: rgb(158, 206, 106),
        warning: rgb(224, 175, 104),
        error: rgb(247, 118, 142),
    }
}

pub fn tokyo_night_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(213, 214, 219),
        surface: rgb(208, 213, 227),
        border: rgb(132, 140, 176),
        selection: rgb(208, 213, 227),
        text_primary: rgb(52, 59, 88),
        text_secondary: rgb(46, 126, 233),
        text_muted: rgb(132, 140, 176),
        accent: rgb(90, 74, 120),
        success: rgb(72, 94, 48),
        warning: rgb(140, 108, 62),
        error: rgb(210, 15, 57),
    }
}

pub fn kanagawa_wave() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(22, 22, 30),
        surface: rgb(42, 42, 55),
        border: rgb(114, 113, 105),
        selection: rgb(42, 42, 55),
        text_primary: rgb(220, 215, 186),
        text_secondary: rgb(126, 156, 216),
        text_muted: rgb(114, 113, 105),
        accent: rgb(210, 126, 153),
        success: rgb(118, 148, 106),
        warning: rgb(220, 165, 97),
        error: rgb(210, 126, 153),
    }
}

pub fn kanagawa_dragon() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(20, 20, 20),
        surface: rgb(40, 39, 39),
        border: rgb(166, 166, 156),
        selection: rgb(40, 39, 39),
        text_primary: rgb(197, 201, 197),
        text_secondary: rgb(139, 164, 176),
        text_muted: rgb(166, 166, 156),
        accent: rgb(210, 126, 153),
        success: rgb(135, 169, 135),
        warning: rgb(200, 170, 109),
        error: rgb(210, 126, 153),
    }
}

pub fn kanagawa_lotus() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(246, 243, 228),
        surface: rgb(231, 219, 160),
        border: rgb(196, 178, 138),
        selection: rgb(231, 219, 160),
        text_primary: rgb(84, 84, 100),
        text_secondary: rgb(77, 105, 155),
        text_muted: rgb(196, 178, 138),
        accent: rgb(160, 154, 190),
        success: rgb(111, 137, 78),
        warning: rgb(119, 113, 63),
        error: rgb(192, 71, 71),
    }
}

pub fn moonfly() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(8, 8, 8),
        surface: rgb(28, 28, 28),
        border: rgb(78, 78, 78),
        selection: rgb(28, 28, 28),
        text_primary: rgb(178, 178, 178),
        text_secondary: rgb(128, 160, 255),
        text_muted: rgb(78, 78, 78),
        accent: rgb(174, 129, 255),
        success: rgb(140, 200, 95),
        warning: rgb(226, 164, 120),
        error: rgb(255, 115, 131),
    }
}

pub fn nightfly() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(1, 22, 38),
        surface: rgb(11, 41, 66),
        border: rgb(75, 100, 121),
        selection: rgb(11, 41, 66),
        text_primary: rgb(172, 187, 203),
        text_secondary: rgb(130, 170, 255),
        text_muted: rgb(75, 100, 121),
        accent: rgb(199, 146, 234),
        success: rgb(161, 205, 94),
        warning: rgb(243, 218, 11),
        error: rgb(252, 87, 73),
    }
}

pub fn oxocarbon() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(22, 22, 22),
        surface: rgb(38, 38, 38),
        border: rgb(82, 82, 82),
        selection: rgb(38, 38, 38),
        text_primary: rgb(242, 244, 248),
        text_secondary: rgb(120, 169, 255),
        text_muted: rgb(82, 82, 82),
        accent: rgb(255, 126, 182),
        success: rgb(66, 190, 101),
        warning: rgb(250, 204, 55),
        error: rgb(255, 97, 101),
    }
}

/// Cyberpunk — inspired by Cyberpunk 2077's signature electric yellow
/// accent with cyan neon highlights against a deep dark background.
pub fn cyberpunk() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(10, 10, 16),
        surface: rgb(20, 20, 30),
        border: rgb(45, 45, 55),
        selection: rgb(50, 48, 20), // warm yellow-tinted selection
        text_primary: rgb(230, 230, 220),
        text_secondary: rgb(0, 210, 235), // cyan neon
        text_muted: rgb(90, 90, 100),
        accent: rgb(252, 238, 10), // electric yellow — CP2077 signature
        success: rgb(0, 220, 180),
        warning: rgb(255, 150, 0),
        error: rgb(255, 50, 70),
    }
}

pub fn rose_pine() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(25, 23, 36),
        surface: rgb(38, 35, 55),
        border: rgb(110, 106, 134),
        selection: rgb(64, 61, 82),
        text_primary: rgb(224, 222, 244),
        text_secondary: rgb(156, 207, 216),
        text_muted: rgb(110, 106, 134),
        accent: rgb(196, 167, 231),
        success: rgb(49, 116, 143),
        warning: rgb(246, 193, 119),
        error: rgb(235, 111, 146),
    }
}

pub fn rose_pine_moon() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(35, 33, 54),
        surface: rgb(48, 46, 70),
        border: rgb(110, 106, 134),
        selection: rgb(68, 65, 90),
        text_primary: rgb(224, 222, 244),
        text_secondary: rgb(156, 207, 216),
        text_muted: rgb(110, 106, 134),
        accent: rgb(196, 167, 231),
        success: rgb(62, 143, 176),
        warning: rgb(246, 193, 119),
        error: rgb(235, 111, 146),
    }
}

pub fn rose_pine_dawn() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(250, 244, 237),
        surface: rgb(242, 233, 221),
        border: rgb(152, 147, 165),
        selection: rgb(223, 218, 217),
        text_primary: rgb(87, 82, 121),
        text_secondary: rgb(86, 148, 159),
        text_muted: rgb(152, 147, 165),
        accent: rgb(144, 122, 169),
        success: rgb(40, 105, 131),
        warning: rgb(234, 157, 52),
        error: rgb(180, 99, 122),
    }
}

pub fn ayu_mirage() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(31, 36, 48),
        surface: rgb(42, 48, 62),
        border: rgb(104, 104, 104),
        selection: rgb(64, 159, 255),
        text_primary: rgb(204, 202, 194),
        text_secondary: rgb(115, 208, 255),
        text_muted: rgb(104, 104, 104),
        accent: rgb(115, 208, 255),
        success: rgb(135, 217, 108),
        warning: rgb(250, 204, 110),
        error: rgb(237, 130, 116),
    }
}

pub fn everforest_dark() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(30, 35, 38),
        surface: rgb(42, 48, 50),
        border: rgb(166, 176, 160),
        selection: rgb(76, 55, 67),
        text_primary: rgb(211, 198, 170),
        text_secondary: rgb(127, 187, 179),
        text_muted: rgb(122, 132, 120),
        accent: rgb(167, 192, 128),
        success: rgb(167, 192, 128),
        warning: rgb(219, 188, 127),
        error: rgb(230, 126, 128),
    }
}

pub fn atom_one_dark() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(33, 37, 43),
        surface: rgb(45, 50, 58),
        border: rgb(118, 118, 118),
        selection: rgb(50, 56, 68),
        text_primary: rgb(171, 178, 191),
        text_secondary: rgb(97, 175, 239),
        text_muted: rgb(118, 118, 118),
        accent: rgb(97, 175, 239),
        success: rgb(152, 195, 121),
        warning: rgb(229, 192, 123),
        error: rgb(224, 108, 117),
    }
}

pub fn atom_one_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(249, 249, 249),
        surface: rgb(237, 237, 237),
        border: rgb(180, 180, 180),
        selection: rgb(237, 237, 237),
        text_primary: rgb(42, 44, 51),
        text_secondary: rgb(47, 90, 243),
        text_muted: rgb(118, 118, 118),
        accent: rgb(47, 90, 243),
        success: rgb(63, 149, 58),
        warning: rgb(210, 182, 124),
        error: rgb(222, 62, 53),
    }
}

pub fn night_owl() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(1, 22, 39),
        surface: rgb(12, 35, 55),
        border: rgb(87, 86, 86),
        selection: rgb(95, 126, 151),
        text_primary: rgb(214, 222, 235),
        text_secondary: rgb(130, 170, 255),
        text_muted: rgb(87, 86, 86),
        accent: rgb(130, 170, 255),
        success: rgb(34, 218, 110),
        warning: rgb(173, 219, 103),
        error: rgb(239, 83, 80),
    }
}

pub fn poimandres() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(26, 30, 40),
        surface: rgb(38, 42, 55),
        border: rgb(100, 106, 130),
        selection: rgb(50, 55, 75),
        text_primary: rgb(166, 172, 205),
        text_secondary: rgb(137, 221, 255),
        text_muted: rgb(100, 106, 130),
        accent: rgb(93, 228, 199),
        success: rgb(93, 228, 199),
        warning: rgb(255, 250, 194),
        error: rgb(208, 103, 157),
    }
}

pub fn flexoki_dark() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(16, 15, 15),
        surface: rgb(30, 28, 28),
        border: rgb(87, 86, 83),
        selection: rgb(64, 62, 60),
        text_primary: rgb(206, 205, 195),
        text_secondary: rgb(67, 133, 190),
        text_muted: rgb(87, 86, 83),
        accent: rgb(67, 133, 190),
        success: rgb(135, 154, 57),
        warning: rgb(208, 162, 21),
        error: rgb(209, 77, 65),
    }
}

pub fn flexoki_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: rgb(255, 252, 240),
        surface: rgb(242, 238, 222),
        border: rgb(183, 181, 172),
        selection: rgb(206, 205, 195),
        text_primary: rgb(16, 15, 15),
        text_secondary: rgb(32, 94, 166),
        text_muted: rgb(111, 110, 105),
        accent: rgb(32, 94, 166),
        success: rgb(102, 128, 11),
        warning: rgb(173, 131, 1),
        error: rgb(175, 48, 41),
    }
}

pub fn carbonfox() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(22, 22, 22),
        surface: rgb(35, 35, 35),
        border: rgb(72, 72, 72),
        selection: rgb(42, 42, 42),
        text_primary: rgb(242, 244, 248),
        text_secondary: rgb(120, 169, 255),
        text_muted: rgb(100, 100, 110),
        accent: rgb(120, 169, 255),
        success: rgb(37, 190, 106),
        warning: rgb(8, 189, 186),
        error: rgb(238, 83, 150),
    }
}

pub fn andromeda() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(38, 42, 51),
        surface: rgb(50, 55, 66),
        border: rgb(102, 102, 102),
        selection: rgb(90, 92, 98),
        text_primary: rgb(229, 229, 229),
        text_secondary: rgb(15, 168, 205),
        text_muted: rgb(102, 102, 102),
        accent: rgb(5, 188, 121),
        success: rgb(5, 188, 121),
        warning: rgb(229, 229, 18),
        error: rgb(205, 49, 49),
    }
}

pub fn synthwave() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: rgb(10, 8, 16),
        surface: rgb(20, 16, 30),
        border: rgb(127, 112, 148),
        selection: rgb(25, 50, 60),
        text_primary: rgb(218, 217, 199),
        text_secondary: rgb(18, 195, 226),
        text_muted: rgb(127, 112, 148),
        accent: rgb(246, 24, 143),
        success: rgb(30, 187, 43),
        warning: rgb(253, 248, 52),
        error: rgb(246, 24, 143),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_count_matches_names() {
        assert_eq!(THEME_NAMES.len(), THEME_COUNT);
    }

    #[test]
    fn all_themes_resolve() {
        for i in 0..THEME_COUNT {
            let t = theme_by_index(i);
            let has_colour = t.text_primary.r > 0 || t.text_primary.g > 0 || t.text_primary.b > 0;
            assert!(has_colour, "theme index {i} has zero text_primary");
        }
    }

    #[test]
    fn cyberpunk_is_dark() {
        assert!(cyberpunk().is_dark);
    }

    #[test]
    fn cyberpunk_is_at_index_28() {
        assert_eq!(THEME_NAMES[1], "Default Light");
        assert_eq!(THEME_NAMES[28], "Cyberpunk");
        assert_eq!(THEME_NAMES[THEME_COUNT - 1], "Synthwave");
    }

    #[test]
    fn default_is_first() {
        assert_eq!(THEME_NAMES[0], "Default");
    }

    #[test]
    fn out_of_range_wraps() {
        let a = theme_by_index(THEME_COUNT);
        let b = theme_by_index(0);
        assert_eq!(a.background, b.background);
    }

    #[test]
    fn theme_index_by_name_finds_cyberpunk() {
        assert_eq!(theme_index_by_name("Cyberpunk"), Some(28));
    }

    #[test]
    fn theme_index_by_name_unknown_returns_none() {
        assert!(theme_index_by_name("NotATheme").is_none());
    }

    #[test]
    fn rose_pine_is_dark() {
        assert!(rose_pine().is_dark);
    }

    #[test]
    fn rose_pine_dawn_is_light() {
        assert!(!rose_pine_dawn().is_dark);
    }

    #[test]
    fn atom_one_light_is_light() {
        assert!(!atom_one_light().is_dark);
    }

    #[test]
    fn flexoki_light_is_light() {
        assert!(!flexoki_light().is_dark);
    }

    #[test]
    fn synthwave_accent_is_pink() {
        use crate::Rgb;
        assert_eq!(synthwave().accent, Rgb::new(246, 24, 143));
    }
}
