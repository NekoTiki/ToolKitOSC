//! Finding, launching and checking on a local Intiface Central.
use std::path::PathBuf;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use std::process::Command;

/// Best-effort lookup for a local Intiface Central install, so Settings can offer to launch it
/// instead of just reporting a failed connection. Intiface Central doesn't register itself
/// anywhere queryable (no protocol handler, no registry key we can rely on), so this only checks
/// the well-known per-OS install locations from its own installer/packaging — a custom install
/// directory won't be found. One path (or bundle/app id) per OS, so unlike Windows/macOS this
/// doesn't need `find` — a single tail expression per `#[cfg]` block is the whole function body.
fn find() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // The installer runs per-user (unprivileged) by default, installing under %APPDATA%; an
        // admin/all-users install instead lands under %ProgramFiles%. Check both.
        [
            std::env::var("ProgramFiles").ok(),
            std::env::var("APPDATA").ok(),
        ]
        .into_iter()
        .flatten()
        .map(|dir| {
            PathBuf::from(dir)
                .join("IntifaceCentral")
                .join("intiface_central.exe")
        })
        .find(|path| path.is_file())
    }

    #[cfg(target_os = "macos")]
    {
        let path = PathBuf::from("/Applications/Intiface Central.app");
        path.is_dir().then_some(path)
    }

    #[cfg(target_os = "linux")]
    {
        // The Flathub package (com.nonpolynomial.intiface_central) is the officially distributed
        // build - other install methods (AUR, manual builds) aren't detected.
        let user_install = std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home).join(".local/share/flatpak/app/com.nonpolynomial.intiface_central")
        });

        [
            user_install,
            Some(PathBuf::from(
                "/var/lib/flatpak/app/com.nonpolynomial.intiface_central",
            )),
        ]
        .into_iter()
        .flatten()
        .find(|path| path.is_dir())
    }
}

pub fn available() -> bool {
    find().is_some()
}

pub fn start() -> Result<(), String> {
    let path = find()
        .ok_or_else(|| "Intiface Central was not found on this system".to_string())?;

    #[cfg(target_os = "windows")]
    {
        Command::new(&path).spawn().map_err(|e| e.to_string())?;
    }

    // .app bundles are launched through `open`, not by executing a binary inside them directly -
    // it's the standard way to start a macOS application and avoids having to know/guess the
    // bundle's internal executable name.
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-a")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("flatpak")
            .args(["run", "com.nonpolynomial.intiface_central"])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Whether anything is listening on Intiface's address, to tell "it's running but refused us"
/// apart from "nothing there" after a failed handshake.
/// A closed port doesn't mean Intiface Central isn't running, though: it stops listening while a
/// client is connected (confirmed live) - see `intiface_central_running` for that.
pub async fn port_open(host: &str, port: u16) -> bool {
    let connect = tokio::net::TcpStream::connect((host, port));
    matches!(tokio::time::timeout(std::time::Duration::from_secs(1), connect).await, Ok(Ok(_)))
}

/// Whether an Intiface Central process is running on this machine, so auto-launch never opens a
/// second copy - whether its server is busy with another client, stopped, or still starting up.
/// Matches the process name rather than the path found by `find`, so a copy
/// installed somewhere else still counts: `intiface_central.exe` on Windows, `intiface_central`
/// in the Flatpak, `Intiface Central` on macOS.
pub fn running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());

    system.processes().values().any(|process| {
        let name = process.name().to_string_lossy().to_lowercase();
        name.starts_with("intiface_central") || name.starts_with("intiface central")
    })
}
