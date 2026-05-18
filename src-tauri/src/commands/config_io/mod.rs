//! commands 配下で共有する設定ファイル（TOML）I/O ヘルパ。

use crate::commands::dto::ErrorResponse;
use serde::Serialize;
use std::path::Path;

pub(crate) fn resolve_config_dir() -> std::path::PathBuf {
    let relative = std::path::PathBuf::from("../config");
    if relative.exists() {
        return relative;
    }
    std::path::PathBuf::from("config")
}

pub(crate) fn write_toml_atomic<T: Serialize>(
    file_name: &str,
    value: &T,
) -> Result<(), ErrorResponse> {
    let config_dir = resolve_config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| ErrorResponse {
        error: format!("Failed to create config directory: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    let target_path = config_dir.join(file_name);
    let tmp_file_name = format!("{}.tmp", file_name);
    let tmp_path = config_dir.join(&tmp_file_name);

    let toml_text = toml::to_string_pretty(value).map_err(|e| ErrorResponse {
        error: format!("Failed to serialize {}: {}", file_name, e),
        code: "SERIALIZE_ERROR".to_string(),
    })?;

    std::fs::write(&tmp_path, &toml_text).map_err(|e| ErrorResponse {
        error: format!("Failed to write {}: {}", tmp_file_name, e),
        code: "IO_ERROR".to_string(),
    })?;

    replace_file_atomically(&tmp_path, &target_path, &toml_text, file_name)
}

fn replace_file_atomically(
    tmp_path: &Path,
    target_path: &Path,
    content: &str,
    label: &str,
) -> Result<(), ErrorResponse> {
    if target_path.exists() {
        let _ = std::fs::remove_file(target_path);
    }

    if let Err(rename_error) = std::fs::rename(tmp_path, target_path) {
        std::fs::write(target_path, content).map_err(|write_error| ErrorResponse {
            error: format!(
                "Failed to replace {} (rename: {}; write fallback: {})",
                label, rename_error, write_error
            ),
            code: "IO_ERROR".to_string(),
        })?;
        let _ = std::fs::remove_file(tmp_path);
    }

    Ok(())
}
