//! Where Steam is installed on Windows, read from the install-path registry key Steam itself
//! relies on.
use std::path::PathBuf;

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

pub fn steam_roots() -> Vec<PathBuf> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Valve\\Steam")
        .and_then(|key| key.get_value::<String, _>("SteamPath"))
        .map(PathBuf::from)
        .into_iter()
        .collect()
}
