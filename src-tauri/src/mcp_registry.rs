//! MCP Registry API 集成模块
//!
//! 封装对官方 MCP 注册表 API 的 HTTP 调用：
//! - GET /v0.1/servers?limit=100&cursor=...&version=latest
//! - GET /v0.1/servers/{name}/versions/latest
//!
//! 参考：https://registry.modelcontextprotocol.io/

use crate::error::AppError;
use serde::{Deserialize, Serialize};

const REGISTRY_BASE_URL: &str = "https://registry.modelcontextprotocol.io";

// ───────────────────── Registry API 响应类型 ─────────────────────

/// 注册表服务器列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryListResponse {
    pub servers: Vec<RegistryServerEntry>,
    pub metadata: RegistryListMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryListMetadata {
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
    pub count: Option<u32>,
}

/// 注册表单个服务器条目（含元信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryServerEntry {
    pub server: RegistryServer,
    #[serde(rename = "_meta")]
    pub meta: Option<serde_json::Value>,
}

/// 注册表服务器详情响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryServerDetail {
    pub server: RegistryServer,
    #[serde(rename = "_meta")]
    pub meta: Option<serde_json::Value>,
}

/// 注册表中服务器的核心字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryServer {
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub remotes: Vec<RegistryRemote>,
    #[serde(default)]
    pub repository: Option<RegistryRepository>,
    #[serde(rename = "websiteUrl")]
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    // 其他动态字段保留在 extras 中
    #[serde(flatten)]
    pub extras: serde_json::Value,
}

/// 远程连接方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryRemote {
    #[serde(rename = "type")]
    pub remote_type: String,
    pub url: String,
}

/// 仓库信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryRepository {
    pub url: Option<String>,
    pub source: Option<String>,
}

// ───────────────────── HTTP 调用函数 ─────────────────────

/// 搜索/列出注册表服务器
pub async fn search_servers(
    query: Option<&str>,
    cursor: Option<&str>,
    limit: Option<u32>,
) -> Result<RegistryListResponse, AppError> {
    let client = reqwest::Client::new();
    let limit = limit.unwrap_or(50).min(100);

    let mut url = format!(
        "{}/v0.1/servers?limit={}&version=latest",
        REGISTRY_BASE_URL, limit
    );

    // 如果传入了查询关键词，先尝试通过 registry API 搜索
    // 注意：registry API 可能不直接支持搜索，这里先获取列表，前端做客户端过滤
    // 如果有 cursor，添加到 URL
    if let Some(c) = cursor {
        if !c.is_empty() {
            url.push_str(&format!("&cursor={}", urlencoding(&c)));
        }
    }

    // 如果提供了 query 且 registry 支持 search 参数
    if let Some(q) = query {
        if !q.is_empty() {
            url.push_str(&format!("&search={}", urlencoding(q)));
        }
    }

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Network(format!("MCP Registry 请求失败: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Network(format!(
            "MCP Registry 返回错误 ({}): {}",
            status, body
        )));
    }

    let list: RegistryListResponse = response
        .json()
        .await
        .map_err(|e| AppError::Serialization(format!("解析注册表响应失败: {e}")))?;

    Ok(list)
}

/// 获取单个服务器详情
pub async fn get_server_detail(name: &str) -> Result<RegistryServerDetail, AppError> {
    let client = reqwest::Client::new();
    // URL 编码 server name（其中的 / 需编码为 %2F）
    let encoded_name = urlencoding(name);

    let url = format!(
        "{}/v0.1/servers/{}/versions/latest",
        REGISTRY_BASE_URL, encoded_name
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Network(format!("MCP Registry 请求失败: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Network(format!(
            "MCP Registry 返回错误 ({}): {}",
            status, body
        )));
    }

    let detail: RegistryServerDetail = response
        .json()
        .await
        .map_err(|e| AppError::Serialization(format!("解析注册表响应失败: {e}")))?;

    Ok(detail)
}

// ───────────────────── 数据映射工具 ─────────────────────

/// 将注册表服务器信息映射为 McpServer 结构
/// 优先使用 streamable-http 类型的 remote，因为最通用
pub fn registry_to_mcp_server(
    registry: &RegistryServer,
    enabled_apps: &crate::app_config::McpApps,
) -> Result<crate::app_config::McpServer, AppError> {
    // 生成唯一 id：将 name 中的 / 替换为 -
    let id = registry
        .name
        .replace(['/', '@', '.'], "-")
        .trim_matches('-')
        .to_string();

    // 显示名称：优先 title，否则用 name 的最后一段
    let display_name = registry
        .title
        .clone()
        .unwrap_or_else(|| registry.name.clone());

    // 选择 remote：优先 streamable-http，其次 http
    let remote = registry
        .remotes
        .iter()
        .find(|r| r.remote_type == "streamable-http")
        .or_else(|| registry.remotes.first());

    let server_spec = if let Some(remote) = remote {
        // 确定 server type
        let server_type = match remote.remote_type.as_str() {
            "sse" => "sse",
            "streamable-http" => "http",
            _ => "http",
        };

        let mut spec = serde_json::json!({
            "type": server_type,
            "url": remote.url,
        });

        // 如果有多个 remotes，一并保留
        if registry.remotes.len() > 1 {
            if let serde_json::Value::Object(ref mut map) = spec {
                let remotes_array: Vec<serde_json::Value> = registry
                    .remotes
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "type": r.remote_type,
                            "url": r.url,
                        })
                    })
                    .collect();
                map.insert(
                    "remotes".to_string(),
                    serde_json::Value::Array(remotes_array),
                );
            }
        }

        spec
    } else {
        // 没有 remote，创建空的 stdio 占位
        serde_json::json!({
            "type": "stdio",
            "command": "",
            "args": [],
        })
    };

    let server = crate::app_config::McpServer {
        id,
        name: display_name,
        server: server_spec,
        apps: enabled_apps.clone(),
        description: registry.description.clone(),
        homepage: registry.website_url.clone(),
        docs: registry.repository.as_ref().and_then(|r| r.url.clone()),
        tags: if registry.tags.is_empty() {
            Vec::new()
        } else {
            registry.tags.clone()
        },
    };

    Ok(server)
}

/// URL 编码辅助函数（简单实现，不需要额外依赖）
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
