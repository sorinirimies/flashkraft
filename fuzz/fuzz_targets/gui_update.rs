#![no_main]
use flashkraft_gui::core::{message::Message, state::FlashKraft};
use flashkraft_core::domain::DriveInfo;
use libfuzzer_sys::fuzz_target;

fn drive(n: u8) -> DriveInfo {
    let mut d = DriveInfo::new(format!("D{n}"), format!("/m/{n}"), f64::from(n) / 4.0, format!("/dev/sd{}", (b'a' + n % 6) as char));
    d.is_system = n % 5 == 0;
    d.is_read_only = n % 7 == 0;
    d
}

fn msg(b: [u8; 2]) -> Message {
    let n = b[1];
    match b[0] % 22 {
        0 => Message::SelectImageClicked,
        1 => Message::RefreshDrivesClicked,
        2 => Message::UsbHotplugDetected,
        3 => Message::TargetDriveClicked(drive(n)),
        4 => Message::ToggleMarkTarget(drive(n)),
        5 => Message::OpenDeviceSelection,
        6 => Message::CloseDeviceSelection,
        7 => Message::FlashClicked,
        8 => Message::ResetClicked,
        9 => Message::CancelClicked,
        10 => Message::CancelFlash,
        11 => Message::AnimationTick,
        12 => Message::ImageSelected(Some(format!("/nonexistent/{n}.iso").into())),
        13 => Message::ImageSelected(None),
        14 => Message::DrivesRefreshed((0..n % 6).map(drive).collect()),
        15 => Message::FlashProgressUpdate(f32::from(n) / 100.0, u64::from(n) << 20, f32::from(n)),
        16 => Message::VerifyProgressUpdate(f32::from(n) / 255.0, "verify", 1, 0, f32::NAN),
        17 => Message::Status(format!("s{n}")),
        18 => Message::FlashCompleted(if n % 2 == 0 { Ok(()) } else { Err("boom".into()) }),
        19 => Message::UpdateCheckCompleted(if n % 2 == 0 { Some("9.9.9".into()) } else { None }),
        20 => Message::DismissUpdateBanner,
        _ => Message::FlashProgressUpdate(f32::INFINITY, u64::MAX, -1.0),
    }
}

// Random message sequences must never panic the GUI reducer. Returned tasks are
// dropped, so no dialog opens and nothing is flashed.
fuzz_target!(|data: &[u8]| {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let home = std::env::temp_dir().join("flashkraft-fuzz-gui");
        let _ = std::fs::create_dir_all(&home);
        unsafe {
            std::env::set_var("FLASHKRAFT_DEMO", "1");
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &home);
        }
    });
    let mut app = FlashKraft::new();
    for pair in data.chunks_exact(2).take(256) {
        let _ = app.update(msg([pair[0], pair[1]]));
    }
});
