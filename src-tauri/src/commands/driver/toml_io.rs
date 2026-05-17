//! drivers.toml / tags.toml のアトミック書き出し処理。
//! 一時ファイルへ書いてから rename で差し替える方式で、失敗時は通常 write へフォールバックする。

use crate::commands::dto::ErrorResponse;
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub(super) struct TagsTomlFile {
    #[serde(rename = "scan_group")]
    pub scan_group: Vec<ScanGroupConfig>,
    #[serde(rename = "tag")]
    pub tag: Vec<TagConfig>,
}

#[derive(Serialize)]
pub(super) struct DriversTomlFile {
    #[serde(rename = "driver")]
    pub driver: Vec<DriverConfig>,
}

pub(super) fn resolve_config_dir() -> std::path::PathBuf {
    let relative = std::path::PathBuf::from("../config");
    if relative.exists() {
        return relative;
    }
    std::path::PathBuf::from("config")
}

pub(super) fn write_tags_toml_atomic(
    scan_groups: &[ScanGroupConfig],
    tags: &[TagConfig],
) -> Result<(), ErrorResponse> {
    let config_dir = resolve_config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| ErrorResponse {
        error: format!("Failed to create config directory: {}", e),
        code: "IO_ERROR".to_string(),
    })?;
    let tags_path = config_dir.join("tags.toml");
    let tmp_path = config_dir.join("tags.toml.tmp");

    let toml_text = toml::to_string_pretty(&TagsTomlFile {
        scan_group: scan_groups.to_vec(),
        tag: tags.to_vec(),
    })
    .map_err(|e| ErrorResponse {
        error: format!("Failed to serialize tags.toml: {}", e),
        code: "SERIALIZE_ERROR".to_string(),
    })?;

    std::fs::write(&tmp_path, &toml_text).map_err(|e| ErrorResponse {
        error: format!("Failed to write tags.toml.tmp: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    replace_file_atomically(&tmp_path, &tags_path, &toml_text, "tags.toml")
}

pub(super) fn write_drivers_toml_atomic(drivers: &[DriverConfig]) -> Result<(), ErrorResponse> {
    let config_dir = resolve_config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| ErrorResponse {
        error: format!("Failed to create config directory: {}", e),
        code: "IO_ERROR".to_string(),
    })?;
    let drivers_path = config_dir.join("drivers.toml");
    let tmp_path = config_dir.join("drivers.toml.tmp");

    let toml_text = toml::to_string_pretty(&DriversTomlFile {
        driver: drivers.to_vec(),
    })
    .map_err(|e| ErrorResponse {
        error: format!("Failed to serialize drivers.toml: {}", e),
        code: "SERIALIZE_ERROR".to_string(),
    })?;

    std::fs::write(&tmp_path, &toml_text).map_err(|e| ErrorResponse {
        error: format!("Failed to write drivers.toml.tmp: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    replace_file_atomically(&tmp_path, &drivers_path, &toml_text, "drivers.toml")
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
