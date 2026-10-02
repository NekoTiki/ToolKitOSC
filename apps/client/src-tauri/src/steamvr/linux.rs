//! SteamVR's native Linux build.
use std::path::PathBuf;

pub const SUPPORTED: bool = true;

/// The OpenVR library inside a SteamVR install.
pub const OPENVR_API_PATH: &str = "bin/linux64/libopenvr_api.so";

pub const BINARY_PATH_KEY: &str = "binary_path_linux";

/// `$XDG_CONFIG_HOME/openvr/openvrpaths.vrpath`, i.e. `~/.config/openvr/openvrpaths.vrpath`.
pub fn openvr_paths_file() -> Option<PathBuf> {
    Some(directories::BaseDirs::new()?.config_dir().join("openvr").join("openvrpaths.vrpath"))
}

/// Inside an AppImage, `current_exe()` points into the image's temporary mount (`/tmp/.mount_*`),
/// which is gone once the app exits - SteamVR has to be given the AppImage file itself, which the
/// AppImage runtime exposes as `$APPIMAGE`. That path changes when the AppImage is moved or
/// updated, which is fine: the registration is redone at every app start while the toggle is on.
pub fn app_binary() -> Result<PathBuf, String> {
    match std::env::var_os("APPIMAGE") {
        Some(appimage) => Ok(PathBuf::from(appimage)),
        None => std::env::current_exe().map_err(|e| e.to_string()),
    }
}
