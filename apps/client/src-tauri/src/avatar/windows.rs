//! Where VRChat keeps its data on Windows.
use std::path::PathBuf;

pub fn vrchat_data_dir() -> Option<PathBuf> {
    let base = directories::BaseDirs::new()?;
    // base.data_dir() == %AppData%\Roaming, matching Electron's app.getPath('appData') that the
    // original implementation used.
    Some(
        base.data_dir()
            .join("..")
            .join("LocalLow")
            .join("VRChat")
            .join("VRChat"),
    )
}
