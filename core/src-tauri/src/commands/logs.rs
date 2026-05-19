use crate::app_logs;

#[tauri::command]
pub fn list_app_logs(limit: Option<usize>) -> Vec<String> {
    app_logs::list_logs(limit.unwrap_or(500))
}

#[tauri::command]
pub fn clear_app_logs() {
    app_logs::clear_logs();
}
