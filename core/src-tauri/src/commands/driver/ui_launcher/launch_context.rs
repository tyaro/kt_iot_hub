use crate::app_state::AppState;
use crate::commands::dto::ErrorResponse;
use chrono::Utc;
use kt_driver_ui_protocol::{
    DriverUiLaunchContext, DriverUiLaunchData, DriverUiLaunchDriver, DriverUiLaunchScanGroup,
    DriverUiLaunchSession, DriverUiLaunchTag,
};
use tracing::{info, warn};

pub(super) async fn build_driver_ui_launch_context(
    state: &tauri::State<'_, AppState>,
    session_id: &str,
    driver_id: Option<String>,
    driver_type: String,
    output_json_path: String,
    editing_tag_id: Option<String>,
) -> Result<DriverUiLaunchContext, ErrorResponse> {
    let scan_groups = state.scan_groups.read().await.clone();
    let tags = state.registry.list_all().await;
    let driver_configs = state.driver_configs.read().await.clone();

    let existing_driver_ids: Vec<String> = driver_configs
        .iter()
        .filter(|cfg| cfg.driver_type == driver_type)
        .map(|cfg| cfg.id.clone())
        .collect();

    let total_tags_for_driver = tags
        .iter()
        .filter(|tag| {
            if let Some(ref target_driver_id) = driver_id {
                tag.driver_id == *target_driver_id
            } else {
                false
            }
        })
        .count();

    let total_scan_groups_for_driver = scan_groups
        .iter()
        .filter(|group| {
            if let Some(ref target_driver_id) = driver_id {
                group.driver == *target_driver_id
            } else {
                false
            }
        })
        .count();

    let driver_settings = driver_id.as_ref().and_then(|target_driver_id| {
        driver_configs
            .iter()
            .find(|cfg| cfg.id == *target_driver_id)
            .map(|cfg| cfg.settings.clone())
    });

    let tags_by_scan_group: std::collections::HashMap<String, Vec<_>> = tags
        .iter()
        .filter(|tag| {
            if let Some(ref target_driver_id) = driver_id {
                tag.driver_id == *target_driver_id
            } else {
                false
            }
        })
        .fold(std::collections::HashMap::new(), |mut map, tag| {
            map.entry(tag.scan_group_id.clone())
                .or_insert_with(Vec::new)
                .push(tag);
            map
        });

    let filtered_scan_groups: Vec<DriverUiLaunchScanGroup> = scan_groups
        .into_iter()
        .filter(|group| {
            if let Some(ref target_driver_id) = driver_id {
                group.driver == *target_driver_id
            } else {
                false
            }
        })
        .map(|group| {
            let group_tags = tags_by_scan_group
                .get(&group.id)
                .map(|tag_refs| {
                    tag_refs
                        .iter()
                        .map(|tag| DriverUiLaunchTag {
                            id: tag.id.0.clone(),
                            name: tag.name.clone(),
                            data_type: tag.data_type.as_str().to_string(),
                            driver_id: tag.driver_id.clone(),
                            scan_group_id: tag.scan_group_id.clone(),
                            enabled: true,
                            driver_spec: tag.driver_spec.clone(),
                            metadata: tag.metadata.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            DriverUiLaunchScanGroup {
                id: group.id,
                driver: group.driver,
                scan_rate_ms: group.scan_rate_ms,
                schema: group.schema,
                table: group.table,
                timestamp_column: group.timestamp_column,
                node: group.node,
                tags: group_tags,
            }
        })
        .collect();

    let launched_tag_count = filtered_scan_groups
        .iter()
        .map(|group| group.tags.len())
        .sum::<usize>();

    info!(
        "Driver UI launch context summary: driver_id={} driver_type={} scan_groups={} tags={} existing_same_type_driver_ids={}",
        driver_id.clone().unwrap_or_else(|| "<new>".to_string()),
        driver_type,
        filtered_scan_groups.len(),
        launched_tag_count,
        existing_driver_ids.len(),
    );

    if driver_id.is_some()
        && (total_scan_groups_for_driver > 0 || total_tags_for_driver > 0)
        && (filtered_scan_groups.is_empty() || launched_tag_count == 0)
    {
        warn!(
            "Driver UI launch context might be incomplete: driver_id={} total_scan_groups_for_driver={} total_tags_for_driver={} filtered_scan_groups={} launched_tags={}",
            driver_id.clone().unwrap_or_else(|| "<new>".to_string()),
            total_scan_groups_for_driver,
            total_tags_for_driver,
            filtered_scan_groups.len(),
            launched_tag_count,
        );
    }

    Ok(DriverUiLaunchContext {
        schema_version: 1,
        request_id: format!("req-{}", session_id),
        generated_at: Utc::now().to_rfc3339(),
        direction: "host-to-driver".to_string(),
        session: DriverUiLaunchSession {
            session_id: session_id.to_string(),
            mode: "create-or-edit".to_string(),
            output_json_path,
            editing_tag_id,
        },
        driver: DriverUiLaunchDriver {
            driver_type,
            driver_id,
        },
        context: DriverUiLaunchData {
            scan_groups: filtered_scan_groups,
            existing_driver_ids,
            driver_settings,
        },
    })
}
