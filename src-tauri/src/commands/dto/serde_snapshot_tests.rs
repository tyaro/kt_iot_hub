use super::{DriverDto, LaunchDriverUiResponse, SaveDriverRequest, TagPayload};
use serde_json::json;

#[test]
fn driver_dto_serialization_keeps_existing_snake_case_keys() {
    let dto = DriverDto {
        id: "drv-1".to_string(),
        driver_type: "postgres".to_string(),
        enabled: true,
        registration_ui_available: false,
        host: "localhost".to_string(),
        port: 5432,
        database: "iot".to_string(),
        username: "user".to_string(),
    };

    let value = serde_json::to_value(dto).expect("serialize DriverDto");
    assert_eq!(
        value,
        json!({
            "id": "drv-1",
            "driver_type": "postgres",
            "enabled": true,
            "registration_ui_available": false,
            "host": "localhost",
            "port": 5432,
            "database": "iot",
            "username": "user"
        })
    );
}

#[test]
fn launch_driver_ui_response_serialization_keeps_existing_snake_case_keys() {
    let dto = LaunchDriverUiResponse {
        session_id: "session-1".to_string(),
        output_json_path: "C:/tmp/out.json".to_string(),
        driver_id: Some("drv-1".to_string()),
        driver_type: "postgres".to_string(),
    };

    let value = serde_json::to_value(dto).expect("serialize LaunchDriverUiResponse");
    assert_eq!(
        value,
        json!({
            "session_id": "session-1",
            "output_json_path": "C:/tmp/out.json",
            "driver_id": "drv-1",
            "driver_type": "postgres"
        })
    );
}

#[test]
fn tag_payload_serialization_stays_camel_case() {
    let dto = TagPayload {
        id: "tag-1".to_string(),
        name: "Tag 1".to_string(),
        data_type: "number".to_string(),
        driver_id: "drv-1".to_string(),
        scan_group_id: "sg-1".to_string(),
        driver_spec: json!({"key": "value"}),
    };

    let value = serde_json::to_value(dto).expect("serialize TagPayload");
    assert_eq!(
        value,
        json!({
            "id": "tag-1",
            "name": "Tag 1",
            "dataType": "number",
            "driverId": "drv-1",
            "scanGroupId": "sg-1",
            "driverSpec": {"key": "value"}
        })
    );
}

#[test]
fn save_driver_request_deserializes_existing_snake_case_keys() {
    let value = json!({
        "id": "drv-1",
        "original_id": "drv-old",
        "driver_type": "postgres",
        "enabled": true,
        "host": "localhost",
        "port": 5432,
        "database": "iot",
        "username": "user",
        "password": "secret"
    });

    let req: SaveDriverRequest = serde_json::from_value(value).expect("deserialize SaveDriverRequest");

    assert_eq!(req.id, "drv-1");
    assert_eq!(req.original_id.as_deref(), Some("drv-old"));
    assert_eq!(req.driver_type, "postgres");
    assert!(req.enabled);
    assert_eq!(req.host, "localhost");
    assert_eq!(req.port, 5432);
    assert_eq!(req.database, "iot");
    assert_eq!(req.username, "user");
    assert_eq!(req.password, "secret");
}
