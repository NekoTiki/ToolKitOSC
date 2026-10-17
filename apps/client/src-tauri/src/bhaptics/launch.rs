//! Finding and starting a local bHaptics Player. Windows only: there's no Player for Linux.
//!
//! bHaptics doesn't document where Player installs, so this checks the usual folders and gives
//! up quietly otherwise. Check these against a real install when there is one.
use std::path::PathBuf;

#[cfg(target_os = "windows")]
fn candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(dir) = std::env::var(var) {
            let dir = PathBuf::from(dir).join("bHaptics");
            paths.push(dir.join("bHaptics Player").join("BhapticsPlayer.exe"));
            paths.push(dir.join("bHaptics Player").join("bHaptics Player.exe"));
        }
    }
    if let Ok(dir) = std::env::var("LOCALAPPDATA") {
        paths.push(PathBuf::from(dir).join("Programs").join("bhaptics-player").join("bHaptics Player.exe"));
    }
    paths
}

#[cfg(not(target_os = "windows"))]
fn candidates() -> Vec<PathBuf> {
    Vec::new()
}

fn find() -> Option<PathBuf> {
    candidates().into_iter().find(|path| path.is_file())
}

pub fn available() -> bool {
    find().is_some()
}

pub fn start() -> Result<(), String> {
    let path = find().ok_or_else(|| "bHaptics Player was not found on this system".to_string())?;
    std::process::Command::new(&path).spawn().map_err(|e| e.to_string())?;
    Ok(())
}
