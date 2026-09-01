//! Resolves avatar metadata by reading VRChat's own OSC config JSON, mirroring the original
//! `getAvatarDetails` from src/main/vrc-osc.ts. Windows-only — VRChat has no Mac/Linux client,
//! so this genuinely never needs to run anywhere else. Unlike the original, this can't crash the
//! whole process on a missing directory: it just returns `None`.
use std::path::PathBuf;

use crate::types::AvatarDetails;

#[cfg(target_os = "windows")]
fn vrchat_osc_dir() -> Option<PathBuf> {
    let base = directories::BaseDirs::new()?;
    // base.data_dir() == %AppData%\Roaming, matching Electron's app.getPath('appData') that the
    // original implementation used.
    Some(
        base.data_dir()
            .join("..")
            .join("LocalLow")
            .join("VRChat")
            .join("VRChat")
            .join("OSC")
    )
}

#[cfg(not(target_os = "windows"))]
fn vrchat_osc_dir() -> Option<PathBuf> {
    None
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
                eprintln!("Failed to read avatar config at {candidate:?}: {err}");
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
                eprintln!("Failed to parse avatar config at {candidate:?}: {err}");
                continue;
            }
        }
    }

    None
}
