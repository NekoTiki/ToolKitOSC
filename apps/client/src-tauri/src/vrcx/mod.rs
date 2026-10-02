//! "Launch with VRChat" via VRCX's own Auto-Launch Folder feature. VRCX has no plugin/API surface
//! for other apps to hook into - its actual public integration point is a well-known folder,
//! `<VRCX data dir>/startup`, containing shortcuts that VRCX runs whenever it detects VRChat
//! launching (its `desktop`/`vr` subfolders scope a shortcut to one launch mode only; the root
//! folder used here runs regardless of mode). See VRCX's docs/issue #1383 for the folder layout:
//! https://github.com/vrcx-team/VRCX/issues/1383
//!
//! Only Windows is supported. VRCX's native Linux build would use `~/.config/VRCX/startup`, but
//! it never runs the Auto-Launch feature there: the settings are hidden, VRChat process watching
//! isn't started, and the folder scan only recognises Windows `.lnk`/`.url` files (see
//! `Dotnet/AutoAppLaunchManager.cs` and `Program.cs` in VRCX).
use std::path::PathBuf;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::{create_shortcut, startup_dir, SHORTCUT_NAME, SUPPORTED};

#[cfg(not(target_os = "windows"))]
mod unsupported;
#[cfg(not(target_os = "windows"))]
use unsupported::{create_shortcut, startup_dir, SHORTCUT_NAME, SUPPORTED};

/// Whether VRCX's Auto-Launch Folder can work on this platform at all, as opposed to VRCX just
/// not being installed.
pub fn is_supported() -> bool {
    SUPPORTED
}

pub fn is_vrcx_installed() -> bool {
    startup_dir().is_some_and(|dir| dir.is_dir())
}

fn shortcut_path() -> Result<PathBuf, String> {
    startup_dir()
        .filter(|dir| dir.is_dir())
        .ok_or_else(|| "VRCX doesn't appear to be installed on this system".to_string())
        .map(|dir| dir.join(SHORTCUT_NAME))
}

pub fn set_auto_launch(enable: bool) -> Result<(), String> {
    let path = shortcut_path()?;

    if !enable {
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    create_shortcut(&path)
}

pub fn get_auto_launch() -> bool {
    shortcut_path().map(|path| path.is_file()).unwrap_or(false)
}
