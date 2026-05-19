#[allow(unused_imports)]
pub use kt_driver_ui_host::dto::error_code;
/// `ErrorResponse` は本体とドライバUI 双方で利用する共通エラー型。
/// その他の PostgreSQL DTO 群はドライバUI 専用のため、ここでは re-export しない
/// （必要な場合は `kt_driver_ui_host::dto` から直接 import すること）。
pub use kt_driver_ui_host::dto::ErrorResponse;
