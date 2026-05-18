// タグ定義と値型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// タグの一意識別子 (newtype for type safety)
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct TagId(pub String);

/// データ型の列挙
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "i32")]
    I32,
    #[serde(rename = "i64")]
    I64,
    #[serde(rename = "f32")]
    F32,
    #[serde(rename = "f64")]
    F64,
    #[serde(rename = "string")]
    String,
}

impl DataType {
    /// JSON スキーマ型文字列を返す（UI 検証用）
    #[allow(dead_code)]
    pub fn json_schema_type(&self) -> &'static str {
        match self {
            DataType::Bool => "boolean",
            DataType::I32 | DataType::I64 => "integer",
            DataType::F32 | DataType::F64 => "number",
            DataType::String => "string",
        }
    }
}

impl DataType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::Bool => "bool",
            DataType::I32 => "i32",
            DataType::I64 => "i64",
            DataType::F32 => "f32",
            DataType::F64 => "f64",
            DataType::String => "string",
        }
    }
}

impl FromStr for DataType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "bool" => Ok(DataType::Bool),
            "i32" => Ok(DataType::I32),
            "i64" => Ok(DataType::I64),
            "f32" => Ok(DataType::F32),
            "f64" => Ok(DataType::F64),
            "string" => Ok(DataType::String),
            _ => Err(anyhow::anyhow!("Unsupported data_type: {}", s)),
        }
    }
}

/// 品質フラグ
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    /// 正常な値
    Good,
    /// 値は古い（タイムアウト等）
    Uncertain,
    /// エラーまたは無効な値
    Bad,
}

/// スキャングループ（テーブル/デバイス単位の周期管理単位）
/// 同一グループのタグは1クエリでまとめて取得される
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanGroup {
    /// グループの一意識別子
    pub id: String,
    /// このグループを管理するドライバID
    pub driver_id: String,
    /// スキャン周期（ミリ秒）
    pub scan_rate_ms: u32,
    /// ドライバ固有のグループ設定
    /// - PostgreSQL: { "table": "sensors", "timestamp_column": "measured_at" }
    /// - JoyWatcher: { "node": "PLC1" }  （将来用）
    pub spec: serde_json::Value,
}

/// タグの定義（設定時点での静的メタデータ）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    /// タグの一意識別子
    pub id: TagId,
    /// ユーザー表示名
    pub name: String,
    /// データ型
    pub data_type: DataType,
    /// このタグを供給するドライバID
    pub driver_id: String,
    /// 所属するスキャングループID
    pub scan_group_id: String,
    /// ドライバ固有の読み出し設定
    /// - PostgreSQL: { "value_column": "temperature" }
    /// - JoyWatcher: { "tag_path": "Line1/Tank/Level" }  （将来用）
    pub driver_spec: serde_json::Value,
    /// メタデータ（追加情報用JSON）
    pub metadata: Option<serde_json::Value>,
}

/// タグの値（実行時のデータポイント）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagValue {
    /// タグID
    pub tag_id: TagId,
    /// 値（DataType に対応したJSON値）
    pub value: serde_json::Value,
    /// 品質
    pub quality: Quality,
    /// タイムスタンプ（UTC）
    pub timestamp: DateTime<Utc>,
}

impl TagValue {
    /// 新規 TagValue を生成
    #[allow(dead_code)]
    pub fn new(tag_id: TagId, value: serde_json::Value, quality: Quality) -> Self {
        Self {
            tag_id,
            value,
            quality,
            timestamp: Utc::now(),
        }
    }

    /// Good 品質の値を生成
    #[allow(dead_code)]
    pub fn good(tag_id: TagId, value: serde_json::Value) -> Self {
        Self::new(tag_id, value, Quality::Good)
    }

    /// Bad 品質の値を生成（エラー時用）
    #[allow(dead_code)]
    pub fn bad(tag_id: TagId, value: serde_json::Value) -> Self {
        Self::new(tag_id, value, Quality::Bad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datatype_schema() {
        assert_eq!(DataType::Bool.json_schema_type(), "boolean");
        assert_eq!(DataType::I32.json_schema_type(), "integer");
        assert_eq!(DataType::F32.json_schema_type(), "number");
        assert_eq!(DataType::String.json_schema_type(), "string");
    }

    #[test]
    fn test_tag_value_creation() {
        let tag_id = TagId("test-001".to_string());
        let value = serde_json::json!(42.5);
        let tv = TagValue::good(tag_id.clone(), value);
        assert_eq!(tv.tag_id, tag_id);
        assert_eq!(tv.quality, Quality::Good);
    }
}
