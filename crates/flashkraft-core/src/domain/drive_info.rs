//! Drive Information Domain Model
//!
//! This module contains the DriveInfo struct which represents
//! information about a storage drive in the system.

// ---------------------------------------------------------------------------
// USB metadata (populated by sysfs / diskutil / wmic enumeration)
// ---------------------------------------------------------------------------

/// Rich metadata sourced from USB descriptors via the native OS enumeration
/// (sysfs on Linux, `system_profiler` on macOS, wmic PNPDeviceID on Windows).
///
/// Only present when the drive was detected as a USB Mass Storage device.
/// Internal SATA / NVMe / eMMC drives will have `usb_info: None`.
#[derive(Debug, Clone)]
pub struct UsbInfo {
    /// USB Vendor ID (idVendor), e.g. 0x0781 = SanDisk.
    pub vendor_id: u16,
    /// USB Product ID (idProduct), e.g. 0x5581 = Ultra.
    pub product_id: u16,
    /// Manufacturer string from USB descriptor, if available.
    pub manufacturer: Option<String>,
    /// Product string from USB descriptor, if available.
    pub product: Option<String>,
    /// Serial number string from USB descriptor, if available.
    pub serial: Option<String>,
    /// Human-readable connection speed, e.g. `"SuperSpeed (5 Gbps)"`.
    pub speed: Option<String>,
}

impl UsbInfo {
    /// Build a display label from the available USB descriptor strings.
    ///
    /// Priority: `"{manufacturer} {product}"` → `"{product}"` → `"{vendor_id}:{product_id}"`.
    pub fn display_label(&self) -> String {
        match (&self.manufacturer, &self.product) {
            (Some(mfr), Some(prd)) => format!("{} {}", mfr.trim(), prd.trim()),
            (None, Some(prd)) => prd.trim().to_string(),
            (Some(mfr), None) => mfr.trim().to_string(),
            (None, None) => format!("{:04x}:{:04x}", self.vendor_id, self.product_id),
        }
    }
}

// ---------------------------------------------------------------------------
// DriveInfo
// ---------------------------------------------------------------------------

/// Information about a storage drive visible to the system.
#[derive(Debug, Clone)]
pub struct DriveInfo {
    /// Human-readable display name (model, vendor, or device node).
    pub name: String,
    /// Mount point of the drive or one of its partitions.
    /// Falls back to the device path when not mounted.
    pub mount_point: String,
    /// Drive capacity in gigabytes.
    pub size_gb: f64,
    /// Kernel block device path, e.g. `/dev/sdb`.
    pub device_path: String,
    /// `true` when the drive (or one of its partitions) is mounted at a
    /// critical system location (`/`, `/boot`, `/usr`, …).
    pub is_system: bool,
    /// `true` when the kernel reports the device as read-only (`/sys/block/X/ro == 1`).
    pub is_read_only: bool,
    /// `true` when a constraint check determined this drive must not be
    /// selected (too small, read-only, source drive, …).
    pub disabled: bool,
    /// USB descriptor metadata — `Some` for USB drives, `None` for internal
    /// SATA / NVMe / eMMC devices.
    pub usb_info: Option<UsbInfo>,
}

impl DriveInfo {
    /// Return the drive capacity in bytes.
    pub fn size_bytes(&self) -> u64 {
        (self.size_gb * 1_073_741_824.0) as u64
    }

    /// Create a `DriveInfo` with the four essential fields.
    ///
    /// `is_system`, `is_read_only`, `disabled`, and `usb_info` all default
    /// to `false` / `None`.
    pub fn new(name: String, mount_point: String, size_gb: f64, device_path: String) -> Self {
        Self {
            name,
            mount_point,
            size_gb,
            device_path,
            is_system: false,
            is_read_only: false,
            disabled: false,
            usb_info: None,
        }
    }

    /// Create a `DriveInfo` with constraint flags set explicitly.
    ///
    /// `disabled` defaults to `false`; `usb_info` defaults to `None`.
    pub fn with_constraints(
        name: String,
        mount_point: String,
        size_gb: f64,
        device_path: String,
        is_system: bool,
        is_read_only: bool,
    ) -> Self {
        Self {
            name,
            mount_point,
            size_gb,
            device_path,
            is_system,
            is_read_only,
            disabled: false,
            usb_info: None,
        }
    }

    /// Attach USB descriptor metadata to this drive and return `self`.
    ///
    /// Intended for use in a builder chain:
    /// ```rust
    /// # use flashkraft_core::domain::drive_info::{DriveInfo, UsbInfo};
    /// let drive = DriveInfo::new(
    ///     "SanDisk Ultra".into(), "/dev/sdb".into(), 32.0, "/dev/sdb".into(),
    /// )
    /// .with_usb_info(UsbInfo {
    ///     vendor_id: 0x0781,
    ///     product_id: 0x5581,
    ///     manufacturer: Some("SanDisk".into()),
    ///     product: Some("Ultra".into()),
    ///     serial: Some("AA01234567890".into()),
    ///     speed: Some("SuperSpeed (5 Gbps)".into()),
    /// });
    /// assert!(drive.usb_info.is_some());
    /// ```
    pub fn with_usb_info(mut self, info: UsbInfo) -> Self {
        self.usb_info = Some(info);
        self
    }

    /// Return `true` if this is a USB-attached drive (has USB descriptor info).
    pub fn is_usb(&self) -> bool {
        self.usb_info.is_some()
    }
}

impl PartialEq for DriveInfo {
    /// Two drives are equal when they refer to the same kernel block device.
    fn eq(&self, other: &Self) -> bool {
        self.device_path == other.device_path
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn usb_info_fixture() -> UsbInfo {
        UsbInfo {
            vendor_id: 0x0781,
            product_id: 0x5581,
            manufacturer: Some("SanDisk".into()),
            product: Some("Ultra".into()),
            serial: Some("SN123456".into()),
            speed: Some("SuperSpeed (5 Gbps)".into()),
        }
    }

    #[test]
    fn test_new_defaults() {
        let d = DriveInfo::new(
            "USB Drive".into(),
            "/media/usb".into(),
            32.0,
            "/dev/sdb".into(),
        );
        assert!(!d.is_system);
        assert!(!d.is_read_only);
        assert!(!d.disabled);
        assert!(d.usb_info.is_none());
        assert!(!d.is_usb());
    }

    #[test]
    fn test_with_constraints_defaults_usb_info_none() {
        let d = DriveInfo::with_constraints(
            "USB".into(),
            "/media/usb".into(),
            32.0,
            "/dev/sdb".into(),
            true,
            false,
        );
        assert!(d.is_system);
        assert!(!d.is_read_only);
        assert!(d.usb_info.is_none());
    }

    #[test]
    fn test_with_usb_info_builder() {
        let d = DriveInfo::new(
            "SanDisk Ultra".into(),
            "/media/usb".into(),
            32.0,
            "/dev/sdb".into(),
        )
        .with_usb_info(usb_info_fixture());

        assert!(d.is_usb());
        let info = d.usb_info.as_ref().unwrap();
        assert_eq!(info.vendor_id, 0x0781);
        assert_eq!(info.product_id, 0x5581);
        assert_eq!(info.serial.as_deref(), Some("SN123456"));
        assert_eq!(info.speed.as_deref(), Some("SuperSpeed (5 Gbps)"));
    }

    #[test]
    fn test_usb_info_display_label_both() {
        let info = usb_info_fixture();
        assert_eq!(info.display_label(), "SanDisk Ultra");
    }

    #[test]
    fn test_usb_info_display_label_product_only() {
        let info = UsbInfo {
            vendor_id: 0x1234,
            product_id: 0x5678,
            manufacturer: None,
            product: Some("MyDrive".into()),
            serial: None,
            speed: None,
        };
        assert_eq!(info.display_label(), "MyDrive");
    }

    #[test]
    fn test_usb_info_display_label_fallback_to_ids() {
        let info = UsbInfo {
            vendor_id: 0x1234,
            product_id: 0xabcd,
            manufacturer: None,
            product: None,
            serial: None,
            speed: None,
        };
        assert_eq!(info.display_label(), "1234:abcd");
    }

    #[test]
    fn test_drive_equality_by_device_path() {
        let d1 = DriveInfo::new("A".into(), "/mnt/a".into(), 32.0, "/dev/sdb".into());
        let d2 = DriveInfo::new("B".into(), "/mnt/b".into(), 64.0, "/dev/sdb".into());
        let d3 = DriveInfo::new("C".into(), "/mnt/c".into(), 32.0, "/dev/sdc".into());
        assert_eq!(d1, d2, "same device_path → equal");
        assert_ne!(d1, d3, "different device_path → not equal");
    }

    #[test]
    fn test_drive_equality_ignores_usb_info() {
        let d1 = DriveInfo::new("A".into(), "/mnt/a".into(), 32.0, "/dev/sdb".into())
            .with_usb_info(usb_info_fixture());
        let d2 = DriveInfo::new("A".into(), "/mnt/a".into(), 32.0, "/dev/sdb".into());
        assert_eq!(d1, d2, "usb_info must not affect equality");
    }
}
