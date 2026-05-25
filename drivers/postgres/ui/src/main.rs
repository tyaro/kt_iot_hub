#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// PostgreSQL レジストレーション用のドライバUI Tauri アプリ。
// コマンド実装は kt_driver_ui_host クレートに集約しているため、
// ここでは単にハンドラに並べて登録するのみとする。

use kt_driver_ui_host::{bridge, postgres};

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
            postgres::postgres_test_connection,
            postgres::postgres_list_tables,
            postgres::postgres_list_columns,
        ])
        .run(tauri::generate_context!())
        .expect("error while running driver_ui_postgres");
}
