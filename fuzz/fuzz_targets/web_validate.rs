#![no_main]
use flashkraft_core::domain::{DriveInfo, ImageInfo};
use flash::{validate, StartRequest};
use libfuzzer_sys::fuzz_target;

// Compiled straight from the web crate's source so the fuzz binary does not
// need to link Topcoat (its link-section tricks break some linkers).
#[allow(dead_code)]
#[path = "../../crates/flashkraft-web/src/flash.rs"]
mod flash;

// Whatever JSON the browser sends, validation must not panic, and a request
// that passes may only name detected, non-system, writable drives — with the
// confirmed count equal to the number of distinct targets.
fuzz_target!(|data: &[u8]| {
    // Raw JSON rarely parses, so also derive a request from plain text lines:
    // first line = image, then devices; confirm = the byte count mod 20.
    let req = match serde_json::from_slice::<StartRequest>(data) {
        Ok(r) => r,
        Err(_) => {
            let text = String::from_utf8_lossy(data);
            let mut lines = text.lines();
            let image = lines.next().unwrap_or("").to_string();
            let devices: Vec<String> = lines.map(str::to_string).collect();
            StartRequest { image, confirm: data.len() % 20, devices }
        }
    };

    let mut drives = Vec::new();
    for (i, gb) in [32.0, 0.001, 500.0, 16.0].into_iter().enumerate() {
        let mut d = DriveInfo::new(format!("Drive {i}"), format!("/media/{i}"), gb, format!("/dev/sd{}", (b'b' + i as u8) as char));
        d.is_system = i == 2;
        d.is_read_only = i == 3 && data.len() % 2 == 0;
        drives.push(d);
    }
    let size_mb = (data.len() as f64) * 3.0;
    let image = if data.first() == Some(&b'{') && data.len() % 7 != 0 {
        Ok(ImageInfo { path: req.image.clone().into(), name: "x.iso".into(), size_mb })
    } else {
        Err("no image".to_string())
    };

    if let Ok((_, targets)) = validate(&req, &drives, image) {
        assert!(!targets.is_empty() && targets.len() <= flash::MAX_TARGETS);
        assert_eq!(targets.len(), req.confirm);
        for t in &targets {
            assert!(req.devices.contains(&t.device_path));
            assert!(!t.is_system && !t.is_read_only && !t.disabled);
            assert!(drives.iter().any(|d| d.device_path == t.device_path));
        }
    }
});
