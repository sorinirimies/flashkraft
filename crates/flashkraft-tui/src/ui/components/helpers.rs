use ratatui::layout::Rect;
use ratatui::style::Color;
use tui_piechart::PieSlice;

use super::super::slice_color;
use crate::core::message::UsbEntry;

// ── Layout helpers ────────────────────────────────────────────────────────────

/// Centre a `width × height` rect inside `r`.
pub(crate) fn centred_rect(r: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: r.x + r.width.saturating_sub(width) / 2,
        y: r.y + r.height.saturating_sub(height) / 2,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

// ── File-type classification ──────────────────────────────────────────────────

pub(crate) fn classify_ext(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "iso" | "img" | "bin" | "dmg" | "vhd" | "vmdk" => "Disk Images",
        "exe" | "msi" | "deb" | "rpm" | "apk" | "appimage" => "Executables",
        "sh" | "bat" | "cmd" | "ps1" | "py" | "rb" | "pl" => "Scripts",
        "txt" | "md" | "rst" | "log" | "cfg" | "conf" | "ini" | "toml" | "yaml" | "yml"
        | "json" | "xml" => "Text / Config",
        "jpg" | "jpeg" | "png" | "gif" | "svg" | "bmp" | "ico" | "webp" => "Images",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "Video",
        "mp3" | "flac" | "ogg" | "wav" | "aac" | "m4a" => "Audio",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" => "Archives",
        "efi" | "sys" | "ko" | "so" | "dll" | "o" | "a" | "lib" => "System / Libs",
        _ => "Other",
    }
}

pub(crate) fn file_icon(name: &str) -> &'static str {
    match classify_ext(name) {
        "Disk Images" => "\u{1f4bf}",
        "Executables" => "\u{2699}",
        "Scripts" => "\u{1f4dc}",
        "Text / Config" => "\u{1f4c4}",
        "Images" => "\u{1f5bc}",
        "Video" => "\u{1f3ac}",
        "Audio" => "\u{1f3b5}",
        "Archives" => "\u{1f4e6}",
        "System / Libs" => "\u{1f527}",
        _ => "\u{1f4c4}",
    }
}

/// Build `PieSlice`s and a legend from a list of USB entries.
///
/// Returns `(slices, legend)` where each legend entry is `(label, count, color)`.
pub(crate) fn build_filetype_piechart(
    entries: &[UsbEntry],
) -> (Vec<PieSlice<'_>>, Vec<(String, usize, Color)>) {
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for e in entries {
        if !e.is_dir {
            *counts.entry(classify_ext(&e.name)).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        return (vec![], vec![]);
    }

    let total: usize = counts.values().sum();
    let mut slices = Vec::new();
    let mut legend = Vec::new();

    for (i, (label, count)) in counts.iter().enumerate() {
        let pct = *count as f64 / total as f64 * 100.0;
        let color = slice_color(i);
        slices.push(PieSlice::new(label, pct, color));
        legend.push((label.to_string(), *count, color));
    }

    (slices, legend)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_ext ──────────────────────────────────────────────────────

    #[test]
    fn classify_ext_iso() {
        assert_eq!(classify_ext("ubuntu-22.04.iso"), "Disk Images");
        assert_eq!(classify_ext("image.img"), "Disk Images");
        assert_eq!(classify_ext("disk.vmdk"), "Disk Images");
    }

    #[test]
    fn classify_ext_text_config() {
        assert_eq!(classify_ext("readme.txt"), "Text / Config");
        assert_eq!(classify_ext("config.toml"), "Text / Config");
        assert_eq!(classify_ext("data.json"), "Text / Config");
        assert_eq!(classify_ext("notes.md"), "Text / Config");
    }

    #[test]
    fn classify_ext_executables() {
        assert_eq!(classify_ext("setup.exe"), "Executables");
        assert_eq!(classify_ext("package.deb"), "Executables");
        assert_eq!(classify_ext("app.appimage"), "Executables");
    }

    #[test]
    fn classify_ext_unknown_returns_other() {
        assert_eq!(classify_ext("mystery.xyz"), "Other");
        assert_eq!(classify_ext("noextension"), "Other");
        assert_eq!(classify_ext(".hidden"), "Other");
    }

    #[test]
    fn classify_ext_case_insensitive() {
        assert_eq!(classify_ext("IMAGE.ISO"), "Disk Images");
        assert_eq!(classify_ext("README.TXT"), "Text / Config");
        assert_eq!(classify_ext("Setup.EXE"), "Executables");
    }

    // ── file_icon ─────────────────────────────────────────────────────────

    #[test]
    fn file_icon_disk_image() {
        assert_eq!(file_icon("boot.iso"), "\u{1f4bf}");
    }

    #[test]
    fn file_icon_executable() {
        assert_eq!(file_icon("setup.exe"), "\u{2699}");
    }

    #[test]
    fn file_icon_script() {
        assert_eq!(file_icon("run.sh"), "\u{1f4dc}");
    }

    #[test]
    fn file_icon_text() {
        assert_eq!(file_icon("readme.txt"), "\u{1f4c4}");
    }

    #[test]
    fn file_icon_image() {
        assert_eq!(file_icon("photo.png"), "\u{1f5bc}");
    }

    #[test]
    fn file_icon_video() {
        assert_eq!(file_icon("movie.mp4"), "\u{1f3ac}");
    }

    #[test]
    fn file_icon_audio() {
        assert_eq!(file_icon("song.mp3"), "\u{1f3b5}");
    }

    #[test]
    fn file_icon_archive() {
        assert_eq!(file_icon("backup.tar"), "\u{1f4e6}");
    }

    #[test]
    fn file_icon_system() {
        assert_eq!(file_icon("driver.sys"), "\u{1f527}");
    }

    #[test]
    fn file_icon_unknown_falls_back() {
        assert_eq!(file_icon("data.xyz"), "\u{1f4c4}");
    }

    // ── centred_rect ──────────────────────────────────────────────────────

    #[test]
    fn centred_rect_basic() {
        let outer = Rect::new(0, 0, 100, 100);
        let inner = centred_rect(outer, 50, 50);
        assert_eq!(inner.x, 25);
        assert_eq!(inner.y, 25);
        assert_eq!(inner.width, 50);
        assert_eq!(inner.height, 50);
    }

    #[test]
    fn centred_rect_larger_than_container_clamps() {
        let outer = Rect::new(0, 0, 40, 30);
        let inner = centred_rect(outer, 80, 60);
        // width and height are clamped to the container
        assert_eq!(inner.width, 40);
        assert_eq!(inner.height, 30);
    }

    #[test]
    fn centred_rect_with_offset() {
        let outer = Rect::new(10, 20, 100, 100);
        let inner = centred_rect(outer, 50, 50);
        assert_eq!(inner.x, 10 + 25);
        assert_eq!(inner.y, 20 + 25);
        assert_eq!(inner.width, 50);
        assert_eq!(inner.height, 50);
    }

    // ── build_filetype_piechart ───────────────────────────────────────────

    #[test]
    fn build_filetype_piechart_empty() {
        let entries: Vec<UsbEntry> = vec![];
        let (slices, legend) = build_filetype_piechart(&entries);
        assert!(slices.is_empty());
        assert!(legend.is_empty());
    }

    #[test]
    fn build_filetype_piechart_dirs_ignored() {
        let entries = vec![UsbEntry {
            name: "folder".into(),
            size_bytes: 0,
            is_dir: true,
            depth: 0,
        }];
        let (slices, legend) = build_filetype_piechart(&entries);
        assert!(slices.is_empty());
        assert!(legend.is_empty());
    }

    #[test]
    fn build_filetype_piechart_groups_by_category() {
        let entries = vec![
            UsbEntry {
                name: "a.iso".into(),
                size_bytes: 100,
                is_dir: false,
                depth: 0,
            },
            UsbEntry {
                name: "b.img".into(),
                size_bytes: 200,
                is_dir: false,
                depth: 0,
            },
            UsbEntry {
                name: "c.txt".into(),
                size_bytes: 50,
                is_dir: false,
                depth: 0,
            },
        ];
        let (slices, legend) = build_filetype_piechart(&entries);

        // Two categories: "Disk Images" (2 files) and "Text / Config" (1 file)
        assert_eq!(slices.len(), 2);
        assert_eq!(legend.len(), 2);

        // BTreeMap sorts keys, so "Disk Images" comes before "Text / Config"
        assert_eq!(legend[0].0, "Disk Images");
        assert_eq!(legend[0].1, 2);
        assert_eq!(legend[1].0, "Text / Config");
        assert_eq!(legend[1].1, 1);
    }
}
