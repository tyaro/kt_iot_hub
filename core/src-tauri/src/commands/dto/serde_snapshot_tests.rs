use super::{
    DriverDto, LaunchDriverUiResponse, PublisherDto, SaveDriverRequest, SavePublisherRequest,
    TagPayload,
};
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
        password_key: Some("driver/drv-1/password".to_string()),
        tls_enabled: true,
        tls_ca_path: Some("C:/certs/ca.pem".to_string()),
        tls_client_cert_path: Some("C:/certs/client.crt".to_string()),
        tls_client_key_path: Some("C:/certs/client.key".to_string()),
        connect_timeout_ms: Some(5000),
        statement_timeout_ms: Some(3000),
        auto_restart: true,
        max_restart_per_minute: Some(3),
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
            "username": "user",
            "passwordKey": "driver/drv-1/password",
            "tlsEnabled": true,
            "tlsCaPath": "C:/certs/ca.pem",
            "tlsClientCertPath": "C:/certs/client.crt",
            "tlsClientKeyPath": "C:/certs/client.key",
            "connectTimeoutMs": 5000,
            "statementTimeoutMs": 3000,
            "autoRestart": true,
            "maxRestartPerMinute": 3
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
        "password": "secret",
        "password_key": "driver/drv-1/password",
        "tls_enabled": true,
        "tls_ca_path": "C:/certs/ca.pem",
        "tls_client_cert_path": "C:/certs/client.crt",
        "tls_client_key_path": "C:/certs/client.key",
        "connect_timeout_ms": 5000,
        "statement_timeout_ms": 3000,
        "auto_restart": true,
        "max_restart_per_minute": 3
    });

    let req: SaveDriverRequest =
        serde_json::from_value(value).expect("deserialize SaveDriverRequest");

    assert_eq!(req.id, "drv-1");
    assert_eq!(req.original_id.as_deref(), Some("drv-old"));
    assert_eq!(req.driver_type, "postgres");
    assert!(req.enabled);
    assert_eq!(req.host, "localhost");
    assert_eq!(req.port, 5432);
    assert_eq!(req.database, "iot");
    assert_eq!(req.username, "user");
    assert_eq!(req.password, "secret");
    assert_eq!(req.password_key.as_deref(), Some("driver/drv-1/password"));
    assert!(req.tls_enabled);
    assert_eq!(req.tls_ca_path.as_deref(), Some("C:/certs/ca.pem"));
    assert_eq!(
        req.tls_client_cert_path.as_deref(),
        Some("C:/certs/client.crt")
    );
    assert_eq!(
        req.tls_client_key_path.as_deref(),
        Some("C:/certs/client.key")
    );
    assert_eq!(req.connect_timeout_ms, Some(5000));
    assert_eq!(req.statement_timeout_ms, Some(3000));
    assert!(req.auto_restart);
    assert_eq!(req.max_restart_per_minute, Some(3));
}

#[test]
fn publisher_dto_serialization_keeps_existing_camel_case_keys() {
    let dto = PublisherDto {
        id: "pub-1".to_string(),
        publisher_type: "mqtt".to_string(),
        broker: "localhost".to_string(),
        port: 1883,
        username: "user".to_string(),
        client_id: "client-1".to_string(),
        qos: 1,
        retain: false,
        topic: "plant".to_string(),
        password_key: Some("publisher/pub-1/password".to_string()),
        tls_enabled: true,
        tls_ca_path: Some("C:/certs/ca.pem".to_string()),
        tls_client_cert_path: Some("C:/certs/client.crt".to_string()),
        tls_client_key_path: Some("C:/certs/client.key".to_string()),
        reconnect_backoff_ms: Some(500),
        max_reconnect_backoff_ms: Some(30000),
        publish_mode_default: "scan_interval".to_string(),
        publish_mode_by_driver: Default::default(),
        publish_mode_by_scan_group: Default::default(),
    };

    let value = serde_json::to_value(dto).expect("serialize PublisherDto");
    assert_eq!(
        value,
        json!({
            "id": "pub-1",
            "publisher_type": "mqtt",
            "broker": "localhost",
            "port": 1883,
            "username": "user",
            "client_id": "client-1",
            "qos": 1,
            "retain": false,
            "topic": "plant",
            "passwordKey": "publisher/pub-1/password",
            "tlsEnabled": true,
            "tlsCaPath": "C:/certs/ca.pem",
            "tlsClientCertPath": "C:/certs/client.crt",
            "tlsClientKeyPath": "C:/certs/client.key",
            "reconnectBackoffMs": 500,
            "maxReconnectBackoffMs": 30000,
            "publishModeDefault": "scan_interval",
            "publishModeByDriver": {},
            "publishModeByScanGroup": {}
        })
    );
}

#[test]
fn save_publisher_request_deserializes_existing_snake_case_keys() {
    let value = json!({
        "id": "pub-1",
        "publisher_type": "mqtt",
        "broker": "localhost",
        "port": 1883,
        "username": "user",
        "password": "secret",
        "client_id": "client-1",
        "qos": 1,
        "retain": false,
        "topic": "plant"
    });

    let req: SavePublisherRequest =
        serde_json::from_value(value).expect("deserialize SavePublisherRequest");

    assert_eq!(req.id, "pub-1");
    assert_eq!(req.publisher_type, "mqtt");
    assert_eq!(req.broker, "localhost");
    assert_eq!(req.port, 1883);
    assert_eq!(req.username, "user");
    assert_eq!(req.password, "secret");
    assert_eq!(req.password_key, None);
    assert!(!req.tls_enabled);
    assert_eq!(req.tls_ca_path, None);
    assert_eq!(req.tls_client_cert_path, None);
    assert_eq!(req.tls_client_key_path, None);
    assert_eq!(req.reconnect_backoff_ms, None);
    assert_eq!(req.max_reconnect_backoff_ms, None);
}
