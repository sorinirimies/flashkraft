#![no_main]
use flashkraft_core::domain::constraints::*;
use flashkraft_core::domain::{DriveInfo, ImageInfo};
use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

// Drive/image checks run on whatever the OS reports (NaN sizes, odd paths).
fuzz_target!(|data: &[u8]| {
    if data.len() < 17 {
        return;
    }
    let f = |b: &[u8]| f64::from_le_bytes(b.try_into().unwrap());
    let (size_gb, size_mb) = (f(&data[0..8]), f(&data[8..16]));
    let flags = data[16];
    let text = String::from_utf8_lossy(&data[17..]).into_owned();
    let mid = text.char_indices().nth(text.chars().count() / 2).map_or(0, |(i, _)| i);
    let (a, b) = text.split_at(mid);

    let mut drive = DriveInfo::new(a.to_string(), b.to_string(), size_gb, format!("/dev/{a}"));
    drive.is_system = flags & 1 != 0;
    drive.is_read_only = flags & 2 != 0;
    let image = ImageInfo {
        path: PathBuf::from(b),
        name: a.to_string(),
        size_mb,
    };
    let img = (flags & 4 == 0).then_some(&image);

    let _ = is_drive_valid(&drive, img);
    let _ = is_source_drive(&drive, img);
    let _ = is_drive_large_enough(&drive, img);
    let _ = get_drive_image_compatibility_statuses(&drive, img);
    let mut all = vec![drive];
    mark_invalid_drives(&mut all, img);
});
