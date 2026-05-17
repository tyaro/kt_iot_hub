#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

//! JoyWatcher レジストレーション用のドライバUI Tauri アプリ。
//! 初期段階では DLL 呼び出しをまだ実装せず、
//! 本体との launch context / 保存導線を先に確立する。

mod joywatcher_bridge_client;

use kt_driver_ui_host::bridge;

#[tauri::command]
fn close_driver_ui_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window
        .close()
        .map_err(|e| format!("failed to close driver ui window: {}", e))
}

#[allow(non_snake_case)]
#[tauri::command]
fn resolve_joywatcher_tag(
    endpoint: String,
    userId: i32,
    password: String,
    tagPath: String,
) -> Result<i32, String> {
    let tag_path = tagPath.trim();
    if tag_path.is_empty() {
        return Err("タグパスを入力してください".to_string());
    }

    joywatcher_bridge_client::resolve_single_tag(&endpoint, userId, &password, tag_path)
}

#[allow(non_snake_case)]
#[tauri::command]
fn browse_joywatcher_tags(
    endpoint: String,
    userId: i32,
    password: String,
) -> Result<Vec<String>, String> {
    joywatcher_bridge_client::browse_tags(&endpoint, userId, &password)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            close_driver_ui_window,
            browse_joywatcher_tags,
            resolve_joywatcher_tag,
            bridge::get_driver_ui_launch_context,
            bridge::save_driver_ui_output,
        ])
        .run(tauri::generate_context!())
        .expect("error while running driver_ui_joywatcher");
}
