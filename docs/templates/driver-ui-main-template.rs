#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

//! 新しい Driver UI 用 `src/main.rs` の最小テンプレート。
//!
//! 使い方:
//! - `YOUR_DRIVER_COMMANDS` を実際のドライバ固有コマンドへ置き換える
//! - `kt_driver_ui_host` の共通 bridge を必ず残す

use kt_driver_ui_host::bridge;

#[tauri::command]
fn close_driver_ui_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window
        .close()
        .map_err(|e| format!("failed to close driver ui window: {}", e))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            close_driver_ui_window,
            bridge::get_driver_ui_launch_context,
            bridge::save_driver_ui_output,
            // YOUR_DRIVER_COMMANDS,
        ])
        .run(tauri::generate_context!())
        .expect("error while running driver ui");
}