//! Every place a Steam install commonly lives on Linux: native, the legacy `~/.steam` symlinks,
//! Flatpak and Snap.
use std::path::PathBuf;

pub fn steam_roots() -> Vec<PathBuf> {
    let Some(base) = directories::BaseDirs::new() else {
        return Vec::new();
    };
    let home = base.home_dir();

    vec![
        // $XDG_DATA_HOME/Steam, i.e. ~/.local/share/Steam by default.
        base.data_dir().join("Steam"),
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home.join("snap/steam/common/.local/share/Steam"),
    ]
}
