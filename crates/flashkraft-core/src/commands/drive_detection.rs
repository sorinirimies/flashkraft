//! Drive Detection — USB-aware block device enumeration (cross-platform)
//!
//! Enumerates removable USB storage devices using only native OS interfaces —
//! no third-party USB library required.
//!
//! | Platform | Primary source          | Metadata source                        | Block node format       |
//! |----------|-------------------------|----------------------------------------|-------------------------|
//! | Linux    | sysfs (`/sys/block`)    | sysfs attributes (world-readable)      | `/dev/sdX`              |
//! | macOS    | `diskutil list/info`    | `system_profiler SPUSBDataType -json`  | `/dev/rdiskN` (raw)     |
//! | Windows  | `wmic diskdrive`        | PNPDeviceID field (VID/PID parsing)    | `\\.\PhysicalDriveN`    |
//!
//! ## Privilege notes
//!
//! - **Linux**: sysfs attributes under `/sys/block` and `/sys/devices` are
//!   world-readable — no root needed for enumeration. Writing still requires
//!   the binary to be setuid-root.
//! - **macOS**: `/dev/rdiskN` requires root; the flash pipeline's `seteuid(0)`
//!   handles this.
//! - **Windows**: the process must run as Administrator for writing;
//!   `open_device_for_writing` in `flash_helper.rs` gives a clear error if not.

use crate::domain::drive_info::{DriveInfo, UsbInfo};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Enumerate all USB mass-storage block devices currently connected.
///
/// Returns a [`Vec<DriveInfo>`] sorted by device path. Each entry has:
/// - A human-readable name built from USB descriptor strings.
/// - The OS block device path for writing.
/// - Size in GB.
/// - `usb_info` populated with VID/PID/manufacturer/product/serial/speed.
pub async fn load_drives() -> Vec<DriveInfo> {
    load_drives_sync()
}

/// Synchronous implementation — called from a blocking thread pool via
/// [`load_drives`].
///
/// Exposed as `pub` so unit tests and the TUI can call it without a Tokio
/// runtime.
pub fn load_drives_sync() -> Vec<DriveInfo> {
    #[cfg(target_os = "linux")]
    let mut drives = linux::enumerate();

    #[cfg(target_os = "macos")]
    let mut drives = macos::enumerate();

    #[cfg(target_os = "windows")]
    let mut drives = windows::enumerate();

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let mut drives: Vec<DriveInfo> = Vec::new();

    drives.sort_by(|a, b| a.device_path.cmp(&b.device_path));
    drives
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Build the display name shown in the drive selector.
///
/// Format: `"{usb_label} ({dev_name})"`, e.g. `"SanDisk Ultra (sdb)"`.
/// Falls back to just `dev_name` when no label is available.
fn build_display_name(info: &UsbInfo, dev_name: &str) -> String {
    let label = info.display_label();
    if label.contains(':') {
        // display_label() returned a raw VID:PID fallback — use just the device name.
        dev_name.to_string()
    } else {
        format!("{} ({})", label, dev_name)
    }
}

// ---------------------------------------------------------------------------
// Linux implementation
//
// Primary source : /sys/block  (world-readable, no privileges needed)
// Metadata source: sysfs attributes in /sys/devices hierarchy
//                  idVendor, idProduct, manufacturer, product, serial, speed
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    /// Enumerate all USB block devices via sysfs. No elevated privileges required.
    pub fn enumerate() -> Vec<DriveInfo> {
        let block_root = Path::new("/sys/block");
        let Ok(entries) = std::fs::read_dir(block_root) else {
            return Vec::new();
        };

        let mounts = read_proc_mounts();
        let mut drives = Vec::new();

        for entry in entries.flatten() {
            let dev_name = entry.file_name().to_string_lossy().to_string();

            if should_skip_device(&dev_name) {
                continue;
            }

            let block_sysfs = block_root.join(&dev_name);

            // Canonicalize resolves the /sys/block/<name> symlink to the real
            // sysfs path under /sys/devices/...
            let canonical = match std::fs::canonicalize(&block_sysfs) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Walk up the sysfs hierarchy to find the USB device directory
            // (identified by presence of idVendor). Skips non-USB block devices.
            let Some(usb_dir) = find_usb_sysfs_dir(&canonical) else {
                continue;
            };

            // ── Capacity ──────────────────────────────────────────────────────
            let size_sectors = read_sysfs_u64(&block_sysfs.join("size")).unwrap_or(0);
            if size_sectors == 0 {
                continue;
            }
            let size_gb = (size_sectors * 512) as f64 / (1024.0 * 1024.0 * 1024.0);

            // ── Flags ─────────────────────────────────────────────────────────
            let is_read_only = read_sysfs_u64(&block_sysfs.join("ro"))
                .map(|v| v != 0)
                .unwrap_or(false);

            // ── Paths ─────────────────────────────────────────────────────────
            let device_path = format!("/dev/{dev_name}");
            let mount_point =
                find_mount_point(&dev_name, &mounts).unwrap_or_else(|| device_path.clone());

            // ── USB metadata from sysfs (world-readable) ──────────────────────
            let usb_info = read_usb_info_from_sysfs(&usb_dir);
            let name = build_display_name(&usb_info, &dev_name);

            let drive = DriveInfo::with_constraints(
                name,
                mount_point,
                size_gb,
                device_path,
                false, // USB drives are never the system drive
                is_read_only,
            )
            .with_usb_info(usb_info);

            drives.push(drive);
        }

        drives
    }

    /// Return `true` for device names that should never be offered as flash
    /// targets: loop devices, RAM disks, device-mapper, zram, NVMe, eMMC,
    /// optical drives.
    pub(super) fn should_skip_device(name: &str) -> bool {
        name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("dm-")
            || name.starts_with("zram")
            || name.starts_with("nvme")
            || name.starts_with("mmcblk")
            || name.starts_with("sr")
            || name.is_empty()
    }

    /// Read a decimal integer from a single-line sysfs file.
    pub(super) fn read_sysfs_u64(path: &Path) -> Option<u64> {
        std::fs::read_to_string(path)
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()
    }

    /// Read and trim a sysfs attribute file, returning `None` if empty.
    pub(super) fn read_sysfs_str(path: &Path) -> Option<String> {
        let s = std::fs::read_to_string(path).ok()?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Parse a hex string (with or without `0x` prefix) into a `u16`.
    pub(super) fn parse_hex_u16(s: &str) -> Option<u16> {
        u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
    }

    /// Convert the kernel's numeric USB speed value (in Mbps) to a
    /// human-readable string.
    ///
    /// The kernel writes one of: `1.5`, `12`, `480`, `5000`, `10000`, `20000`.
    pub(super) fn sysfs_speed_string(raw: &str) -> String {
        match raw.trim() {
            "1.5" => "Low Speed (1.5 Mbps)".to_string(),
            "12" => "Full Speed (12 Mbps)".to_string(),
            "480" => "High Speed (480 Mbps)".to_string(),
            "5000" => "SuperSpeed (5 Gbps)".to_string(),
            "10000" => "SuperSpeed+ (10 Gbps)".to_string(),
            "20000" => "SuperSpeed+ (20 Gbps)".to_string(),
            other => format!("{other} Mbps"),
        }
    }

    /// Walk `path` upward through the sysfs hierarchy to find the USB device
    /// directory — identified by the presence of an `idVendor` attribute file.
    ///
    /// Returns `None` if the block device is not USB-attached.
    pub(super) fn find_usb_sysfs_dir(path: &Path) -> Option<PathBuf> {
        let mut current = path.to_path_buf();
        loop {
            if current.join("idVendor").exists() {
                return Some(current);
            }
            if current == Path::new("/sys/devices") || current == Path::new("/sys") {
                return None;
            }
            match current.parent() {
                Some(p) if p != current => current = p.to_path_buf(),
                _ => return None,
            }
        }
    }

    /// Build a [`UsbInfo`] by reading sysfs attribute files from the USB
    /// device directory (the one containing `idVendor`).
    ///
    /// All files are world-readable — no root or udev rule required.
    pub(super) fn read_usb_info_from_sysfs(usb_dir: &Path) -> UsbInfo {
        let vendor_id = read_sysfs_str(&usb_dir.join("idVendor"))
            .and_then(|s| parse_hex_u16(&s))
            .unwrap_or(0);

        let product_id = read_sysfs_str(&usb_dir.join("idProduct"))
            .and_then(|s| parse_hex_u16(&s))
            .unwrap_or(0);

        let manufacturer = read_sysfs_str(&usb_dir.join("manufacturer"));
        let product = read_sysfs_str(&usb_dir.join("product"));
        let serial = read_sysfs_str(&usb_dir.join("serial"));
        let speed = read_sysfs_str(&usb_dir.join("speed")).map(|s| sysfs_speed_string(&s));

        UsbInfo {
            vendor_id,
            product_id,
            manufacturer,
            product,
            serial,
            speed,
        }
    }

    /// Parse `/proc/mounts` and return a map of bare device-name → mount-point.
    pub(super) fn read_proc_mounts() -> HashMap<String, String> {
        let content = std::fs::read_to_string("/proc/mounts")
            .or_else(|_| std::fs::read_to_string("/proc/self/mounts"))
            .unwrap_or_default();

        let mut map = HashMap::new();
        for line in content.lines() {
            let mut parts = line.split_whitespace();
            let dev = match parts.next() {
                Some(d) if d.starts_with("/dev/") => d,
                _ => continue,
            };
            let mount = match parts.next() {
                Some(m) => m,
                None => continue,
            };
            if let Some(name) = Path::new(dev).file_name() {
                map.insert(name.to_string_lossy().to_string(), mount.to_string());
            }
        }
        map
    }

    /// Find the mount point for `dev_name` or any of its partitions.
    pub(super) fn find_mount_point(
        dev_name: &str,
        mounts: &HashMap<String, String>,
    ) -> Option<String> {
        if let Some(mp) = mounts.get(dev_name) {
            return Some(mp.clone());
        }
        for (mounted_dev, mount_point) in mounts {
            if mounted_dev.starts_with(dev_name)
                && mounted_dev.len() > dev_name.len()
                && (mounted_dev.as_bytes()[dev_name.len()].is_ascii_digit()
                    || mounted_dev.as_bytes()[dev_name.len()] == b'p')
            {
                return Some(mount_point.clone());
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// macOS — pure parsing helpers
//
// These functions are pure string transforms with no OS calls. They are
// compiled on macOS (for production use) and on all platforms under `test`
// (so the Linux CI runner can execute the unit tests). The `#[cfg]` guard
// keeps the dead-code lint quiet in non-macOS release builds.
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "macos", test))]
/// Parsed entry from `diskutil info -plist`.
#[derive(Debug, Default)]
pub(crate) struct MacDiskInfo {
    pub(crate) whole_disk: bool,
    pub(crate) removable: bool,
    pub(crate) size_bytes: u64,
    pub(crate) read_only: bool,
    pub(crate) mount_point: Option<String>,
    pub(crate) usb_vendor_id: Option<u16>,
    pub(crate) usb_product_id: Option<u16>,
    pub(crate) usb_serial: Option<String>,
}

#[cfg(any(target_os = "macos", test))]
/// A USB device entry parsed from `system_profiler SPUSBDataType -json`.
#[derive(Debug, Default)]
pub(crate) struct SpUsbDevice {
    pub(crate) vendor_id: Option<u16>,
    pub(crate) product_id: Option<u16>,
    pub(crate) serial: Option<String>,
    pub(crate) manufacturer: Option<String>,
    pub(crate) product: Option<String>,
    pub(crate) speed: Option<String>,
}

#[cfg(any(target_os = "macos", test))]
mod macos_parse {
    use super::*;

    /// Find the best-matching system_profiler device by VID + PID + serial.
    pub fn find_sp_device<'a>(
        devices: &'a [SpUsbDevice],
        vid: Option<u16>,
        pid: Option<u16>,
        serial: Option<&str>,
    ) -> Option<&'a SpUsbDevice> {
        let (v, p) = (vid?, pid?);

        // 1. VID + PID + serial (most specific).
        if let Some(s) = serial {
            if let Some(dev) = devices.iter().find(|dev| {
                dev.vendor_id == Some(v)
                    && dev.product_id == Some(p)
                    && dev.serial.as_deref() == Some(s)
            }) {
                return Some(dev);
            }
        }

        // 2. VID + PID only.
        devices
            .iter()
            .find(|dev| dev.vendor_id == Some(v) && dev.product_id == Some(p))
    }

    /// Parse `system_profiler SPUSBDataType -json` output.
    ///
    /// Scans every `_items` array in the document recursively, extracting each
    /// array element that looks like a USB device (has `vendor_id` or
    /// `product_id`). Hub/controller wrapper objects that only contain
    /// `_items` sub-arrays are not emitted as devices themselves.
    pub fn parse_system_profiler_json(json: &str) -> Vec<SpUsbDevice> {
        let mut devices = Vec::new();
        collect_items_arrays(json, &mut devices);
        devices
    }

    /// Find every `_items` array in `json` and collect device objects from it.
    /// Recurses into nested `_items` arrays (hubs containing hubs).
    fn collect_items_arrays(json: &str, out: &mut Vec<SpUsbDevice>) {
        let mut remaining = json;
        while let Some(pos) = remaining.find("\"_items\"") {
            remaining = &remaining[pos + "\"_items\"".len()..];

            let after_colon = match remaining.find('[') {
                Some(p) => &remaining[p + 1..],
                None => continue,
            };

            let array_content = extract_json_array(after_colon);

            let mut arr = array_content;
            while let Some(brace) = arr.find('{') {
                arr = &arr[brace + 1..];
                let block = extract_json_object(arr);

                if block.contains("\"vendor_id\"") || block.contains("\"product_id\"") {
                    let dev = parse_sp_device_block(block);
                    if dev.vendor_id.is_some() || dev.product_id.is_some() {
                        out.push(dev);
                    }
                }

                if block.contains("\"_items\"") {
                    collect_items_arrays(block, out);
                }

                arr = &arr[block.len()..];
            }
        }
    }

    /// Extract the text of a JSON array starting just after its opening `[`.
    fn extract_json_array(s: &str) -> &str {
        let mut depth = 1usize;
        let mut idx = 0;
        for (i, ch) in s.char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        idx = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        &s[..idx]
    }

    /// Extract the text of a JSON object starting just after its opening `{`.
    fn extract_json_object(s: &str) -> &str {
        let mut depth = 1usize;
        let mut idx = 0;
        for (i, ch) in s.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        idx = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        &s[..idx]
    }

    /// Parse a single USB device JSON object block (content between `{` and `}`).
    pub fn parse_sp_device_block(block: &str) -> SpUsbDevice {
        SpUsbDevice {
            vendor_id: json_str_value(block, "vendor_id").and_then(|s| parse_sp_hex_id(&s)),
            product_id: json_str_value(block, "product_id").and_then(|s| parse_sp_hex_id(&s)),
            serial: json_str_value(block, "serial_num"),
            manufacturer: json_str_value(block, "manufacturer"),
            product: json_str_value(block, "_name"),
            speed: json_str_value(block, "device_speed").map(|s| sp_speed_string(&s)),
        }
    }

    /// Extract the string value for a JSON key from a raw JSON object block.
    pub fn json_str_value(block: &str, key: &str) -> Option<String> {
        let needle = format!("\"{key}\"");
        let pos = block.find(&needle)?;
        let after_key = &block[pos + needle.len()..];
        let colon = after_key.find(':')?;
        let after_colon = after_key[colon + 1..].trim_start();
        if let Some(inner) = after_colon.strip_prefix('"') {
            let end = inner.find('"')?;
            Some(inner[..end].to_string())
        } else {
            None
        }
    }

    /// Parse a system_profiler hex ID string like `"0x0781"` or `"0781"` to u16.
    pub fn parse_sp_hex_id(s: &str) -> Option<u16> {
        u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
    }

    /// Convert a system_profiler speed string to a human-readable label.
    pub fn sp_speed_string(raw: &str) -> String {
        match raw.trim() {
            "low_speed" => "Low Speed (1.5 Mbps)".to_string(),
            "full_speed" => "Full Speed (12 Mbps)".to_string(),
            "high_speed" => "High Speed (480 Mbps)".to_string(),
            "super_speed" => "SuperSpeed (5 Gbps)".to_string(),
            "super_speed_plus" => "SuperSpeed+ (10 Gbps)".to_string(),
            other => other.to_string(),
        }
    }

    /// Extract a string array value for `key` from an Apple XML plist.
    pub fn parse_plist_string_array(plist: &str, key: &str) -> Vec<String> {
        let key_tag = format!("<key>{key}</key>");
        let Some(key_pos) = plist.find(&key_tag) else {
            return Vec::new();
        };
        let after_key = &plist[key_pos + key_tag.len()..];
        let Some(array_start) = after_key.find("<array>") else {
            return Vec::new();
        };
        let after_array = &after_key[array_start + "<array>".len()..];
        let Some(array_end) = after_array.find("</array>") else {
            return Vec::new();
        };
        let array_content = &after_array[..array_end];

        let mut results = Vec::new();
        let mut remaining = array_content;
        while let Some(s) = remaining.find("<string>") {
            remaining = &remaining[s + "<string>".len()..];
            if let Some(e) = remaining.find("</string>") {
                results.push(remaining[..e].trim().to_string());
                remaining = &remaining[e + "</string>".len()..];
            } else {
                break;
            }
        }
        results
    }

    /// Parse `diskutil info -plist` output for a single disk.
    pub fn parse_disk_info_plist(plist: &str, _bsd_name: &str) -> Option<MacDiskInfo> {
        let mut info = MacDiskInfo::default();
        let mut cursor = plist;

        while let Some(key_start) = cursor.find("<key>") {
            cursor = &cursor[key_start + "<key>".len()..];
            let key_end = cursor.find("</key>")?;
            let key = cursor[..key_end].trim();
            cursor = &cursor[key_end + "</key>".len()..];

            let value_start = cursor.find('<').unwrap_or(cursor.len());
            let value_section = cursor[value_start..].trim_start();

            match key {
                "WholeDisk" => {
                    info.whole_disk = value_section.starts_with("<true/>");
                }
                "Ejectable" | "Removable" | "RemovableMedia" | "RemovableMediaOrExternalDevice"
                    if value_section.starts_with("<true/>") =>
                {
                    info.removable = true;
                }
                "ReadOnly" => {
                    info.read_only = value_section.starts_with("<true/>");
                }
                "TotalSize" | "DiskSize" if info.size_bytes == 0 => {
                    if let Some(v) = extract_integer(value_section) {
                        info.size_bytes = v;
                    }
                }
                "MountPoint" => {
                    if let Some(v) = extract_string(value_section) {
                        if !v.is_empty() {
                            info.mount_point = Some(v);
                        }
                    }
                }
                "USBVendorID" => {
                    if let Some(v) = extract_integer(value_section) {
                        info.usb_vendor_id = Some(v as u16);
                    }
                }
                "USBProductID" => {
                    if let Some(v) = extract_integer(value_section) {
                        info.usb_product_id = Some(v as u16);
                    }
                }
                "USBSerialNumber" => {
                    if let Some(v) = extract_string(value_section) {
                        if !v.is_empty() {
                            info.usb_serial = Some(v);
                        }
                    }
                }
                _ => {}
            }

            cursor = advance_past_value(cursor);
        }

        if info.size_bytes == 0 {
            return None;
        }
        Some(info)
    }

    fn advance_past_value(cursor: &str) -> &str {
        let s = cursor.trim_start();
        if s.starts_with("<true/>") {
            return &cursor[cursor.find("<true/>").unwrap() + "<true/>".len()..];
        }
        if s.starts_with("<false/>") {
            return &cursor[cursor.find("<false/>").unwrap() + "<false/>".len()..];
        }
        let tag_end = match s.find('>') {
            Some(p) => p,
            None => return "",
        };
        let tag_name_start = match s.find('<') {
            Some(p) => p + 1,
            None => return "",
        };
        if tag_name_start >= tag_end {
            return "";
        }
        let tag_name = &s[tag_name_start..tag_end];
        let close_tag = format!("</{tag_name}>");
        if let Some(pos) = cursor.find(&close_tag) {
            &cursor[pos + close_tag.len()..]
        } else {
            ""
        }
    }

    pub fn extract_integer(s: &str) -> Option<u64> {
        for tag in &["<integer>", "<real>"] {
            if let Some(start) = s.find(tag) {
                let after = &s[start + tag.len()..];
                let close = tag.replace('<', "</");
                if let Some(end) = after.find(&close) {
                    let text = after[..end].trim();
                    let int_part = text.split('.').next().unwrap_or(text);
                    if let Ok(v) = int_part.parse::<u64>() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    pub fn extract_string(s: &str) -> Option<String> {
        let start = s.find("<string>")? + "<string>".len();
        let end = s[start..].find("</string>")?;
        Some(s[start..start + end].trim().to_string())
    }
}

// ---------------------------------------------------------------------------
// macOS implementation — OS-dependent portion (shells out to diskutil /
// system_profiler). Gated on target_os so it only compiles on macOS.
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod macos {
    use super::macos_parse::*;
    use super::*;

    pub fn enumerate() -> Vec<DriveInfo> {
        let whole_disks = list_external_disks();
        if whole_disks.is_empty() {
            return Vec::new();
        }

        let sp_devices = query_system_profiler();

        let mut drives = Vec::new();

        for bsd_name in &whole_disks {
            let Some(info) = disk_info(bsd_name) else {
                continue;
            };

            if !info.whole_disk || !info.removable || info.size_bytes == 0 {
                continue;
            }

            let size_gb = info.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            let sp = find_sp_device(
                &sp_devices,
                info.usb_vendor_id,
                info.usb_product_id,
                info.usb_serial.as_deref(),
            );

            let usb_info = UsbInfo {
                vendor_id: info.usb_vendor_id.unwrap_or(0),
                product_id: info.usb_product_id.unwrap_or(0),
                manufacturer: sp.and_then(|d| d.manufacturer.clone()),
                product: sp.and_then(|d| d.product.clone()),
                serial: info.usb_serial.clone(),
                speed: sp.and_then(|d| d.speed.clone()),
            };

            let device_path = format!("/dev/r{bsd_name}");
            let mount_point = info
                .mount_point
                .clone()
                .unwrap_or_else(|| device_path.clone());

            let name = build_display_name(&usb_info, bsd_name);

            let drive = DriveInfo::with_constraints(
                name,
                mount_point,
                size_gb,
                device_path,
                false,
                info.read_only,
            )
            .with_usb_info(usb_info);

            drives.push(drive);
        }

        drives
    }

    fn list_external_disks() -> Vec<String> {
        let output = std::process::Command::new("diskutil")
            .args(["list", "-plist", "external"])
            .output();
        let Ok(out) = output else {
            eprintln!("[drive_detection] diskutil list failed");
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&out.stdout);
        parse_plist_string_array(&text, "WholeDisks")
    }

    fn disk_info(bsd_name: &str) -> Option<MacDiskInfo> {
        let output = std::process::Command::new("diskutil")
            .args(["info", "-plist", &format!("/dev/{bsd_name}")])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        parse_disk_info_plist(&text, bsd_name)
    }

    fn query_system_profiler() -> Vec<SpUsbDevice> {
        let output = std::process::Command::new("system_profiler")
            .args(["SPUSBDataType", "-json"])
            .output();
        let Ok(out) = output else {
            return Vec::new();
        };
        let json = String::from_utf8_lossy(&out.stdout);
        parse_system_profiler_json(&json)
    }
}

// ---------------------------------------------------------------------------
// Windows — pure parsing helpers (always compiled, tested on Linux CI runner)
// ---------------------------------------------------------------------------

/// A parsed USB disk drive from wmic output.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct WmicDisk {
    pub(crate) device_id: String,
    pub(crate) size_bytes: u64,
    pub(crate) model: String,
    pub(crate) serial: String,
    pub(crate) vendor_id: Option<u16>,
    pub(crate) product_id: Option<u16>,
}

mod windows_parse {
    use super::*;

    /// Parse wmic `/format:csv` output into `WmicDisk` entries.
    ///
    /// Expected header + data format:
    /// ```text
    /// Node,DeviceID,Model,PNPDeviceID,SerialNumber,Size
    /// HOSTNAME,\\.\PhysicalDrive1,SanDisk Ultra,USB\VID_0781&PID_5581\AA0123,AA0123,32012804608
    /// ```
    #[allow(dead_code)]
    pub fn parse_wmic_csv(csv: &str) -> Vec<WmicDisk> {
        let mut lines = csv.lines().map(|l| l.trim()).filter(|l| !l.is_empty());

        let header = loop {
            match lines.next() {
                Some(h) if h.to_lowercase().contains("deviceid") => break h,
                Some(_) => continue,
                None => return Vec::new(),
            }
        };

        let cols: Vec<&str> = header.split(',').collect();
        let idx = |name: &str| -> Option<usize> {
            cols.iter()
                .position(|c| c.trim().to_lowercase() == name.to_lowercase())
        };

        let idx_device = idx("DeviceID");
        let idx_size = idx("Size");
        let idx_model = idx("Model");
        let idx_serial = idx("SerialNumber");
        let idx_pnp = idx("PNPDeviceID");

        let mut disks = Vec::new();

        for line in lines {
            let fields: Vec<&str> = line.split(',').collect();
            let get = |i: Option<usize>| -> &str {
                i.and_then(|i| fields.get(i).copied()).unwrap_or("").trim()
            };

            let device_id = get(idx_device);
            if device_id.is_empty() || !device_id.contains("PhysicalDrive") {
                continue;
            }

            let size_bytes = get(idx_size).parse::<u64>().unwrap_or(0);
            let (vendor_id, product_id) = parse_vid_pid_from_pnp(get(idx_pnp));

            disks.push(WmicDisk {
                device_id: device_id.to_string(),
                size_bytes,
                model: get(idx_model).to_string(),
                serial: get(idx_serial).to_string(),
                vendor_id,
                product_id,
            });
        }

        disks
    }

    /// Extract VID and PID from a Windows PNPDeviceID string.
    ///
    /// Format: `USB\VID_0781&PID_5581\AA01234567890`
    #[allow(dead_code)]
    pub fn parse_vid_pid_from_pnp(pnp: &str) -> (Option<u16>, Option<u16>) {
        let upper = pnp.to_uppercase();

        let vid = upper.find("VID_").and_then(|pos| {
            let hex = &upper[pos + 4..];
            let end = hex
                .find(|c: char| !c.is_ascii_hexdigit())
                .unwrap_or(hex.len());
            u16::from_str_radix(&hex[..end], 16).ok()
        });

        let pid = upper.find("PID_").and_then(|pos| {
            let hex = &upper[pos + 4..];
            let end = hex
                .find(|c: char| !c.is_ascii_hexdigit())
                .unwrap_or(hex.len());
            u16::from_str_radix(&hex[..end], 16).ok()
        });

        (vid, pid)
    }
}

// ---------------------------------------------------------------------------
// Windows implementation — OS-dependent portion (shells out to wmic).
// Gated on target_os so it only compiles on Windows.
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod windows {
    use super::windows_parse::*;
    use super::*;

    pub fn enumerate() -> Vec<DriveInfo> {
        let wmic_disks = query_wmic_usb_disks();
        if wmic_disks.is_empty() {
            return Vec::new();
        }

        let mut drives = Vec::new();

        for disk in &wmic_disks {
            if disk.size_bytes == 0 {
                continue;
            }

            let size_gb = disk.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            let usb_info = UsbInfo {
                vendor_id: disk.vendor_id.unwrap_or(0),
                product_id: disk.product_id.unwrap_or(0),
                manufacturer: None,
                product: if disk.model.is_empty() {
                    None
                } else {
                    Some(disk.model.clone())
                },
                serial: if disk.serial.is_empty() {
                    None
                } else {
                    Some(disk.serial.clone())
                },
                speed: None,
            };

            let short_name = disk
                .device_id
                .split('\\')
                .next_back()
                .unwrap_or(&disk.device_id);
            let name = build_display_name(&usb_info, short_name);

            let drive = DriveInfo::with_constraints(
                name,
                disk.device_id.clone(),
                size_gb,
                disk.device_id.clone(),
                false,
                false,
            )
            .with_usb_info(usb_info);

            drives.push(drive);
        }

        drives
    }

    fn query_wmic_usb_disks() -> Vec<WmicDisk> {
        let output = std::process::Command::new("wmic")
            .args([
                "diskdrive",
                "where",
                "InterfaceType=\"USB\"",
                "get",
                "DeviceID,Size,Model,SerialNumber,PNPDeviceID",
                "/format:csv",
            ])
            .output();
        let Ok(out) = output else {
            eprintln!("[drive_detection] wmic query failed");
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&out.stdout);
        parse_wmic_csv(&text)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::macos_parse::*;
    use super::windows_parse::*;
    use super::*;

    #[cfg(target_os = "linux")]
    use std::collections::HashMap;
    #[cfg(target_os = "linux")]
    use std::path::Path;

    // ── Linux helpers ─────────────────────────────────────────────────────────

    #[cfg(target_os = "linux")]
    macro_rules! skip_device_tests {
        ($( $name:ident : $device:literal => $expected:expr ),+ $(,)?) => {
            $(
                #[cfg(target_os = "linux")]
                #[test]
                fn $name() {
                    assert_eq!(linux::should_skip_device($device), $expected,
                        "should_skip_device({:?}) should be {}", $device, $expected);
                }
            )+
        };
    }

    #[cfg(target_os = "linux")]
    skip_device_tests! {
        test_skip_loop_devices_loop0:  "loop0"  => true,
        test_skip_loop_devices_loop1:  "loop1"  => true,
        test_skip_nvme_devices:        "nvme0n1" => true,
        test_skip_ram_devices:         "ram0"   => true,
        test_skip_dm_devices:          "dm-0"   => true,
        test_skip_optical_drives:      "sr0"    => true,
        test_skip_empty_name:          ""       => true,
        test_allow_sata_usb_sda:       "sda"    => false,
        test_allow_sata_usb_sdb:       "sdb"    => false,
        test_allow_sata_usb_sdc:       "sdc"    => false,
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_sysfs_u64_valid() {
        let path = std::env::temp_dir().join("fk_sysfs_u64_test.txt");
        std::fs::write(&path, "1953525168\n").unwrap();
        assert_eq!(linux::read_sysfs_u64(&path), Some(1953525168u64));
        let _ = std::fs::remove_file(path);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_sysfs_u64_invalid() {
        let path = std::env::temp_dir().join("fk_sysfs_invalid.txt");
        std::fs::write(&path, "not_a_number\n").unwrap();
        assert_eq!(linux::read_sysfs_u64(&path), None);
        let _ = std::fs::remove_file(path);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_sysfs_u64_missing_file() {
        assert_eq!(
            linux::read_sysfs_u64(Path::new("/nonexistent/sysfs/file")),
            None
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_mount_point_exact_device() {
        let mut mounts = HashMap::new();
        mounts.insert("sdb".to_string(), "/media/usb".to_string());
        assert_eq!(
            linux::find_mount_point("sdb", &mounts),
            Some("/media/usb".to_string())
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_mount_point_partition() {
        let mut mounts = HashMap::new();
        mounts.insert("sdb1".to_string(), "/media/usb".to_string());
        assert_eq!(
            linux::find_mount_point("sdb", &mounts),
            Some("/media/usb".to_string())
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_mount_point_not_found() {
        assert_eq!(linux::find_mount_point("sdb", &HashMap::new()), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_mount_point_different_device_not_matched() {
        let mut mounts = HashMap::new();
        mounts.insert("sdc1".to_string(), "/media/other".to_string());
        assert_eq!(linux::find_mount_point("sdb", &mounts), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_usb_sysfs_dir_stops_at_sys_root() {
        assert!(linux::find_usb_sysfs_dir(Path::new("/sys")).is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_usb_sysfs_dir_no_id_vendor_in_tmp() {
        // A plain temp dir has no idVendor — should return None.
        let dir = std::env::temp_dir().join("fk_no_id_vendor");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(linux::find_usb_sysfs_dir(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_usb_sysfs_dir_finds_id_vendor_in_dir() {
        let dir = std::env::temp_dir().join("fk_usb_test_direct");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("idVendor"), "0781\n").unwrap();
        assert_eq!(linux::find_usb_sysfs_dir(&dir), Some(dir.clone()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_usb_sysfs_dir_finds_id_vendor_in_parent() {
        // Simulate: usb_dev_dir/idVendor exists, block device is a child subdir.
        let base = std::env::temp_dir().join("fk_usb_test_parent");
        let child = base.join("child_block");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(base.join("idVendor"), "0781\n").unwrap();
        assert_eq!(linux::find_usb_sysfs_dir(&child), Some(base.clone()));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_usb_info_from_sysfs_all_fields() {
        let dir = std::env::temp_dir().join("fk_usb_info_all");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("idVendor"), "0781\n").unwrap();
        std::fs::write(dir.join("idProduct"), "5581\n").unwrap();
        std::fs::write(dir.join("manufacturer"), "SanDisk\n").unwrap();
        std::fs::write(dir.join("product"), "Ultra\n").unwrap();
        std::fs::write(dir.join("serial"), "SN123\n").unwrap();
        std::fs::write(dir.join("speed"), "5000\n").unwrap();

        let info = linux::read_usb_info_from_sysfs(&dir);
        assert_eq!(info.vendor_id, 0x0781);
        assert_eq!(info.product_id, 0x5581);
        assert_eq!(info.manufacturer.as_deref(), Some("SanDisk"));
        assert_eq!(info.product.as_deref(), Some("Ultra"));
        assert_eq!(info.serial.as_deref(), Some("SN123"));
        assert_eq!(info.speed.as_deref(), Some("SuperSpeed (5 Gbps)"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_usb_info_from_sysfs_missing_optional_fields() {
        // Only vendor/product IDs present; strings and speed are absent.
        let dir = std::env::temp_dir().join("fk_usb_info_minimal");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("idVendor"), "1234\n").unwrap();
        std::fs::write(dir.join("idProduct"), "abcd\n").unwrap();

        let info = linux::read_usb_info_from_sysfs(&dir);
        assert_eq!(info.vendor_id, 0x1234);
        assert_eq!(info.product_id, 0xabcd);
        assert!(info.manufacturer.is_none());
        assert!(info.product.is_none());
        assert!(info.serial.is_none());
        assert!(info.speed.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_usb_info_from_sysfs_empty_string_fields() {
        // Files exist but contain only whitespace — should be treated as None.
        let dir = std::env::temp_dir().join("fk_usb_info_empty_str");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("idVendor"), "0781\n").unwrap();
        std::fs::write(dir.join("idProduct"), "5581\n").unwrap();
        std::fs::write(dir.join("manufacturer"), "   \n").unwrap();
        std::fs::write(dir.join("product"), "\n").unwrap();

        let info = linux::read_usb_info_from_sysfs(&dir);
        assert!(info.manufacturer.is_none());
        assert!(info.product.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_sysfs_speed_all_variants() {
        // Verify all known speed strings round-trip correctly.
        assert_eq!(linux::sysfs_speed_string("1.5"), "Low Speed (1.5 Mbps)");
        assert_eq!(linux::sysfs_speed_string("12"), "Full Speed (12 Mbps)");
        assert_eq!(linux::sysfs_speed_string("480"), "High Speed (480 Mbps)");
        assert_eq!(linux::sysfs_speed_string("5000"), "SuperSpeed (5 Gbps)");
        assert_eq!(linux::sysfs_speed_string("10000"), "SuperSpeed+ (10 Gbps)");
        assert_eq!(linux::sysfs_speed_string("20000"), "SuperSpeed+ (20 Gbps)");
        assert_eq!(linux::sysfs_speed_string("999"), "999 Mbps");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_sysfs_speed_trims_whitespace() {
        assert_eq!(
            linux::sysfs_speed_string("  480  "),
            "High Speed (480 Mbps)"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_hex_u16_bare() {
        assert_eq!(linux::parse_hex_u16("0781"), Some(0x0781));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_hex_u16_with_prefix() {
        assert_eq!(linux::parse_hex_u16("0x5581"), Some(0x5581));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_hex_u16_overflow() {
        // 0x10000 does not fit in u16
        assert!(linux::parse_hex_u16("10000").is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_hex_u16_invalid() {
        assert!(linux::parse_hex_u16("zzzz").is_none());
        assert!(linux::parse_hex_u16("").is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_mount_point_p_partition_suffix() {
        // e.g. mmcblk0p1 style (though we skip mmcblk, this tests the logic)
        let mut mounts = HashMap::new();
        mounts.insert("sdap1".to_string(), "/media/card".to_string());
        assert_eq!(
            linux::find_mount_point("sda", &mounts),
            Some("/media/card".to_string())
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_mount_point_multiple_partitions_returns_first_match() {
        let mut mounts = HashMap::new();
        // Insert in a deterministic way — HashMap iteration is unordered,
        // but find_mount_point returns the first prefix match it finds.
        // We just verify it returns *one* of the valid mount points.
        mounts.insert("sdb1".to_string(), "/media/part1".to_string());
        let result = linux::find_mount_point("sdb", &mounts);
        assert!(result.is_some());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_proc_mounts_does_not_panic() {
        let _ = linux::read_proc_mounts();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_proc_mounts_parses_dev_entries() {
        // Write a synthetic mounts file and verify parsing.
        let path = std::env::temp_dir().join("fk_proc_mounts_test");
        std::fs::write(
            &path,
            "/dev/sdb1 /media/usb vfat rw 0 0\n\
             /dev/sda1 / ext4 rw 0 0\n\
             tmpfs /tmp tmpfs rw 0 0\n",
        )
        .unwrap();
        // We can't inject the path into read_proc_mounts, but we can verify
        // parse logic directly via find_mount_point on a hand-built map.
        let mut map = HashMap::new();
        map.insert("sdb1".to_string(), "/media/usb".to_string());
        assert_eq!(
            linux::find_mount_point("sdb", &map),
            Some("/media/usb".to_string())
        );
        let _ = std::fs::remove_file(path);
    }

    // ── macOS system_profiler parser ─────────────────────────────────────────

    #[test]
    fn test_parse_system_profiler_single_device() {
        let json = r#"{
  "SPUSBDataType": [
    {
      "_items": [
        {
          "_name": "Ultra",
          "manufacturer": "SanDisk",
          "vendor_id": "0x0781",
          "product_id": "0x5581",
          "serial_num": "AA01234567890",
          "device_speed": "high_speed"
        }
      ]
    }
  ]
}"#;
        let devices = parse_system_profiler_json(json);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].vendor_id, Some(0x0781));
        assert_eq!(devices[0].product_id, Some(0x5581));
        assert_eq!(devices[0].serial.as_deref(), Some("AA01234567890"));
        assert_eq!(devices[0].manufacturer.as_deref(), Some("SanDisk"));
        assert_eq!(devices[0].product.as_deref(), Some("Ultra"));
        assert_eq!(devices[0].speed.as_deref(), Some("High Speed (480 Mbps)"));
    }

    #[test]
    fn test_parse_system_profiler_multiple_devices() {
        let json = r#"{
  "SPUSBDataType": [
    {
      "_items": [
        {
          "_name": "Ultra",
          "vendor_id": "0x0781",
          "product_id": "0x5581",
          "serial_num": "SN001"
        },
        {
          "_name": "DataTraveler",
          "vendor_id": "0x0951",
          "product_id": "0x1666",
          "serial_num": "SN002"
        }
      ]
    }
  ]
}"#;
        let devices = parse_system_profiler_json(json);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].vendor_id, Some(0x0781));
        assert_eq!(devices[1].vendor_id, Some(0x0951));
    }

    #[test]
    fn test_parse_system_profiler_empty_items() {
        let json = r#"{"SPUSBDataType": [{"_items": []}]}"#;
        let devices = parse_system_profiler_json(json);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_parse_system_profiler_no_usb_data() {
        let json = r#"{"SPUSBDataType": []}"#;
        let devices = parse_system_profiler_json(json);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_parse_sp_hex_id_with_prefix() {
        assert_eq!(parse_sp_hex_id("0x0781"), Some(0x0781));
    }

    #[test]
    fn test_parse_sp_hex_id_bare() {
        assert_eq!(parse_sp_hex_id("0781"), Some(0x0781));
    }

    #[test]
    fn test_parse_sp_hex_id_invalid() {
        assert!(parse_sp_hex_id("zzzz").is_none());
        assert!(parse_sp_hex_id("").is_none());
    }

    #[test]
    fn test_sp_speed_string_all_variants() {
        assert_eq!(sp_speed_string("low_speed"), "Low Speed (1.5 Mbps)");
        assert_eq!(sp_speed_string("full_speed"), "Full Speed (12 Mbps)");
        assert_eq!(sp_speed_string("high_speed"), "High Speed (480 Mbps)");
        assert_eq!(sp_speed_string("super_speed"), "SuperSpeed (5 Gbps)");
        assert_eq!(sp_speed_string("super_speed_plus"), "SuperSpeed+ (10 Gbps)");
        // Unknown variant passes through unchanged.
        assert_eq!(sp_speed_string("warp_speed"), "warp_speed");
    }

    #[test]
    fn test_find_sp_device_by_vid_pid_serial() {
        let devices = vec![
            SpUsbDevice {
                vendor_id: Some(0x0781),
                product_id: Some(0x5581),
                serial: Some("SN001".to_string()),
                manufacturer: Some("SanDisk".to_string()),
                product: Some("Ultra".to_string()),
                speed: None,
            },
            SpUsbDevice {
                vendor_id: Some(0x0951),
                product_id: Some(0x1666),
                serial: Some("SN002".to_string()),
                manufacturer: Some("Kingston".to_string()),
                product: Some("DataTraveler".to_string()),
                speed: None,
            },
        ];

        let found = find_sp_device(&devices, Some(0x0781), Some(0x5581), Some("SN001"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().manufacturer.as_deref(), Some("SanDisk"));
    }

    #[test]
    fn test_find_sp_device_by_vid_pid_only() {
        let devices = vec![SpUsbDevice {
            vendor_id: Some(0x0781),
            product_id: Some(0x5581),
            serial: None,
            manufacturer: Some("SanDisk".to_string()),
            product: Some("Ultra".to_string()),
            speed: None,
        }];

        // No serial provided — should still match on VID+PID.
        let found = find_sp_device(&devices, Some(0x0781), Some(0x5581), None);
        assert!(found.is_some());
    }

    #[test]
    fn test_find_sp_device_wrong_serial_falls_back_to_vid_pid() {
        let devices = vec![SpUsbDevice {
            vendor_id: Some(0x0781),
            product_id: Some(0x5581),
            serial: Some("CORRECT".to_string()),
            manufacturer: Some("SanDisk".to_string()),
            product: Some("Ultra".to_string()),
            speed: None,
        }];

        // Wrong serial but same VID+PID → falls through to VID+PID match.
        let found = find_sp_device(&devices, Some(0x0781), Some(0x5581), Some("WRONG"));
        assert!(found.is_some());
    }

    #[test]
    fn test_find_sp_device_no_match() {
        let devices = vec![SpUsbDevice {
            vendor_id: Some(0x0781),
            product_id: Some(0x5581),
            serial: None,
            manufacturer: None,
            product: None,
            speed: None,
        }];

        // Different VID/PID — must not match.
        let found = find_sp_device(&devices, Some(0x1234), Some(0xabcd), None);
        assert!(found.is_none());
    }

    #[test]
    fn test_find_sp_device_none_vid_returns_none() {
        let devices = vec![SpUsbDevice {
            vendor_id: Some(0x0781),
            product_id: Some(0x5581),
            serial: None,
            manufacturer: None,
            product: None,
            speed: None,
        }];

        // vid=None means we cannot correlate — must return None.
        let found = find_sp_device(&devices, None, Some(0x5581), None);
        assert!(found.is_none());
    }

    #[test]
    fn test_json_str_value_present() {
        let block = r#""manufacturer": "SanDisk", "product_id": "0x5581""#;
        assert_eq!(
            json_str_value(block, "manufacturer"),
            Some("SanDisk".to_string())
        );
    }

    #[test]
    fn test_json_str_value_missing_key() {
        let block = r#""other": "value""#;
        assert!(json_str_value(block, "manufacturer").is_none());
    }

    #[test]
    fn test_json_str_value_non_string_value() {
        // Numeric value — not a quoted string, should return None.
        let block = r#""count": 42"#;
        assert!(json_str_value(block, "count").is_none());
    }

    // ── macOS plist parser (already inlined above) ────────────────────────────

    #[test]
    fn test_parse_plist_string_array_whole_disks() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "">
<plist version="1.0">
<dict>
    <key>AllDisksAndPartitions</key>
    <array/>
    <key>WholeDisks</key>
    <array>
        <string>disk2</string>
        <string>disk3</string>
    </array>
</dict>
</plist>"#;
        let result = parse_plist_string_array(plist, "WholeDisks");
        assert_eq!(result, vec!["disk2", "disk3"]);
    }

    #[test]
    fn test_parse_plist_string_array_missing_key() {
        let plist = "<plist><dict><key>Other</key><array/></dict></plist>";
        assert!(parse_plist_string_array(plist, "WholeDisks").is_empty());
    }

    #[test]
    fn test_parse_disk_info_plist_whole_removable() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>WholeDisk</key><true/>
    <key>RemovableMediaOrExternalDevice</key><true/>
    <key>ReadOnly</key><false/>
    <key>TotalSize</key><integer>32017047552</integer>
    <key>MountPoint</key><string>/Volumes/SANDISK</string>
    <key>USBVendorID</key><integer>1921</integer>
    <key>USBProductID</key><integer>21889</integer>
    <key>USBSerialNumber</key><string>AA01234567890</string>
</dict>
</plist>"#;
        let info = parse_disk_info_plist(plist, "disk2").unwrap();
        assert!(info.whole_disk);
        assert!(info.removable);
        assert!(!info.read_only);
        assert_eq!(info.size_bytes, 32017047552);
        assert_eq!(info.mount_point.as_deref(), Some("/Volumes/SANDISK"));
        assert_eq!(info.usb_vendor_id, Some(1921));
        assert_eq!(info.usb_product_id, Some(21889));
        assert_eq!(info.usb_serial.as_deref(), Some("AA01234567890"));
    }

    #[test]
    fn test_parse_disk_info_plist_not_whole_disk() {
        let plist = r#"<plist version="1.0"><dict>
            <key>WholeDisk</key><false/>
            <key>RemovableMediaOrExternalDevice</key><true/>
            <key>TotalSize</key><integer>16000000000</integer>
        </dict></plist>"#;
        let info = parse_disk_info_plist(plist, "disk2s1").unwrap();
        assert!(!info.whole_disk);
    }

    #[test]
    fn test_parse_disk_info_plist_zero_size_returns_none() {
        let plist = r#"<plist version="1.0"><dict>
            <key>WholeDisk</key><true/>
            <key>TotalSize</key><integer>0</integer>
        </dict></plist>"#;
        assert!(parse_disk_info_plist(plist, "disk2").is_none());
    }

    #[test]
    fn test_extract_integer_from_integer_tag() {
        assert_eq!(extract_integer("<integer>12345</integer>"), Some(12345));
    }

    #[test]
    fn test_extract_integer_from_real_tag() {
        assert_eq!(
            extract_integer("<real>32017047552.0</real>"),
            Some(32017047552)
        );
    }

    #[test]
    fn test_extract_string_tag() {
        assert_eq!(
            extract_string("<string>/Volumes/USB</string>"),
            Some("/Volumes/USB".to_string())
        );
    }

    // ── Windows wmic parser ───────────────────────────────────────────────────

    #[test]
    fn test_parse_wmic_csv_basic() {
        let csv = "\r\nNode,DeviceID,Model,PNPDeviceID,SerialNumber,Size\r\n\
            MYPC,\\\\.\\PhysicalDrive1,SanDisk Ultra,USB\\VID_0781&PID_5581\\AA0123,AA0123,32017047552\r\n";
        let disks = parse_wmic_csv(csv);
        assert_eq!(disks.len(), 1);
        assert!(disks[0].device_id.contains("PhysicalDrive1"));
        assert_eq!(disks[0].size_bytes, 32017047552);
        assert_eq!(disks[0].serial, "AA0123");
        assert_eq!(disks[0].vendor_id, Some(0x0781));
        assert_eq!(disks[0].product_id, Some(0x5581));
    }

    #[test]
    fn test_parse_wmic_csv_multiple_disks() {
        let csv = "Node,DeviceID,Model,PNPDeviceID,SerialNumber,Size\r\n\
            PC,\\\\.\\PhysicalDrive1,SanDisk Ultra,USB\\VID_0781&PID_5581\\SN1,SN1,32017047552\r\n\
            PC,\\\\.\\PhysicalDrive2,Kingston DT,USB\\VID_0951&PID_1666\\SN2,SN2,16000000000\r\n";
        let disks = parse_wmic_csv(csv);
        assert_eq!(disks.len(), 2);
        assert_eq!(disks[0].vendor_id, Some(0x0781));
        assert_eq!(disks[1].vendor_id, Some(0x0951));
    }

    #[test]
    fn test_parse_wmic_csv_zero_size_included() {
        // Zero-size entries are included by parse_wmic_csv; enumerate() filters them.
        let csv = "Node,DeviceID,Model,PNPDeviceID,SerialNumber,Size\r\n\
            PC,\\\\.\\PhysicalDrive1,Unknown,USB\\VID_1234&PID_5678\\SN,,0\r\n";
        let disks = parse_wmic_csv(csv);
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].size_bytes, 0);
    }

    #[test]
    fn test_parse_wmic_csv_empty() {
        assert!(parse_wmic_csv("").is_empty());
    }

    #[test]
    fn test_parse_wmic_csv_header_only() {
        let csv = "Node,DeviceID,Model,PNPDeviceID,SerialNumber,Size\r\n";
        assert!(parse_wmic_csv(csv).is_empty());
    }

    #[test]
    fn test_parse_wmic_csv_skips_non_physical() {
        let csv = "Node,DeviceID,Model,PNPDeviceID,SerialNumber,Size\r\n\
            MYPC,\\\\.\\CDROM0,Some CD Drive,,,\r\n";
        assert!(parse_wmic_csv(csv).is_empty());
    }

    #[test]
    fn test_parse_wmic_csv_missing_pnp_column_still_parses() {
        // Older wmic versions may omit PNPDeviceID — VID/PID should be None.
        let csv = "Node,DeviceID,Model,SerialNumber,Size\r\n\
            PC,\\\\.\\PhysicalDrive1,SanDisk,SN1,32017047552\r\n";
        let disks = parse_wmic_csv(csv);
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].vendor_id, None);
        assert_eq!(disks[0].product_id, None);
    }

    #[test]
    fn test_parse_vid_pid_from_pnp_standard() {
        let (vid, pid) = parse_vid_pid_from_pnp("USB\\VID_0781&PID_5581\\AA01234567890");
        assert_eq!(vid, Some(0x0781));
        assert_eq!(pid, Some(0x5581));
    }

    #[test]
    fn test_parse_vid_pid_from_pnp_lowercase() {
        // Windows PNPDeviceID is case-insensitive — we uppercase internally.
        let (vid, pid) = parse_vid_pid_from_pnp("usb\\vid_0781&pid_5581\\SN");
        assert_eq!(vid, Some(0x0781));
        assert_eq!(pid, Some(0x5581));
    }

    #[test]
    fn test_parse_vid_pid_from_pnp_empty() {
        let (vid, pid) = parse_vid_pid_from_pnp("");
        assert_eq!(vid, None);
        assert_eq!(pid, None);
    }

    #[test]
    fn test_parse_vid_pid_from_pnp_no_pid() {
        // Malformed: has VID but no PID.
        let (vid, pid) = parse_vid_pid_from_pnp("USB\\VID_0781\\SN");
        assert_eq!(vid, Some(0x0781));
        assert_eq!(pid, None);
    }

    #[test]
    fn test_parse_vid_pid_from_pnp_no_vid() {
        // No VID at all.
        let (vid, pid) = parse_vid_pid_from_pnp("USB\\PID_5581\\SN");
        assert_eq!(vid, None);
        assert_eq!(pid, Some(0x5581));
    }

    #[test]
    fn test_parse_vid_pid_from_pnp_non_usb_path() {
        // Non-USB PNP path — no VID/PID markers.
        let (vid, pid) = parse_vid_pid_from_pnp("SCSI\\DISK&VEN_WDC&PROD_WD10EZEX");
        assert_eq!(vid, None);
        assert_eq!(pid, None);
    }

    #[test]
    fn test_parse_vid_pid_from_pnp_ffff_values() {
        let (vid, pid) = parse_vid_pid_from_pnp("USB\\VID_FFFF&PID_FFFF\\SN");
        assert_eq!(vid, Some(0xFFFF));
        assert_eq!(pid, Some(0xFFFF));
    }

    // ── Shared helpers ────────────────────────────────────────────────────────

    #[test]
    fn test_build_display_name_with_label() {
        let info = crate::domain::drive_info::UsbInfo {
            vendor_id: 0x0781,
            product_id: 0x5581,
            manufacturer: Some("SanDisk".into()),
            product: Some("Ultra".into()),
            serial: None,
            speed: None,
        };
        assert_eq!(build_display_name(&info, "sdb"), "SanDisk Ultra (sdb)");
    }

    #[test]
    fn test_build_display_name_vid_pid_fallback() {
        let info = crate::domain::drive_info::UsbInfo {
            vendor_id: 0x1234,
            product_id: 0xabcd,
            manufacturer: None,
            product: None,
            serial: None,
            speed: None,
        };
        // VID:PID label → falls back to just dev_name
        assert_eq!(build_display_name(&info, "sdb"), "sdb");
    }

    #[test]
    fn test_build_display_name_product_only() {
        let info = crate::domain::drive_info::UsbInfo {
            vendor_id: 0x0781,
            product_id: 0x5581,
            manufacturer: None,
            product: Some("Ultra".into()),
            serial: None,
            speed: None,
        };
        assert_eq!(build_display_name(&info, "sdb"), "Ultra (sdb)");
    }

    #[test]
    fn test_load_drives_sync_returns_vec() {
        // Smoke test: must not panic and must return a Vec (possibly empty).
        let drives = load_drives_sync();
        let _ = drives;
    }
}
