import type { PublisherDto, SavePublisherRequest } from '$lib/ipc';

export function createDefaultForm(): SavePublisherRequest {
  return {
    id: 'mqtt-main',
    publisher_type: 'mqtt',
    enabled: false,
    broker: '127.0.0.1',
    port: 1883,
    username: '',
    password: '',
    client_id: 'kt_iot_hub',
    qos: 1,
    retain: false,
    topic: '',
    publish_mode_default: 'scan_interval',
    publish_mode_by_driver: {},
    publish_mode_by_scan_group: {},
  };
}

export function toForm(publisher: PublisherDto): SavePublisherRequest {
  return {
    ...publisher,
    password: '',
    topic: publisher.topic ?? '',
    qos: (publisher.qos as 0 | 1 | 2) ?? 1,
  };
}
