use super::driver_ui_protocol::DriverUiImportPayload;
use super::dto::{DriverUiLaunchContextDto, ErrorResponse, SaveDriverUiOutputRequest};
use serde::Deserialize;

#[derive(Debug, Default)]
struct CliArgs {
    session_id: Option<String>,
    driver_type: Option<String>,
    driver_id: Option<String>,
    input_json_path: Option<String>,
    output_json_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputSession {
    #[allow(dead_code)]
    session_id: Option<String>,
    output_json_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputDriver {
    driver_type: Option<String>,
    driver_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputLaunchContext {
    request_id: Option<String>,
    session: Option<InputSession>,
    driver: Option<InputDriver>,
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

        let parsed: InputLaunchContext = serde_json::from_str(&text).map_err(|e| ErrorResponse {
            error: format!("Failed to parse input json: {}", e),
            code: "INVALID_JSON".to_string(),
        })?;

        request_id = parsed.request_id;
        if output_json_path.is_none() {
            output_json_path = parsed.session.and_then(|s| s.output_json_path);
        }
        if driver_type.is_none() {
            driver_type = parsed.driver.as_ref().and_then(|d| d.driver_type.clone());
        }
        if driver_id.is_none() {
            driver_id = parsed.driver.and_then(|d| d.driver_id);
        }
    }

    let launched_as_driver_ui = args.session_id.is_some()
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
pub async fn save_driver_ui_output(req: SaveDriverUiOutputRequest) -> Result<String, ErrorResponse> {
    let output_path = resolve_output_path(req.output_json_path)?;

    serde_json::from_value::<DriverUiImportPayload>(req.payload.clone()).map_err(|e| ErrorResponse {
        error: format!("Invalid driver-ui response payload: {}", e),
        code: "VALIDATION_ERROR".to_string(),
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
        error: "outputJsonPath is required (CLI --output-json or request.outputJsonPath)".to_string(),
        code: "INVALID_INPUT".to_string(),
    })
}

fn parse_cli_args() -> CliArgs {
    let mut cli = CliArgs::default();
    let mut iter = std::env::args().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--session-id" => cli.session_id = iter.next(),
            "--driver-type" => cli.driver_type = iter.next(),
            "--driver-id" => cli.driver_id = iter.next(),
            "--input-json" => cli.input_json_path = iter.next(),
            "--output-json" => cli.output_json_path = iter.next(),
            _ => {}
        }
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
