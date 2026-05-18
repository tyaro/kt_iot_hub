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
