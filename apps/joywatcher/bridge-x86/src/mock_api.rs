use std::collections::HashMap;

use anyhow::Result;

use crate::connection::JoyWatcherBridgeApi;
use crate::protocol::{MockValue, ReadValuePayload, ResolvedTag};

#[derive(Debug, Default)]
pub struct MockJoyWatcherApi {
    known_tags: HashMap<String, i32>,
    next_tag_id: i32,
}

impl MockJoyWatcherApi {
    pub fn resolve_tags(&mut self, tags: &[String]) -> Vec<ResolvedTag> {
        tags.iter()
            .map(|tag_path| {
                let tag_id = *self.known_tags.entry(tag_path.clone()).or_insert_with(|| {
                    self.next_tag_id += 1;
                    1000 + self.next_tag_id
                });
                ResolvedTag {
                    tag_path: tag_path.clone(),
                    tag_id,
                }
            })
            .collect()
    }

    pub fn read_tags(&self, tag_ids: &[i32]) -> Vec<ReadValuePayload> {
        tag_ids
            .iter()
            .map(|tag_id| ReadValuePayload {
                tag_id: *tag_id,
                quality: "good".to_string(),
                value: MockValue::Number(*tag_id as f64 / 10.0),
            })
            .collect()
    }
}

impl JoyWatcherBridgeApi for MockJoyWatcherApi {
    fn mode(&self) -> &'static str {
        "mock"
    }

    fn connect_net(&mut self) -> Result<()> {
        Ok(())
    }

    fn disconnect_net(&mut self) -> Result<()> {
        Ok(())
    }

    fn disconnect_net_force(&mut self) -> Result<()> {
        Ok(())
    }

    fn resolve_tags(&mut self, tags: &[String]) -> Result<Vec<ResolvedTag>> {
        Ok(self.resolve_tags(tags))
    }

    fn read_tags(&self, tag_ids: &[i32]) -> Result<Vec<ReadValuePayload>> {
        Ok(self.read_tags(tag_ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_tags_reuses_same_id_for_same_path() {
        let mut api = MockJoyWatcherApi::default();
        let tags = vec!["Line1/Tank/Level".to_string()];

        let first = api.resolve_tags(&tags);
        let second = api.resolve_tags(&tags);

        assert_eq!(first[0].tag_id, second[0].tag_id);
    }

    #[test]
    fn read_tags_returns_mock_numeric_values() {
        let api = MockJoyWatcherApi::default();
        let values = api.read_tags(&[1010, 1011]);

        assert_eq!(values.len(), 2);
        assert_eq!(values[0].quality, "good");
    }
}
