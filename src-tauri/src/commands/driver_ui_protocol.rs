use serde::{Deserialize, Serialize};

/// 外部ドライバUIから本体へ返却される JSON ペイロード（初期版）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverUiImportPayload {
    #[serde(default)]
    pub schema_version: Option<u32>,
  pub driver: DriverUiDriverPayload,
    #[serde(default)]
    pub tags: Vec<DriverUiTagPayload>,
    #[serde(default)]
    pub scan_groups: Vec<DriverUiScanGroupPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverUiTagPayload {
    pub id: String,
    pub name: String,
    #[serde(alias = "dataType")]
    pub data_type: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(alias = "driverSpec")]
    pub driver_spec: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverUiScanGroupPayload {
    pub id: String,
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default)]
    pub timestamp_column: Option<String>,
    #[serde(default)]
    pub scan_rate_ms: Option<u32>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub node: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverUiDriverPayload {
    pub id: String,
  pub driver_type: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

  /// 本体からドライバUIへ渡す初期コンテキスト JSON
  #[derive(Debug, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DriverUiLaunchContext {
    pub schema_version: u32,
    pub request_id: String,
    pub generated_at: String,
    pub direction: String,
    pub session: DriverUiLaunchSession,
    pub driver: DriverUiLaunchDriver,
    pub context: DriverUiLaunchData,
  }

  #[derive(Debug, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DriverUiLaunchSession {
    pub session_id: String,
    pub mode: String,
    pub output_json_path: String,
  }

  #[derive(Debug, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DriverUiLaunchDriver {
    pub driver_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_id: Option<String>,
  }

  #[derive(Debug, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DriverUiLaunchData {
    pub scan_groups: Vec<DriverUiLaunchScanGroup>,
    pub tags: Vec<DriverUiLaunchTag>,
  }

  #[derive(Debug, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DriverUiLaunchScanGroup {
    pub id: String,
    pub driver: String,
    pub scan_rate_ms: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_column: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
  }

  #[derive(Debug, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DriverUiLaunchTag {
    pub id: String,
    pub name: String,
    pub data_type: String,
    pub driver_id: String,
    pub scan_group_id: String,
    pub enabled: bool,
    pub driver_spec: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
  }

#[cfg(test)]
mod tests {
    use super::DriverUiImportPayload;

    #[test]
    fn parse_driver_ui_payload_with_driver_block() {
        let json = r#"{
          "schemaVersion": 1,
          "driver": {
            "id": "postgres-main",
            "driverType": "postgres",
            "enabled": true,
            "settings": {
              "host": "localhost",
              "port": 5432
            }
          },
          "scanGroups": [
            { "id": "line1_1000ms", "scanRateMs": 1000, "table": "line1", "timestampColumn": "measured_at" }
          ],
          "tags": [
            {
              "id": "tag-0001",
              "name": "temperature",
              "dataType": "f32",
              "driverSpec": {
                "kind": "postgres",
                "scanGroup": "line1_1000ms",
                "valueColumn": "temperature"
              }
            }
          ]
        }"#;

        let payload: DriverUiImportPayload = serde_json::from_str(json).expect("payload parse");
        assert_eq!(payload.schema_version, Some(1));
        assert_eq!(payload.driver.id, "postgres-main");
        assert_eq!(payload.scan_groups.len(), 1);
        assert_eq!(payload.tags.len(), 1);
    }
}
