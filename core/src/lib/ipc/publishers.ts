import { ipcInvoke } from './_invoke';

interface PublisherCoreFields {
  id: string;
  publisher_type: string;
  broker: string;
  port: number;
  username: string;
  client_id: string;
  qos: 0 | 1 | 2;
  retain: boolean;
  topic: string;
  publish_mode_default: MqttPublishMode;
  publish_mode_by_driver: Record<string, MqttPublishMode>;
  publish_mode_by_scan_group: Record<string, MqttPublishMode>;
}

export type MqttPublishMode = 'scan_interval' | 'on_change';

export interface PublisherDto extends PublisherCoreFields {}

export interface SavePublisherRequest extends PublisherCoreFields {
  password: string;
}

export interface GetMqttPublishModeRequest {
  driver_id: string;
  scan_group_id?: string | null;
}

export interface GetMqttPublishModeResponse {
  publisher_id: string;
  mode: MqttPublishMode;
  source: 'default' | 'driver' | 'scan_group';
}

export interface SetMqttPublishModeRequest {
  publisher_id?: string | null;
  scope: 'driver' | 'scan_group';
  driver_id: string;
  scan_group_id?: string | null;
  mode: MqttPublishMode;
}

export interface SetMqttPublishModeResponse {
  publisher_id: string;
  scope: 'driver' | 'scan_group';
  driver_id: string;
  scan_group_id?: string | null;
  mode: MqttPublishMode;
}

/**
 * すべてのパブリッシャ設定を取得する
 */
export async function listPublishers(): Promise<PublisherDto[]> {
  return ipcInvoke('list_publishers');
}

/**
 * パブリッシャ設定を保存する
 */
export async function savePublisher(req: SavePublisherRequest): Promise<void> {
  return ipcInvoke('save_publisher', { req });
}

export async function getMqttPublishMode(req: GetMqttPublishModeRequest): Promise<GetMqttPublishModeResponse> {
  return ipcInvoke('get_mqtt_publish_mode', { req });
}

export async function setMqttPublishMode(req: SetMqttPublishModeRequest): Promise<SetMqttPublishModeResponse> {
  return ipcInvoke('set_mqtt_publish_mode', { req });
}
