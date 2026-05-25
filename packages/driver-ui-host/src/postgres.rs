//! PostgreSQL レジストレーション用の共通 Tauri コマンド。
//! 本体・ドライバUI どちらからも同一実装を使う。

use crate::dto::{
    ErrorResponse, PostgresColumnDto, PostgresColumnsRequest, PostgresConnectionParams,
    PostgresConnectionTestResult, PostgresTableDto,
};
use anyhow::{Error as AnyhowError, Result};
use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use std::fs;
use std::time::Duration;
use tokio_postgres::{Config, NoTls};

fn map_pg_err(err: tokio_postgres::Error) -> ErrorResponse {
    ErrorResponse::from(AnyhowError::from(err))
}

fn build_config(conn: &PostgresConnectionParams) -> Config {
    let mut config = Config::new();
    config.host(&conn.host);
    config.port(conn.port);
    config.dbname(&conn.database);
    config.user(&conn.username);
    config.password(&conn.password);

    if let Some(connect_timeout_ms) = conn.connect_timeout_ms {
        config.connect_timeout(Duration::from_millis(connect_timeout_ms));
    }

    config
}

fn should_use_tls(conn: &PostgresConnectionParams) -> bool {
    conn.tls_enabled
        || conn
            .ssl_mode
            .as_deref()
            .map(|mode| !mode.trim().is_empty() && !mode.eq_ignore_ascii_case("disable"))
            .unwrap_or(false)
}

fn build_tls_connector(
    conn: &PostgresConnectionParams,
) -> anyhow::Result<Option<MakeTlsConnector>> {
    if !should_use_tls(conn) {
        return Ok(None);
    }

    let mut builder = TlsConnector::builder();

    if let Some(ca_path) = conn
        .tls_ca_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        let pem = fs::read(ca_path).map_err(|e| {
            anyhow::anyhow!("failed to read PostgreSQL TLS CA file {}: {}", ca_path, e)
        })?;
        let cert = Certificate::from_pem(&pem).map_err(|e| {
            anyhow::anyhow!("failed to parse PostgreSQL TLS CA file {}: {}", ca_path, e)
        })?;
        builder.add_root_certificate(cert);
    }

    let connector = builder
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build PostgreSQL TLS connector: {}", e))?;
    Ok(Some(MakeTlsConnector::new(connector)))
}

async fn connect_client(
    conn: &PostgresConnectionParams,
) -> Result<(tokio_postgres::Client, tokio::task::JoinHandle<()>)> {
    let config = build_config(conn);

    if let Some(tls) = build_tls_connector(conn)? {
        let (client, connection) = config.connect(tls).await?;
        let conn_task = tokio::spawn(async move {
            let _ = connection.await;
        });

        if let Some(statement_timeout_ms) = conn.statement_timeout_ms {
            client
                .batch_execute(&format!("SET statement_timeout = {}", statement_timeout_ms))
                .await?;
        }

        Ok((client, conn_task))
    } else {
        let (client, connection) = config.connect(NoTls).await?;
        let conn_task = tokio::spawn(async move {
            let _ = connection.await;
        });

        if let Some(statement_timeout_ms) = conn.statement_timeout_ms {
            client
                .batch_execute(&format!("SET statement_timeout = {}", statement_timeout_ms))
                .await?;
        }

        Ok((client, conn_task))
    }
}

#[tauri::command]
pub async fn postgres_test_connection(
    conn: PostgresConnectionParams,
) -> Result<PostgresConnectionTestResult, ErrorResponse> {
    let (client, conn_task) = connect_client(&conn).await.map_err(ErrorResponse::from)?;

    let _ = client
        .query_one("SELECT 1", &[])
        .await
        .map_err(map_pg_err)?;

    conn_task.abort();

    Ok(PostgresConnectionTestResult {
        ok: true,
        message: "Connection test succeeded".to_string(),
    })
}

#[tauri::command]
pub async fn postgres_list_tables(
    conn: PostgresConnectionParams,
) -> Result<Vec<PostgresTableDto>, ErrorResponse> {
    let (client, conn_task) = connect_client(&conn).await.map_err(ErrorResponse::from)?;

    let rows = client
        .query(
            r#"
            SELECT table_schema, table_name
            FROM information_schema.tables
            WHERE table_type = 'BASE TABLE'
              AND table_schema NOT IN ('pg_catalog', 'information_schema')
            ORDER BY table_schema, table_name
            "#,
            &[],
        )
        .await
        .map_err(map_pg_err)?;

    conn_task.abort();

    Ok(rows
        .into_iter()
        .map(|row| PostgresTableDto {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
        })
        .collect())
}

#[tauri::command]
pub async fn postgres_list_columns(
    req: PostgresColumnsRequest,
) -> Result<Vec<PostgresColumnDto>, ErrorResponse> {
    if req.table.trim().is_empty() {
        return Err(ErrorResponse {
            error: "table is required".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let schema = req
        .schema
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "public".to_string());

    let (client, conn_task) = connect_client(&req.conn)
        .await
        .map_err(ErrorResponse::from)?;

    let rows = client
        .query(
            r#"
            SELECT column_name, data_type, is_nullable
            FROM information_schema.columns
            WHERE table_schema = $1
              AND table_name = $2
            ORDER BY ordinal_position
            "#,
            &[&schema, &req.table],
        )
        .await
        .map_err(map_pg_err)?;

    conn_task.abort();

    Ok(rows
        .into_iter()
        .map(|row| {
            let is_nullable_text = row.get::<_, String>(2);
            PostgresColumnDto {
                name: row.get::<_, String>(0),
                data_type: row.get::<_, String>(1),
                is_nullable: is_nullable_text.eq_ignore_ascii_case("YES"),
            }
        })
        .collect())
}
