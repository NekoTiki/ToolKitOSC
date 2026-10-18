//! Commands about the app itself rather than anything it talks to: opening links, the tray, and
//! whether an optional integration can be offered.

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Whether an optional integration (SteamVR, VRCX) can be offered, so Settings can tell "can't
/// work on this OS" (hide the toggle) apart from "not installed" (show it disabled, saying why).
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Available,
    NotInstalled,
    Unsupported,
}

impl Availability {
    pub fn new(supported: bool, installed: bool) -> Self {
        match (supported, installed) {
            (false, _) => Self::Unsupported,
            (true, false) => Self::NotInstalled,
            (true, true) => Self::Available,
        }
    }
}

/// Keeps the window's native close handler and the tray icon (see `lib.rs`) in sync with the
/// frontend's localStorage-backed "keep running in the tray" setting - called once at startup and
/// on every change, since the setting itself lives in the webview, out of reach of either.
#[tauri::command]
pub fn set_minimize_to_tray(state: State<AppState>, enabled: bool) {
    state.minimize_to_tray.store(enabled, Ordering::Relaxed);

    if let Some(tray) = state.tray_icon.lock().unwrap().as_ref() {
        let _ = tray.set_visible(enabled);
    }
}

/// What the JS `Update` class from `@tauri-apps/plugin-updater` is built from - mirrors the
/// plugin's own `check` result, so its download and install commands find the update by `rid`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadata {
    rid: tauri::ResourceId,
    current_version: String,
    version: String,
    date: Option<String>,
    body: Option<String>,
    raw_json: serde_json::Value,
}

/// Only release tags from our own repo, so the webview can't point the updater anywhere else.
fn is_client_tag(tag: &str) -> bool {
    tag.strip_prefix("client-v").is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    })
}

/// Settings > About's Beta updates: checks one release's latest.json instead of the
/// `releases/latest` one in tauri.conf.json, which skips prereleases. The plugin's JS `check()`
/// can't change the endpoint, so the frontend picks the newest tag and asks here.
#[tauri::command]
pub async fn updater_check_tag(
    webview: tauri::Webview,
    tag: String,
) -> Result<Option<UpdateMetadata>, String> {
    use tauri::Manager;
    use tauri_plugin_updater::UpdaterExt;

    if !is_client_tag(&tag) {
        return Err(format!("Not a client release tag: {tag}"));
    }
    let url = format!("https://github.com/NekoTiki/ToolKitOSC/releases/download/{tag}/latest.json");
    let url = tauri::Url::parse(&url).map_err(|e| e.to_string())?;
    let update = webview
        .updater_builder()
        .endpoints(vec![url])
        .and_then(|builder| builder.build())
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;

    Ok(update.map(|update| UpdateMetadata {
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        date: update
            .raw_json
            .get("pub_date")
            .and_then(|d| d.as_str())
            .map(str::to_owned),
        body: update.body.clone(),
        raw_json: update.raw_json.clone(),
        rid: webview.resources_table().add(update),
    }))
}

#[cfg(test)]
mod tests {
    use super::is_client_tag;

    #[test]
    fn client_tags_only() {
        assert!(is_client_tag("client-v1.4.0"));
        assert!(is_client_tag("client-v1.5.0-beta.1"));
        assert!(!is_client_tag("client-v"));
        assert!(!is_client_tag("server-v1.4.0"));
        assert!(!is_client_tag("client-v1.0.0/../../evil"));
        assert!(!is_client_tag("client-v1.0.0?x=1"));
    }
}
