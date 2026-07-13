//! MCP 服务器连接测试模块
//!
//! 根据 MCP 协议规范，通过发送 JSON-RPC 2.0 `initialize` 请求来测试连接：
//! - HTTP/SSE 类型：POST 请求 + 状态码判断
//! - stdio 类型：spawn 子进程 + stdin/stdout 通信
//!
//! 返回统一的状态结果供前端展示。

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

const TEST_TIMEOUT_SECS: u64 = 8;

// ───────────────────── 测试结果类型 ─────────────────────

/// 连接测试状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionStatus {
    /// 连接成功，服务器正常响应
    Connected,
    /// 需要身份验证（HTTP 401/403）
    AuthRequired,
    /// 无法连接到服务器（连接拒绝/DNS 解析失败等）
    Unreachable,
    /// 连接超时
    Timeout,
    /// 服务器返回了非预期的响应（不是有效的 JSON-RPC）
    UnexpectedResponse,
    /// 命令无法执行（stdio 类型，命令不存在或启动失败）
    CommandFailed,
}

/// 连接测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnectionTestResult {
    pub status: McpConnectionStatus,
    /// 人类可读的状态消息
    pub message: String,
    /// 服务器返回的信息（连接成功时）
    pub server_info: Option<McpServerInfo>,
    /// 错误详情（失败时）
    pub error_detail: Option<String>,
}

/// MCP 服务器信息（从 initialize 响应中提取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: Option<String>,
    pub version: Option<String>,
}

// ───────────────────── JSON-RPC initialize 请求 ─────────────────────

fn build_initialize_request() -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "OpenSunstar",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}

fn parse_initialize_response(body: &str) -> Result<McpServerInfo, AppError> {
    let resp: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::Serialization(format!("JSON 解析失败: {e}")))?;

    if let Some(err) = resp.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("未知错误");
        return Err(AppError::Message(format!("服务器返回错误: {msg}")));
    }

    let result = resp
        .get("result")
        .ok_or_else(|| AppError::Message("响应中缺少 result 字段".into()))?;

    let server_info = McpServerInfo {
        name: result
            .get("serverInfo")
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from),
        version: result
            .get("serverInfo")
            .and_then(|s| s.get("version"))
            .and_then(|v| v.as_str())
            .map(String::from),
    };

    Ok(server_info)
}

// ───────────────────── HTTP/SSE 连接测试 ─────────────────────

pub async fn test_http_connection(
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<McpConnectionTestResult, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(TEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Network(format!("构建 HTTP 客户端失败: {e}")))?;

    let init_request = build_initialize_request();
    let body_str = serde_json::to_string(&init_request)
        .map_err(|e| AppError::Serialization(format!("序列化失败: {e}")))?;

    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body_str);

    // 添加用户配置的 headers
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            req = req.header(key.as_str(), value.as_str());
        }
    }

    // 发送请求
    let response = match timeout(Duration::from_secs(TEST_TIMEOUT_SECS), req.send()).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            let status = if e.is_timeout() {
                McpConnectionStatus::Timeout
            } else if e.is_connect() {
                McpConnectionStatus::Unreachable
            } else {
                McpConnectionStatus::Unreachable
            };
            let msg = status_msg(&status);
            return Ok(McpConnectionTestResult {
                status,
                message: msg,
                server_info: None,
                error_detail: Some(format!("{e}")),
            });
        }
        Err(_) => {
            return Ok(McpConnectionTestResult {
                status: McpConnectionStatus::Timeout,
                message: status_msg(&McpConnectionStatus::Timeout),
                server_info: None,
                error_detail: Some("请求超时".into()),
            });
        }
    };

    let status_code = response.status();

    match status_code.as_u16() {
        200 => {
            let body = response.text().await.unwrap_or_default();
            match parse_initialize_response(&body) {
                Ok(server_info) => Ok(McpConnectionTestResult {
                    status: McpConnectionStatus::Connected,
                    message: status_msg(&McpConnectionStatus::Connected),
                    server_info: Some(server_info),
                    error_detail: None,
                }),
                Err(e) => {
                    // 即使响应解析失败，200 状态码也说明服务器可达
                    // 尝试判断是否是非 MCP 端点
                    Ok(McpConnectionTestResult {
                        status: McpConnectionStatus::Connected,
                        message: "服务器可达，但未返回标准 MCP initialize 响应".into(),
                        server_info: None,
                        error_detail: Some(format!("{e}")),
                    })
                }
            }
        }
        401 | 403 => Ok(McpConnectionTestResult {
            status: McpConnectionStatus::AuthRequired,
            message: status_msg(&McpConnectionStatus::AuthRequired),
            server_info: None,
            error_detail: Some(format!("HTTP {status_code}")),
        }),
        404 => Ok(McpConnectionTestResult {
            status: McpConnectionStatus::Unreachable,
            message: "端点不存在 (404)".into(),
            server_info: None,
            error_detail: Some(format!("HTTP {status_code}")),
        }),
        _ => Ok(McpConnectionTestResult {
            status: McpConnectionStatus::UnexpectedResponse,
            message: format!("服务器返回异常状态码: {status_code}"),
            server_info: None,
            error_detail: Some(format!("HTTP {status_code}")),
        }),
    }
}

pub async fn test_sse_connection(
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<McpConnectionTestResult, AppError> {
    // SSE 传输：先尝试 GET 连接获取 endpoint，再 POST initialize
    // 简化处理：尝试直接对 URL 发 POST（许多 SSE 实现也接受 POST）
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(TEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Network(format!("构建 HTTP 客户端失败: {e}")))?;

    // 先尝试 GET 检测 SSE endpoint 是否存在
    let mut get_req = client.get(url).header("Accept", "text/event-stream");
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            get_req = get_req.header(key.as_str(), value.as_str());
        }
    }

    let sse_check = timeout(Duration::from_secs(5), get_req.send()).await;

    match sse_check {
        Ok(Ok(resp)) => {
            let status = resp.status();
            if status.as_u16() == 200 {
                // SSE endpoint 存在，尝试 POST JSON-RPC 来实际验证
                // 许多 SSE MCP 服务器的 POST endpoint 是同一 URL
                return test_http_connection(url, headers).await;
            }
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Ok(McpConnectionTestResult {
                    status: McpConnectionStatus::AuthRequired,
                    message: status_msg(&McpConnectionStatus::AuthRequired),
                    server_info: None,
                    error_detail: Some(format!("SSE GET HTTP {status}")),
                });
            }
            // 其他状态码，仍然尝试 POST
            return test_http_connection(url, headers).await;
        }
        Ok(Err(e)) => {
            if e.is_connect() {
                return Ok(McpConnectionTestResult {
                    status: McpConnectionStatus::Unreachable,
                    message: status_msg(&McpConnectionStatus::Unreachable),
                    server_info: None,
                    error_detail: Some(format!("{e}")),
                });
            }
            // 网络错误，仍然尝试 POST
            return test_http_connection(url, headers).await;
        }
        Err(_) => {
            return Ok(McpConnectionTestResult {
                status: McpConnectionStatus::Timeout,
                message: status_msg(&McpConnectionStatus::Timeout),
                server_info: None,
                error_detail: Some("SSE 检测超时".into()),
            });
        }
    }
}

// ───────────────────── stdio 连接测试 ─────────────────────

pub async fn test_stdio_connection(
    command: &str,
    args: &[String],
    env_vars: Option<&std::collections::HashMap<String, String>>,
) -> Result<McpConnectionTestResult, AppError> {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::Command;

    if command.trim().is_empty() {
        return Ok(McpConnectionTestResult {
            status: McpConnectionStatus::CommandFailed,
            message: "命令为空".into(),
            server_info: None,
            error_detail: None,
        });
    }

    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // 设置环境变量
    if let Some(env_vars) = env_vars {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    // 尝试启动进程
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let detail = format!("无法启动命令 '{}': {}", command, e);
            return Ok(McpConnectionTestResult {
                status: McpConnectionStatus::CommandFailed,
                message: status_msg(&McpConnectionStatus::CommandFailed),
                server_info: None,
                error_detail: Some(detail),
            });
        }
    };

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Message("无法获取子进程 stdin".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Message("无法获取子进程 stdout".into()))?;
    let stderr = child.stderr.take();

    // 构建 initialize 请求（带换行符，MCP stdio 协议要求）
    let init_request = build_initialize_request();
    let request_str = serde_json::to_string(&init_request)
        .map_err(|e| AppError::Serialization(format!("序列化失败: {e}")))?;
    let request_with_newline = format!("{}\n", request_str);

    // 写入请求到 stdin
    let write_result = timeout(
        Duration::from_secs(3),
        stdin.write_all(request_with_newline.as_bytes()),
    )
    .await;

    if write_result.is_err() {
        // 超时或写入失败
        let _ = child.kill().await;
        return Ok(McpConnectionTestResult {
            status: McpConnectionStatus::CommandFailed,
            message: "向进程写入请求超时".into(),
            server_info: None,
            error_detail: None,
        });
    }

    if let Err(e) = write_result.unwrap() {
        let _ = child.kill().await;
        return Ok(McpConnectionTestResult {
            status: McpConnectionStatus::CommandFailed,
            message: format!("向进程写入失败: {e}"),
            server_info: None,
            error_detail: Some(format!("{e}")),
        });
    }

    // 释放 stdin 以让进程开始处理
    drop(stdin);

    // 读取响应（带超时）
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    let read_result = timeout(
        Duration::from_secs(TEST_TIMEOUT_SECS),
        reader.read_line(&mut line),
    )
    .await;

    // 无论如何先 kill 进程
    let _ = child.kill().await;
    // 也读一下 stderr 用于诊断
    let stderr_output = if let Some(stderr) = stderr {
        let mut stderr_reader = BufReader::new(stderr);
        let mut err_line = String::new();
        let _ = timeout(
            Duration::from_secs(1),
            stderr_reader.read_line(&mut err_line),
        )
        .await;
        err_line
    } else {
        String::new()
    };

    match read_result {
        Ok(Ok(n)) if n > 0 => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Ok(McpConnectionTestResult {
                    status: McpConnectionStatus::Connected,
                    message: "进程启动成功（响应为空，可能需要额外参数）".into(),
                    server_info: None,
                    error_detail: if stderr_output.is_empty() {
                        None
                    } else {
                        Some(stderr_output)
                    },
                });
            }

            match parse_initialize_response(trimmed) {
                Ok(server_info) => Ok(McpConnectionTestResult {
                    status: McpConnectionStatus::Connected,
                    message: status_msg(&McpConnectionStatus::Connected),
                    server_info: Some(server_info),
                    error_detail: None,
                }),
                Err(_) => {
                    // JSON 解析失败，但仍然说明进程可以通信
                    Ok(McpConnectionTestResult {
                        status: McpConnectionStatus::Connected,
                        message: "进程响应成功（非标准 MCP 响应格式）".into(),
                        server_info: None,
                        error_detail: Some(format!(
                            "响应: {}",
                            trimmed.chars().take(200).collect::<String>()
                        )),
                    })
                }
            }
        }
        Ok(Ok(_)) => {
            // 读取到 0 字节
            Ok(McpConnectionTestResult {
                status: McpConnectionStatus::CommandFailed,
                message: "进程未返回任何数据".into(),
                server_info: None,
                error_detail: if stderr_output.is_empty() {
                    None
                } else {
                    Some(stderr_output)
                },
            })
        }
        Ok(Err(e)) => Ok(McpConnectionTestResult {
            status: McpConnectionStatus::CommandFailed,
            message: format!("读取进程输出失败: {e}"),
            server_info: None,
            error_detail: if stderr_output.is_empty() {
                Some(format!("{e}"))
            } else {
                Some(format!("stderr: {} | stdout err: {}", stderr_output, e))
            },
        }),
        Err(_) => Ok(McpConnectionTestResult {
            status: McpConnectionStatus::Timeout,
            message: "进程响应超时".into(),
            server_info: None,
            error_detail: if stderr_output.is_empty() {
                None
            } else {
                Some(stderr_output)
            },
        }),
    }
}

// ───────────────────── 辅助函数 ─────────────────────

fn status_msg(status: &McpConnectionStatus) -> String {
    match status {
        McpConnectionStatus::Connected => "已连接".into(),
        McpConnectionStatus::AuthRequired => "需要身份验证".into(),
        McpConnectionStatus::Unreachable => "无法连接".into(),
        McpConnectionStatus::Timeout => "连接超时".into(),
        McpConnectionStatus::UnexpectedResponse => "服务器返回异常响应".into(),
        McpConnectionStatus::CommandFailed => "命令执行失败".into(),
    }
}
