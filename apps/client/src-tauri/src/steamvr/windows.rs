//! SteamVR on Windows.
use std::path::PathBuf;

pub const SUPPORTED: bool = true;

/// The OpenVR library inside a SteamVR install.
pub const OPENVR_API_PATH: &str = "bin/win64/openvr_api.dll";

pub const BINARY_PATH_KEY: &str = "binary_path_windows";

/// `%LocalAppData%\openvr\openvrpaths.vrpath`.
pub fn openvr_paths_file() -> Option<PathBuf> {
    Some(directories::BaseDirs::new()?.data_local_dir().join("openvr").join("openvrpaths.vrpath"))
}

pub fn app_binary() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| e.to_string())
}
