import { invoke } from '@tauri-apps/api/core';

export interface MqttMonitorPublisherDto {
  id: string;
  broker: string;
  port: number;
  topic: string;
}

export interface MqttMonitorStatusDto {
  connected: boolean;
  subscribing: boolean;
  include_sys: boolean;
  publisher_id: string | null;
  broker: string;
  port: number;
  topic_filter: string;
  message_count: number;
  last_message_at: string | null;
  last_error: string | null;
}

export interface MqttMonitorMessageDto {
  timestamp: string;
  topic: string;
  payload: string;
  qos: 0 | 1 | 2;
  retain: boolean;
}

export interface StartMqttMonitorRequest {
  publisher_id: string;
  topic_filter: string;
  include_sys: boolean;
}

type MqttMonitorStatusDtoRaw = {
  connected: boolean;
  subscribing: boolean;
  includeSys: boolean;
  publisherId?: string | null;
  broker: string;
  port: number;
  topicFilter: string;
  messageCount: number;
  lastMessageAt?: string | null;
  lastError?: string | null;
};

function normalizeStatus(raw: MqttMonitorStatusDtoRaw): MqttMonitorStatusDto {
  return {
    connected: raw.connected,
    subscribing: raw.subscribing,
    include_sys: raw.includeSys,
    publisher_id: raw.publisherId ?? null,
    broker: raw.broker,
    port: raw.port,
    topic_filter: raw.topicFilter,
    message_count: raw.messageCount,
    last_message_at: raw.lastMessageAt ?? null,
    last_error: raw.lastError ?? null,
  };
}

export async function listMqttMonitorPublishers(): Promise<MqttMonitorPublisherDto[]> {
  return invoke('list_mqtt_monitor_publishers');
}

export async function openMqttMonitorWindow(): Promise<void> {
  return invoke('open_mqtt_monitor_window');
}

export async function getMqttMonitorStatus(): Promise<MqttMonitorStatusDto> {
  const raw = await invoke<MqttMonitorStatusDtoRaw>('get_mqtt_monitor_status');
  return normalizeStatus(raw);
}

export async function listMqttMonitorMessages(): Promise<MqttMonitorMessageDto[]> {
  return invoke('list_mqtt_monitor_messages');
}

export async function clearMqttMonitorMessages(): Promise<void> {
  return invoke('clear_mqtt_monitor_messages');
}

export async function startMqttMonitor(
  req: StartMqttMonitorRequest,
): Promise<MqttMonitorStatusDto> {
  const raw = await invoke<MqttMonitorStatusDtoRaw>('start_mqtt_monitor', {
    req: {
      publisherId: req.publisher_id,
      topicFilter: req.topic_filter,
      includeSys: req.include_sys,
    },
  });
  return normalizeStatus(raw);
}

export async function stopMqttMonitor(): Promise<MqttMonitorStatusDto> {
  const raw = await invoke<MqttMonitorStatusDtoRaw>('stop_mqtt_monitor');
  return normalizeStatus(raw);
}
