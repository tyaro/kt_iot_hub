// ドライバマニフェスト処理
// driver-manifest.json の読み込み、検証、Discovery ロジック

use anyhow::{Context, Result};
use kt_driver_ui_protocol::{DriverManifest, ProtocolVersion};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Discovery で見つかったドライバパッケージの情報
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDriverPackage {
    pub driver_type: String,
    pub display_name: String,
    pub manifest_path: PathBuf,
    pub registration_ui_path: PathBuf,
    pub runtime_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub available: bool,
    pub status_code: String,
    pub status_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    pub capabilities: Vec<String>,
}

/// マニフェストの検証エラー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ManifestValidationError {
    ParseError { reason: String },
    SchemaError { reason: String },
    ExecutableNotFound { reason: String },
    VersionIncompatible { reason: String },
}

/// マニフェストファイル（driver-manifest.json）を指定パスから読み込む
pub fn load_manifest(manifest_path: &Path) -> Result<DriverManifest> {
    let contents =
        std::fs::read_to_string(manifest_path).context("Failed to read manifest file")?;

    let manifest: DriverManifest =
        serde_json::from_str(&contents).context("Failed to parse manifest JSON")?;

    Ok(manifest)
}

/// マニフェストのバリデーション
/// 必須フィールド、スキーマバージョン、相対パス解決を検証
pub fn validate_manifest(
    manifest: &DriverManifest,
    manifest_dir: &Path,
) -> Result<DiscoveredDriverPackage, ManifestValidationError> {
    // スキーマバージョン確認
    if manifest.manifest_version != 1 {
        return Err(ManifestValidationError::SchemaError {
            reason: format!(
                "Unsupported manifest version: {} (expected 1)",
                manifest.manifest_version
            ),
        });
    }

    // driver_type の形式を確認 ([a-z0-9][a-z0-9_-]*)
    if !is_valid_driver_type(&manifest.driver_type) {
        return Err(ManifestValidationError::SchemaError {
            reason: format!(
                "Invalid driver_type format: '{}'. Must match [a-z0-9][a-z0-9_-]*",
                manifest.driver_type
            ),
        });
    }

    // manifest_path を計算（バリデーション時に渡された dir を使用）
    let manifest_path = manifest_dir.join("driver-manifest.json");

    // 相対パスを解決
    let registration_ui_path = manifest_dir.join(&manifest.registration_ui);
    let runtime_path = manifest_dir.join(&manifest.runtime);

    // 実行ファイル存在確認
    if !registration_ui_path.exists() {
        return Err(ManifestValidationError::ExecutableNotFound {
            reason: format!(
                "Registration UI not found: {}",
                registration_ui_path.display()
            ),
        });
    }

    if !runtime_path.exists() {
        return Err(ManifestValidationError::ExecutableNotFound {
            reason: format!("Runtime executable not found: {}", runtime_path.display()),
        });
    }

    // Protocol version を確認（将来の互換性判定用）
    validate_protocol_version(&manifest.protocol)?;

    Ok(DiscoveredDriverPackage {
        driver_type: manifest.driver_type.clone(),
        display_name: manifest.display_name.clone(),
        manifest_path,
        registration_ui_path,
        runtime_path,
        version: manifest.version.clone(),
        available: true,
        status_code: "AVAILABLE".to_string(),
        status_message: "Driver package is available".to_string(),
        vendor: manifest.vendor.clone(),
        capabilities: manifest.capabilities.clone(),
    })
}

/// Protocol version の妥当性確認
fn validate_protocol_version(protocol: &ProtocolVersion) -> Result<(), ManifestValidationError> {
    // Version 文字列が空でないことを確認
    if protocol.driver_ui_request_version.is_empty() {
        return Err(ManifestValidationError::SchemaError {
            reason: "driver_ui_request_version must not be empty".to_string(),
        });
    }

    if protocol.driver_ui_response_version.is_empty() {
        return Err(ManifestValidationError::SchemaError {
            reason: "driver_ui_response_version must not be empty".to_string(),
        });
    }

    Ok(())
}

/// ドライバマニフェストディスカバリの結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub available: Vec<DiscoveredDriverPackage>,
    pub invalid: Vec<InvalidDriverPackage>,
}

/// 無効なドライバパッケージ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidDriverPackage {
    pub manifest_path: PathBuf,
    pub status_code: String,
    pub status_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_type_hint: Option<String>,
}

/// driver_type の形式が有効かチェック
/// 正規表現: [a-z0-9][a-z0-9_-]*
fn is_valid_driver_type(driver_type: &str) -> bool {
    if driver_type.is_empty() {
        return false;
    }

    // 最初の文字は小文字アルファベットか数字
    let first_char = driver_type.chars().next().unwrap();
    if !first_char.is_ascii_lowercase() && !first_char.is_ascii_digit() {
        return false;
    }

    // 2 文字目以降は小文字アルファベット、数字、アンダースコア、ハイフン
    driver_type
        .chars()
        .skip(1)
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_driver_type() {
        assert!(is_valid_driver_type("postgres"));
        assert!(is_valid_driver_type("joywatcher"));
        assert!(is_valid_driver_type("d1"));
        assert!(is_valid_driver_type("driver-1"));
        assert!(is_valid_driver_type("my_driver_1"));
    }

    #[test]
    fn test_invalid_driver_type() {
        assert!(!is_valid_driver_type(""));
        assert!(!is_valid_driver_type("PostgreSQL")); // uppercase
        assert!(!is_valid_driver_type("-postgres")); // starts with hyphen
        assert!(!is_valid_driver_type("driver type")); // space
        assert!(!is_valid_driver_type("driver.type")); // dot
    }

    #[test]
    fn test_validate_protocol_version() {
        let valid = ProtocolVersion {
            driver_ui_request_version: "1".to_string(),
            driver_ui_response_version: "1".to_string(),
        };
        assert!(validate_protocol_version(&valid).is_ok());

        let invalid_request = ProtocolVersion {
            driver_ui_request_version: "".to_string(),
            driver_ui_response_version: "1".to_string(),
        };
        assert!(validate_protocol_version(&invalid_request).is_err());
    }

    #[test]
    fn test_load_postgres_manifest_from_ops_source_of_truth() {
        // workspace root を起点に ops/driver-ui 正本の manifest を参照する
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("ops/driver-ui/postgres/driver-manifest.json");

        let manifest = load_manifest(&manifest_path);
        assert!(
            manifest.is_ok(),
            "Failed to load postgres manifest: {:?}",
            manifest.err()
        );

        let manifest = manifest.unwrap();
        assert_eq!(manifest.driver_type, "postgres");
        assert_eq!(manifest.display_name, "PostgreSQL 接続");
        assert_eq!(manifest.manifest_version, 1);
        assert_eq!(manifest.registration_ui, "registration-ui.exe");
        assert_eq!(manifest.runtime, "driver-postgres.exe");
    }

    #[test]
    fn test_validate_postgres_manifest_schema() {
        let manifest = DriverManifest {
            manifest_version: 1,
            driver_type: "postgres".to_string(),
            display_name: "PostgreSQL 接続".to_string(),
            registration_ui: "registration-ui.exe".to_string(),
            runtime: "driver-postgres.exe".to_string(),
            protocol: ProtocolVersion {
                driver_ui_request_version: "1".to_string(),
                driver_ui_response_version: "1".to_string(),
            },
            description: Some("Test manifest".to_string()),
            vendor: Some("kt_iot_hub".to_string()),
            version: Some("0.1.0".to_string()),
            homepage: None,
            min_hub_version: None,
            max_hub_version: None,
            capabilities: vec!["connectionTest".to_string()],
        };

        // executables が存在しないため、バリデーションは失敗するはず
        let manifest_dir = PathBuf::from("/nonexistent");
        let result = validate_manifest(&manifest, &manifest_dir);
        assert!(
            result.is_err(),
            "Expected validation to fail for nonexistent paths"
        );
    }

    #[test]
    fn test_validate_manifest_bad_schema_version() {
        let manifest = DriverManifest {
            manifest_version: 2, // Invalid version
            driver_type: "postgres".to_string(),
            display_name: "PostgreSQL 接続".to_string(),
            registration_ui: "registration-ui.exe".to_string(),
            runtime: "driver-postgres.exe".to_string(),
            protocol: ProtocolVersion {
                driver_ui_request_version: "1".to_string(),
                driver_ui_response_version: "1".to_string(),
            },
            description: None,
            vendor: None,
            version: None,
            homepage: None,
            min_hub_version: None,
            max_hub_version: None,
            capabilities: vec![],
        };

        let manifest_dir = PathBuf::from("/tmp");
        let result = validate_manifest(&manifest, &manifest_dir);
        assert!(result.is_err());
        match result {
            Err(ManifestValidationError::SchemaError { reason }) => {
                assert!(reason.contains("Unsupported manifest version"));
            }
            _ => panic!("Expected SchemaError"),
        }
    }

    #[test]
    fn test_validate_manifest_invalid_driver_type() {
        let manifest = DriverManifest {
            manifest_version: 1,
            driver_type: "PostgreSQL".to_string(), // Invalid: uppercase
            display_name: "PostgreSQL 接続".to_string(),
            registration_ui: "registration-ui.exe".to_string(),
            runtime: "driver-postgres.exe".to_string(),
            protocol: ProtocolVersion {
                driver_ui_request_version: "1".to_string(),
                driver_ui_response_version: "1".to_string(),
            },
            description: None,
            vendor: None,
            version: None,
            homepage: None,
            min_hub_version: None,
            max_hub_version: None,
            capabilities: vec![],
        };

        let manifest_dir = PathBuf::from("/tmp");
        let result = validate_manifest(&manifest, &manifest_dir);
        assert!(result.is_err());
        match result {
            Err(ManifestValidationError::SchemaError { reason }) => {
                assert!(reason.contains("Invalid driver_type format"));
            }
            _ => panic!("Expected SchemaError"),
        }
    }
}
