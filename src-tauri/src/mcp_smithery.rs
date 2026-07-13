//! Smithery Registry API 集成模块
//!
//! 封装对 Smithery Registry API 的 HTTP 调用：
//! - GET /servers?page=N&pageSize=N&verified=true&remote=false
//! - GET /servers/{qualifiedName}  （详情，含 connections/tools）
//!
//! 参考：https://smithery.ai / https://registry.smithery.ai

use crate::error::AppError;
use serde::{Deserialize, Serialize};

const SMITHERY_BASE_URL: &str = "https://registry.smithery.ai";

// ───────────────────── Smithery API 响应类型 ─────────────────────

/// 列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmitheryListResponse {
    pub servers: Vec<SmitheryServer>,
    pub pagination: SmitheryPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmitheryPagination {
    #[serde(rename = "currentPage")]
    pub current_page: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
    #[serde(rename = "totalCount")]
    pub total_count: u32,
}

/// 列表中的单个服务器（摘要信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmitheryServer {
    pub id: String,
    #[serde(rename = "qualifiedName")]
    pub qualified_name: String,
    pub namespace: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "iconUrl")]
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub verified: bool,
    #[serde(rename = "useCount")]
    #[serde(default)]
    pub use_count: u32,
    #[serde(default)]
    pub remote: bool,
    #[serde(rename = "isDeployed")]
    #[serde(default)]
    pub is_deployed: bool,
    #[serde(rename = "createdAt")]
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(rename = "bySmithery")]
    #[serde(default)]
    pub by_smithery: bool,
}

/// 详情响应（包含 connections + tools）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmitheryServerDetail {
    #[serde(rename = "qualifiedName")]
    pub qualified_name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "iconUrl")]
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub remote: bool,
    #[serde(rename = "deploymentUrl")]
    #[serde(default)]
    pub deployment_url: Option<String>,
    #[serde(default)]
    pub connections: Vec<SmitheryConnection>,
    #[serde(default)]
    pub tools: Vec<SmitheryTool>,
    #[serde(default)]
    pub resources: Vec<serde_json::Value>,
    #[serde(default)]
    pub prompts: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmitheryConnection {
    #[serde(rename = "type")]
    pub conn_type: String,
    #[serde(rename = "deploymentUrl")]
    #[serde(default)]
    pub deployment_url: Option<String>,
    #[serde(rename = "bundleUrl")]
    #[serde(default)]
    pub bundle_url: Option<String>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(rename = "configSchema")]
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmitheryTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
}

// ───────────────────── HTTP 调用函数 ─────────────────────

/// 搜索/列出 Smithery 服务器
///
/// - `page`: 页码（从 1 开始）
/// - `page_size`: 每页数量（默认 50，最大 100）
/// - `verified`: 仅返回认证服务器
/// - `remote_filter`: Some(true)=仅远程, Some(false)=仅本地 stdio, None=全部
pub async fn search_servers(
    page: Option<u32>,
    page_size: Option<u32>,
    verified: Option<bool>,
    remote_filter: Option<bool>,
) -> Result<SmitheryListResponse, AppError> {
    let client = reqwest::Client::new();
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(50).min(100);

    let mut url = format!(
        "{}/servers?page={}&pageSize={}",
        SMITHERY_BASE_URL, page, page_size
    );

    if let Some(v) = verified {
        if v {
            url.push_str("&verified=true");
        }
    }

    if let Some(r) = remote_filter {
        if r {
            url.push_str("&remote=true");
        } else {
            url.push_str("&remote=false");
        }
    }

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Smithery Registry 请求失败: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Network(format!(
            "Smithery Registry 返回错误 ({}): {}",
            status, body
        )));
    }

    let list: SmitheryListResponse = response
        .json()
        .await
        .map_err(|e| AppError::Serialization(format!("解析 Smithery 响应失败: {e}")))?;

    Ok(list)
}

/// 获取单个 Smithery 服务器详情
pub async fn get_server_detail(qualified_name: &str) -> Result<SmitheryServerDetail, AppError> {
    let client = reqwest::Client::new();

    let url = format!(
        "{}/servers/{}",
        SMITHERY_BASE_URL,
        urlencoding(qualified_name)
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Smithery Registry 请求失败: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Network(format!(
            "Smithery Registry 返回错误 ({}): {}",
            status, body
        )));
    }

    let detail: SmitheryServerDetail = response
        .json()
        .await
        .map_err(|e| AppError::Serialization(format!("解析 Smithery 响应失败: {e}")))?;

    Ok(detail)
}

// ───────────────────── 数据映射工具 ─────────────────────

/// 将 Smithery 服务器信息映射为 McpServer 结构
///
/// - Remote 服务器：使用 deploymentUrl 作为 http/sse 连接地址
/// - Stdio 服务器：使用 bundleUrl + runtime 构建 npx 命令
pub fn smithery_to_mcp_server(
    detail: &SmitheryServerDetail,
    enabled_apps: &crate::app_config::McpApps,
) -> Result<crate::app_config::McpServer, AppError> {
    // 生成唯一 id
    let id = detail
        .qualified_name
        .replace(['/', '@', '.'], "-")
        .trim_matches('-')
        .to_string();

    let display_name = detail.display_name.clone();

    // 确定连接方式
    let connection = detail.connections.first();

    let server_spec = if detail.remote {
        // Remote 服务器：使用 deploymentUrl
        let url = detail
            .deployment_url
            .as_deref()
            .or_else(|| connection.and_then(|c| c.deployment_url.as_deref()))
            .unwrap_or("");

        serde_json::json!({
            "type": "http",
            "url": url,
        })
    } else {
        // Stdio 服务器：使用 npx + @smithery-ai/cli
        // 典型命令：npx -y @smithery/cli run <qualifiedName>
        let qn = &detail.qualified_name;
        serde_json::json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "@smithery/cli", "run", qn],
        })
    };

    let server = crate::app_config::McpServer {
        id,
        name: display_name,
        server: server_spec,
        apps: enabled_apps.clone(),
        description: detail.description.clone(),
        homepage: None, // Smithery detail 不包含 homepage
        docs: None,
        tags: Vec::new(),
    };

    Ok(server)
}

/// 从列表摘要数据直接映射（无需请求详情，适合批量安装场景）
pub fn smithery_summary_to_mcp_server(
    server: &SmitheryServer,
    enabled_apps: &crate::app_config::McpApps,
) -> crate::app_config::McpServer {
    let id = server
        .qualified_name
        .replace(['/', '@', '.'], "-")
        .trim_matches('-')
        .to_string();

    let server_spec = if server.remote {
        // Remote：使用 Smithery 托管的 SSE 端点
        let url = format!("https://smithery.ai/server/{}/sse", server.qualified_name);
        serde_json::json!({
            "type": "sse",
            "url": url,
        })
    } else {
        // Stdio：npx -y @smithery/cli run <qualifiedName>
        serde_json::json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "@smithery/cli", "run", &server.qualified_name],
        })
    };

    crate::app_config::McpServer {
        id,
        name: server.display_name.clone(),
        server: server_spec,
        apps: enabled_apps.clone(),
        description: server.description.clone(),
        homepage: server.homepage.clone(),
        docs: None,
        tags: Vec::new(),
    }
}

/// URL 编码辅助函数
fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b'/' => {
                result.push_str("%2F");
            }
            b' ' => {
                result.push_str("%20");
            }
            b':' => {
                result.push_str("%3A");
            }
            b'@' => {
                result.push_str("%40");
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
