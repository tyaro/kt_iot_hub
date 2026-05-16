//! 本体と外部ドライバUIで共通の Tauri コマンド・DTO を提供する。
//!
//! 設計方針:
//! - DTO は `dto` モジュールに集約し、両プロセスから同一の型を参照する。
//! - PostgreSQL のレジストレーション API は `postgres` モジュールに、
//!   ドライバUI 起動コンテキストの受け渡しは `bridge` モジュールに置く。
//! - すべての公開コマンドは `#[tauri::command]` 付きの async 関数で、
//!   呼び出し側は `tauri::generate_handler!` の引数にパスを並べるだけで再利用できる。

pub mod bridge;
pub mod dto;
pub mod postgres;

// よく使う型を crate ルートから直接参照できるようにする。
pub use dto::{
    DriverUiLaunchContextDto, ErrorResponse, PostgresColumnDto, PostgresColumnsRequest,
    PostgresConnectionParams, PostgresConnectionTestResult, PostgresTableDto,
    SaveDriverUiOutputRequest,
};
