//! Resolves avatar metadata by reading VRChat's own OSC config JSON, mirroring the original
//! `getAvatarDetails` from src/main/vrc-osc.ts. VRChat only ships a Windows client: on Windows
//! the files live under %LocalAppData%Low, on Linux they live inside the Proton prefix Steam
//! created for VRChat. macOS has no way to run it, so there's nothing to resolve there. Unlike
//! the original, this can't crash the whole process on a missing directory: it just returns
//! `None`.
use std::path::PathBuf;

use crate::types::AvatarDetails;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::vrchat_data_dir;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::vrchat_data_dir;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn vrchat_data_dir() -> Option<PathBuf> {
    None
}

/// Manual override for setups none of the platform lookups cover (a non-default Wine/Proton prefix,
/// a Steam install somewhere unusual...). Points at VRChat's data folder, the one holding `OSC`.
const DATA_DIR_ENV: &str = "VRCHAT_DATA_DIR";

fn vrchat_osc_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(DATA_DIR_ENV).map(PathBuf::from) {
        if dir.is_dir() {
            return Some(dir.join("OSC"));
        }
        tracing::warn!("{DATA_DIR_ENV} is set to {dir:?}, which isn't a directory - ignoring it");
    }

    let dir = vrchat_data_dir();
    if dir.is_none() {
        tracing::warn!("VRChat's data folder wasn't found - avatar details won't be available");
    }
    Some(dir?.join("OSC"))
}

pub fn get_avatar_details(avatar_id: &str) -> Option<AvatarDetails> {
    let osc_dir = vrchat_osc_dir()?;
    let user_dirs = std::fs::read_dir(&osc_dir).ok()?;

    for entry in user_dirs.flatten() {
        let candidate = entry.path().join("Avatars").join(format!("{avatar_id}.json"));

        if !candidate.exists() {
            continue;
        }

        let data = match std::fs::read_to_string(&candidate) {
            Ok(data) => data,
            Err(err) => {
                tracing::warn!("Failed to read avatar config at {candidate:?}: {err}");
                continue;
            }
        };

        // VRChat (a Unity/.NET app) writes these files with a leading UTF-8 BOM. JS's
        // `String.prototype.trim()` (used by the original Electron implementation) silently
        // strips it since ECMAScript classes U+FEFF as whitespace; Rust's `str::trim()` does not
        // (BOM is Unicode category "Format", not "Whitespace"), so `serde_json` would otherwise
        // fail on every file with "expected value at line 1 column 1".
        let trimmed = data.trim().trim_start_matches('\u{FEFF}').trim();

        match serde_json::from_str::<AvatarDetails>(trimmed) {
            Ok(details) => return Some(details),
            Err(err) => {
                tracing::warn!("Failed to parse avatar config at {candidate:?}: {err}");
                continue;
            }
        }
    }

    None
}
