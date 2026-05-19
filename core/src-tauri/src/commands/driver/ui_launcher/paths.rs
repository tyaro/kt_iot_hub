//! ドライバUI 実行ファイルのパス解決と、共通の文字列正規化ヘルパ。

use crate::config::DriverConfig;
use crate::drivers::manifest;
use std::path::PathBuf;

/// 単一ドライバ設定から登録UI 実行ファイルの絶対パスを解決する。
pub(in crate::commands::driver) fn resolve_driver_ui_path(config: &DriverConfig) -> Option<String> {
    resolve_driver_ui_path_with_base(config, None)
}

/// ベースディレクトリを明示指定して解決する版。
pub(super) fn resolve_driver_ui_path_with_base(
    config: &DriverConfig,
    driver_ui_base_dir: Option<&str>,
) -> Option<String> {
    find_default_driver_ui_path(&config.driver_type, driver_ui_base_dir)
}

/// ドライバタイプ名と既存設定群からパスを解決する。
/// driver_id 指定なしで UI を起動する場合に利用する。
pub(super) fn resolve_driver_ui_path_for_type(
    configs: &[DriverConfig],
    driver_type: &str,
    driver_ui_base_dir: Option<&str>,
) -> Option<String> {
    configs
        .iter()
        .filter(|cfg| cfg.driver_type == driver_type)
        .find_map(|cfg| resolve_driver_ui_path_with_base(cfg, driver_ui_base_dir))
        .or_else(|| find_default_driver_ui_path(driver_type, driver_ui_base_dir))
}

/// 標準の配置規約 `driver-ui/<driver_type>/(registration-ui|driver-ui).exe` を探索する。
pub(super) fn find_default_driver_ui_path(
    driver_type: &str,
    driver_ui_base_dir: Option<&str>,
) -> Option<String> {
    let driver_type = driver_type.trim();
    if driver_type.is_empty() {
        return None;
    }

    let file_names = [
        format!("{}-registration-ui.exe", driver_type),
        "registration-ui.exe".to_string(),
        "driver-ui.exe".to_string(),
        format!("{}-registration-ui", driver_type),
        "registration-ui".to_string(),
        "driver-ui".to_string(),
    ];

    for root in app_root_candidates(driver_ui_base_dir) {
        if let Some(manifest_path) = resolve_manifest_driver_ui_path(driver_type, &root) {
            return Some(path_to_string(manifest_path));
        }

        for file_name in &file_names {
            // <root>/ops/driver-ui/<type>/<file> (新構成)
            let ops_root_style = root
                .join("ops")
                .join("driver-ui")
                .join(driver_type)
                .join(file_name);
            if ops_root_style.exists() {
                return Some(path_to_string(ops_root_style));
            }
            // <root>/driver-ui/<type>/<file> (後方互換)
            let app_root_style = root.join("driver-ui").join(driver_type).join(file_name);
            if app_root_style.exists() {
                return Some(path_to_string(app_root_style));
            }
            // <root>/<type>/<file> (base_dir に直接 driver-ui を指定されたケース)
            let driver_ui_root_style = root.join(driver_type).join(file_name);
            if driver_ui_root_style.exists() {
                return Some(path_to_string(driver_ui_root_style));
            }
        }
    }

    None
}

fn resolve_manifest_driver_ui_path(driver_type: &str, root: &std::path::Path) -> Option<PathBuf> {
    for manifest_path in manifest_candidates_for_root(driver_type, root) {
        if !manifest_path.exists() {
            continue;
        }

        let manifest_dir = manifest_path.parent()?;
        let loaded_manifest = manifest::load_manifest(&manifest_path).ok()?;
        if loaded_manifest.driver_type != driver_type {
            continue;
        }

        let registration_ui_path = manifest_dir.join(&loaded_manifest.registration_ui);
        if registration_ui_path.exists() {
            return Some(registration_ui_path);
        }
    }

    None
}

fn manifest_candidates_for_root(driver_type: &str, root: &std::path::Path) -> Vec<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();

    candidates.push(
        root.join("ops")
            .join("driver-ui")
            .join(driver_type)
            .join("driver-manifest.json"),
    );
    candidates.push(
        root.join("driver-ui")
            .join(driver_type)
            .join("driver-manifest.json"),
    );
    candidates.push(root.join(driver_type).join("driver-manifest.json"));
    candidates.push(
        root.join("drivers")
            .join(driver_type)
            .join("driver-ui")
            .join(driver_type)
            .join("driver-manifest.json"),
    );

    let mut unique = Vec::<PathBuf>::new();
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }

    unique
}

pub(super) fn describe_driver_ui_search_locations(
    driver_type: &str,
    driver_ui_base_dir: Option<&str>,
) -> String {
    let driver_type = driver_type.trim();
    if driver_type.is_empty() {
        return "<driver_type is empty>".to_string();
    }

    let mut locations = Vec::<String>::new();
    for root in app_root_candidates(driver_ui_base_dir) {
        locations.push(path_to_string(
            root.join("ops").join("driver-ui").join(driver_type),
        ));
        locations.push(path_to_string(root.join("driver-ui").join(driver_type)));
        locations.push(path_to_string(root.join(driver_type)));
    }

    let mut unique = Vec::<String>::new();
    for path in locations {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }

    unique.join(", ")
}

fn app_root_candidates(driver_ui_base_dir: Option<&str>) -> Vec<PathBuf> {
    let mut roots = Vec::<PathBuf>::new();

    // 優先順①: 設定画面で指定された base_dir
    if let Some(base_dir) = driver_ui_base_dir {
        let trimmed = base_dir.trim();
        if !trimmed.is_empty() {
            roots.push(PathBuf::from(trimmed));
        }
    }

    // 優先順②: DRIVER_BIN_DIR 環境変数
    if let Ok(driver_bin_dir) = std::env::var("DRIVER_BIN_DIR") {
        let trimmed = driver_bin_dir.trim();
        if !trimmed.is_empty() {
            roots.push(PathBuf::from(trimmed));
        }
    }

    // 優先順③: 実行バイナリ配置場所（resources / app directory）
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            roots.push(exe_dir.join("resources"));
            if let Some(parent) = exe_dir.parent() {
                roots.push(parent.join("Resources"));
            }

            roots.push(exe_dir.to_path_buf());
        }
    }

    // 優先順④: 後方互換のための祖先探索（current_dir / exe_dir）
    if let Ok(current_dir) = std::env::current_dir() {
        let mut cursor = Some(current_dir.as_path());
        while let Some(path) = cursor {
            roots.push(path.to_path_buf());
            cursor = path.parent();
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let mut cursor = exe_dir.parent();
            while let Some(path) = cursor {
                roots.push(path.to_path_buf());
                cursor = path.parent();
            }
        }
    }

    let mut unique = Vec::<PathBuf>::new();
    for path in roots {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    unique
}

fn path_to_string(path: PathBuf) -> String {
    match path.canonicalize() {
        Ok(canonical) => canonical.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

pub(in crate::commands::driver) fn normalize_optional_string(
    value: Option<String>,
) -> Option<String> {
    crate::commands::util::normalize_optional_string(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "kt_iot_hub_ui_launcher_{}_{}",
            name,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn find_default_driver_ui_path_prefers_manifest_registration_ui() {
        let root = unique_temp_dir("manifest_preferred");
        let root_str = root.to_string_lossy().to_string();
        let manifest_dir = root.join("driver-ui").join("postgres");
        let manifest_path = manifest_dir.join("driver-manifest.json");
        let registration_ui_path = manifest_dir.join("registration-ui.exe");
        let runtime_path = manifest_dir.join("driver-postgres.exe");
        let legacy_ui_path = root
            .join("ops")
            .join("driver-ui")
            .join("postgres")
            .join("driver-ui.exe");

        fs::create_dir_all(&manifest_dir).unwrap();
        fs::create_dir_all(legacy_ui_path.parent().unwrap()).unwrap();
        fs::write(&registration_ui_path, b"test").unwrap();
        fs::write(&runtime_path, b"test").unwrap();
        fs::write(&legacy_ui_path, b"legacy").unwrap();
        fs::write(
            &manifest_path,
            r#"{
  "manifestVersion": 1,
  "driverType": "postgres",
  "displayName": "PostgreSQL 接続",
  "registrationUi": "registration-ui.exe",
  "runtime": "driver-postgres.exe",
  "protocol": {
    "driverUiRequestVersion": "1",
    "driverUiResponseVersion": "1"
  }
}"#,
        )
        .unwrap();

        let resolved = find_default_driver_ui_path("postgres", Some(&root_str));
        assert_eq!(resolved, Some(path_to_string(registration_ui_path.clone())));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn find_default_driver_ui_path_falls_back_when_manifest_ui_missing() {
        let root = unique_temp_dir("manifest_fallback");
        let root_str = root.to_string_lossy().to_string();
        let manifest_dir = root.join("driver-ui").join("postgres");
        let manifest_path = manifest_dir.join("driver-manifest.json");
        let runtime_path = manifest_dir.join("driver-postgres.exe");
        let legacy_ui_path = root
            .join("ops")
            .join("driver-ui")
            .join("postgres")
            .join("registration-ui.exe");

        fs::create_dir_all(&manifest_dir).unwrap();
        fs::create_dir_all(legacy_ui_path.parent().unwrap()).unwrap();
        fs::write(&runtime_path, b"test").unwrap();
        fs::write(&legacy_ui_path, b"legacy").unwrap();
        fs::write(
            &manifest_path,
            r#"{
  "manifestVersion": 1,
  "driverType": "postgres",
  "displayName": "PostgreSQL 接続",
  "registrationUi": "registration-ui.exe",
  "runtime": "driver-postgres.exe",
  "protocol": {
    "driverUiRequestVersion": "1",
    "driverUiResponseVersion": "1"
  }
}"#,
        )
        .unwrap();

        let resolved = find_default_driver_ui_path("postgres", Some(&root_str));
        assert_eq!(resolved, Some(path_to_string(legacy_ui_path.clone())));

        let _ = fs::remove_dir_all(&root);
    }
}
