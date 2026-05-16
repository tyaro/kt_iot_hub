//! ドライバ管理関連の Tauri コマンドモジュール。
//!
//! 旧 driver.rs を責務単位で分割した構成:
//! - `crud`        : 一覧/保存/削除コマンド
//! - `ui_launcher` : ドライバUI 起動・結果ファイル確認
//! - `import`      : ドライバUI 結果 JSON のバリデーションと取り込み
//! - `ui_paths`    : 登録UI 実行ファイルのパス解決ヘルパ
//! - `toml_io`     : drivers.toml / tags.toml のアトミック書き出し
//! - `runtime_sync`: 設定変更後の DriverManager 反映処理

// 注意: `tauri::generate_handler!` マクロは `pub use` 越しの再エクスポートを辿らず、
// 指定パス直下の `__cmd__<name>` シンボルを直接探す。
// そのため main.rs からは `commands::driver::<submod>::<fn>` の形で参照すること。
pub mod crud;
pub mod import;
pub mod ui_launcher;

mod runtime_sync;
mod toml_io;
mod ui_paths;
