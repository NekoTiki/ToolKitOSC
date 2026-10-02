//! Finds the local Steam install and every Steam library it knows about, so features that look for
//! a Steam game's files (SteamVR's OpenVR runtime, VRChat's Proton prefix) don't each re-implement
//! it. Only Windows and Linux have Steam layouts worth resolving here.
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::steam_roots;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::steam_roots;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn steam_roots() -> Vec<PathBuf> {
    Vec::new()
}

/// Every Steam library folder on this machine (each holding a `steamapps` folder): every Steam
/// install's own folder plus the extra libraries listed in its `libraryfolders.vdf`, each listed
/// once even when several install paths are symlinks to the same place.
pub fn libraries() -> Vec<PathBuf> {
    let mut libraries: Vec<PathBuf> = Vec::new();

    for root in steam_roots() {
        for library in std::iter::once(root.clone()).chain(extra_libraries(&root)) {
            let library = library.canonicalize().unwrap_or(library);
            if library.is_dir() && !libraries.contains(&library) {
                libraries.push(library);
            }
        }
    }

    libraries
}

/// The extra libraries listed in a Steam install's `libraryfolders.vdf`, as
/// `"path"		"D:\\SteamLibrary"` entries. A full VDF parser is overkill for this - the only thing
/// worth extracting is the quoted string following a `"path"` key on its own line.
fn extra_libraries(root: &Path) -> Vec<PathBuf> {
    let Ok(contents) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) else {
        return Vec::new();
    };

    contents
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("\"path\"")?.trim();
            // VDF escapes backslashes, which only ever matters for Windows paths.
            Some(PathBuf::from(rest.trim_matches('"').replace("\\\\", "\\")))
        })
        .collect()
}
