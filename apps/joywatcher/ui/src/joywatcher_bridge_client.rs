use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const BRIDGE_EXE_NAME: &str = "joywatcher-bridge-x86.exe";
const DLL_FILE_NAME: &str = "JoyWaApi.dll";

pub fn resolve_single_tag(
    endpoint: &str,
    user_id: i32,
    password: &str,
    tag_path: &str,
) -> Result<i32, String> {
    let mut bridge = JoyWatcherUiBridgeClient::start()?;
    bridge.ping()?;
    bridge.connect(endpoint, user_id, password)?;
    bridge.resolve_tag(tag_path)
}

pub fn browse_tags(endpoint: &str, user_id: i32, password: &str) -> Result<Vec<String>, String> {
    let mut bridge = JoyWatcherUiBridgeClient::start()?;
    bridge.ping()?;
    bridge.connect(endpoint, user_id, password)?;
    bridge.browse_tags()
}

struct JoyWatcherUiBridgeClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    connected: bool,
}

impl JoyWatcherUiBridgeClient {
    fn start() -> Result<Self, String> {
        let exe_path = resolve_bridge_exe_path()
            .ok_or_else(|| format!("JoyWatcher bridge executable not found: {BRIDGE_EXE_NAME}"))?;
        let dll_path = resolve_dll_path()
            .ok_or_else(|| format!("JoyWaApi.dll not found in known search roots: {DLL_FILE_NAME}"))?;

        let mut command = Command::new(&exe_path);
        command
            .arg("--mode")
            .arg("dll")
            .arg("--dll-path")
            .arg(&dll_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = command
            .spawn()
            .map_err(|e| format!("failed to spawn JoyWatcher bridge '{}': {e}", exe_path.display()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to capture JoyWatcher bridge stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "failed to capture JoyWatcher bridge stdout".to_string())?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            connected: false,
        })
    }

    fn ping(&mut self) -> Result<(), String> {
        let response = self.send_request(r#"{"type":"ping"}"#)?;
        if !response.contains(r#""type":"pong""#) {
            return Err(format!("unexpected bridge ping response: {response}"));
        }
        Ok(())
    }

    fn connect(&mut self, endpoint: &str, user_id: i32, password: &str) -> Result<(), String> {
        let response = self.send_request(&format!(
            "{{\"type\":\"connect\",\"endpoint\":\"{}\",\"user_id\":{},\"password\":\"{}\"}}",
            escape_json(endpoint),
            user_id,
            escape_json(password)
        ))?;
        if !response.contains(r#""type":"connected""#) {
            return Err(describe_bridge_response("connect", &response));
        }
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), String> {
        let response = self.send_request(r#"{"type":"disconnect"}"#)?;
        if !response.contains(r#""type":"disconnected""#) {
            return Err(describe_bridge_response("disconnect", &response));
        }
        self.connected = false;
        Ok(())
    }

    fn resolve_tag(&mut self, tag_path: &str) -> Result<i32, String> {
        let response = self.send_request(&format!(
            "{{\"type\":\"resolveTags\",\"tags\":[\"{}\"]}}",
            escape_json(tag_path)
        ))?;

        if !response.contains(r#""type":"resolvedTags""#) {
            return Err(describe_bridge_response("resolveTags", &response));
        }

        extract_i32_field(&response, r#""tagId":"#)
            .ok_or_else(|| format!("tagId not found in bridge response: {response}"))
    }

    fn browse_tags(&mut self) -> Result<Vec<String>, String> {
        let response = self.send_request(r#"{"type":"browseTags"}"#)?;

        if !response.contains(r#""type":"browsedTags""#) {
            return Err(describe_bridge_response("browseTags", &response));
        }

        extract_string_array_field(&response, r#""items":["#)
            .ok_or_else(|| format!("items not found in bridge response: {response}"))
    }

    fn send_request(&mut self, request: &str) -> Result<String, String> {
        writeln!(self.stdin, "{request}")
            .map_err(|e| format!("failed to write bridge request: {e}"))?;
        self.stdin
            .flush()
            .map_err(|e| format!("failed to flush bridge stdin: {e}"))?;

        loop {
            let mut line = String::new();
            let bytes = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| format!("failed to read bridge response: {e}"))?;
            if bytes == 0 {
                return Err("bridge returned EOF before responding".to_string());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                continue;
            }

            return Ok(trimmed.to_string());
        }
    }
}

impl Drop for JoyWatcherUiBridgeClient {
    fn drop(&mut self) {
        if self.connected {
            let _ = self.disconnect();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn describe_bridge_response(action: &str, response: &str) -> String {
    if let Some(message) = extract_string_field(response, r#""message":"#) {
        return format!("bridge {action} failed: {message}");
    }
    format!("unexpected bridge {action} response: {response}")
}

fn extract_i32_field(text: &str, key: &str) -> Option<i32> {
    let start = text.find(key)? + key.len();
    let value = &text[start..];
    let end = value
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(value.len());
    value[..end].parse().ok()
}

fn extract_string_field(text: &str, key: &str) -> Option<String> {
    let start = text.find(key)? + key.len();
    let value = text[start..].strip_prefix('"').unwrap_or(&text[start..]);
    let end = value.find('"')?;
    Some(value[..end].replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn extract_string_array_field(text: &str, key: &str) -> Option<Vec<String>> {
    let start = text.find(key)? + key.len();
    let end = text[start..].find(']')? + start;
    let body = &text[start..end];

    if body.is_empty() {
        return Some(Vec::new());
    }

    Some(
        body.split("\",\"")
            .map(|item| item.trim_matches('"').replace("\\\"", "\"").replace("\\\\", "\\"))
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn resolve_bridge_exe_path() -> Option<PathBuf> {
    bridge_exe_candidates().into_iter().find(|path| path.exists())
}

fn resolve_dll_path() -> Option<PathBuf> {
    dll_candidates().into_iter().find(|path| path.exists())
}

fn bridge_exe_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Ok(explicit_path) = std::env::var("JOYWATCHER_BRIDGE_EXE") {
        push_unique(&mut candidates, PathBuf::from(explicit_path));
    }

    if let Ok(bridge_dir) = std::env::var("JOYWATCHER_BRIDGE_DIR") {
        push_unique(&mut candidates, PathBuf::from(bridge_dir).join(BRIDGE_EXE_NAME));
    }

    if let Some(repo_root) = find_repo_root() {
        push_unique(
            &mut candidates,
            repo_root.join("driver-ui").join("joywatcher").join(BRIDGE_EXE_NAME),
        );
        push_unique(
            &mut candidates,
            repo_root
                .join("target")
                .join("i686-pc-windows-msvc")
                .join("debug")
                .join(BRIDGE_EXE_NAME),
        );
        push_unique(
            &mut candidates,
            repo_root
                .join("target")
                .join("i686-pc-windows-msvc")
                .join("release")
                .join(BRIDGE_EXE_NAME),
        );
    }

    if let Ok(current_dir) = std::env::current_dir() {
        push_unique(&mut candidates, current_dir.join(BRIDGE_EXE_NAME));
        if let Some(parent) = current_dir.parent() {
            push_unique(&mut candidates, parent.join(BRIDGE_EXE_NAME));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            push_unique(&mut candidates, exe_dir.join(BRIDGE_EXE_NAME));
            if let Some(parent) = exe_dir.parent() {
                push_unique(&mut candidates, parent.join(BRIDGE_EXE_NAME));
            }
        }
    }

    candidates
}

fn dll_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Ok(configured_path) = std::env::var("JOYWATCHER_DLL_PATH") {
        push_unique(&mut candidates, PathBuf::from(configured_path));
    }

    if let Ok(configured_dir) = std::env::var("JOYWATCHER_DLL_DIR") {
        push_unique(&mut candidates, PathBuf::from(configured_dir).join(DLL_FILE_NAME));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        push_unique(&mut candidates, current_dir.join(DLL_FILE_NAME));
        if let Some(parent) = current_dir.parent() {
            push_unique(&mut candidates, parent.join(DLL_FILE_NAME));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            push_unique(&mut candidates, exe_dir.join(DLL_FILE_NAME));
            if let Some(parent) = exe_dir.parent() {
                push_unique(&mut candidates, parent.join(DLL_FILE_NAME));
            }
        }
    }

    if let Some(repo_root) = find_repo_root() {
        push_unique(&mut candidates, repo_root.join("参考").join(DLL_FILE_NAME));
        push_unique(
            &mut candidates,
            repo_root.join("参考").join("JoyWaApi").join(DLL_FILE_NAME),
        );
        push_unique(
            &mut candidates,
            repo_root.join("参考").join("JoyWaApi").join("BC").join(DLL_FILE_NAME),
        );
    }

    if let Ok(windir) = std::env::var("WINDIR") {
        push_unique(
            &mut candidates,
            PathBuf::from(windir).join("SysWOW64").join(DLL_FILE_NAME),
        );
    }

    candidates
}

fn find_repo_root() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    find_ancestor_with(&current_dir, |dir| dir.join("Cargo.toml").exists())
}

fn find_ancestor_with(start: &Path, predicate: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut cursor = Some(start);
    while let Some(path) = cursor {
        if predicate(path) {
            return Some(path.to_path_buf());
        }
        cursor = path.parent();
    }
    None
}

fn push_unique(vec: &mut Vec<PathBuf>, path: PathBuf) {
    if !vec.contains(&path) {
        vec.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tag_id_from_resolved_response() {
        let response = r#"{"type":"resolvedTags","items":[{"tagPath":"Line1/Tank/Level","tagId":101}]}"#;
        assert_eq!(extract_i32_field(response, r#""tagId":"#), Some(101));
    }

    #[test]
    fn extract_bridge_error_message() {
        let response = r#"{"type":"error","code":"X","message":"resolve failed"}"#;
        assert_eq!(
            describe_bridge_response("resolveTags", response),
            "bridge resolveTags failed: resolve failed"
        );
    }

    #[test]
    fn extract_string_array_field_reads_items() {
        let response = r#"{"type":"browsedTags","items":["Line1/Tank/Level","Line1/Tank/Temp"]}"#;
        assert_eq!(
            extract_string_array_field(response, r#""items":["#),
            Some(vec![
                "Line1/Tank/Level".to_string(),
                "Line1/Tank/Temp".to_string()
            ])
        );
    }
}
