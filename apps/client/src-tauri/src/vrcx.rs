//! "Launch with VRChat" via VRCX's own Auto-Launch Folder feature. VRCX has no plugin/API surface
//! for other apps to hook into - its actual public integration point is a well-known folder,
//! `%AppData%\VRCX\startup`, containing `.lnk` shortcuts that VRCX runs whenever it detects
//! VRChat launching (its `desktop`/`vr` subfolders scope a shortcut to one launch mode only; the
//! root folder used here runs regardless of mode). See VRCX's docs/issue #1383 for the folder
//! layout: https://github.com/vrcx-team/VRCX/issues/1383
use std::path::PathBuf;

const SHORTCUT_NAME: &str = "ToolKitOSC.lnk";

#[cfg(target_os = "windows")]
fn startup_dir() -> Option<PathBuf> {
    Some(directories::BaseDirs::new()?.data_dir().join("VRCX").join("startup"))
}

// The startup folder holds Windows .lnk shortcuts, so there's nothing to hook into elsewhere:
// VRCX reads as not installed and the toggle stays unavailable.
#[cfg(not(target_os = "windows"))]
fn startup_dir() -> Option<PathBuf> {
    None
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

#[cfg(target_os = "windows")]
fn create_shortcut(path: &std::path::Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_str().ok_or("Executable path contains invalid characters")?;
    let path = path.to_str().ok_or("VRCX startup folder path contains invalid characters")?;

    let link = mslnk::ShellLink::new(exe).map_err(|e| e.to_string())?;
    link.create_lnk(path).map_err(|e| e.to_string())
}

// Unreachable in practice: shortcut_path() already fails when startup_dir() is None.
#[cfg(not(target_os = "windows"))]
fn create_shortcut(_path: &std::path::Path) -> Result<(), String> {
    Err("VRCX auto-launch is only supported on Windows".to_string())
}

pub fn get_auto_launch() -> bool {
    shortcut_path().map(|path| path.is_file()).unwrap_or(false)
}
