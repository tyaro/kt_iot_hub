/// 任意文字列の空白除去と空判定。`None` / 空文字を一律 `None` に正規化する。
pub(crate) fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
