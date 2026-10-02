//! Where VRChat keeps its data on Linux, inside the Proton prefix Steam created for it.
use std::path::PathBuf;

const VRCHAT_STEAM_APP_ID: &str = "438100";

/// VRChat runs under Proton, so its data sits inside the Wine prefix Steam keeps for it, in the
/// `compatdata` folder of whichever Steam library VRChat is installed in.
pub fn vrchat_data_dir() -> Option<PathBuf> {
    crate::steam::libraries().into_iter().find_map(|library| {
        let dir = library
            .join("steamapps/compatdata")
            .join(VRCHAT_STEAM_APP_ID)
            .join("pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat");
        dir.is_dir().then_some(dir)
    })
}
