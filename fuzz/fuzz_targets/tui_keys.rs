#![no_main]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use flashkraft_tui::{App, handle_key};
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

// Only keys that cannot mutate the filesystem. The file explorer binds
// delete/cut/paste/rename/confirm to letters, Delete and modifier combos, and as
// root in CI it can walk up out of the sandbox HOME and remove the corpus dir
// ("No such file or directory: /tmp/corpus/tui_keys"). So: no letters except
// harmless ones, no Delete, no Ctrl/Alt.
const SAFE: [KeyCode; 16] = [
    KeyCode::Enter, KeyCode::Esc, KeyCode::Backspace, KeyCode::Tab,
    KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right,
    KeyCode::Home, KeyCode::End, KeyCode::PageUp, KeyCode::PageDown,
    KeyCode::Char(' '), KeyCode::Char('/'), KeyCode::Char('m'), KeyCode::Char('f'),
];

fn key(b: [u8; 2]) -> KeyEvent {
    let code = SAFE[(b[0] as usize) % SAFE.len()];
    // Shift only; Ctrl/Alt carry the destructive explorer bindings.
    let mods = if b[1] & 1 == 1 { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
    KeyEvent::new(code, mods)
}

// Random key sequences must never panic the TUI state machine. Demo mode and a
// throw-away HOME guarantee no real drive is detected or written.
fuzz_target!(|data: &[u8]| {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let home = std::env::temp_dir().join("flashkraft-fuzz-tui");
        let _ = std::fs::create_dir_all(&home);
        // SAFETY: before any other thread exists.
        unsafe {
            std::env::set_var("FLASHKRAFT_DEMO", "1");
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &home);
        }
    });
    let rt = RT.get_or_init(|| tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap());
    let _guard = rt.enter();

    let mut app = App::new();
    for pair in data.chunks_exact(2).take(256) {
        let _ = handle_key(&mut app, key([pair[0], pair[1]]));
        if app.should_quit { break; }
    }
});
