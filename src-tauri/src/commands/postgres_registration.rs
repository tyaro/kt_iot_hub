use super::dto::{
    ErrorResponse, PostgresColumnDto, PostgresColumnsRequest, PostgresConnectionParams,
    PostgresConnectionTestResult, PostgresTableDto,
};
use anyhow::Error as AnyhowError;
use tokio_postgres::NoTls;

fn build_dsn(conn: &PostgresConnectionParams) -> String {
    let ssl_mode = conn
        .ssl_mode
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "disable".to_string());

    format!(
        "host={} port={} dbname={} user={} password={} sslmode={}",
        conn.host, conn.port, conn.database, conn.username, conn.password, ssl_mode
    )
}

fn map_pg_err(err: tokio_postgres::Error) -> ErrorResponse {
    ErrorResponse::from(AnyhowError::from(err))
}

#[tauri::command]
pub async fn postgres_test_connection(
    conn: PostgresConnectionParams,
) -> Result<PostgresConnectionTestResult, ErrorResponse> {
    let dsn = build_dsn(&conn);
    let (client, connection) = tokio_postgres::connect(&dsn, NoTls)
        .await
        .map_err(map_pg_err)?;

    let conn_task = tokio::spawn(async move {
        let _ = connection.await;
    });

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
    let dsn = build_dsn(&conn);
    let (client, connection) = tokio_postgres::connect(&dsn, NoTls)
        .await
        .map_err(map_pg_err)?;

    let conn_task = tokio::spawn(async move {
        let _ = connection.await;
    });

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

    let dsn = build_dsn(&req.conn);
    let (client, connection) = tokio_postgres::connect(&dsn, NoTls)
        .await
        .map_err(map_pg_err)?;

    let conn_task = tokio::spawn(async move {
        let _ = connection.await;
    });

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
