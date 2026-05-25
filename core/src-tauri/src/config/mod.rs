// TOML ベースの設定管理
// tags.toml, drivers.toml, publishers.toml, runtime.toml を読み込む

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

/// タグ定義の TOML スキーマ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagConfig {
    pub id: String,
    pub name: String,
    pub data_type: String,
    /// 使用するドライバID（drivers.toml の id と一致）
    pub driver: String,
    /// 所属するスキャングループID（tags.toml の [[scan_group]].id と一致）
    pub scan_group: String,
    /// ドライバ固有の読み出し設定
    /// - PostgreSQL: { "value_column": "temperature" }
    /// - JoyWatcher: { "tag_path": "Line1/Tank/Level" }  （将来用）
    pub driver_spec: serde_json::Value,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// スキャングループ設定（テーブル/デバイス単位の周期管理）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanGroupConfig {
    pub id: String,
    /// 使用するドライバID
    pub driver: String,
    /// スキャン周期（ミリ秒）
    pub scan_rate_ms: u32,
    /// PostgreSQL: スキーマ名（省略時は接続既定スキーマ）
    pub schema: Option<String>,
    /// PostgreSQL: 読み出しテーブル名
    pub table: Option<String>,
    /// PostgreSQL: タイムスタンプカラム名（最新行取得に使用）
    pub timestamp_column: Option<String>,
    /// JoyWatcher/SLMP: デバイスノード（将来用）
    pub node: Option<String>,
}

/// ドライバ設定の TOML スキーマ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverConfig {
    pub id: String,
    pub driver_type: String,
    pub enabled: Option<bool>,
    #[serde(flatten)]
    pub settings: serde_json::Value,
}

/// パブリッシャ設定の TOML スキーマ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherConfig {
    pub id: String,
    pub publisher_type: String,
    pub enabled: Option<bool>,
    #[serde(flatten)]
    pub settings: serde_json::Value,
}

/// ランタイム設定の TOML スキーマ
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub auto_start_runtime_services: bool,
}

/// アプリ全体の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub tags: Vec<TagConfig>,
    pub scan_groups: Vec<ScanGroupConfig>,
    pub drivers: Vec<DriverConfig>,
    pub publishers: Vec<PublisherConfig>,
    pub runtime: RuntimeConfig,
}

impl AppConfig {
    /// 設定ファイルを読み込む
    pub fn load_from_files<P: AsRef<Path>>(config_dir: P) -> Result<Self> {
        let config_dir = config_dir.as_ref();

        let tags_path = config_dir.join("tags.toml");
        let drivers_path = config_dir.join("drivers.toml");
        let publishers_path = config_dir.join("publishers.toml");
        let runtime_path = config_dir.join("runtime.toml");

        // tags.toml から [[tag]] と [[scan_group]] を読み込む
        let (tags, scan_groups) = if tags_path.exists() {
            let content = std::fs::read_to_string(&tags_path)?;
            let data: toml::Value = toml::from_str(&content)?;

            let tags = data
                .get("tag")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|v| {
                    serde_json::from_value::<TagConfig>(
                        serde_json::to_value(v)
                            .map_err(|e| anyhow!("Failed to convert TOML to JSON: {}", e))?,
                    )
                    .map_err(|e| anyhow!("Failed to deserialize TagConfig: {}", e))
                })
                .collect::<Result<Vec<_>>>()?;

            let scan_groups = data
                .get("scan_group")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|v| {
                    serde_json::from_value::<ScanGroupConfig>(
                        serde_json::to_value(v)
                            .map_err(|e| anyhow!("Failed to convert TOML to JSON: {}", e))?,
                    )
                    .map_err(|e| anyhow!("Failed to deserialize ScanGroupConfig: {}", e))
                })
                .collect::<Result<Vec<_>>>()?;

            (tags, scan_groups)
        } else {
            info!("tags.toml not found, starting with empty list");
            (vec![], vec![])
        };

        let drivers = if drivers_path.exists() {
            let content = std::fs::read_to_string(&drivers_path)?;
            let data: toml::Value = toml::from_str(&content)?;
            data.get("driver")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|v| {
                    serde_json::from_value::<DriverConfig>(
                        serde_json::to_value(v)
                            .map_err(|e| anyhow!("Failed to convert TOML to JSON: {}", e))?,
                    )
                    .map_err(|e| anyhow!("Failed to deserialize DriverConfig: {}", e))
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            info!("drivers.toml not found");
            vec![]
        };

        let publishers = if publishers_path.exists() {
            let content = std::fs::read_to_string(&publishers_path)?;
            let data: toml::Value = toml::from_str(&content)?;
            data.get("publisher")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|v| {
                    serde_json::from_value::<PublisherConfig>(
                        serde_json::to_value(v)
                            .map_err(|e| anyhow!("Failed to convert TOML to JSON: {}", e))?,
                    )
                    .map_err(|e| anyhow!("Failed to deserialize PublisherConfig: {}", e))
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            info!("publishers.toml not found");
            vec![]
        };

        let runtime = if runtime_path.exists() {
            let content = std::fs::read_to_string(&runtime_path)?;
            toml::from_str::<RuntimeConfig>(&content)
                .map_err(|e| anyhow!("Failed to deserialize RuntimeConfig: {}", e))?
        } else {
            // 互換維持: runtime.toml が未導入の既存環境では publishers.toml の enabled を継承する。
            RuntimeConfig {
                auto_start_runtime_services: publishers
                    .iter()
                    .any(|cfg| cfg.enabled.unwrap_or(false)),
            }
        };

        info!(
            "Loaded {} tags, {} scan_groups, {} drivers, {} publishers (runtime.auto_start_runtime_services={})",
            tags.len(),
            scan_groups.len(),
            drivers.len(),
            publishers.len(),
            runtime.auto_start_runtime_services
        );

        Ok(Self {
            tags,
            scan_groups,
            drivers,
            publishers,
            runtime,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_config_deserialize() {
        let toml_str = r#"
id = "test-001"
name = "TestTag"
data_type = "f32"
driver = "postgres-main"
scan_group = "sensors-fast"
driver_spec = { value_column = "temperature" }
"#;
        let config: TagConfig = toml::from_str(toml_str).expect("parse failed");
        assert_eq!(config.scan_group, "sensors-fast");
        assert_eq!(
            config
                .driver_spec
                .get("value_column")
                .and_then(|v| v.as_str()),
            Some("temperature")
        );
    }
}
