//! Thin Tauri adapters for toolchain services.

use crate::services::toolchain;
use std::collections::HashMap;

pub use toolchain::{ToolInstallationReport, ToolVersion, WslShellPreferenceInput};

#[tauri::command]
pub async fn get_tool_versions(
    tools: Option<Vec<String>>,
    wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>>,
) -> Result<Vec<ToolVersion>, String> {
    toolchain::get_tool_versions(tools, wsl_shell_by_tool).await
}

#[tauri::command]
pub async fn run_tool_lifecycle_action(
    tools: Vec<String>,
    action: String,
    wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>>,
) -> Result<(), String> {
    toolchain::run_tool_lifecycle_action(tools, action, wsl_shell_by_tool).await
}

#[tauri::command]
pub async fn probe_tool_installations(
    tools: Vec<String>,
) -> Result<Vec<ToolInstallationReport>, String> {
    toolchain::probe_tool_installations(tools).await
}
