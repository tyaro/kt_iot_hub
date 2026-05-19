// ドライバマニフェストディスカバリロジック
// manifest.rs から分離したディレクトリ走査・発見処理

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::manifest::{
    load_manifest, validate_manifest, DiscoveredDriverPackage, DiscoveryResult,
    InvalidDriverPackage, ManifestValidationError,
};

/// 指定ベースフォルダ配下の manifest 正本/staging を走査してドライバを発見
///
/// 許可する起点は以下の通り:
/// - <base>/ops/driver-ui/<type>/driver-manifest.json （開発時正本）
/// - <base>/driver-ui/<type>/driver-manifest.json     （bundle staging / 配布物）
/// - <base> 自体が driver-ui ルートのケース
pub fn discover_driver_packages(driver_ui_base_dir: &Path) -> Result<DiscoveryResult> {
    let mut available = Vec::new();
    let mut invalid = Vec::new();
    let mut found_driver_types = std::collections::HashMap::new();

    let discovery_roots = discovery_root_candidates(driver_ui_base_dir);
    if discovery_roots.is_empty() {
        return Ok(DiscoveryResult { available, invalid });
    }

    for discovery_root in discovery_roots {
        if let Ok(entries) = std::fs::read_dir(&discovery_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("driver-manifest.json");

                    match discover_single_manifest(&manifest_path, &path) {
                        Ok(package) => {
                            if found_driver_types.contains_key(&package.driver_type) {
                                let first_idx =
                                    available.iter().position(|p: &DiscoveredDriverPackage| {
                                        p.driver_type == package.driver_type
                                    });
                                if let Some(idx) = first_idx {
                                    let existing = available.remove(idx);
                                    invalid.push(InvalidDriverPackage {
                                            manifest_path: existing.manifest_path.clone(),
                                            status_code: "DUPLICATE_DRIVER_TYPE".to_string(),
                                            status_message: format!(
                                                "Duplicate driver_type '{}' found at multiple locations",
                                                package.driver_type
                                            ),
                                            driver_type_hint: Some(package.driver_type.clone()),
                                        });
                                }
                                invalid.push(InvalidDriverPackage {
                                    manifest_path: package.manifest_path.clone(),
                                    status_code: "DUPLICATE_DRIVER_TYPE".to_string(),
                                    status_message: format!(
                                        "Duplicate driver_type '{}' found at multiple locations",
                                        package.driver_type
                                    ),
                                    driver_type_hint: Some(package.driver_type.clone()),
                                });
                            } else {
                                found_driver_types
                                    .insert(package.driver_type.clone(), package.clone());
                                available.push(package);
                            }
                        }
                        Err(error) => {
                            let (status_code, status_message, hint) = match error {
                                DiscoverError::ParseFailed { reason } => {
                                    ("MANIFEST_PARSE_ERROR", reason, None)
                                }
                                DiscoverError::ValidationFailed {
                                    reason,
                                    driver_type,
                                } => ("MANIFEST_VALIDATION_ERROR", reason, driver_type),
                            };

                            invalid.push(InvalidDriverPackage {
                                manifest_path,
                                status_code: status_code.to_string(),
                                status_message,
                                driver_type_hint: hint,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(DiscoveryResult { available, invalid })
}

fn discovery_root_candidates(base_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    candidates.push(base_dir.join("ops").join("driver-ui"));
    candidates.push(base_dir.join("driver-ui"));
    if contains_manifest_children(base_dir) {
        candidates.push(base_dir.to_path_buf());
    }

    let mut unique = Vec::new();
    for candidate in candidates {
        if candidate.exists() && candidate.is_dir() && !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }

    unique
}

fn contains_manifest_children(base_dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(base_dir) else {
        return false;
    };

    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        path.is_dir() && path.join("driver-manifest.json").exists()
    })
}

/// 1 つのマニフェストを読み込んで検証
fn discover_single_manifest(
    manifest_path: &Path,
    manifest_dir: &Path,
) -> Result<DiscoveredDriverPackage, DiscoverError> {
    if !manifest_path.exists() {
        return Err(DiscoverError::ParseFailed {
            reason: "Manifest file not found".to_string(),
        });
    }

    let manifest = load_manifest(manifest_path).map_err(|e| DiscoverError::ParseFailed {
        reason: format!("Failed to parse manifest: {}", e),
    })?;

    let driver_type = manifest.driver_type.clone();

    validate_manifest(&manifest, manifest_dir).map_err(|e| match e {
        ManifestValidationError::SchemaError { reason } => DiscoverError::ValidationFailed {
            reason,
            driver_type: Some(driver_type),
        },
        ManifestValidationError::ExecutableNotFound { reason } => DiscoverError::ValidationFailed {
            reason,
            driver_type: Some(driver_type),
        },
        ManifestValidationError::ParseError { reason } => DiscoverError::ParseFailed { reason },
        ManifestValidationError::VersionIncompatible { reason } => {
            DiscoverError::ValidationFailed {
                reason,
                driver_type: Some(driver_type),
            }
        }
    })
}

/// Discovery 処理中の内部エラー型
#[derive(Debug)]
enum DiscoverError {
    ParseFailed {
        reason: String,
    },
    ValidationFailed {
        reason: String,
        driver_type: Option<String>,
    },
}
