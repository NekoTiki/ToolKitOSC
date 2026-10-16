//! The OpenShock commands the webview calls (see `super`).

use tauri::AppHandle;

#[tauri::command]
pub fn openshock_state(app: AppHandle) -> super::View {
    super::view(&app)
}

/// Once, from the webview's old localStorage values (see `super::migrate`).
#[tauri::command]
pub async fn openshock_migrate(app: AppHandle, token: Option<String>, enabled: Option<bool>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || super::migrate(&app, token, enabled))
        .await
        .map_err(|e| e.to_string())?
}

/// An empty token removes it.
#[tauri::command]
pub async fn openshock_set_token(app: AppHandle, token: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || super::set_token(&app, token))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn openshock_set_enabled(app: AppHandle, enabled: bool) {
    super::set_enabled(&app, enabled);
}

#[tauri::command]
pub fn openshock_recheck(app: AppHandle) {
    super::recheck(&app);
}

#[tauri::command]
pub async fn openshock_shockers(app: AppHandle) -> Result<Vec<super::Hub>, String> {
    super::shockers(&app).await
}

/// Rejects when OpenShock isn't available. Failures while sending arrive as `openshock-error`.
#[tauri::command]
pub fn openshock_fire(app: AppHandle, fire: super::Fire) -> Result<(), String> {
    super::fire(&app, fire)
}

#[tauri::command]
pub async fn openshock_stop_all(app: AppHandle) -> Result<(), String> {
    super::stop_all(&app).await
}
