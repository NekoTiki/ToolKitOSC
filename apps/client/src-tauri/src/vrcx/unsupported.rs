//! Every platform but Windows: VRCX's Auto-Launch Folder isn't available (see the module docs),
//! so VRCX reads as not installed and the toggle stays unavailable.
use std::path::{Path, PathBuf};

pub const SUPPORTED: bool = false;

pub const SHORTCUT_NAME: &str = "";

pub fn startup_dir() -> Option<PathBuf> {
    None
}

// Unreachable in practice: shortcut_path() already fails when startup_dir() is None.
pub fn create_shortcut(_path: &Path) -> Result<(), String> {
    Err("VRCX auto-launch is only supported on Windows".to_string())
}
