//! Drive Constraints Module
//!
//! This module contains validation logic for checking drive/image compatibility
//! based on various constraints similar to Etcher's constraint checking.

use crate::domain::{DriveInfo, ImageInfo};

/// The default unknown size for things such as images and drives
const UNKNOWN_SIZE: f64 = 0.0;

/// 128GB threshold for large drive warnings
pub const LARGE_DRIVE_SIZE: f64 = 128.0;

/// Compatibility status types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatusType {
    /// Warning status - user can still proceed but should be cautious
    Warning,
    /// Error status - drive should not be selectable
    Error,
}

/// Compatibility status with type and message
#[derive(Debug, Clone)]
pub struct CompatibilityStatus {
    /// Type of status (warning or error)
    pub status_type: CompatibilityStatusType,
    /// Human-readable message describing the issue
    pub message: String,
}

impl CompatibilityStatus {
    /// Create a new compatibility status
    pub fn new(status_type: CompatibilityStatusType, message: String) -> Self {
        Self {
            status_type,
            message,
        }
    }

    /// Create an error status
    pub fn error(message: String) -> Self {
        Self::new(CompatibilityStatusType::Error, message)
    }

    /// Create a warning status
    pub fn warning(message: String) -> Self {
        Self::new(CompatibilityStatusType::Warning, message)
    }
}

/// Check if a drive is a system drive
///
/// In the context of FlashKraft, a system drive is one that contains
/// the operating system or critical system files.
pub fn is_system_drive(drive: &DriveInfo) -> bool {
    drive.is_system
}

/// Check if a drive is the source drive
///
/// A source drive is one that contains the image file being flashed.
pub fn is_source_drive(drive: &DriveInfo, image: Option<&ImageInfo>) -> bool {
    if let Some(img) = image {
        let image_path = img.path.as_path();

        // `Path::starts_with` compares path components, avoiding lexical false
        // positives such as `/media/usb2` matching `/media/usb`.
        if !drive.mount_point.is_empty()
            && drive.mount_point != drive.device_path
            && image_path.starts_with(std::path::Path::new(&drive.mount_point))
        {
            return true;
        }

        if image_path.starts_with(std::path::Path::new(&drive.device_path)) {
            return true;
        }
    }
    false
}

/// Check if a drive is large enough for an image
///
/// Returns true if the drive has sufficient space for the image,
/// or if no image is provided (always valid).
pub fn is_drive_large_enough(drive: &DriveInfo, image: Option<&ImageInfo>) -> bool {
    let drive_size_gb = if drive.size_gb > 0.0 {
        drive.size_gb
    } else {
        UNKNOWN_SIZE
    };

    if let Some(img) = image {
        let image_size_gb = img.size_mb / 1024.0;

        // If we don't know the drive size, assume it's not large enough
        if drive_size_gb <= 0.0 {
            return false;
        }

        // Drive must be at least as large as the image
        return drive_size_gb >= image_size_gb;
    }

    // No image provided, so any drive is "large enough"
    true
}

/// Check if a drive meets the recommended size
///
/// Some images may specify a recommended drive size larger than
/// the image itself (for performance or other reasons).
pub fn is_drive_size_recommended(_drive: &DriveInfo, _image: Option<&ImageInfo>) -> bool {
    // For now, we don't have a recommendedDriveSize field in ImageInfo
    // So we'll just return true. This can be extended later.
    true
}

/// Check if a drive's size is considered "large"
///
/// Large drives (>128GB) trigger a warning to prevent accidental
/// formatting of large storage devices.
pub fn is_drive_size_large(drive: &DriveInfo) -> bool {
    drive.size_gb > LARGE_DRIVE_SIZE
}

/// Check if a drive is valid for flashing
///
/// A drive is valid if it's not disabled, large enough for the image,
/// and doesn't contain the source image.
pub fn is_drive_valid(drive: &DriveInfo, image: Option<&ImageInfo>) -> bool {
    !drive.disabled
        && !drive.is_system
        && !drive.is_read_only
        && is_drive_large_enough(drive, image)
        && !is_source_drive(drive, image)
}

/// Get all compatibility statuses for a drive/image pair
///
/// Returns a list of compatibility issues, which may be empty if
/// the drive is fully compatible.
pub fn get_drive_image_compatibility_statuses(
    drive: &DriveInfo,
    image: Option<&ImageInfo>,
) -> Vec<CompatibilityStatus> {
    let mut statuses = Vec::new();

    // Check if drive is locked/read-only
    if drive.is_read_only {
        statuses.push(CompatibilityStatus::error(
            "This drive is read-only and cannot be flashed.".to_string(),
        ));
    }

    // Check if drive is too small
    if !is_drive_large_enough(drive, image) {
        if let Some(img) = image {
            statuses.push(CompatibilityStatus::error(format!(
                "Drive is too small. Need at least {:.2} GB, but drive is {:.2} GB.",
                img.size_mb / 1024.0,
                drive.size_gb
            )));
        } else {
            statuses.push(CompatibilityStatus::error(
                "Drive is too small for the image.".to_string(),
            ));
        }
    } else {
        // Only check these if drive is large enough

        // System drives must never be selectable.
        if is_system_drive(drive) {
            statuses.push(CompatibilityStatus::error(
                "This is a system drive and cannot be flashed.".to_string(),
            ));
        } else if is_drive_size_large(drive) {
            // Check if drive is very large (warning)
            statuses.push(CompatibilityStatus::warning(format!(
                "This drive is larger than {}GB. Are you sure this is the right drive?",
                LARGE_DRIVE_SIZE as i32
            )));
        }

        // Check if drive contains the source image
        if is_source_drive(drive, image) {
            statuses.push(CompatibilityStatus::error(
                "This drive contains the source image and cannot be selected.".to_string(),
            ));
        }

        // Check recommended size
        if !is_drive_size_recommended(drive, image) {
            statuses.push(CompatibilityStatus::warning(
                "Drive size is smaller than recommended for optimal performance.".to_string(),
            ));
        }
    }

    statuses
}

/// Get compatibility statuses for a list of drives
///
/// Returns all compatibility statuses across all drives.
pub fn get_list_drive_image_compatibility_statuses(
    drives: &[DriveInfo],
    image: Option<&ImageInfo>,
) -> Vec<CompatibilityStatus> {
    drives
        .iter()
        .flat_map(|drive| get_drive_image_compatibility_statuses(drive, image))
        .collect()
}

/// Check if a drive has any compatibility issues
///
/// Returns true if there are any compatibility statuses (warnings or errors).
pub fn has_drive_image_compatibility_status(drive: &DriveInfo, image: Option<&ImageInfo>) -> bool {
    !get_drive_image_compatibility_statuses(drive, image).is_empty()
}

/// Mark drives as disabled based on compatibility checks
///
/// This updates the `disabled` field on drives that have error-level
/// compatibility issues. Drives with only warnings remain enabled.
pub fn mark_invalid_drives(drives: &mut [DriveInfo], image: Option<&ImageInfo>) {
    for drive in drives.iter_mut() {
        let statuses = get_drive_image_compatibility_statuses(drive, image);

        // Disable if there are any error-level statuses
        drive.disabled = statuses
            .iter()
            .any(|s| s.status_type == CompatibilityStatusType::Error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_drive(size_gb: f64, is_system: bool, is_read_only: bool) -> DriveInfo {
        DriveInfo::with_constraints(
            "Test Drive".to_string(),
            "/media/test".to_string(),
            size_gb,
            "/dev/sdb".to_string(),
            is_system,
            is_read_only,
        )
    }

    fn create_test_image(size_mb: f64) -> ImageInfo {
        ImageInfo {
            path: PathBuf::from("/tmp/test.img"),
            name: "test.img".to_string(),
            size_mb,
        }
    }

    #[test]
    fn test_is_drive_large_enough() {
        let drive = create_test_drive(16.0, false, false);
        let image = create_test_image(8000.0); // 8GB

        assert!(is_drive_large_enough(&drive, Some(&image)));
    }

    #[test]
    fn test_is_drive_too_small() {
        let drive = create_test_drive(4.0, false, false);
        let image = create_test_image(8000.0); // 8GB

        assert!(!is_drive_large_enough(&drive, Some(&image)));
    }

    #[test]
    fn test_is_drive_size_large() {
        let small_drive = create_test_drive(64.0, false, false);
        let large_drive = create_test_drive(256.0, false, false);

        assert!(!is_drive_size_large(&small_drive));
        assert!(is_drive_size_large(&large_drive));
    }

    #[test]
    fn test_system_drive_error() {
        let drive = create_test_drive(32.0, true, false);
        let image = create_test_image(4000.0); // 4GB

        let statuses = get_drive_image_compatibility_statuses(&drive, Some(&image));

        assert!(!statuses.is_empty());
        assert!(statuses
            .iter()
            .any(|s| s.status_type == CompatibilityStatusType::Error));
        assert!(!is_drive_valid(&drive, Some(&image)));
    }

    #[test]
    fn test_source_drive_uses_path_components() {
        let drive = create_test_drive(32.0, false, false);
        let on_drive = ImageInfo {
            path: PathBuf::from("/media/test/images/os.img"),
            name: "os.img".into(),
            size_mb: 1.0,
        };
        let similar_prefix = ImageInfo {
            path: PathBuf::from("/media/test-backup/os.img"),
            name: "os.img".into(),
            size_mb: 1.0,
        };

        assert!(is_source_drive(&drive, Some(&on_drive)));
        assert!(!is_source_drive(&drive, Some(&similar_prefix)));
    }

    #[test]
    fn test_read_only_drive_error() {
        let drive = create_test_drive(32.0, false, true);
        let image = create_test_image(4000.0); // 4GB

        let statuses = get_drive_image_compatibility_statuses(&drive, Some(&image));

        assert!(!statuses.is_empty());
        assert!(statuses
            .iter()
            .any(|s| s.status_type == CompatibilityStatusType::Error));
    }

    #[test]
    fn test_mark_invalid_drives() {
        let drives = vec![
            create_test_drive(32.0, false, false), // Valid
            create_test_drive(2.0, false, false),  // Too small
            create_test_drive(16.0, false, true),  // Read-only
        ];

        let image = create_test_image(4000.0); // 4GB

        let mut drives_mut = drives;
        mark_invalid_drives(&mut drives_mut, Some(&image));

        assert!(!drives_mut[0].disabled); // Valid drive
        assert!(drives_mut[1].disabled); // Too small
        assert!(drives_mut[2].disabled); // Read-only
    }

    #[test]
    fn test_is_drive_valid() {
        let mut valid_drive = create_test_drive(32.0, false, false);
        let invalid_drive = create_test_drive(2.0, false, false);
        let image = create_test_image(4000.0); // 4GB

        assert!(is_drive_valid(&valid_drive, Some(&image)));
        assert!(!is_drive_valid(&invalid_drive, Some(&image)));

        // Test disabled flag
        valid_drive.disabled = true;
        assert!(!is_drive_valid(&valid_drive, Some(&image)));
    }
}
