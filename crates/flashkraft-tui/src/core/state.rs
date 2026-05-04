//! TUI Application State
//!
//! Defines the complete state machine for the FlashKraft terminal UI.
//! All state mutations go through `handle_key` keeping the logic
//! centralised and easy to reason about.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::sync::mpsc;

use crate::domain::{DriveInfo, ImageInfo};
use tui_file_explorer::{FileExplorer, Theme};

use super::storage::TuiStorage;
use crate::ui::theme::{all_app_themes, TuiPalette};
use flashkraft_core::THEME_NAMES;

use super::message::{
    AppScreen, ClipOp, FileClipboard, FileOpMode, FlashEvent, InputMode, UsbEntry,
};

// ---------------------------------------------------------------------------
// Main application state
// ---------------------------------------------------------------------------

pub struct App {
    // ── Screen ───────────────────────────────────────────────────────────────
    /// Active screen.
    pub screen: AppScreen,

    // ── Image selection ───────────────────────────────────────────────────────
    /// Raw text the user typed in the image-path field.
    pub image_input: String,
    /// Cursor position inside `image_input`.
    pub image_cursor: usize,
    /// Current input mode.
    pub input_mode: InputMode,
    /// Resolved & validated image, set after the user confirms.
    pub selected_image: Option<ImageInfo>,

    // ── Drive selection ───────────────────────────────────────────────────────
    /// All drives detected on the system.
    pub available_drives: Vec<DriveInfo>,
    /// Index of the currently highlighted drive in the list.
    pub drive_cursor: usize,
    /// The drive the user confirmed for flashing.
    pub selected_drive: Option<DriveInfo>,
    /// True while the background drive-detection task is running.
    pub drives_loading: bool,
    /// Channel receiving the result of the async drive-detection task.
    pub drives_rx: Option<mpsc::UnboundedReceiver<Vec<DriveInfo>>>,
    /// Channel receiving bare hotplug triggers from the filesystem watch task.
    /// Each received value means "a block device connected or disconnected —
    /// re-enumerate drives".  The payload is `()` because all drive data
    /// comes from the existing sysfs / diskutil / wmic enumeration code.
    pub hotplug_rx: Option<mpsc::UnboundedReceiver<()>>,

    // ── Flash operation ───────────────────────────────────────────────────────
    /// Progress 0.0–1.0.
    pub flash_progress: f32,
    /// Bytes written so far.
    pub flash_bytes: u64,
    /// Current write speed in MB/s.
    pub flash_speed: f32,
    /// Human-readable stage label (e.g. "Writing…").
    pub flash_stage: String,
    /// Recent log lines from the flash helper.
    pub flash_log: Vec<String>,
    /// Shared cancellation token.
    pub cancel_token: Arc<AtomicBool>,
    /// Channel receiving [`FlashEvent`]s from the background flash task.
    pub flash_rx: Option<mpsc::UnboundedReceiver<FlashEvent>>,

    // ── Verification ──────────────────────────────────────────────────────────
    /// Overall verification progress 0.0–1.0 across both passes (None = not yet started).
    pub verify_progress: Option<f32>,
    /// Current verification read speed in MB/s.
    pub verify_speed: f32,
    /// Active verification pass: "image", "device", or "" when not verifying.
    pub verify_phase: &'static str,

    // ── Completion ────────────────────────────────────────────────────────────
    /// File/directory entries found on the USB drive after flashing.
    pub usb_contents: Vec<UsbEntry>,
    /// Scroll offset for the USB contents list.
    pub contents_scroll: usize,

    // ── File explorer ─────────────────────────────────────────────────────────
    /// File-browser state used on the `BrowseImage` screen.
    pub file_explorer: FileExplorer,
    /// All available themes, as `(name, Theme)` pairs.
    pub explorer_themes: Vec<(String, Theme)>,
    /// All available app-level palettes, parallel to `explorer_themes`.
    pub app_themes: Vec<(String, TuiPalette)>,
    /// Index into `explorer_themes` / `app_themes` for the currently active theme.
    pub explorer_theme_idx: usize,
    /// Whether the global theme-picker side panel is visible (shown on every screen).
    pub show_app_theme_panel: bool,
    /// Whether the browse-screen options panel (sort, hidden files) is open.
    pub show_browse_options: bool,
    /// Whether the browse-screen editor-picker panel is open.
    pub show_browse_editor: bool,
    /// Cursor position inside the global theme panel (for keyboard navigation).
    pub app_theme_panel_cursor: usize,
    /// The `explorer_theme_idx` that was active when the panel was opened —
    /// restored on Esc so the live preview is rolled back if cancelled.
    pub theme_panel_prev_idx: usize,
    /// Redb-backed preference store — used to persist the active theme across restarts.
    pub storage: TuiStorage,
    /// Clipboard entry for copy/cut operations inside the file explorer.
    pub file_clipboard: Option<FileClipboard>,
    /// Pending confirmation mode for destructive file operations.
    pub file_op_mode: FileOpMode,
    /// Status message shown in the file-explorer overlay (e.g. "Yanked: foo.iso").
    pub file_op_status: String,

    // ── Error ─────────────────────────────────────────────────────────────────
    /// Error message shown on the error screen.
    pub error_message: String,

    // ── Misc ──────────────────────────────────────────────────────────────────
    /// Ticked upward on every terminal frame; used for animations.
    pub tick_count: u64,
    /// Set to true by the event loop when the user requests quit.
    pub should_quit: bool,
}

/// Generate a pair of cursor-navigation methods that clamp at list bounds.
macro_rules! cursor_nav {
    (up: $fn_up:ident, down: $fn_down:ident, cursor: $cursor:ident, list: $list:ident) => {
        impl App {
            pub fn $fn_up(&mut self) {
                if self.$cursor > 0 {
                    self.$cursor -= 1;
                }
            }

            pub fn $fn_down(&mut self) {
                if !self.$list.is_empty() && self.$cursor < self.$list.len().saturating_sub(1) {
                    self.$cursor += 1;
                }
            }
        }
    };
}

cursor_nav!(up: drive_up,        down: drive_down,        cursor: drive_cursor,           list: available_drives);
cursor_nav!(up: contents_up,      down: contents_down,      cursor: contents_scroll,        list: usb_contents);

impl App {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    pub fn new() -> Self {
        // Start the file explorer in the user's home directory (or current
        // working directory as a fallback), pre-filtered for image files.
        let start_dir = dirs::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/"));

        // Open persistent storage once and resolve the saved theme index.
        let storage = TuiStorage::open();
        let explorer_theme_idx = storage
            .load_theme()
            .and_then(|name| THEME_NAMES.iter().position(|n| *n == name))
            .unwrap_or(0);

        Self {
            screen: AppScreen::SelectImage,
            image_input: String::new(),
            image_cursor: 0,
            input_mode: InputMode::Editing, // start in edit mode for convenience
            selected_image: None,
            available_drives: Vec::new(),
            drive_cursor: 0,
            selected_drive: None,
            drives_loading: false,
            drives_rx: None,
            hotplug_rx: None,
            flash_progress: 0.0,
            flash_bytes: 0,
            flash_speed: 0.0,
            flash_stage: "Initialising\u{2026}".to_string(),
            flash_log: Vec::new(),
            cancel_token: Arc::new(AtomicBool::new(false)),
            flash_rx: None,
            verify_progress: None,
            verify_speed: 0.0,
            verify_phase: "",
            usb_contents: Vec::new(),
            contents_scroll: 0,
            file_explorer: FileExplorer::new(start_dir, vec!["iso".into(), "img".into()]),
            explorer_themes: Theme::all_presets()
                .into_iter()
                .map(|(name, _, t)| (name.to_string(), t))
                .collect(),
            app_themes: all_app_themes(),
            explorer_theme_idx,
            show_app_theme_panel: false,
            show_browse_options: false,
            show_browse_editor: false,
            app_theme_panel_cursor: 0,
            theme_panel_prev_idx: 0,
            storage,
            file_clipboard: None,
            file_op_mode: FileOpMode::Normal,
            file_op_status: String::new(),
            error_message: String::new(),
            tick_count: 0,
            should_quit: false,
        }
    }

    // -----------------------------------------------------------------------
    // Channel polling — called on every tick from the main event loop
    // -----------------------------------------------------------------------

    /// Kick off an asynchronous drive-detection task.
    ///
    /// Sets the loading flag, clears the current drive list, creates a
    /// channel, and spawns a Tokio task that calls
    /// [`crate::core::commands::load_drives`].  The result is later consumed
    /// by [`Self::poll_drives`].
    ///
    /// This is called from the hotplug handler, from key-event refresh
    /// (`r` / `F5`), and after confirming an image path.
    pub fn start_drive_detection(&mut self) {
        self.drives_loading = true;
        self.available_drives.clear();
        self.drive_cursor = 0;

        let (tx, rx) = mpsc::unbounded_channel::<Vec<crate::domain::DriveInfo>>();
        self.drives_rx = Some(rx);

        tokio::spawn(async move {
            let drives = crate::core::commands::load_drives().await;
            let _ = tx.send(drives);
        });
    }

    /// Drain the USB hotplug channel and trigger drive re-detection on any event.
    ///
    /// Called on every tick from the main event loop.  A received `()` means
    /// a USB device was connected or disconnected; we respond by kicking off
    /// a fresh drive enumeration — identical to the user pressing `r`/`F5`.
    ///
    /// The hotplug task is started once in `run_app` and runs for the lifetime
    /// of the application.  We keep the receiver here so poll is cheap
    /// (non-blocking `try_recv`) and the watcher is never re-created.
    pub fn poll_hotplug(&mut self) {
        let mut rx = match self.hotplug_rx.take() {
            Some(r) => r,
            None => return,
        };

        // Drain all pending events (there may be bursts when a hub is
        // plugged in with multiple devices attached).  A single drive
        // re-detection is enough regardless of burst size.
        let mut triggered = false;
        while rx.try_recv().is_ok() {
            triggered = true;
        }

        // Put the receiver back before potentially starting detection,
        // so that future ticks can continue to receive events.
        self.hotplug_rx = Some(rx);

        if triggered && !self.drives_loading {
            // Only re-detect when we are not already mid-detection and not
            // actively flashing (disturbing the drive list during a flash
            // would be confusing and is unnecessary).
            let flashing = matches!(self.screen, AppScreen::Flashing);
            if !flashing {
                self.start_drive_detection();
            }
        }
    }

    /// Drain the drive-detection channel (if active) and apply results.
    pub fn poll_drives(&mut self) {
        // Take ownership temporarily so we can mutate `self` freely.
        let mut rx = match self.drives_rx.take() {
            Some(r) => r,
            None => return,
        };

        if let Ok(drives) = rx.try_recv() {
            self.available_drives = drives;
            self.drives_loading = false;
            self.drive_cursor = 0;
            // Don't put the receiver back — detection is one-shot.
        } else {
            // Not ready yet — put it back.
            self.drives_rx = Some(rx);
        }
    }

    /// Drain the flash-progress channel (if active) and apply results.
    pub fn poll_flash(&mut self) {
        let mut rx = match self.flash_rx.take() {
            Some(r) => r,
            None => return,
        };

        loop {
            match rx.try_recv() {
                Ok(event) => self.apply_flash_event(event),
                Err(mpsc::error::TryRecvError::Empty) => {
                    self.flash_rx = Some(rx);
                    break;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Task ended — channel closed; don't put receiver back.
                    break;
                }
            }
        }
    }

    fn apply_flash_event(&mut self, event: FlashEvent) {
        match event {
            FlashEvent::Progress {
                progress,
                bytes_written,
                speed_mb_s,
            } => {
                // Writing phase occupies 0–80 % of the overall progress bar.
                // Clamp to 0.80 so the bar never jumps back when post-write
                // stages set a stage-floor above the raw write percentage.
                self.flash_progress = (progress * 0.80).clamp(0.0, 0.80);
                self.flash_bytes = bytes_written;
                self.flash_speed = speed_mb_s;
            }
            FlashEvent::VerifyProgress {
                phase,
                overall,
                bytes_read: _,
                total_bytes: _,
                speed_mb_s,
            } => {
                self.verify_progress = Some(overall);
                self.verify_speed = speed_mb_s;
                self.verify_phase = phase;
                // Drive the main bar through the verify window (92–100 %).
                let bar = 0.92 + overall * 0.08;
                if bar > self.flash_progress {
                    self.flash_progress = bar;
                }
            }
            FlashEvent::Message(s) => {
                // Advance the progress bar floor for post-write stages so it
                // keeps moving rather than sitting at 80 % (or 100 % from the
                // final write-progress event) while sync / verify are running.
                //
                // Stage → overall-progress floor mapping:
                //   Unmounting  →  0 %   (pre-write housekeeping)
                //   Writing     →  0 %   (real progress comes from Progress events)
                //   Syncing     → 80 %   (flushing write-back caches)
                //   Rereading   → 88 %   (kernel re-reads partition table)
                //   Verifying   → 92 %   (SHA-256 read-back comparison)
                //   Done        → 99 %   (pipeline Done event sets 100 %)
                let syncing_str = flashkraft_core::FlashStage::Syncing.to_string();
                let rereading_str = flashkraft_core::FlashStage::Rereading.to_string();
                let verifying_str = flashkraft_core::FlashStage::Verifying.to_string();
                let floor: f32 = if s == syncing_str {
                    flashkraft_core::FlashStage::Syncing.progress_floor()
                } else if s == rereading_str {
                    flashkraft_core::FlashStage::Rereading.progress_floor()
                } else if s == verifying_str {
                    flashkraft_core::FlashStage::Verifying.progress_floor()
                } else {
                    0.0
                };
                // Only ever move forward — never reduce progress.
                if floor > self.flash_progress {
                    self.flash_progress = floor;
                }
                self.flash_stage = s.clone();
                self.push_log(s);
            }
            FlashEvent::Completed => {
                self.flash_progress = 1.0;
                self.flash_stage = "Complete!".to_string();
                self.push_log("Flash operation completed successfully.".to_string());
                self.scan_usb_contents();
                self.screen = AppScreen::Complete;
            }
            FlashEvent::Failed(err) => {
                self.error_message = err;
                self.screen = AppScreen::Error;
            }
        }
    }

    fn push_log(&mut self, msg: String) {
        const MAX_LOG: usize = 200;
        self.flash_log.push(msg);
        if self.flash_log.len() > MAX_LOG {
            self.flash_log.drain(0..self.flash_log.len() - MAX_LOG);
        }
    }

    // -----------------------------------------------------------------------
    // Image input helpers
    // -----------------------------------------------------------------------

    /// Insert a character at the current cursor position.
    pub fn image_insert(&mut self, c: char) {
        // Guard against inserting non-UTF-8-safe chars (shouldn't happen via
        // crossterm, but be safe).
        let byte_pos = self
            .image_input
            .char_indices()
            .nth(self.image_cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.image_input.len());
        self.image_input.insert(byte_pos, c);
        self.image_cursor += 1;
    }

    /// Delete the character before the cursor (backspace).
    pub fn image_backspace(&mut self) {
        if self.image_cursor == 0 {
            return;
        }
        let byte_pos = self
            .image_input
            .char_indices()
            .nth(self.image_cursor - 1)
            .map(|(i, _)| i)
            .unwrap_or(self.image_input.len());
        self.image_input.remove(byte_pos);
        self.image_cursor -= 1;
    }

    /// Move the cursor one character to the left.
    pub fn image_cursor_left(&mut self) {
        if self.image_cursor > 0 {
            self.image_cursor -= 1;
        }
    }

    /// Move the cursor one character to the right.
    pub fn image_cursor_right(&mut self) {
        let len = self.image_input.chars().count();
        if self.image_cursor < len {
            self.image_cursor += 1;
        }
    }

    // -----------------------------------------------------------------------
    // State transitions
    // -----------------------------------------------------------------------

    /// Validate the image path and advance to the drive-selection screen.
    ///
    /// Returns `Err` with a human-readable message if the path is invalid.
    pub fn confirm_image(&mut self) -> Result<(), String> {
        let path = PathBuf::from(self.image_input.trim());
        if !path.exists() {
            return Err(format!("File not found: {}", path.display()));
        }
        if !path.is_file() {
            return Err(format!("Not a file: {}", path.display()));
        }

        let info = ImageInfo::from_path(path);
        if info.size_mb == 0.0 {
            return Err("Image file appears to be empty.".to_string());
        }

        self.selected_image = Some(info);
        self.input_mode = InputMode::Normal;
        self.screen = AppScreen::SelectDrive;
        Ok(())
    }

    /// Confirm the highlighted drive and advance to the drive-info screen.
    pub fn confirm_drive(&mut self) -> Result<(), String> {
        let drive = self
            .available_drives
            .get(self.drive_cursor)
            .cloned()
            .ok_or_else(|| "No drive selected.".to_string())?;

        if drive.is_system {
            return Err(format!(
                "{} is a system drive and cannot be used as a flash target.",
                drive.name
            ));
        }
        if drive.is_read_only {
            return Err(format!("{} is read-only.", drive.name));
        }

        self.selected_drive = Some(drive);
        self.screen = AppScreen::DriveInfo;
        Ok(())
    }

    /// Advance from drive-info to the confirmation screen.
    pub fn advance_to_confirm(&mut self) {
        self.screen = AppScreen::ConfirmFlash;
    }

    /// Begin the flash operation.
    ///
    /// Returns `Err` if state is inconsistent (image / drive not set).
    pub fn begin_flash(&mut self) -> Result<(), String> {
        let image = self
            .selected_image
            .as_ref()
            .ok_or("No image selected.")?
            .clone();
        let drive = self
            .selected_drive
            .as_ref()
            .ok_or("No drive selected.")?
            .clone();

        // Reset flash state.
        self.flash_progress = 0.0;
        self.flash_bytes = 0;
        self.flash_speed = 0.0;
        self.flash_stage = "Starting…".to_string();
        self.flash_log.clear();
        self.cancel_token = Arc::new(AtomicBool::new(false));
        self.verify_progress = None;
        self.verify_speed = 0.0;
        self.verify_phase = "";

        let (tx, rx) = mpsc::unbounded_channel::<FlashEvent>();
        self.flash_rx = Some(rx);

        let cancel = self.cancel_token.clone();

        // Spawn the flash task onto the Tokio runtime that owns this thread.
        tokio::spawn(crate::core::flash_runner::run_flash(
            image.path,
            PathBuf::from(&drive.device_path),
            cancel,
            tx,
        ));

        self.screen = AppScreen::Flashing;
        Ok(())
    }

    /// Cancel an in-progress flash operation.
    pub fn cancel_flash(&mut self) {
        self.cancel_token.store(true, Ordering::SeqCst);
        self.error_message = "Flash operation cancelled by user.".to_string();
        self.screen = AppScreen::Error;
    }

    /// Full reset — go back to the first screen.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Go back one step (where meaningful).
    pub fn go_back(&mut self) {
        match self.screen {
            AppScreen::BrowseImage => {
                self.screen = AppScreen::SelectImage;
                self.input_mode = InputMode::Editing;
            }
            AppScreen::SelectDrive => {
                self.screen = AppScreen::SelectImage;
                self.input_mode = InputMode::Editing;
            }
            AppScreen::DriveInfo => {
                self.screen = AppScreen::SelectDrive;
            }
            AppScreen::ConfirmFlash => {
                self.screen = AppScreen::DriveInfo;
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // File explorer helpers
    // -----------------------------------------------------------------------

    /// Open the interactive file browser, navigating to the directory of the
    /// currently typed path (if valid) or falling back to the home directory.
    pub fn open_file_explorer(&mut self) {
        // Try to derive a sensible starting directory from the current input.
        let typed = PathBuf::from(self.image_input.trim());
        let start_dir = if typed.is_dir() {
            typed
        } else if let Some(parent) = typed.parent().filter(|p| p.is_dir()) {
            parent.to_path_buf()
        } else {
            dirs::home_dir()
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("/"))
        };

        self.file_explorer.navigate_to(start_dir);
        self.screen = AppScreen::BrowseImage;
    }

    /// Accept a path chosen by the file explorer, populate the image-input
    /// field, and return to the `SelectImage` screen.
    pub fn apply_explorer_selection(&mut self, path: PathBuf) {
        self.image_input = path.to_string_lossy().to_string();
        self.image_cursor = self.image_input.chars().count();
        self.input_mode = InputMode::Editing;
        self.screen = AppScreen::SelectImage;
    }

    // -----------------------------------------------------------------------
    // File explorer — theme helpers
    // -----------------------------------------------------------------------

    /// The currently active file-explorer theme.
    ///
    /// Clamps the index to the explorer's smaller catalogue (27 entries vs
    /// the full 28-entry app palette), so themes beyond the explorer's range
    /// (e.g. Cyberpunk) fall back to the Default explorer theme.
    pub fn current_explorer_theme(&self) -> &Theme {
        let idx = self
            .explorer_theme_idx
            .min(self.explorer_themes.len().saturating_sub(1));
        &self.explorer_themes[idx].1
    }

    /// The currently active TUI application palette.
    pub fn palette(&self) -> &TuiPalette {
        &self.app_themes[self.explorer_theme_idx].1
    }

    /// The display name of the currently active theme.
    pub fn current_theme_name(&self) -> &str {
        &self.app_themes[self.explorer_theme_idx].0
    }

    /// Advance to the next theme (wraps around) and persist the choice.
    pub fn next_explorer_theme(&mut self) {
        self.explorer_theme_idx = (self.explorer_theme_idx + 1) % self.app_themes.len();
        self.persist_theme();
    }

    /// Go back to the previous theme (wraps around) and persist the choice.
    pub fn prev_explorer_theme(&mut self) {
        let n = self.app_themes.len();
        self.explorer_theme_idx = (self.explorer_theme_idx + n - 1) % n;
        self.persist_theme();
    }

    // -----------------------------------------------------------------------
    // Global theme panel
    // -----------------------------------------------------------------------

    /// Open the global theme panel, positioning the cursor at the active theme.
    pub fn open_app_theme_panel(&mut self) {
        self.theme_panel_prev_idx = self.explorer_theme_idx; // save for Esc rollback
        self.app_theme_panel_cursor = self.explorer_theme_idx;
        self.show_app_theme_panel = true;
    }

    /// Close the global theme panel, reverting the live preview to the original theme.
    pub fn close_app_theme_panel(&mut self) {
        self.explorer_theme_idx = self.theme_panel_prev_idx; // roll back live preview
        self.show_app_theme_panel = false;
    }

    /// Apply the theme currently under the panel cursor, persist it, and close the panel.
    pub fn theme_panel_confirm(&mut self) {
        // explorer_theme_idx is already set to app_theme_panel_cursor via live preview
        self.show_app_theme_panel = false;
        self.persist_theme();
    }

    /// Move the theme-panel cursor up and immediately preview the theme.
    pub fn theme_panel_up(&mut self) {
        if self.app_theme_panel_cursor > 0 {
            self.app_theme_panel_cursor -= 1;
            self.explorer_theme_idx = self.app_theme_panel_cursor;
        }
    }

    /// Move the theme-panel cursor down and immediately preview the theme.
    pub fn theme_panel_down(&mut self) {
        let last = self.app_themes.len().saturating_sub(1);
        if self.app_theme_panel_cursor < last {
            self.app_theme_panel_cursor += 1;
            self.explorer_theme_idx = self.app_theme_panel_cursor;
        }
    }

    /// Write the current theme name to the JSON settings file.
    fn persist_theme(&mut self) {
        let name = self.app_themes[self.explorer_theme_idx].0.clone();
        self.storage.save_theme(&name);
    }

    // -----------------------------------------------------------------------
    // File explorer — clipboard / file operations
    // -----------------------------------------------------------------------

    /// Yank (copy or cut) the currently highlighted entry.
    pub fn explorer_yank(&mut self, op: ClipOp) {
        if let Some(entry) = self.file_explorer.current_entry() {
            let label = match op {
                ClipOp::Copy => "Yanked",
                ClipOp::Cut => "Cut",
            };
            self.file_op_status = format!("{}: {}", label, entry.name);
            self.file_clipboard = Some(FileClipboard {
                path: entry.path.clone(),
                op,
            });
        }
    }

    /// Initiate a paste: if destination exists, request overwrite confirmation;
    /// otherwise perform the operation immediately.
    pub fn explorer_initiate_paste(&mut self) {
        let Some(clip) = self.file_clipboard.clone() else {
            self.file_op_status = "Clipboard is empty.".to_string();
            return;
        };
        let dst = self
            .file_explorer
            .current_dir
            .join(clip.path.file_name().unwrap_or_default());
        if dst.exists() {
            self.file_op_mode = FileOpMode::ConfirmOverwrite {
                src: clip.path,
                dst,
                op: clip.op,
            };
        } else {
            self.explorer_do_paste(&clip.path.clone(), &dst.clone(), clip.op);
        }
    }

    /// Perform the actual copy/move after confirmation.
    pub fn explorer_do_paste(&mut self, src: &std::path::Path, dst: &std::path::Path, op: ClipOp) {
        match explorer_fs_copy(src, dst) {
            Ok(()) => {
                if op == ClipOp::Cut {
                    if let Err(e) = explorer_fs_delete(src) {
                        self.file_op_status = format!("Cut: copy OK but delete failed: {e}");
                    } else {
                        self.file_clipboard = None;
                        self.file_op_status = format!(
                            "Moved to {}",
                            dst.file_name().unwrap_or_default().to_string_lossy()
                        );
                    }
                } else {
                    self.file_op_status = format!(
                        "Copied to {}",
                        dst.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
            }
            Err(e) => self.file_op_status = format!("Paste failed: {e}"),
        }
        self.file_op_mode = FileOpMode::Normal;
        self.file_explorer.reload();
    }

    /// Enter delete-confirmation mode for the highlighted entry.
    pub fn explorer_initiate_delete(&mut self) {
        if let Some(entry) = self.file_explorer.current_entry() {
            self.file_op_mode = FileOpMode::ConfirmDelete(entry.path.clone());
        }
    }

    /// Perform the delete after the user confirmed with y.
    pub fn explorer_do_delete(&mut self, path: std::path::PathBuf) {
        match explorer_fs_delete(&path) {
            Ok(()) => {
                self.file_op_status = format!(
                    "Deleted: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            }
            Err(e) => self.file_op_status = format!("Delete failed: {e}"),
        }
        self.file_op_mode = FileOpMode::Normal;
        self.file_explorer.reload();
    }

    // -----------------------------------------------------------------------
    // USB content scanning (post-flash)
    // -----------------------------------------------------------------------

    /// Scan the USB device's first mount point for files and populate
    /// `self.usb_contents`.  Falls back gracefully if the device is not
    /// yet mounted or the scan fails.
    fn scan_usb_contents(&mut self) {
        self.usb_contents.clear();

        let device_path = match &self.selected_drive {
            Some(d) => d.device_path.clone(),
            None => return,
        };

        // Try to find a mount point for the device (or any of its partitions).
        let mount_point = find_mount_point(&device_path);

        if let Some(mp) = mount_point {
            let root = PathBuf::from(&mp);
            let mut entries = Vec::new();
            collect_entries(&root, 0, &mut entries, 3); // max 3 levels deep
            self.usb_contents = entries;
        } else {
            // Device not (yet) mounted — show a placeholder.
            self.usb_contents.push(UsbEntry {
                name: "(device not mounted — re-plug to browse)".to_string(),
                size_bytes: 0,
                is_dir: false,
                depth: 0,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Convenience accessors
    // -----------------------------------------------------------------------

    /// Image size in bytes (0 if no image selected).
    pub fn image_size_bytes(&self) -> u64 {
        self.selected_image
            .as_ref()
            .map(|i| (i.size_mb * 1024.0 * 1024.0) as u64)
            .unwrap_or(0)
    }

    /// Drive capacity in bytes (0 if no drive selected).
    pub fn drive_size_bytes(&self) -> u64 {
        self.selected_drive
            .as_ref()
            .map(|d| (d.size_gb * 1024.0 * 1024.0 * 1024.0) as u64)
            .unwrap_or(0)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Parse `/proc/mounts` (Linux) or `/etc/mtab` to find a mount point for the
/// given block device or any of its partitions.
fn find_mount_point(device_path: &str) -> Option<String> {
    let device_base = std::path::Path::new(device_path)
        .file_name()?
        .to_string_lossy()
        .to_string();

    // Try /proc/mounts first (Linux), fall back to /etc/mtab.
    let mounts_content = std::fs::read_to_string("/proc/mounts")
        .or_else(|_| std::fs::read_to_string("/etc/mtab"))
        .ok()?;

    for line in mounts_content.lines() {
        let mut parts = line.split_whitespace();
        let dev = parts.next().unwrap_or("");
        let mp = parts.next().unwrap_or("");

        if mp.is_empty() || dev.is_empty() {
            continue;
        }

        let dev_base = std::path::Path::new(dev)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Match the device itself or any of its partitions (sdb, sdb1, sdb2…).
        if dev_base == device_base || dev_base.starts_with(&device_base) {
            // Skip pseudo filesystems.
            if !mp.starts_with('/') || mp == "/" {
                continue;
            }
            return Some(mp.to_string());
        }
    }

    None
}

/// Recursively collect file/directory entries up to `max_depth` levels.
fn collect_entries(dir: &PathBuf, depth: usize, out: &mut Vec<UsbEntry>, max_depth: usize) {
    if depth > max_depth {
        return;
    }

    let read_result = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };

    // Collect and sort entries for a deterministic display order.
    let mut entries: Vec<_> = read_result.flatten().collect();
    entries.sort_by_key(|e| {
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        // Directories first, then files, each sorted alphabetically.
        (!is_dir, e.file_name().to_string_lossy().to_lowercase())
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files / special entries at the root level.
        if depth == 0 && name.starts_with('.') {
            continue;
        }

        let meta = path.metadata();
        let (is_dir, size) = meta
            .as_ref()
            .map(|m| (m.is_dir(), m.len()))
            .unwrap_or((false, 0));

        out.push(UsbEntry {
            name,
            size_bytes: size,
            is_dir,
            depth,
        });

        if is_dir && depth < max_depth {
            collect_entries(&path, depth + 1, out, max_depth);
        }
    }
}

// ---------------------------------------------------------------------------
// File-system helpers (used by file-op methods above)
// ---------------------------------------------------------------------------

fn explorer_fs_copy(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    use std::fs;
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)?.flatten() {
            explorer_fs_copy(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

fn explorer_fs_delete(path: &std::path::Path) -> std::io::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // ── Test helpers ─────────────────────────────────────────────────────────

    /// Create a test drive with the given flags.
    fn make_drive(name: &str, device: &str, system: bool, ro: bool) -> DriveInfo {
        DriveInfo::with_constraints(
            name.into(),
            format!("/media/{name}"),
            16.0,
            device.into(),
            system,
            ro,
        )
    }

    /// Create a test ImageInfo with a known size.
    fn make_image(size_mb: f64) -> ImageInfo {
        ImageInfo {
            path: PathBuf::from("/tmp/test_image.img"),
            name: "test_image.img".into(),
            size_mb,
        }
    }

    // ── Initial state ─────────────────────────────────────────────────────────

    #[test]
    fn test_new_initial_state() {
        let app = App::new();
        assert_eq!(app.screen, AppScreen::SelectImage);
        assert_eq!(app.input_mode, InputMode::Editing);
        assert!(app.image_input.is_empty());
        assert_eq!(app.image_cursor, 0);
        assert!(app.available_drives.is_empty());
        assert!(app.selected_image.is_none());
        assert!(app.selected_drive.is_none());
        assert!(!app.should_quit);
        assert_eq!(app.flash_progress, 0.0);
        assert_eq!(app.flash_bytes, 0);
        assert_eq!(app.flash_speed, 0.0);
        assert!(app.flash_log.is_empty());
        assert_eq!(app.tick_count, 0);
        assert!(app.error_message.is_empty());
        assert!(app.usb_contents.is_empty());
        assert_eq!(app.contents_scroll, 0);
        assert!(!app.drives_loading);
    }

    #[test]
    fn test_default_equals_new() {
        let a = App::new();
        let b = App::default();
        // Both should be in the same initial screen
        assert_eq!(a.screen, b.screen);
        assert_eq!(a.input_mode, b.input_mode);
        assert_eq!(a.image_input, b.image_input);
    }

    // ── Image text field ──────────────────────────────────────────────────────

    #[test]
    fn test_image_insert_appends_and_advances_cursor() {
        let mut app = App::new();
        app.image_insert('h');
        app.image_insert('i');
        assert_eq!(app.image_input, "hi");
        assert_eq!(app.image_cursor, 2);
    }

    #[test]
    fn test_image_insert_at_middle_position() {
        let mut app = App::new();
        for c in "abcd".chars() {
            app.image_insert(c);
        }
        // Move cursor to position 2 (between 'b' and 'c')
        app.image_cursor = 2;
        app.image_insert('X');
        assert_eq!(app.image_input, "abXcd");
        assert_eq!(app.image_cursor, 3);
    }

    #[test]
    fn test_image_insert_unicode_chars() {
        let mut app = App::new();
        app.image_insert('\u{2192}');
        app.image_insert('\u{221e}');
        assert_eq!(app.image_input, "\u{2192}\u{221e}");
        assert_eq!(app.image_cursor, 2);
    }

    #[test]
    fn test_image_backspace_deletes_previous_char() {
        let mut app = App::new();
        for c in "hello".chars() {
            app.image_insert(c);
        }
        app.image_backspace();
        assert_eq!(app.image_input, "hell");
        assert_eq!(app.image_cursor, 4);
    }

    #[test]
    fn test_image_backspace_at_start_is_noop() {
        let mut app = App::new();
        app.image_insert('a');
        app.image_cursor = 0;
        app.image_backspace();
        // Nothing deleted, cursor stays at 0
        assert_eq!(app.image_input, "a");
        assert_eq!(app.image_cursor, 0);
    }

    #[test]
    fn test_image_backspace_empty_string_is_noop() {
        let mut app = App::new();
        app.image_backspace(); // should not panic
        assert!(app.image_input.is_empty());
        assert_eq!(app.image_cursor, 0);
    }

    #[test]
    fn test_image_cursor_left_clamps_at_zero() {
        let mut app = App::new();
        app.image_cursor_left(); // already 0 — no-op
        assert_eq!(app.image_cursor, 0);
    }

    #[test]
    fn test_image_cursor_left_decrements() {
        let mut app = App::new();
        for c in "abc".chars() {
            app.image_insert(c);
        }
        app.image_cursor_left();
        assert_eq!(app.image_cursor, 2);
    }

    #[test]
    fn test_image_cursor_right_clamps_at_end() {
        let mut app = App::new();
        for c in "abc".chars() {
            app.image_insert(c);
        }
        // Cursor is already at end (3)
        app.image_cursor_right();
        assert_eq!(app.image_cursor, 3);
    }

    #[test]
    fn test_image_cursor_right_increments() {
        let mut app = App::new();
        for c in "abc".chars() {
            app.image_insert(c);
        }
        app.image_cursor = 0;
        app.image_cursor_right();
        assert_eq!(app.image_cursor, 1);
    }

    #[test]
    fn test_image_cursor_full_left_right_round_trip() {
        let mut app = App::new();
        let text = "/home/user/ubuntu.iso";
        for c in text.chars() {
            app.image_insert(c);
        }
        let end = text.chars().count();
        assert_eq!(app.image_cursor, end);

        // Move all the way left
        for _ in 0..end {
            app.image_cursor_left();
        }
        assert_eq!(app.image_cursor, 0);

        // Move all the way right
        for _ in 0..end {
            app.image_cursor_right();
        }
        assert_eq!(app.image_cursor, end);
    }

    // ── confirm_image ─────────────────────────────────────────────────────────

    #[test]
    fn test_confirm_image_nonexistent_file_returns_err() {
        let mut app = App::new();
        app.image_input = "/nonexistent/path/does_not_exist.iso".into();
        let result = app.confirm_image();
        assert!(result.is_err(), "expected Err for missing file");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("not found") || msg.contains("Not a file") || msg.contains("File"),
            "error should mention the file: {msg}"
        );
        // Screen must not have advanced
        assert_eq!(app.screen, AppScreen::SelectImage);
        assert!(app.selected_image.is_none());
    }

    #[test]
    fn test_confirm_image_real_file_advances_screen() {
        use std::io::Write;
        let path = std::env::temp_dir().join("fk_test_confirm_image.img");
        {
            let mut f = std::fs::File::create(&path).expect("create temp");
            f.write_all(&[0xABu8; 2048]).expect("write");
        }

        let mut app = App::new();
        app.image_input = path.to_string_lossy().into();
        let result = app.confirm_image();
        let _ = std::fs::remove_file(&path);

        assert!(
            result.is_ok(),
            "expected Ok for real file, got: {:?}",
            result
        );
        assert_eq!(app.screen, AppScreen::SelectDrive);
        assert_eq!(app.input_mode, InputMode::Normal);
        let img = app.selected_image.expect("image should be set");
        assert_eq!(img.name, "fk_test_confirm_image.img");
        assert!(img.size_mb > 0.0);
    }

    #[test]
    fn test_confirm_image_directory_returns_err() {
        let mut app = App::new();
        app.image_input = std::env::temp_dir().to_string_lossy().into();
        let result = app.confirm_image();
        assert!(result.is_err(), "expected Err for directory path");
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    // ── Drive navigation ──────────────────────────────────────────────────────

    #[test]
    fn test_drive_up_clamps_at_zero() {
        let mut app = App::new();
        app.available_drives = vec![
            make_drive("d0", "/dev/sdb", false, false),
            make_drive("d1", "/dev/sdc", false, false),
        ];
        app.drive_cursor = 0;
        app.drive_up(); // should clamp
        assert_eq!(app.drive_cursor, 0);
    }

    #[test]
    fn test_drive_down_increments() {
        let mut app = App::new();
        app.available_drives = vec![
            make_drive("d0", "/dev/sdb", false, false),
            make_drive("d1", "/dev/sdc", false, false),
            make_drive("d2", "/dev/sdd", false, false),
        ];
        app.drive_down();
        assert_eq!(app.drive_cursor, 1);
        app.drive_down();
        assert_eq!(app.drive_cursor, 2);
    }

    #[test]
    fn test_drive_down_clamps_at_last() {
        let mut app = App::new();
        app.available_drives = vec![
            make_drive("d0", "/dev/sdb", false, false),
            make_drive("d1", "/dev/sdc", false, false),
        ];
        app.drive_cursor = 1; // already at last
        app.drive_down();
        assert_eq!(app.drive_cursor, 1);
    }

    #[test]
    fn test_drive_up_decrements() {
        let mut app = App::new();
        app.available_drives = vec![
            make_drive("d0", "/dev/sdb", false, false),
            make_drive("d1", "/dev/sdc", false, false),
        ];
        app.drive_cursor = 1;
        app.drive_up();
        assert_eq!(app.drive_cursor, 0);
    }

    #[test]
    fn test_drive_navigation_on_empty_list() {
        let mut app = App::new();
        app.drive_up(); // should not panic
        app.drive_down(); // should not panic
        assert_eq!(app.drive_cursor, 0);
    }

    // ── confirm_drive ─────────────────────────────────────────────────────────

    #[test]
    fn test_confirm_drive_ok_advances_to_drive_info() {
        let mut app = App::new();
        app.screen = AppScreen::SelectDrive;
        app.available_drives = vec![make_drive("usb0", "/dev/sdb", false, false)];
        app.drive_cursor = 0;

        let result = app.confirm_drive();
        assert!(result.is_ok());
        assert_eq!(app.screen, AppScreen::DriveInfo);
        let drive = app.selected_drive.expect("drive should be set");
        assert_eq!(drive.name, "usb0");
        assert_eq!(drive.device_path, "/dev/sdb");
    }

    #[test]
    fn test_confirm_drive_system_drive_rejected() {
        let mut app = App::new();
        app.screen = AppScreen::SelectDrive;
        app.available_drives = vec![make_drive("sysdrv", "/dev/sda", true, false)];
        app.drive_cursor = 0;

        let result = app.confirm_drive();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("system") || msg.contains("sysdrv"),
            "error should mention system: {msg}"
        );
        assert_eq!(app.screen, AppScreen::SelectDrive);
        assert!(app.selected_drive.is_none());
    }

    #[test]
    fn test_confirm_drive_readonly_rejected() {
        let mut app = App::new();
        app.screen = AppScreen::SelectDrive;
        app.available_drives = vec![make_drive("ro_usb", "/dev/sdb", false, true)];
        app.drive_cursor = 0;

        let result = app.confirm_drive();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("read-only") || msg.contains("ro_usb"),
            "error should mention read-only: {msg}"
        );
    }

    #[test]
    fn test_confirm_drive_empty_list_returns_err() {
        let mut app = App::new();
        app.screen = AppScreen::SelectDrive;
        // no drives
        let result = app.confirm_drive();
        assert!(result.is_err());
        assert!(app.selected_drive.is_none());
    }

    #[test]
    fn test_confirm_drive_cursor_out_of_bounds_returns_err() {
        let mut app = App::new();
        app.screen = AppScreen::SelectDrive;
        app.available_drives = vec![make_drive("usb0", "/dev/sdb", false, false)];
        app.drive_cursor = 99; // out of bounds

        let result = app.confirm_drive();
        assert!(result.is_err());
    }

    // ── Screen transitions ────────────────────────────────────────────────────

    #[test]
    fn test_advance_to_confirm() {
        let mut app = App::new();
        app.screen = AppScreen::DriveInfo;
        app.advance_to_confirm();
        assert_eq!(app.screen, AppScreen::ConfirmFlash);
    }

    #[test]
    fn test_go_back_confirm_flash_to_drive_info() {
        let mut app = App::new();
        app.screen = AppScreen::ConfirmFlash;
        app.go_back();
        assert_eq!(app.screen, AppScreen::DriveInfo);
    }

    #[test]
    fn test_go_back_drive_info_to_select_drive() {
        let mut app = App::new();
        app.screen = AppScreen::DriveInfo;
        app.go_back();
        assert_eq!(app.screen, AppScreen::SelectDrive);
    }

    #[test]
    fn test_go_back_select_drive_to_select_image_and_editing() {
        let mut app = App::new();
        app.screen = AppScreen::SelectDrive;
        app.input_mode = InputMode::Normal;
        app.go_back();
        assert_eq!(app.screen, AppScreen::SelectImage);
        assert_eq!(
            app.input_mode,
            InputMode::Editing,
            "go_back from SelectDrive should re-enable editing mode"
        );
    }

    #[test]
    fn test_go_back_is_noop_from_flashing() {
        let mut app = App::new();
        app.screen = AppScreen::Flashing;
        app.go_back();
        assert_eq!(app.screen, AppScreen::Flashing);
    }

    #[test]
    fn test_go_back_is_noop_from_complete() {
        let mut app = App::new();
        app.screen = AppScreen::Complete;
        app.go_back();
        assert_eq!(app.screen, AppScreen::Complete);
    }

    #[test]
    fn test_go_back_is_noop_from_error() {
        let mut app = App::new();
        app.screen = AppScreen::Error;
        app.go_back();
        assert_eq!(app.screen, AppScreen::Error);
    }

    #[test]
    fn test_go_back_is_noop_from_select_image() {
        let mut app = App::new();
        app.screen = AppScreen::SelectImage;
        app.go_back();
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    // ── Flash event handling ──────────────────────────────────────────────────

    #[test]
    fn test_apply_flash_event_progress_updates_fields() {
        let mut app = App::new();
        app.apply_flash_event(FlashEvent::Progress {
            progress: 0.42,
            bytes_written: 1_048_576,
            speed_mb_s: 28.5,
        });
        assert!((app.flash_progress - 0.42 * 0.80).abs() < 1e-5);
        assert_eq!(app.flash_bytes, 1_048_576);
        assert_eq!(app.flash_speed, 28.5);
    }

    #[test]
    fn test_apply_flash_event_progress_overwrites_previous() {
        let mut app = App::new();
        app.apply_flash_event(FlashEvent::Progress {
            progress: 0.2,
            bytes_written: 512,
            speed_mb_s: 10.0,
        });
        app.apply_flash_event(FlashEvent::Progress {
            progress: 0.7,
            bytes_written: 2048,
            speed_mb_s: 35.0,
        });
        assert!((app.flash_progress - 0.7 * 0.80).abs() < 1e-5);
        assert_eq!(app.flash_bytes, 2048);
        assert_eq!(app.flash_speed, 35.0);
    }

    #[test]
    fn test_apply_flash_event_stage_sets_label_and_logs() {
        let mut app = App::new();
        app.apply_flash_event(FlashEvent::Message(
            "Writing image to device\u{2026}".to_string(),
        ));
        assert_eq!(app.flash_stage, "Writing image to device\u{2026}");
        assert!(app
            .flash_log
            .contains(&"Writing image to device\u{2026}".to_string()));
    }

    #[test]
    fn test_apply_flash_event_log_appends_to_log() {
        let mut app = App::new();
        app.apply_flash_event(FlashEvent::Message("SHA-256 verified \u{2713}".to_string()));
        assert!(app
            .flash_log
            .contains(&"SHA-256 verified \u{2713}".to_string()));
    }

    #[test]
    fn test_apply_flash_event_multiple_logs_accumulate() {
        let mut app = App::new();
        for i in 0..5 {
            app.apply_flash_event(FlashEvent::Message(format!("log {i}")));
        }
        assert_eq!(app.flash_log.len(), 5);
    }

    #[test]
    fn test_apply_flash_event_completed_sets_full_progress_and_screen() {
        let mut app = App::new();
        app.screen = AppScreen::Flashing;
        app.flash_progress = 0.9;
        app.apply_flash_event(FlashEvent::Completed);
        assert_eq!(app.flash_progress, 1.0);
        assert_eq!(app.screen, AppScreen::Complete);
        // A completion message should have been logged
        assert!(!app.flash_log.is_empty());
        assert!(app
            .flash_log
            .iter()
            .any(|l| l.contains("complet") || l.contains("success")));
    }

    #[test]
    fn test_apply_flash_event_failed_sets_error_screen() {
        let mut app = App::new();
        app.screen = AppScreen::Flashing;
        app.apply_flash_event(FlashEvent::Failed("Write failed: I/O error".to_string()));
        assert_eq!(app.screen, AppScreen::Error);
        assert_eq!(app.error_message, "Write failed: I/O error");
    }

    // ── push_log capacity ─────────────────────────────────────────────────────

    #[test]
    fn test_push_log_enforces_max_capacity() {
        let mut app = App::new();
        // Push more than MAX_LOG (200) entries via apply_flash_event
        for i in 0..250 {
            app.apply_flash_event(FlashEvent::Message(format!("log line {i}")));
        }
        assert!(
            app.flash_log.len() <= 200,
            "log should be capped at 200, got {}",
            app.flash_log.len()
        );
        // The most recent entry must be retained
        assert_eq!(
            app.flash_log.last().unwrap(),
            "log line 249",
            "most-recent log entry must be at the tail"
        );
    }

    #[test]
    fn test_push_log_exactly_at_limit_does_not_trim() {
        let mut app = App::new();
        for i in 0..200 {
            app.apply_flash_event(FlashEvent::Message(format!("entry {i}")));
        }
        assert_eq!(app.flash_log.len(), 200);
    }

    // ── Channel polling ───────────────────────────────────────────────────────

    #[test]
    fn test_poll_drives_applies_result_and_clears_loading() {
        let (tx, rx) = mpsc::unbounded_channel::<Vec<DriveInfo>>();
        let drives = vec![
            make_drive("sdb", "/dev/sdb", false, false),
            make_drive("sdc", "/dev/sdc", false, false),
        ];
        tx.send(drives).unwrap();

        let mut app = App::new();
        app.drives_loading = true;
        app.drives_rx = Some(rx);

        app.poll_drives();

        assert_eq!(app.available_drives.len(), 2);
        assert!(!app.drives_loading, "loading flag should be cleared");
        assert_eq!(app.drive_cursor, 0, "cursor should reset");
        assert!(
            app.drives_rx.is_none(),
            "one-shot channel should be consumed"
        );
    }

    #[test]
    fn test_poll_drives_keeps_receiver_when_channel_empty() {
        let (_tx, rx) = mpsc::unbounded_channel::<Vec<DriveInfo>>();

        let mut app = App::new();
        app.drives_loading = true;
        app.drives_rx = Some(rx);

        app.poll_drives();

        assert!(app.drives_loading, "loading should stay true");
        assert!(app.drives_rx.is_some(), "receiver must be retained");
    }

    #[test]
    fn test_poll_drives_does_nothing_when_no_receiver() {
        let mut app = App::new();
        // drives_rx is None by default
        app.poll_drives(); // should not panic
        assert!(app.available_drives.is_empty());
    }

    #[test]
    fn test_poll_flash_applies_progress_event() {
        let (tx, rx) = mpsc::unbounded_channel::<FlashEvent>();
        tx.send(FlashEvent::Progress {
            progress: 0.5,
            bytes_written: 1024,
            speed_mb_s: 22.0,
        })
        .unwrap();

        let mut app = App::new();
        app.flash_rx = Some(rx);
        app.poll_flash();

        assert!((app.flash_progress - 0.5 * 0.80).abs() < 1e-5);
        assert_eq!(app.flash_bytes, 1024);
        assert_eq!(app.flash_speed, 22.0);
    }

    #[test]
    fn test_poll_flash_drains_multiple_events() {
        let (tx, rx) = mpsc::unbounded_channel::<FlashEvent>();
        // Use a real stage string so flash_stage is set predictably, then a
        // second Message so we can confirm both are logged.  Since every
        // Message updates flash_stage, the last one wins.
        let writing_str = flashkraft_core::FlashStage::Writing.to_string();
        let second_msg = "Chunk 1 written".to_string();
        tx.send(FlashEvent::Message(writing_str.clone())).unwrap();
        tx.send(FlashEvent::Message(second_msg.clone())).unwrap();
        tx.send(FlashEvent::Progress {
            progress: 0.25,
            bytes_written: 256,
            speed_mb_s: 15.0,
        })
        .unwrap();

        let mut app = App::new();
        app.flash_rx = Some(rx);
        app.poll_flash();

        // flash_stage is set by every Message; last Message wins.
        assert_eq!(app.flash_stage, second_msg);
        assert!((app.flash_progress - 0.25 * 0.80).abs() < 1e-5);
        // Both Message events are logged.
        assert!(app.flash_log.len() >= 2);
        assert!(app.flash_log.contains(&writing_str));
        assert!(app.flash_log.contains(&second_msg));
    }

    #[test]
    fn test_poll_flash_keeps_receiver_when_channel_empty() {
        let (_tx, rx) = mpsc::unbounded_channel::<FlashEvent>();

        let mut app = App::new();
        app.flash_rx = Some(rx);
        app.poll_flash();

        assert!(
            app.flash_rx.is_some(),
            "receiver must be retained while channel is open"
        );
    }

    #[test]
    fn test_poll_flash_drops_receiver_when_disconnected() {
        let (tx, rx) = mpsc::unbounded_channel::<FlashEvent>();
        drop(tx); // close the sender

        let mut app = App::new();
        app.flash_rx = Some(rx);
        app.poll_flash();

        assert!(
            app.flash_rx.is_none(),
            "receiver should be dropped after disconnect"
        );
    }

    // ── begin_flash validation ────────────────────────────────────────────────

    #[test]
    fn test_begin_flash_without_image_returns_err() {
        let mut app = App::new();
        app.selected_drive = Some(make_drive("usb0", "/dev/sdb", false, false));
        let result = app.begin_flash();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("image"),
            "error should mention missing image: {msg}"
        );
    }

    #[test]
    fn test_begin_flash_without_drive_returns_err() {
        let mut app = App::new();
        app.selected_image = Some(make_image(512.0));
        let result = app.begin_flash();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("drive"),
            "error should mention missing drive: {msg}"
        );
    }

    #[test]
    fn test_begin_flash_without_both_returns_err() {
        let mut app = App::new();
        let result = app.begin_flash();
        assert!(result.is_err());
    }

    // ── cancel_flash ──────────────────────────────────────────────────────────

    #[test]
    fn test_cancel_flash_sets_cancel_token() {
        use std::sync::atomic::Ordering;

        let mut app = App::new();
        app.screen = AppScreen::Flashing;
        assert!(
            !app.cancel_token.load(Ordering::SeqCst),
            "cancel token must start as false"
        );

        app.cancel_flash();

        assert!(
            app.cancel_token.load(Ordering::SeqCst),
            "cancel token must be set after cancel_flash"
        );
    }

    #[test]
    fn test_cancel_flash_transitions_to_error_screen() {
        let mut app = App::new();
        app.screen = AppScreen::Flashing;
        app.cancel_flash();
        assert_eq!(app.screen, AppScreen::Error);
    }

    #[test]
    fn test_cancel_flash_sets_error_message() {
        let mut app = App::new();
        app.screen = AppScreen::Flashing;
        app.cancel_flash();
        assert!(!app.error_message.is_empty(), "error_message must be set");
        assert!(
            app.error_message.to_lowercase().contains("cancel"),
            "error message should mention cancellation: {}",
            app.error_message
        );
    }

    // ── reset ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_reset_returns_to_factory_defaults() {
        let mut app = App::new();

        // Dirty up the state
        app.screen = AppScreen::Complete;
        app.image_input = "/some/path.iso".into();
        app.image_cursor = 5;
        app.flash_progress = 0.8;
        app.flash_bytes = 999_999;
        app.flash_speed = 42.0;
        app.flash_stage = "Done".into();
        app.flash_log = vec!["a".into(), "b".into(), "c".into()];
        app.tick_count = 1234;
        app.error_message = "previous error".into();
        app.should_quit = false;
        app.selected_image = Some(make_image(100.0));
        app.selected_drive = Some(make_drive("usb0", "/dev/sdb", false, false));

        app.reset();

        assert_eq!(app.screen, AppScreen::SelectImage);
        assert!(app.image_input.is_empty(), "image_input must be cleared");
        assert_eq!(app.image_cursor, 0);
        assert_eq!(app.flash_progress, 0.0);
        assert_eq!(app.flash_bytes, 0);
        assert_eq!(app.flash_speed, 0.0);
        assert!(app.flash_log.is_empty());
        assert_eq!(app.tick_count, 0);
        assert!(app.selected_image.is_none());
        assert!(app.selected_drive.is_none());
        assert!(!app.should_quit);
    }

    // ── Convenience accessors ─────────────────────────────────────────────────

    #[test]
    fn test_image_size_bytes_returns_zero_when_no_image() {
        let app = App::new();
        assert_eq!(app.image_size_bytes(), 0);
    }

    #[test]
    fn test_image_size_bytes_converts_mb_to_bytes() {
        let mut app = App::new();
        app.selected_image = Some(make_image(512.0)); // 512 MB
        let expected = (512.0_f64 * 1024.0 * 1024.0) as u64;
        // Allow +/-1 for floating-point truncation
        assert!(
            (app.image_size_bytes() as i64 - expected as i64).abs() <= 1,
            "expected ~{expected} bytes, got {}",
            app.image_size_bytes()
        );
    }

    #[test]
    fn test_drive_size_bytes_returns_zero_when_no_drive() {
        let app = App::new();
        assert_eq!(app.drive_size_bytes(), 0);
    }

    #[test]
    fn test_drive_size_bytes_converts_gb_to_bytes() {
        let mut app = App::new();
        app.selected_drive = Some(make_drive("usb", "/dev/sdb", false, false));
        // make_drive uses 16.0 GB
        let expected = (16.0_f64 * 1024.0 * 1024.0 * 1024.0) as u64;
        assert!(
            (app.drive_size_bytes() as i64 - expected as i64).abs() <= 1,
            "expected ~{expected} bytes, got {}",
            app.drive_size_bytes()
        );
    }

    // ── USB contents scroll ───────────────────────────────────────────────────

    fn make_usb_entries(n: usize) -> Vec<UsbEntry> {
        (0..n)
            .map(|i| UsbEntry {
                name: format!("file_{i:02}.txt"),
                size_bytes: (i as u64 + 1) * 512,
                is_dir: i % 4 == 0,
                depth: 0,
            })
            .collect()
    }

    #[test]
    fn test_contents_up_clamps_at_zero() {
        let mut app = App::new();
        app.usb_contents = make_usb_entries(5);
        app.contents_scroll = 0;
        app.contents_up(); // should not underflow
        assert_eq!(app.contents_scroll, 0);
    }

    #[test]
    fn test_contents_down_increments_scroll() {
        let mut app = App::new();
        app.usb_contents = make_usb_entries(5);
        app.contents_down();
        assert_eq!(app.contents_scroll, 1);
        app.contents_down();
        assert_eq!(app.contents_scroll, 2);
    }

    #[test]
    fn test_contents_down_clamps_at_last_entry() {
        let mut app = App::new();
        app.usb_contents = make_usb_entries(3); // indices 0..2
        app.contents_scroll = 2; // already at last
        app.contents_down();
        assert_eq!(app.contents_scroll, 2, "should clamp at len-1");
    }

    #[test]
    fn test_contents_up_decrements_scroll() {
        let mut app = App::new();
        app.usb_contents = make_usb_entries(5);
        app.contents_scroll = 3;
        app.contents_up();
        assert_eq!(app.contents_scroll, 2);
    }

    #[test]
    fn test_contents_scroll_on_empty_list_is_noop() {
        let mut app = App::new();
        app.contents_up(); // should not panic
        app.contents_down(); // should not panic
        assert_eq!(app.contents_scroll, 0);
    }

    // ── poll_drives / poll_flash edge cases ───────────────────────────────────

    #[test]
    fn test_poll_flash_completed_event_via_channel() {
        let (tx, rx) = mpsc::unbounded_channel::<FlashEvent>();
        tx.send(FlashEvent::Completed).unwrap();

        let mut app = App::new();
        app.screen = AppScreen::Flashing;
        app.flash_rx = Some(rx);
        app.poll_flash();

        assert_eq!(app.screen, AppScreen::Complete);
        assert_eq!(app.flash_progress, 1.0);
    }

    #[test]
    fn test_poll_flash_failed_event_via_channel() {
        let (tx, rx) = mpsc::unbounded_channel::<FlashEvent>();
        tx.send(FlashEvent::Failed("Disk full".to_string()))
            .unwrap();

        let mut app = App::new();
        app.screen = AppScreen::Flashing;
        app.flash_rx = Some(rx);
        app.poll_flash();

        assert_eq!(app.screen, AppScreen::Error);
        assert_eq!(app.error_message, "Disk full");
    }

    // ── tick_count wrapping ───────────────────────────────────────────────────

    #[test]
    fn test_tick_count_wraps_on_overflow() {
        let mut app = App::new();
        app.tick_count = u64::MAX;
        app.tick_count = app.tick_count.wrapping_add(1);
        assert_eq!(app.tick_count, 0, "tick_count should wrap to 0 at u64::MAX");
    }

    // ── AppScreen & InputMode derive behaviour ────────────────────────────────

    #[test]
    fn test_app_screen_default_is_select_image() {
        assert_eq!(AppScreen::default(), AppScreen::SelectImage);
    }

    #[test]
    fn test_input_mode_default_is_normal() {
        // Default for InputMode::Normal
        assert_eq!(InputMode::default(), InputMode::Normal);
    }

    #[test]
    fn test_app_screen_equality() {
        assert_eq!(AppScreen::SelectImage, AppScreen::SelectImage);
        assert_ne!(AppScreen::SelectImage, AppScreen::SelectDrive);
        assert_ne!(AppScreen::Complete, AppScreen::Error);
    }

    // ── open_file_explorer ────────────────────────────────────────────────────

    #[test]
    fn test_open_file_explorer_transitions_to_browse_image() {
        let mut app = App::new();
        app.open_file_explorer();
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn test_open_file_explorer_uses_typed_dir_when_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let iso = tmp.path().join("test.iso");
        std::fs::write(&iso, b"x").unwrap();

        let mut app = App::new();
        app.image_input = iso.to_string_lossy().to_string();
        app.open_file_explorer();

        // The explorer should have navigated to the iso's parent directory.
        assert_eq!(app.file_explorer.current_dir, tmp.path());
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn test_open_file_explorer_uses_dir_input_directly() {
        let tmp = tempfile::tempdir().unwrap();

        let mut app = App::new();
        app.image_input = tmp.path().to_string_lossy().to_string();
        app.open_file_explorer();

        assert_eq!(app.file_explorer.current_dir, tmp.path());
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn test_open_file_explorer_falls_back_when_input_invalid() {
        let mut app = App::new();
        app.image_input = "/this/path/does/not/exist/at/all.iso".to_string();
        app.open_file_explorer();
        // Should still transition — fallback to home or cwd.
        assert_eq!(app.screen, AppScreen::BrowseImage);
        // The directory must actually exist.
        assert!(app.file_explorer.current_dir.is_dir());
    }

    #[test]
    fn test_open_file_explorer_empty_input_falls_back() {
        let mut app = App::new();
        app.image_input = String::new();
        app.open_file_explorer();
        assert_eq!(app.screen, AppScreen::BrowseImage);
        assert!(app.file_explorer.current_dir.is_dir());
    }

    // ── apply_explorer_selection ──────────────────────────────────────────────

    #[test]
    fn test_apply_explorer_selection_populates_image_input() {
        let mut app = App::new();
        let path = PathBuf::from("/some/path/ubuntu.iso");
        app.apply_explorer_selection(path.clone());
        assert_eq!(app.image_input, "/some/path/ubuntu.iso");
    }

    #[test]
    fn test_apply_explorer_selection_sets_cursor_to_end() {
        let mut app = App::new();
        let path = PathBuf::from("/some/ubuntu.iso");
        app.apply_explorer_selection(path.clone());
        let expected_len = path.to_string_lossy().chars().count();
        assert_eq!(app.image_cursor, expected_len);
    }

    #[test]
    fn test_apply_explorer_selection_returns_to_select_image() {
        let mut app = App::new();
        app.screen = AppScreen::BrowseImage;
        app.apply_explorer_selection(PathBuf::from("/some/ubuntu.iso"));
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    #[test]
    fn test_apply_explorer_selection_sets_editing_mode() {
        let mut app = App::new();
        app.input_mode = InputMode::Normal;
        app.apply_explorer_selection(PathBuf::from("/some/ubuntu.iso"));
        assert_eq!(app.input_mode, InputMode::Editing);
    }

    #[test]
    fn test_apply_explorer_selection_unicode_path() {
        let mut app = App::new();
        let path = PathBuf::from("/home/utilisateur/t\u{e9}l\u{e9}chargements/debian.iso");
        app.apply_explorer_selection(path.clone());
        let expected_len = path.to_string_lossy().chars().count();
        assert_eq!(app.image_cursor, expected_len);
        assert_eq!(app.image_input, path.to_string_lossy().as_ref());
    }

    // ── go_back from BrowseImage ──────────────────────────────────────────────

    #[test]
    fn test_go_back_browse_image_to_select_image() {
        let mut app = App::new();
        app.screen = AppScreen::BrowseImage;
        app.go_back();
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    #[test]
    fn test_go_back_browse_image_restores_editing_mode() {
        let mut app = App::new();
        app.screen = AppScreen::BrowseImage;
        app.input_mode = InputMode::Normal;
        app.go_back();
        assert_eq!(app.input_mode, InputMode::Editing);
    }
}
