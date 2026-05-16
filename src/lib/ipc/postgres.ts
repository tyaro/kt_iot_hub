import { invoke } from '@tauri-apps/api/core';

export interface PostgresConnectionParams {
  host: string;
  port: number;
  database: string;
  username: string;
  password: string;
  ssl_mode?: string | null;
}

export interface PostgresTableDto {
  schema: string;
  name: string;
}

export interface PostgresColumnDto {
  name: string;
  data_type: string;
  is_nullable: boolean;
}

export interface PostgresColumnsRequest {
  conn: PostgresConnectionParams;
  schema?: string | null;
  table: string;
}

export interface PostgresConnectionTestResult {
  ok: boolean;
  message: string;
}

/**
 * PostgreSQL 接続テスト
 */
export async function postgresTestConnection(
  conn: PostgresConnectionParams,
): Promise<PostgresConnectionTestResult> {
  return invoke('postgres_test_connection', { conn });
}

/**
 * PostgreSQL テーブル一覧取得
 */
export async function postgresListTables(
  conn: PostgresConnectionParams,
): Promise<PostgresTableDto[]> {
  return invoke('postgres_list_tables', { conn });
}

/**
 * PostgreSQL カラム一覧取得
 */
export async function postgresListColumns(
  req: PostgresColumnsRequest,
): Promise<PostgresColumnDto[]> {
  return invoke('postgres_list_columns', { req });
}
