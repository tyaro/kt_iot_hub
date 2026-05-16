//! ドライバUI 実行ファイルのパス解決と、共通の文字列正規化ヘルパ。

use crate::config::DriverConfig;
use std::path::PathBuf;

/// 単一ドライバ設定から登録UI 実行ファイルの絶対パスを解決する。
pub(super) fn resolve_driver_ui_path(config: &DriverConfig) -> Option<String> {
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
        for file_name in &file_names {
            // <root>/driver-ui/<type>/<file>
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

fn app_root_candidates(driver_ui_base_dir: Option<&str>) -> Vec<PathBuf> {
    let mut roots = Vec::<PathBuf>::new();

    if let Some(base_dir) = driver_ui_base_dir {
        let trimmed = base_dir.trim();
        if !trimmed.is_empty() {
            roots.push(PathBuf::from(trimmed));
        }
    }

    // current_dir から祖先を辿って候補にする (tauri dev の cwd 差異に備える)
    if let Ok(current_dir) = std::env::current_dir() {
        let mut cursor = Some(current_dir.as_path());
        while let Some(path) = cursor {
            roots.push(path.to_path_buf());
            cursor = path.parent();
        }
    }

    // 実行バイナリ配置場所から祖先を辿って候補にする
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let mut cursor = Some(exe_dir);
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

/// 任意文字列の空白除去と空判定。`None` / 空文字を一律 `None` に正規化する。
pub(super) fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
