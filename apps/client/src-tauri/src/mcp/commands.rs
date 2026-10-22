//! The AI assistants commands the webview calls (see `super`).

use tauri::AppHandle;

use super::model::McpSettings;

#[tauri::command]
pub fn mcp_settings(app: AppHandle) -> Result<McpSettings, String> {
    crate::addons::require(&app, "mcp")?;
    Ok(super::store::get(&app))
}

/// Replaces every setting. A new port restarts the server on it.
#[tauri::command]
pub fn mcp_save(app: AppHandle, settings: McpSettings) -> Result<(), String> {
    crate::addons::require(&app, "mcp")?;
    if settings.port < 1024 {
        return Err("Pick a port from 1024 up".to_owned());
    }
    let port_changed = super::store::with(&app, |current| {
        let port_changed = current.port != settings.port;
        let changed = *current != settings;
        *current = settings;
        (port_changed, changed)
    });
    if port_changed {
        super::restart(&app);
    }
    Ok(())
}

/// The key, only when you press Show key or Copy.
#[tauri::command]
pub fn mcp_key(app: AppHandle) -> Result<String, String> {
    crate::addons::require(&app, "mcp")?;
    super::mcp(&app).key.lock().unwrap().clone().ok_or_else(|| "The server hasn't started yet".to_owned())
}

/// Makes a new key, which cuts off every assistant set up with the old one.
#[tauri::command]
pub fn mcp_new_key(app: AppHandle) -> Result<String, String> {
    crate::addons::require(&app, "mcp")?;
    let key = super::new_key();
    crate::secrets::set(&app, super::KEY_SECRET, Some(&key))?;
    *super::mcp(&app).key.lock().unwrap() = Some(key.clone());
    Ok(key)
}

/// What Claude Desktop starts: this program with `--mcp` (see `super::stdio`). An AppImage runs from
/// a temporary mount, so it's the AppImage file itself there.
#[tauri::command]
pub fn mcp_command() -> Result<String, String> {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Ok(appimage.to_string_lossy().into_owned());
    }
    std::env::current_exe().map(|path| path.to_string_lossy().into_owned()).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn mcp_clients(app: AppHandle) -> Vec<super::Client> {
    super::clients(&app)
}

/// The webview's answer to an `mcp-request` (see `super::bridge`): `result`, or `error` when it
/// couldn't do it.
#[tauri::command]
pub fn mcp_respond(app: AppHandle, id: u64, result: Option<serde_json::Value>, error: Option<String>) {
    super::bridge::respond(&app, id, error.map_or(Ok(result.unwrap_or_default()), Err));
}
