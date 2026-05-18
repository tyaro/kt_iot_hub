use anyhow::{anyhow, Result};

use super::protocol::{escape_json_string, parse_read_result};
use super::{BridgeConnectionSettings, BridgeReadValue, JoyWatcherBridgeProcess};

impl JoyWatcherBridgeProcess {
    pub fn ping(&mut self) -> Result<String> {
        let response = self.send_request(r#"{"type":"ping"}"#)?;
        if !response.contains(r#""type":"pong""#) {
            return Err(anyhow!("unexpected bridge ping response: {}", response));
        }
        Ok(response)
    }

    pub fn ensure_connection(&mut self, settings: &BridgeConnectionSettings) -> Result<()> {
        if self.connected && self.connection.as_ref() == Some(settings) {
            return Ok(());
        }

        if self.connected {
            self.disconnect()?;
        }

        self.connect(settings)?;
        Ok(())
    }

    pub fn connect(&mut self, settings: &BridgeConnectionSettings) -> Result<String> {
        let mut request = String::from("{\"type\":\"connect\"");
        if let Some(endpoint) = settings.endpoint.as_deref() {
            request.push_str(&format!(",\"endpoint\":\"{}\"", escape_json_string(endpoint)));
        }
        request.push_str(&format!(",\"user_id\":{}", settings.user_id));
        request.push_str(&format!(
            ",\"password\":\"{}\"}}",
            escape_json_string(&settings.password)
        ));

        let response = self.send_request(&request)?;
        if !response.contains(r#""type":"connected""#) {
            return Err(anyhow!("unexpected bridge connect response: {}", response));
        }
        self.connected = true;
        self.connection = Some(settings.clone());
        Ok(response)
    }

    pub fn disconnect(&mut self) -> Result<String> {
        let response = self.send_request(r#"{"type":"disconnect"}"#)?;
        if !response.contains(r#""type":"disconnected""#) {
            return Err(anyhow!("unexpected bridge disconnect response: {}", response));
        }
        self.connected = false;
        self.connection = None;
        Ok(response)
    }

    pub fn read_tags(&mut self, tag_ids: &[i32]) -> Result<Vec<BridgeReadValue>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }

        let request = format!(
            r#"{{"type":"read","request_id":"joywatcher-read","tag_ids":[{}]}}"#,
            tag_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        let response = self.send_request(&request)?;
        parse_read_result(&response)
    }
}
