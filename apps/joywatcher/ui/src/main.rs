#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

//! JoyWatcher レジストレーション用のドライバUI Tauri アプリ。
//! x86 bridge 経由で TagSel2 / JWGetTagIDS2 / JWRead を利用し、
//! 本体との launch context / 保存導線を扱う。

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
async fn resolve_joywatcher_tag(
    endpoint: String,
    userId: i32,
    password: String,
    tagPath: String,
) -> Result<i32, String> {
    let tag_path = tagPath.trim();
    if tag_path.is_empty() {
        return Err("タグパスを入力してください".to_string());
    }

    let endpoint = endpoint.clone();
    let password = password.clone();
    let tag_path = tag_path.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        joywatcher_bridge_client::resolve_single_tag(&endpoint, userId, &password, &tag_path)
    })
    .await
    .map_err(|error| format!("resolve_joywatcher_tag task failed: {}", error))?
}

#[allow(non_snake_case)]
#[tauri::command]
async fn browse_joywatcher_tags(
    endpoint: String,
    userId: i32,
    password: String,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        joywatcher_bridge_client::browse_tags(&endpoint, userId, &password)
    })
    .await
    .map_err(|error| format!("browse_joywatcher_tags task failed: {}", error))?
}

#[allow(non_snake_case)]
#[tauri::command]
async fn probe_joywatcher_tag_types(
    endpoint: String,
    userId: i32,
    password: String,
    tagIds: Vec<i32>,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        joywatcher_bridge_client::probe_tag_types(&endpoint, userId, &password, &tagIds).map(
            |items| {
                items
                    .into_iter()
                    .map(|item| {
                        let dtype = item.dtype.map(|value| value.to_string()).unwrap_or_default();
                        format!("{}|{}|{}|{}", item.tag_id, item.value_kind, item.quality, dtype)
                    })
                    .collect()
            },
        )
    })
    .await
    .map_err(|error| format!("probe_joywatcher_tag_types task failed: {}", error))?
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            close_driver_ui_window,
            browse_joywatcher_tags,
            resolve_joywatcher_tag,
            probe_joywatcher_tag_types,
            bridge::get_driver_ui_launch_context,
            bridge::save_driver_ui_output,
        ])
        .run(tauri::generate_context!())
        .expect("error while running driver_ui_joywatcher");
}
