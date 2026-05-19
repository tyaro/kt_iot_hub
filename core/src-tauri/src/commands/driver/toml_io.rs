//! drivers.toml / tags.toml のアトミック書き出し処理。
//! 一時ファイルへ書いてから rename で差し替える方式で、失敗時は通常 write へフォールバックする。

use crate::commands::config_io::write_toml_atomic;
use crate::commands::dto::ErrorResponse;
use crate::config::{DriverConfig, ScanGroupConfig, TagConfig};
use serde::Serialize;

#[derive(Serialize)]
struct TagsTomlFile {
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

pub(crate) fn write_tags_toml_atomic(
    scan_groups: &[ScanGroupConfig],
    tags: &[TagConfig],
) -> Result<(), ErrorResponse> {
    write_toml_atomic(
        "tags.toml",
        &TagsTomlFile {
            scan_group: scan_groups.to_vec(),
            tag: tags.to_vec(),
        },
    )
}

pub(super) fn write_drivers_toml_atomic(drivers: &[DriverConfig]) -> Result<(), ErrorResponse> {
    write_toml_atomic(
        "drivers.toml",
        &DriversTomlFile {
            driver: drivers.to_vec(),
        },
    )
}
