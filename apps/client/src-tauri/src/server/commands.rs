//! The share server commands the webview calls: the link, the rules viewer commands are checked
//! against, signing in and the HTTP API (see `super` and `auth`).

use tauri::AppHandle;

/// Connects to the share server's `/host` socket, or keeps the link that's already up to the same
/// server with the same token (a webview reload) and replays its state.
#[tauri::command]
pub fn server_connect(app: AppHandle, url: String, protocol_version: u32) {
    super::connect(&app, url, protocol_version);
}

/// A request to the server's HTTP API, signed with that server's token.
#[tauri::command]
pub async fn server_request(
    app: AppHandle,
    url: String,
    method: String,
    path: String,
    body: Option<serde_json::Value>
) -> Result<super::auth::Response, String> {
    super::auth::request(&app, &url, &method, &path, body).await
}

#[tauri::command]
pub fn auth_state(app: AppHandle) -> super::auth::View {
    super::auth::view(&app)
}

/// Once, from the tokens the webview used to keep in localStorage (server URL -> JWT).
#[tauri::command]
pub async fn auth_migrate(app: AppHandle, tokens: std::collections::BTreeMap<String, String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || super::auth::migrate(&app, tokens))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn auth_sign_out(app: AppHandle, url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || super::auth::set_token(&app, &url, None))
        .await
        .map_err(|e| e.to_string())?
}

/// Starts signing in to `url`. The browser link arrives as `auth-uri`, the result as `auth-changed`.
#[tauri::command]
pub fn auth_sign_in(app: AppHandle, url: String) {
    super::auth::sign_in(&app, url);
}

#[tauri::command]
pub fn auth_sign_in_cancel(app: AppHandle) {
    super::auth::sign_in_cancel(&app);
}

#[tauri::command]
pub fn server_disconnect(app: AppHandle) {
    super::disconnect(&app);
}

/// One `{type, message}` envelope, as JSON text.
#[tauri::command]
pub fn server_send(app: AppHandle, text: String) {
    super::send(&app, text);
}

/// The avatar parameters the share page shows (see `super::set_forwarded`).
#[tauri::command]
pub fn server_set_forwarded_addresses(app: AppHandle, addresses: Vec<String>) {
    super::set_forwarded(&app, addresses);
}

/// The controls and rules viewer commands are checked against (see `super::policy`).
#[tauri::command]
pub fn server_set_policy(app: AppHandle, policy: super::policy::Policy) {
    super::set_policy(&app, policy);
}
