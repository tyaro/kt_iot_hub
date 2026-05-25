use crate::commands::dto::ErrorResponse;

const KEYRING_SERVICE_NAME: &str = "kt_iot_hub";

pub(crate) fn default_password_key(scope: &str, entity_id: &str) -> String {
    format!("{}/{}/password", scope.trim(), entity_id.trim())
}

pub(crate) fn read_password_from_keyring(password_key: &str) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE_NAME, password_key).ok()?;
    entry.get_password().ok()
}

pub(crate) fn write_password_to_keyring(
    password_key: &str,
    password: &str,
) -> Result<(), ErrorResponse> {
    let entry = keyring::Entry::new(KEYRING_SERVICE_NAME, password_key).map_err(|e| {
        ErrorResponse::new(
            "KEYRING_ERROR",
            format!("Failed to open secret store for {}: {}", password_key, e),
        )
    })?;
    entry.set_password(password).map_err(|e| {
        ErrorResponse::new(
            "KEYRING_ERROR",
            format!("Failed to store secret for {}: {}", password_key, e),
        )
    })
}

pub(crate) fn delete_password_from_keyring(password_key: &str) {
    let _ = password_key;
}

pub(crate) fn read_password_setting(settings: &serde_json::Value) -> Option<String> {
    let password_key = settings
        .get("password_key")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty());

    if let Some(password_key) = password_key {
        if let Some(password) = read_password_from_keyring(password_key) {
            return Some(password);
        }
    }

    settings
        .get("password")
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

pub(crate) fn read_password_setting_with_override(
    settings: &serde_json::Value,
    explicit_password_key: Option<&str>,
) -> Option<String> {
    if let Some(explicit_password_key) =
        explicit_password_key.filter(|value| !value.trim().is_empty())
    {
        if let Some(password) = read_password_from_keyring(explicit_password_key) {
            return Some(password);
        }
    }

    read_password_setting(settings)
}
