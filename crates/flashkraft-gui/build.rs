//! build.rs — FlashKraft GUI
//!
//! On Windows this script embeds the UAC application manifest and the app icon
//! into the compiled PE binary so that Windows shows the Administrator elevation
//! prompt when the user launches FlashKraft.
//!
//! On every other platform this script does nothing — it exists only so the
//! file is present and `cargo build` does not complain.
//!
//! ## How the manifest embedding works
//!
//! The Windows linker (MSVC or GNU) accepts `.rc` resource scripts.  The
//! `winres` crate compiles a resource script that references our manifest
//! file and links the resulting `.res` object into the final `.exe`.
//!
//! Without this, Windows runs FlashKraft as a standard (unprivileged) user
//! and every attempt to open `\\.\PhysicalDriveN` for writing fails
//! immediately with ERROR_ACCESS_DENIED (5), because raw physical-disk
//! access requires the `SeManageVolumePrivilege` token privilege that is
//! only present in an elevated Administrator token.
//!
//! ## Dependencies
//!
//! `winres` is a build-dependency declared in `Cargo.toml` under
//! `[build-dependencies]` — it is NOT compiled into the final binary.

fn main() {
    // Only relevant on Windows targets.  On Linux / macOS this entire block
    // compiles away at no cost.
    #[cfg(target_os = "windows")]
    embed_manifest();

    // Tell Cargo to re-run this script if the manifest file changes.
    println!("cargo:rerun-if-changed=flashkraft.exe.manifest");
    println!("cargo:rerun-if-changed=assets/flashkraft.ico");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Compile and link the UAC manifest resource into the PE binary.
///
/// Uses the `winres` crate which wraps the Windows resource compiler
/// (`rc.exe` on MSVC toolchains, `windres` on GNU toolchains).
#[cfg(target_os = "windows")]
fn embed_manifest() {
    let mut res = winres::WindowsResource::new();

    // Point winres at our hand-written manifest file.
    // The path is relative to the crate root (where build.rs lives).
    res.set_manifest_file("flashkraft.exe.manifest");

    // The app icon, shown by Explorer, the taskbar and the Start-menu shortcuts.
    // `just icons` writes it from assets/icons/flashkraft.svg.
    res.set_icon("assets/flashkraft.ico");

    // Compile and emit linker flags.  Any error here is fatal — we want to
    // know immediately if the toolchain can't embed the manifest rather than
    // silently shipping a binary that will fail with ACCESS_DENIED at runtime.
    res.compile().expect(
        "Failed to compile Windows resource manifest.\n\
         Make sure the Windows SDK (rc.exe) or MinGW (windres) is on PATH.\n\
         On MSVC: install the 'Windows 10 SDK' component in Visual Studio.\n\
         On GNU:  install mingw-w64 (e.g. `winget install MinGW`).",
    );
}
