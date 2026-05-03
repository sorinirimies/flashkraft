//! Image Information Domain Model
//!
//! This module contains the ImageInfo struct which represents
//! information about a disk image file.

use std::path::PathBuf;

/// Information about a disk image file
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// Full path to the image file
    pub path: PathBuf,
    /// Display name of the file
    pub name: String,
    /// Size of the file in megabytes
    pub size_mb: f64,
}

impl ImageInfo {
    /// Return the image size in bytes.
    pub fn size_bytes(&self) -> u64 {
        (self.size_mb * 1_048_576.0) as u64
    }

    /// Create a new ImageInfo from a PathBuf
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the image file
    ///
    /// # Returns
    ///
    /// An ImageInfo instance with extracted file name and size
    pub fn from_path(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let size_mb = path
            .metadata()
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);

        Self {
            path,
            name,
            size_mb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_image_info_from_path() {
        // Create a temporary file for testing
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("test_image.img");

        // Write some data to get a known size
        if let Ok(mut file) = File::create(&test_path) {
            let _ = file.write_all(&[0u8; 1024 * 1024]); // 1 MB
        }

        let image_info = ImageInfo::from_path(test_path.clone());

        assert_eq!(image_info.name, "test_image.img");
        assert_eq!(image_info.path, test_path);
        // Size should be approximately 1 MB
        assert!((image_info.size_mb - 1.0).abs() < 0.1);

        // Cleanup
        let _ = std::fs::remove_file(test_path);
    }

    #[test]
    fn test_image_info_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/test.img");
        let image_info = ImageInfo::from_path(path);

        assert_eq!(image_info.name, "test.img");
        assert_eq!(image_info.size_mb, 0.0);
    }
}
