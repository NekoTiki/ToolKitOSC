//! VRCX's Auto-Launch Folder on Windows: `%AppData%\VRCX\startup`, holding `.lnk` shortcuts.
use std::path::{Path, PathBuf};

pub const SUPPORTED: bool = true;

pub const SHORTCUT_NAME: &str = "ToolKitOSC.lnk";

pub fn startup_dir() -> Option<PathBuf> {
    Some(directories::BaseDirs::new()?.data_dir().join("VRCX").join("startup"))
}

pub fn create_shortcut(path: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_str().ok_or("Executable path contains invalid characters")?;
    let path = path.to_str().ok_or("VRCX startup folder path contains invalid characters")?;

    let link = mslnk::ShellLink::new(exe).map_err(|e| e.to_string())?;
    link.create_lnk(path).map_err(|e| e.to_string())
}
