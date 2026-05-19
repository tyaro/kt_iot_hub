pub(super) fn build_driver_ui_io_paths(
    driver_id: Option<&str>,
    driver_type: &str,
    session_id: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let target = driver_id
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("new-{}", driver_type));

    let output_json_path = std::env::temp_dir().join(format!(
        "kt_iot_hub_driver_ui_{}_{}.json",
        target, session_id
    ));
    let input_json_path = std::env::temp_dir().join(format!(
        "kt_iot_hub_driver_ui_input_{}_{}.json",
        target, session_id
    ));

    (input_json_path, output_json_path)
}
