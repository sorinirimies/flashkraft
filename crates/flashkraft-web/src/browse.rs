//! Server-side file browser for picking an image.
//!
//! The browser cannot hand the server a real path, so the page asks the
//! server to list directories. Only directories and image-like files are
//! shown, hidden entries are skipped and the call needs the same `Host` +
//! token guards as flashing (see [`crate::flash::check_caller`]).

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Extensions offered as images (compressed ones included).
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "iso", "img", "raw", "bin", "dmg", "wic", "vhd", "qcow2", "xz", "gz", "bz2", "zst", "zip",
];
/// Most entries returned for one directory.
pub const MAX_ENTRIES: usize = 2000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    /// `dir` | `image`
    pub kind: &'static str,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Listing {
    /// Canonical path being shown.
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<Entry>,
    /// More entries existed than [`MAX_ENTRIES`].
    pub truncated: bool,
}

pub fn is_image_name(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Where the browser opens when no path is given: `~/Downloads`, else `~`, else `/`.
pub fn default_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    home.iter()
        .map(|h| h.join("Downloads"))
        .find(|d| d.is_dir())
        .or(home)
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// List `raw` (empty → [`default_dir`]). Directories first, then images, each
/// sorted case-insensitively.
pub fn list_dir(raw: &str) -> Result<Listing, String> {
    let raw = raw.trim();
    let wanted = if raw.is_empty() {
        default_dir()
    } else if let Some(rest) = raw.strip_prefix("~/") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(rest))
            .unwrap_or_else(|| raw.into())
    } else {
        PathBuf::from(raw)
    };
    let path = wanted
        .canonicalize()
        .map_err(|e| format!("cannot open {}: {e}", wanted.display()))?;
    if !path.is_dir() {
        return Err(format!("{} is not a directory", path.display()));
    }

    let mut entries = Vec::new();
    let mut truncated = false;
    let rd =
        std::fs::read_dir(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    for item in rd.flatten() {
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        // `metadata` follows symlinks, so linked images/dirs show up too.
        let Ok(meta) = item.path().metadata() else {
            continue;
        };
        let entry = if meta.is_dir() {
            Entry {
                name,
                kind: "dir",
                size: 0,
            }
        } else if meta.is_file() && is_image_name(&name) {
            Entry {
                name,
                kind: "image",
                size: meta.len(),
            }
        } else {
            continue;
        };
        if entries.len() >= MAX_ENTRIES {
            truncated = true;
            break;
        }
        entries.push(entry);
    }
    entries.sort_by(|a, b| {
        (a.kind != "dir", a.name.to_lowercase()).cmp(&(b.kind != "dir", b.name.to_lowercase()))
    });

    Ok(Listing {
        parent: path.parent().map(|p| p.display().to_string()),
        path: path.display().to_string(),
        entries,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_names_by_extension() {
        for n in ["a.iso", "A.ISO", "x.img.xz", "d.qcow2", "z.zip"] {
            assert!(is_image_name(n), "{n}");
        }
        for n in ["a.txt", "iso", "noext", "a.iso.txt", ".iso"] {
            assert!(!is_image_name(n), "{n}");
        }
    }

    #[test]
    fn lists_dirs_then_images_and_hides_the_rest() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path();
        std::fs::create_dir(p.join("Zeta")).unwrap();
        std::fs::create_dir(p.join("alpha")).unwrap();
        std::fs::create_dir(p.join(".hidden")).unwrap();
        std::fs::write(p.join("b.iso"), vec![0u8; 10]).unwrap();
        std::fs::write(p.join("A.img"), b"x").unwrap();
        std::fs::write(p.join("notes.txt"), b"x").unwrap();
        std::fs::write(p.join(".secret.iso"), b"x").unwrap();

        let l = list_dir(p.to_str().unwrap()).unwrap();
        let names: Vec<_> = l
            .entries
            .iter()
            .map(|e| (e.name.as_str(), e.kind))
            .collect();
        assert_eq!(
            names,
            [
                ("alpha", "dir"),
                ("Zeta", "dir"),
                ("A.img", "image"),
                ("b.iso", "image")
            ]
        );
        assert_eq!(l.entries[3].size, 10);
        assert!(l.parent.is_some());
        assert!(!l.truncated);
    }

    #[test]
    fn errors_are_reported_not_panicked() {
        assert!(list_dir("/definitely/not/here").is_err());
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("f.iso");
        std::fs::write(&f, b"x").unwrap();
        assert!(
            list_dir(f.to_str().unwrap())
                .unwrap_err()
                .contains("not a directory")
        );
    }

    #[test]
    fn empty_path_opens_a_default_directory() {
        assert!(list_dir("").is_ok());
    }

    #[test]
    fn dotdot_is_canonicalised() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir(t.path().join("a")).unwrap();
        let l = list_dir(&format!("{}/a/..", t.path().display())).unwrap();
        assert_eq!(
            l.path,
            t.path().canonicalize().unwrap().display().to_string()
        );
    }
}
