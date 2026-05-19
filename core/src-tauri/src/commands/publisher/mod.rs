//! パブリッシャ管理関連の Tauri コマンドモジュール。
//!
//! - `crud`        : 一覧/保存コマンド
//! - `runtime_sync`: 設定変更後の PublisherManager 反映処理
//! - `toml_io`     : publishers.toml のアトミック書き出し

pub mod crud;

mod runtime_sync;
mod toml_io;
