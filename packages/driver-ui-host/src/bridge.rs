//! ドライバUI プロセスの起動コンテキスト共通ハンドラ。
//!
//! - `get_driver_ui_launch_context`: CLI 引数 / 環境変数 / 入力 JSON から起動情報を組み立てる。
//! - `save_driver_ui_output`: ドライバUI が編集した結果 JSON を所定のパスへ保存する。

use crate::dto::{DriverUiLaunchContextDto, ErrorResponse, SaveDriverUiOutputRequest};
use kt_driver_ui_protocol::{DriverUiImportPayload, DriverUiLaunchContext};

#[derive(Debug, Default)]
struct CliArgs {
    driver_ui_mode: bool,
    session_id: Option<String>,
    driver_type: Option<String>,
    driver_id: Option<String>,
    input_json_path: Option<String>,
    output_json_path: Option<String>,
}

#[tauri::command]
pub async fn get_driver_ui_launch_context() -> Result<DriverUiLaunchContextDto, ErrorResponse> {
    let args = parse_cli_args();
    let mut request_id = None;
    let mut output_json_path = args.output_json_path.clone();
    let mut driver_type = args.driver_type.clone();
    let mut driver_id = args.driver_id.clone();

    if let Some(input_path) = args.input_json_path.as_ref() {
        let text = std::fs::read_to_string(input_path).map_err(|e| ErrorResponse {
            error: format!("Failed to read input json: {}", e),
            code: "IO_ERROR".to_string(),
        })?;

        let parsed: DriverUiLaunchContext =
            serde_json::from_str(&text).map_err(|e| ErrorResponse {
                error: format!("Failed to parse input json: {}", e),
                code: "INVALID_JSON".to_string(),
            })?;

        request_id = Some(parsed.request_id);
        if output_json_path.is_none() {
            output_json_path = Some(parsed.session.output_json_path);
        }
        if driver_type.is_none() {
            driver_type = Some(parsed.driver.driver_type);
        }
        if driver_id.is_none() {
            driver_id = parsed.driver.driver_id;
        }
    }

    let launched_as_driver_ui = args.driver_ui_mode
        || args.session_id.is_some()
        || output_json_path.is_some()
        || args.input_json_path.is_some();

    Ok(DriverUiLaunchContextDto {
        launched_as_driver_ui,
        session_id: args.session_id,
        driver_type,
        driver_id,
        input_json_path: args.input_json_path,
        output_json_path,
        request_id,
    })
}

#[tauri::command]
pub async fn save_driver_ui_output(
    req: SaveDriverUiOutputRequest,
) -> Result<String, ErrorResponse> {
    let output_path = resolve_output_path(req.output_json_path)?;

    // 受信ペイロードがドライバUI 共通フォーマットに合致するかをチェックする。
    serde_json::from_value::<DriverUiImportPayload>(req.payload.clone()).map_err(|e| {
        ErrorResponse {
            error: format!("Invalid driver-ui response payload: {}", e),
            code: "VALIDATION_ERROR".to_string(),
        }
    })?;

    let text = serde_json::to_string_pretty(&req.payload).map_err(|e| ErrorResponse {
        error: format!("Failed to serialize driver-ui response payload: {}", e),
        code: "SERIALIZE_ERROR".to_string(),
    })?;

    let path = std::path::PathBuf::from(&output_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| ErrorResponse {
                error: format!("Failed to create output directory: {}", e),
                code: "IO_ERROR".to_string(),
            })?;
        }
    }

    std::fs::write(&path, text).map_err(|e| ErrorResponse {
        error: format!("Failed to write output json: {}", e),
        code: "IO_ERROR".to_string(),
    })?;

    Ok(output_path)
}

fn resolve_output_path(explicit_path: Option<String>) -> Result<String, ErrorResponse> {
    if let Some(path) = normalize_optional_string(explicit_path) {
        return Ok(path);
    }

    let args = parse_cli_args();
    normalize_optional_string(args.output_json_path).ok_or(ErrorResponse {
        error: "outputJsonPath is required (CLI --output-json or request.outputJsonPath)"
            .to_string(),
        code: "INVALID_INPUT".to_string(),
    })
}

fn parse_cli_args() -> CliArgs {
    let mut cli = CliArgs::default();
    let mut iter = std::env::args().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--driver-ui-mode" => cli.driver_ui_mode = true,
            "--session-id" => cli.session_id = iter.next(),
            "--driver-type" => cli.driver_type = iter.next(),
            "--driver-id" => cli.driver_id = iter.next(),
            "--input-json" => cli.input_json_path = iter.next(),
            "--output-json" => cli.output_json_path = iter.next(),
            _ => {}
        }
    }

    // CLI 引数が取得できない場合のフォールバック (spawn 元からの env 受け渡し)。
    if cli.session_id.is_none() {
        cli.session_id = std::env::var("KT_IOT_HUB_DRIVER_UI_SESSION_ID").ok();
    }
    if cli.driver_type.is_none() {
        cli.driver_type = std::env::var("KT_IOT_HUB_DRIVER_UI_DRIVER_TYPE").ok();
    }
    if cli.driver_id.is_none() {
        cli.driver_id = std::env::var("KT_IOT_HUB_DRIVER_UI_DRIVER_ID").ok();
    }
    if cli.input_json_path.is_none() {
        cli.input_json_path = std::env::var("KT_IOT_HUB_DRIVER_UI_INPUT_JSON").ok();
    }
    if cli.output_json_path.is_none() {
        cli.output_json_path = std::env::var("KT_IOT_HUB_DRIVER_UI_OUTPUT_JSON").ok();
    }
    if !cli.driver_ui_mode {
        cli.driver_ui_mode = matches!(
            std::env::var("KT_IOT_HUB_DRIVER_UI_MODE").ok().as_deref(),
            Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
        );
    }

    cli
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
