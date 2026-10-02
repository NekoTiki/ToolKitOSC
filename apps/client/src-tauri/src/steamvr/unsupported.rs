//! Every platform but Windows and Linux: SteamVR isn't available, and with no `openvrpaths.vrpath`
//! or Steam library to look in, `find_openvr_api` never finds anything.
use std::path::PathBuf;

pub const SUPPORTED: bool = false;

pub const OPENVR_API_PATH: &str = "";

pub const BINARY_PATH_KEY: &str = "";

pub fn openvr_paths_file() -> Option<PathBuf> {
    None
}

// Unreachable in practice: every caller bails out first when find_openvr_api() finds nothing.
pub fn app_binary() -> Result<PathBuf, String> {
    Err("SteamVR is only supported on Windows and Linux".to_string())
}
