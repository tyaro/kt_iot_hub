// Tauri IPC コマンド層
// UI からのリクエストを受け取り、core ロジックを呼び出す。
// driver_ui_bridge / postgres_registration コマンドは kt_driver_ui_host クレートに集約済み。

pub mod dto;
pub mod driver;
pub mod logs;
pub mod metrics;
pub mod publisher;
pub mod runtime;
pub mod subscriber;
pub mod tag;

pub(crate) mod config_io;
