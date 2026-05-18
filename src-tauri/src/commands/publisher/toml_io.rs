//! publishers.toml のアトミック書き出し処理。

use crate::commands::config_io::write_toml_atomic;
use crate::commands::dto::ErrorResponse;
use crate::config::PublisherConfig;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct PublishersTomlFile {
    #[serde(rename = "publisher")]
    pub publisher: Vec<PublisherConfig>,
}

pub(super) fn write_publishers_toml_atomic(
    publishers: &[PublisherConfig],
) -> Result<(), ErrorResponse> {
    write_toml_atomic(
        "publishers.toml",
        &PublishersTomlFile {
            publisher: publishers.to_vec(),
        },
    )
}
