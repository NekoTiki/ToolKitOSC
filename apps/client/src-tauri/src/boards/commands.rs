//! The ESP32 board commands the webview calls (see `super`).

use tauri::AppHandle;

use super::{BoardView, BoardsSnapshot, OutputConfig, Target};

#[tauri::command]
pub fn boards_list(app: AppHandle) -> BoardsSnapshot {
    super::snapshot(&app)
}

/// Pairs with a board found over mDNS. Resolves once someone pressed its BOOT button (progress
/// arrives as `board-pairing` events), or fails with a message to show.
#[tauri::command]
pub async fn board_add(app: AppHandle, id: String) -> Result<BoardView, String> {
    crate::addons::require(&app, "boards")?;
    super::pair(app, Target::Found(id)).await
}

/// Same, for a board mDNS can't see: `192.168.1.40`, `tkosc-a1b2c3.local`, optionally `:port`.
#[tauri::command]
pub async fn board_add_by_address(app: AppHandle, address: String) -> Result<BoardView, String> {
    crate::addons::require(&app, "boards")?;
    super::pair(app, Target::Address(address)).await
}

#[tauri::command]
pub fn board_pair_cancel(app: AppHandle) {
    super::cancel_pairing(&app);
}

/// Unpairs the board if it's online, and forgets it.
#[tauri::command]
pub fn board_remove(app: AppHandle, id: String) {
    super::remove(&app, &id);
}

/// Saves a board's outputs and sends them to it, now or when it next connects.
#[tauri::command]
pub fn board_save_outputs(app: AppHandle, id: String, outputs: Vec<OutputConfig>) -> Result<(), String> {
    super::save_outputs(&app, &id, outputs)
}

/// Switches a saved output on for half a second.
#[tauri::command]
pub fn board_test_output(app: AppHandle, id: String, pin: u8) -> Result<(), String> {
    crate::addons::require(&app, "boards")?;
    super::test_output(&app, &id, pin)
}

/// Every output of every board off (Stop everything).
#[tauri::command]
pub fn boards_stop_all(app: AppHandle) {
    super::stop_all(&app);
}
