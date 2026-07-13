//! Convert wizard IPC commands (F3 / M2)

use crate::services::bridge::BridgePreview;
use crate::services::convert::{self, ConvertApplyRequest, ConvertApplyResult, ConvertSourceItem};

#[tauri::command]
pub async fn detect_convert_sources(source_app: String) -> Result<Vec<ConvertSourceItem>, String> {
    convert::detect_convert_sources(&source_app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_convert(
    source_app: String,
    target_app: String,
    content: String,
    content_type: String,
) -> Result<BridgePreview, String> {
    Ok(convert::preview_convert_extended(
        &source_app,
        &target_app,
        &content,
        &content_type,
    ))
}

#[tauri::command]
pub async fn apply_convert(req: ConvertApplyRequest) -> Result<ConvertApplyResult, String> {
    convert::apply_convert(&req).map_err(|e| e.to_string())
}
