// ドライバマニフェストディスカバリコマンド
// manifest 正本 / bundle staging 配下のマニフェストを自動発見

use crate::drivers::manifest;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

/// ディスカバリリクエスト
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverDriverPackagesRequest {
    pub driver_ui_base_dir: String,
}

/// ディスカバリレスポンス
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverDriverPackagesResponse {
    pub available_count: usize,
    pub invalid_count: usize,
    pub available: Vec<DiscoveredDriverPackageDto>,
    pub invalid: Vec<InvalidDriverPackageDto>,
}

/// Discovery で見つかったドライバパッケージ（DTO）
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDriverPackageDto {
    pub driver_type: String,
    pub display_name: String,
    pub manifest_path: String,
    pub registration_ui_path: String,
    pub runtime_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    pub capabilities: Vec<String>,
}

/// 無効なドライバパッケージ（DTO）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidDriverPackageDto {
    pub manifest_path: String,
    pub status_code: String,
    pub status_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_type_hint: Option<String>,
}

/// ドライバマニフェストをディスカバリして利用可能なドライバを一覧取得
#[tauri::command]
pub async fn discover_driver_packages(
    req: DiscoverDriverPackagesRequest,
) -> Result<DiscoverDriverPackagesResponse, String> {
    let base_dir = PathBuf::from(&req.driver_ui_base_dir);

    info!("Discovering driver packages from: {}", base_dir.display());

    let result = manifest::discover_driver_packages(&base_dir)
        .map_err(|e| format!("Discovery failed: {}", e))?;

    let available = result
        .available
        .iter()
        .map(|pkg| DiscoveredDriverPackageDto {
            driver_type: pkg.driver_type.clone(),
            display_name: pkg.display_name.clone(),
            manifest_path: pkg.manifest_path.to_string_lossy().to_string(),
            registration_ui_path: pkg.registration_ui_path.to_string_lossy().to_string(),
            runtime_path: pkg.runtime_path.to_string_lossy().to_string(),
            version: pkg.version.clone(),
            vendor: pkg.vendor.clone(),
            capabilities: pkg.capabilities.clone(),
        })
        .collect::<Vec<_>>();

    let invalid = result
        .invalid
        .iter()
        .map(|pkg| InvalidDriverPackageDto {
            manifest_path: pkg.manifest_path.to_string_lossy().to_string(),
            status_code: pkg.status_code.clone(),
            status_message: pkg.status_message.clone(),
            driver_type_hint: pkg.driver_type_hint.clone(),
        })
        .collect::<Vec<_>>();

    let available_count = available.len();
    let invalid_count = invalid.len();

    info!(
        "Discovery completed: {} available, {} invalid",
        available_count, invalid_count
    );

    Ok(DiscoverDriverPackagesResponse {
        available_count,
        invalid_count,
        available,
        invalid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_postgres_manifest() {
        // repo root を指定すると ops/driver-ui 正本が優先走査される
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let req = DiscoverDriverPackagesRequest {
            driver_ui_base_dir: repo_root.to_string_lossy().to_string(),
        };

        let result = discover_driver_packages(req).await;
        assert!(result.is_ok(), "Discovery should succeed");

        let response = result.unwrap();
        assert!(
            response
                .available
                .iter()
                .any(|pkg| pkg.driver_type == "postgres"),
            "Should find postgres manifest from ops/driver-ui source of truth"
        );
    }

    #[tokio::test]
    async fn test_discover_joywatcher_manifest() {
        // ops/driver-ui 自体を直接指定しても discovery できる
        let ops_driver_ui_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("ops/driver-ui");

        let req = DiscoverDriverPackagesRequest {
            driver_ui_base_dir: ops_driver_ui_dir.to_string_lossy().to_string(),
        };

        let result = discover_driver_packages(req).await;
        assert!(result.is_ok(), "Discovery should succeed");

        let response = result.unwrap();
        assert!(
            response
                .available
                .iter()
                .any(|pkg| pkg.driver_type == "joywatcher"),
            "Should find joywatcher manifest when base dir is ops/driver-ui"
        );
    }

    #[tokio::test]
    async fn test_discover_staged_manifests_from_src_tauri() {
        // src-tauri 配下の bundle staging (`driver-ui/<type>/`) を直接走査する
        let src_tauri_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let req = DiscoverDriverPackagesRequest {
            driver_ui_base_dir: src_tauri_dir.to_string_lossy().to_string(),
        };

        let result = discover_driver_packages(req).await;
        assert!(
            result.is_ok(),
            "Discovery should succeed for src-tauri staging dir"
        );

        let response = result.unwrap();
        assert_eq!(
            response.available_count, 2,
            "Should discover postgres and joywatcher manifests from staging"
        );
        assert_eq!(
            response.invalid_count, 0,
            "Staged manifests should be valid"
        );
    }

    #[tokio::test]
    async fn test_discover_nonexistent_directory() {
        let req = DiscoverDriverPackagesRequest {
            driver_ui_base_dir: "/nonexistent/path".to_string(),
        };

        let result = discover_driver_packages(req).await;
        assert!(
            result.is_ok(),
            "Discovery should succeed even for nonexistent directory"
        );

        let response = result.unwrap();
        assert_eq!(
            response.available_count, 0,
            "Should discover nothing for nonexistent directory"
        );
        assert_eq!(response.invalid_count, 0);
    }
}
