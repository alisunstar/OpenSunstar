#![allow(non_snake_case)]

use indexmap::IndexMap;
use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use crate::app_config::AppType;
use crate::claude_mcp;
use crate::services::McpService;
use crate::store::AppState;

/// 获取 Claude MCP 状态
#[tauri::command]
pub async fn get_claude_mcp_status() -> Result<claude_mcp::McpStatus, String> {
    claude_mcp::get_mcp_status().map_err(|e| e.to_string())
}

/// 读取 mcp.json 文本内容
#[tauri::command]
pub async fn read_claude_mcp_config() -> Result<Option<String>, String> {
    claude_mcp::read_mcp_json().map_err(|e| e.to_string())
}

/// 新增或更新一个 MCP 服务器条目
#[tauri::command]
pub async fn upsert_claude_mcp_server(id: String, spec: serde_json::Value) -> Result<bool, String> {
    claude_mcp::upsert_mcp_server(&id, spec).map_err(|e| e.to_string())
}

/// 删除一个 MCP 服务器条目
#[tauri::command]
pub async fn delete_claude_mcp_server(id: String) -> Result<bool, String> {
    claude_mcp::delete_mcp_server(&id).map_err(|e| e.to_string())
}

/// 校验命令是否在 PATH 中可用（不执行）
#[tauri::command]
pub async fn validate_mcp_command(cmd: String) -> Result<bool, String> {
    claude_mcp::validate_command_in_path(&cmd).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct McpConfigResponse {
    pub config_path: String,
    pub servers: HashMap<String, serde_json::Value>,
}

/// 获取 MCP 配置（来自 ~/.OpenSunstar/config.json）
use std::str::FromStr;

#[tauri::command]
#[allow(deprecated)] // 兼容层命令，内部调用已废弃的 Service 方法
pub async fn get_mcp_config(
    state: State<'_, AppState>,
    app: String,
) -> Result<McpConfigResponse, String> {
    let config_path = crate::config::get_app_config_path()
        .to_string_lossy()
        .to_string();
    let app_ty = AppType::from_str(&app).map_err(|e| e.to_string())?;
    let servers = McpService::get_servers(&state, app_ty).map_err(|e| e.to_string())?;
    Ok(McpConfigResponse {
        config_path,
        servers,
    })
}

/// 在 config.json 中新增或更新一个 MCP 服务器定义
/// [已废弃] 该命令仍然使用旧的分应用API，会转换为统一结构
#[tauri::command]
pub async fn upsert_mcp_server_in_config(
    state: State<'_, AppState>,
    app: String,
    id: String,
    spec: serde_json::Value,
    sync_other_side: Option<bool>,
) -> Result<bool, String> {
    use crate::app_config::McpServer;

    let app_ty = AppType::from_str(&app).map_err(|e| e.to_string())?;

    // 读取现有的服务器（如果存在）
    let existing_server = {
        let servers = state.db.get_all_mcp_servers().map_err(|e| e.to_string())?;
        servers.get(&id).cloned()
    };

    // 构建新的统一服务器结构
    let mut new_server = if let Some(mut existing) = existing_server {
        // 更新现有服务器
        existing.server = spec.clone();
        existing.apps.set_enabled_for(&app_ty, true);
        existing
    } else {
        // 创建新服务器
        let mut apps = crate::app_config::McpApps::default();
        apps.set_enabled_for(&app_ty, true);

        // 尝试从 spec 中提取 name，否则使用 id
        let name = spec
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        McpServer {
            id: id.clone(),
            name,
            server: spec,
            apps,
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        }
    };

    // 如果 sync_other_side 为 true，也启用其他应用
    if sync_other_side.unwrap_or(false) {
        new_server.apps.claude = true;
        new_server.apps.codex = true;
        new_server.apps.gemini = true;
        new_server.apps.opencode = true;
    }

    McpService::upsert_server(&state, new_server)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

/// 在 config.json 中删除一个 MCP 服务器定义
#[tauri::command]
pub async fn delete_mcp_server_in_config(
    state: State<'_, AppState>,
    _app: String, // 参数保留用于向后兼容，但在统一结构中不再需要
    id: String,
) -> Result<bool, String> {
    McpService::delete_server(&state, &id).map_err(|e| e.to_string())
}

/// 设置启用状态并同步到客户端配置
#[tauri::command]
#[allow(deprecated)] // 兼容层命令，内部调用已废弃的 Service 方法
pub async fn set_mcp_enabled(
    state: State<'_, AppState>,
    app: String,
    id: String,
    enabled: bool,
) -> Result<bool, String> {
    let app_ty = AppType::from_str(&app).map_err(|e| e.to_string())?;
    McpService::set_enabled(&state, app_ty, &id, enabled).map_err(|e| e.to_string())
}

// ============================================================================
// v3.7.0 新增：统一 MCP 管理命令
// ============================================================================

use crate::app_config::McpServer;

/// 获取所有 MCP 服务器（统一结构）
#[tauri::command]
pub async fn get_mcp_servers(
    state: State<'_, AppState>,
) -> Result<IndexMap<String, McpServer>, String> {
    McpService::get_all_servers(&state).map_err(|e| e.to_string())
}

/// 添加或更新 MCP 服务器
#[tauri::command]
pub async fn upsert_mcp_server(
    state: State<'_, AppState>,
    server: McpServer,
) -> Result<(), String> {
    McpService::upsert_server(&state, server).map_err(|e| e.to_string())
}

/// 删除 MCP 服务器
#[tauri::command]
pub async fn delete_mcp_server(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    McpService::delete_server(&state, &id).map_err(|e| e.to_string())
}

/// 切换 MCP 服务器在指定应用的启用状态
#[tauri::command]
pub async fn toggle_mcp_app(
    state: State<'_, AppState>,
    server_id: String,
    app: String,
    enabled: bool,
) -> Result<(), String> {
    let app_ty = AppType::from_str(&app).map_err(|e| e.to_string())?;
    McpService::toggle_app(&state, &server_id, app_ty, enabled).map_err(|e| e.to_string())
}

/// 从所有应用导入 MCP 服务器（复用已有的导入逻辑）
#[tauri::command]
pub async fn import_mcp_from_apps(state: State<'_, AppState>) -> Result<usize, String> {
    let mut total = 0;
    total += McpService::import_from_claude(&state).unwrap_or(0);
    total += McpService::import_from_codex(&state).unwrap_or(0);
    total += McpService::import_from_gemini(&state).unwrap_or(0);
    total += McpService::import_from_opencode(&state).unwrap_or(0);
    total += McpService::import_from_hermes(&state).unwrap_or(0);
    Ok(total)
}

// ============================================================================
// v3.17.0 新增：MCP 注册表发现
// ============================================================================

use crate::mcp_registry::{self, RegistryListResponse, RegistryServerDetail};

/// 搜索/浏览 MCP 注册表服务器列表
#[tauri::command]
pub async fn search_mcp_registry(
    query: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<RegistryListResponse, String> {
    mcp_registry::search_servers(
        query.as_deref(),
        cursor.as_deref(),
        limit,
    )
    .await
    .map_err(|e| e.to_string())
}

/// 获取 MCP 注册表中单个服务器详情
#[tauri::command]
pub async fn get_mcp_registry_server(name: String) -> Result<RegistryServerDetail, String> {
    mcp_registry::get_server_detail(&name)
        .await
        .map_err(|e| e.to_string())
}

/// 从注册表安装 MCP 服务器到统一管理
#[tauri::command]
pub async fn install_mcp_from_registry(
    state: State<'_, AppState>,
    name: String,
    enabled_apps: crate::app_config::McpApps,
) -> Result<McpServer, String> {
    // 1. 从注册表获取服务器详情
    let detail = mcp_registry::get_server_detail(&name)
        .await
        .map_err(|e| e.to_string())?;

    // 2. 映射为 McpServer
    let server = mcp_registry::registry_to_mcp_server(&detail.server, &enabled_apps)
        .map_err(|e| e.to_string())?;

    // 3. 保存到数据库
    McpService::upsert_server(&state, server.clone())
        .map_err(|e| e.to_string())?;

    Ok(server)
}

// ============================================================================
// v3.17.0 新增：MCP 连接测试
// ============================================================================

use crate::mcp_connection_test::{McpConnectionTestResult, test_http_connection, test_sse_connection, test_stdio_connection};

/// 测试 MCP 服务器连接
/// 根据服务器类型自动选择测试方式：
/// - http/sse：发送 initialize JSON-RPC 请求
/// - stdio：启动子进程并通信
#[tauri::command]
pub async fn test_mcp_connection(
    server_spec: serde_json::Value,
) -> Result<McpConnectionTestResult, String> {
    let typ = server_spec
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("stdio");

    match typ {
        "http" | "streamable-http" => {
            let url = server_spec
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or("");
            if url.is_empty() {
                return Err("HTTP 类型缺少 url 字段".into());
            }
            let headers = server_spec
                .get("headers")
                .and_then(|h| h.as_object())
                .map(|obj| {
                    let mut map = std::collections::HashMap::new();
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            map.insert(k.clone(), s.to_string());
                        }
                    }
                    map
                });
            test_http_connection(url, headers.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
        "sse" => {
            let url = server_spec
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or("");
            if url.is_empty() {
                return Err("SSE 类型缺少 url 字段".into());
            }
            let headers = server_spec
                .get("headers")
                .and_then(|h| h.as_object())
                .map(|obj| {
                    let mut map = std::collections::HashMap::new();
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            map.insert(k.clone(), s.to_string());
                        }
                    }
                    map
                });
            test_sse_connection(url, headers.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
        _ => {
            // stdio（默认）
            let command = server_spec
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let args: Vec<String> = server_spec
                .get("args")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let env_vars = server_spec
                .get("env")
                .and_then(|e| e.as_object())
                .map(|obj| {
                    let mut map = std::collections::HashMap::new();
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            map.insert(k.clone(), s.to_string());
                        }
                    }
                    map
                });
            test_stdio_connection(command, &args, env_vars.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
    }
}
