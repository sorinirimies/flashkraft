//! Demo mode — deterministic fake drives for screenshots, tapes and tests.
//!
//! Set `FLASHKRAFT_DEMO=1` and [`crate::commands::load_drives_sync`] returns
//! [`demo_drives`] instead of touching the hardware. Nothing else changes:
//! the flash pipeline still refuses anything that is not a real, writable
//! device, so demo mode can never write to a disk.

use crate::domain::drive_info::{DriveInfo, UsbInfo};

/// Environment variable that switches demo mode on (`1`, `true`, `yes`, `on`).
pub const DEMO_ENV: &str = "FLASHKRAFT_DEMO";

/// `true` when [`DEMO_ENV`] is set to a truthy value.
pub fn is_demo() -> bool {
    parse_flag(std::env::var(DEMO_ENV).ok().as_deref())
}

fn parse_flag(v: Option<&str>) -> bool {
    matches!(
        v.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

fn usb(vid: u16, pid: u16, mfr: &str, product: &str, serial: &str, speed: &str) -> UsbInfo {
    UsbInfo {
        vendor_id: vid,
        product_id: pid,
        manufacturer: Some(mfr.into()),
        product: Some(product.into()),
        serial: Some(serial.into()),
        speed: Some(speed.into()),
    }
}

/// A fixed, realistic set of drives: three USB sticks and a system disk.
pub fn demo_drives() -> Vec<DriveInfo> {
    let mut system = DriveInfo::with_constraints(
        "Samsung SSD 980 PRO (nvme0n1)".into(),
        "/boot".into(),
        931.5,
        "/dev/nvme0n1".into(),
        true,
        false,
    );
    system.disabled = true;

    vec![
        system,
        DriveInfo::new(
            "SanDisk Ultra (sdb)".into(),
            "/dev/sdb".into(),
            29.8,
            "/dev/sdb".into(),
        )
        .with_usb_info(usb(
            0x0781,
            0x5581,
            "SanDisk",
            "Ultra",
            "AA01234567890",
            "SuperSpeed (5 Gbps)",
        )),
        DriveInfo::new(
            "Kingston DataTraveler (sdc)".into(),
            "/dev/sdc".into(),
            14.6,
            "/dev/sdc".into(),
        )
        .with_usb_info(usb(
            0x0951,
            0x1666,
            "Kingston",
            "DataTraveler 3.0",
            "60A44C413",
            "SuperSpeed (5 Gbps)",
        )),
        DriveInfo::new(
            "Lexar JumpDrive (sdd)".into(),
            "/dev/sdd".into(),
            119.0,
            "/dev/sdd".into(),
        )
        .with_usb_info(usb(
            0x05dc,
            0xa81d,
            "Lexar",
            "JumpDrive S75",
            "0C7B1F2A",
            "High Speed (480 Mbps)",
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_parsing() {
        for on in ["1", "true", "YES", " on "] {
            assert!(parse_flag(Some(on)), "{on}");
        }
        for off in ["", "0", "false", "nope"] {
            assert!(!parse_flag(Some(off)), "{off}");
        }
        assert!(!parse_flag(None));
    }

    #[test]
    fn demo_set_has_usb_sticks_and_one_system_disk() {
        let d = demo_drives();
        assert_eq!(d.iter().filter(|x| x.is_system).count(), 1);
        assert!(d.iter().filter(|x| x.usb_info.is_some()).count() >= 3);
        assert!(d.iter().all(|x| !x.is_read_only));
        let mut paths: Vec<_> = d.iter().map(|x| &x.device_path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), d.len(), "device paths are unique");
    }
}
