//! publishers.toml のアトミック書き出し処理。

use crate::commands::dto::ErrorResponse;
use crate::config::PublisherConfig;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub(super) struct PublishersTomlFile {
    #[serde(rename = "publisher")]
    pub publisher: Vec<PublisherConfig>,
}

pub(super) fn resolve_config_dir() -> std::path::PathBuf {
    let relative = std::path::PathBuf::from("../config");
    if relative.exists() {
        return relative;
    }
    std::path::PathBuf::from("config")
}

pub(super) fn write_publishers_toml_atomic(
    publishers: &[PublisherConfig],
) -> Result<(), ErrorResponse> {
    let config_dir = resolve_config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| ErrorResponse {
        error: format!("Failed to create config directory: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    let target_path = config_dir.join("publishers.toml");
    let tmp_path = config_dir.join("publishers.toml.tmp");
    let toml_text = toml::to_string_pretty(&PublishersTomlFile {
        publisher: publishers.to_vec(),
    })
    .map_err(|e| ErrorResponse {
        error: format!("Failed to serialize publishers.toml: {}", e),
        code: "SERIALIZE_ERROR".to_string(),
    })?;

    std::fs::write(&tmp_path, &toml_text).map_err(|e| ErrorResponse {
        error: format!("Failed to write publishers.toml.tmp: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    replace_file_atomically(&tmp_path, &target_path, &toml_text)
}

fn replace_file_atomically(
    tmp_path: &Path,
    target_path: &Path,
    content: &str,
) -> Result<(), ErrorResponse> {
    if target_path.exists() {
        let _ = std::fs::remove_file(target_path);
    }

    if let Err(rename_error) = std::fs::rename(tmp_path, target_path) {
        std::fs::write(target_path, content).map_err(|write_error| ErrorResponse {
            error: format!(
                "Failed to replace publishers.toml (rename: {}; write fallback: {})",
                rename_error, write_error
            ),
            code: "IO_ERROR".to_string(),
        })?;
        let _ = std::fs::remove_file(tmp_path);
    }

    Ok(())
}
