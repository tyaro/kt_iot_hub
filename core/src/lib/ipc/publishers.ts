import { ipcInvoke } from './_invoke';

interface PublisherCoreFields {
  id: string;
  publisher_type: string;
  enabled: boolean;
  broker: string;
  port: number;
  username: string;
  client_id: string;
  qos: 0 | 1 | 2;
  retain: boolean;
  topic: string;
}

export interface PublisherDto extends PublisherCoreFields {}

export interface SavePublisherRequest extends PublisherCoreFields {
  password: string;
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
