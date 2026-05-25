import type { PublisherDto, SavePublisherRequest } from '$lib/ipc';

export function createDefaultForm(): SavePublisherRequest {
  return {
    id: 'mqtt-main',
    publisher_type: 'mqtt',
    broker: '127.0.0.1',
    port: 1883,
    username: '',
    password: '',
    password_key: null,
    client_id: 'kt_iot_hub',
    qos: 1,
    retain: false,
    topic: '',
    tls_enabled: false,
    tls_ca_path: null,
    tls_client_cert_path: null,
    tls_client_key_path: null,
    reconnect_backoff_ms: null,
    max_reconnect_backoff_ms: null,
    publish_mode_default: 'scan_interval',
    publish_mode_by_driver: {},
    publish_mode_by_scan_group: {},
  };
}

export function toForm(publisher: PublisherDto): SavePublisherRequest {
  return {
    ...publisher,
    password: '',
    password_key: publisher.password_key ?? null,
    topic: publisher.topic ?? '',
    qos: (publisher.qos as 0 | 1 | 2) ?? 1,
    tls_enabled: publisher.tls_enabled ?? false,
    tls_ca_path: publisher.tls_ca_path ?? null,
    tls_client_cert_path: publisher.tls_client_cert_path ?? null,
    tls_client_key_path: publisher.tls_client_key_path ?? null,
    reconnect_backoff_ms: publisher.reconnect_backoff_ms ?? null,
    max_reconnect_backoff_ms: publisher.max_reconnect_backoff_ms ?? null,
  };
}
