// ドライバ実行ファイルのパス解決ヘルパー
// DriverProcessManager から分離した候補生成・manifest 解決ロジック

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tracing::info;

use super::manifest;

/// ドライバ UI のルート候補ディレクトリを列挙する
/// driver_ui_base_dir が指定されている場合はそれを先頭に追加する
pub(super) fn app_root_candidates(driver_ui_base_dir: Option<&str>) -> Vec<PathBuf> {
    let mut roots = Vec::<PathBuf>::new();

    if let Some(base_dir) = driver_ui_base_dir {
        let trimmed = base_dir.trim();
        if !trimmed.is_empty() {
            roots.push(PathBuf::from(trimmed));
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        let mut cursor = Some(current_dir.as_path());
        while let Some(path) = cursor {
            roots.push(path.to_path_buf());
            cursor = path.parent();
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            roots.push(exe_dir.join("resources"));
            if let Some(parent) = exe_dir.parent() {
                roots.push(parent.join("Resources"));
            }

            let mut cursor = Some(exe_dir);
            while let Some(path) = cursor {
                roots.push(path.to_path_buf());
                cursor = path.parent();
            }
        }
    }

    let mut unique = Vec::<PathBuf>::new();
    for root in roots {
        if !unique.contains(&root) {
            unique.push(root);
        }
    }
    unique
}

/// 指定 root から manifest を参照してランタイムパスを解決する
/// 見つかった場合は Some(path) を返し、見つからない場合は None を返す
pub(super) fn resolve_manifest_runtime_path_for_root(
    driver_type: &str,
    root: &Path,
) -> Result<Option<PathBuf>> {
    let driver_type = driver_type.trim();
    if driver_type.is_empty() {
        return Ok(None);
    }

    for manifest_path in manifest_candidates_for_root(driver_type, root) {
        if !manifest_path.exists() {
            continue;
        }

        let Some(manifest_dir) = manifest_path.parent() else {
            continue;
        };

        let loaded_manifest = match manifest::load_manifest(&manifest_path) {
            Ok(manifest) => manifest,
            Err(err) => {
                return Err(anyhow!(
                    "Manifest runtime resolution failed: path={} reason={}",
                    manifest_path.display(),
                    err
                ));
            }
        };

        if loaded_manifest.driver_type != driver_type {
            return Err(anyhow!(
                "Manifest runtime resolution failed: path={} requested_driver_type={} manifest_driver_type={}",
                manifest_path.display(),
                driver_type,
                loaded_manifest.driver_type
            ));
        }

        match manifest::validate_manifest(&loaded_manifest, manifest_dir) {
            Ok(package) => {
                info!(
                    "Resolved driver runtime from manifest: driver_type={} path={}",
                    driver_type,
                    package.runtime_path.display()
                );
                return Ok(Some(package.runtime_path));
            }
            Err(err) => {
                return Err(anyhow!(
                    "Manifest runtime resolution failed: path={} reason={:?}",
                    manifest_path.display(),
                    err
                ));
            }
        }
    }

    Ok(None)
}

/// 指定 root における driver-manifest.json 候補パスを列挙する
fn manifest_candidates_for_root(driver_type: &str, root: &Path) -> Vec<PathBuf> {
    let candidates = vec![
        root.join("ops")
            .join("driver-ui")
            .join(driver_type)
            .join("driver-manifest.json"),
        root.join("driver-ui")
            .join(driver_type)
            .join("driver-manifest.json"),
        root.join(driver_type).join("driver-manifest.json"),
    ];

    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }

    unique
}

/// 指定 root における driver 同居配置候補パスを列挙する
pub(super) fn colocated_runtime_candidates_for_root(
    driver_type: &str,
    exe_name: &str,
    root: &Path,
) -> Vec<PathBuf> {
    let candidates = vec![
        root.join("ops")
            .join("driver-ui")
            .join(driver_type)
            .join(exe_name),
        root.join("driver-ui").join(driver_type).join(exe_name),
        root.join(driver_type).join(exe_name),
    ];

    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }

    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colocated_candidates_support_repo_root_base() {
        let candidates = colocated_runtime_candidates_for_root(
            "postgres",
            "driver-postgres.exe",
            &PathBuf::from(r"D:\develop\kt_iot_hub"),
        );

        assert_eq!(
            candidates[0],
            PathBuf::from(r"D:\develop\kt_iot_hub")
                .join("ops")
                .join("driver-ui")
                .join("postgres")
                .join("driver-postgres.exe")
        );
        assert_eq!(
            candidates[1],
            PathBuf::from(r"D:\develop\kt_iot_hub")
                .join("driver-ui")
                .join("postgres")
                .join("driver-postgres.exe")
        );
    }

    #[test]
    fn colocated_candidates_support_driver_ui_root_base() {
        let candidates = colocated_runtime_candidates_for_root(
            "postgres",
            "driver-postgres.exe",
            &PathBuf::from(r"D:\develop\kt_iot_hub\driver-ui"),
        );

        assert_eq!(
            candidates[2],
            PathBuf::from(r"D:\develop\kt_iot_hub\driver-ui")
                .join("postgres")
                .join("driver-postgres.exe")
        );
    }
}
