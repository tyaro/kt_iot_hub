// タグ操作のコマンド層

use super::dto::{CreateTagRequest, ErrorResponse, TagDto};
use crate::app_state::AppState;
use crate::core::{DataType, Tag, TagId};
use std::str::FromStr;

#[tauri::command]
pub async fn create_tag(
    state: tauri::State<'_, AppState>,
    req: CreateTagRequest,
) -> Result<TagDto, ErrorResponse> {
    // 入力バリデーション
    if req.id.is_empty() {
        return Err(ErrorResponse {
            error: "Tag ID cannot be empty".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
    }

    let data_type = DataType::from_str(&req.data_type).map_err(ErrorResponse::from)?;
    let tag = Tag {
        id: TagId(req.id.clone()),
        name: req.name.clone(),
        data_type,
        driver_id: req.driver_id.clone(),
        scan_group_id: req.scan_group_id.clone(),
        driver_spec: req.driver_spec.clone(),
        metadata: None,
    };
    state.registry.insert(tag).await;

    Ok(TagDto {
        id: req.id,
        name: req.name,
        data_type: req.data_type,
        driver_id: req.driver_id,
        scan_group_id: req.scan_group_id,
        driver_spec: req.driver_spec,
    })
}

#[tauri::command]
pub async fn list_tags(state: tauri::State<'_, AppState>) -> Result<Vec<TagDto>, ErrorResponse> {
    let tags = state.registry.list_all().await;
    Ok(tags
        .into_iter()
        .map(|tag| TagDto {
            id: tag.id.0,
            name: tag.name,
            data_type: tag.data_type.as_str().to_string(),
            driver_id: tag.driver_id,
            scan_group_id: tag.scan_group_id,
            driver_spec: tag.driver_spec,
        })
        .collect())
}

#[tauri::command]
pub async fn delete_tag(
    state: tauri::State<'_, AppState>,
    tag_id: String,
) -> Result<(), ErrorResponse> {
    state.registry.remove(&TagId(tag_id)).await;
    Ok(())
}
