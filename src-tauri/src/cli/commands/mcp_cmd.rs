//! `os mcp` — MCP 服务器连接测试与管理
//!
//! 支持测试 stdio/SSE/HTTP 连接，以及列出已配置的 MCP 服务器。

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub action: McpAction,
}

#[derive(Subcommand)]
pub enum McpAction {
    /// 测试 MCP 服务器连通性
    Test {
        /// MCP 服务器 ID（从数据库读取配置）
        #[arg(long)]
        server_id: Option<String>,
        /// 直接指定 stdio 命令
        #[arg(long)]
        command: Option<String>,
        /// 直接指定 SSE URL
        #[arg(long)]
        url: Option<String>,
    },
    /// 列出所有 MCP 服务器
    List,
}

pub fn run(
    args: McpArgs,
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    match args.action {
        McpAction::Test {
            server_id,
            command,
            url,
        } => run_test(state, server_id, command, url, json),
        McpAction::List => run_list(state, json),
    }
}

fn run_list(
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    let servers = state
        .db
        .get_all_mcp_servers()
        .map_err(|e| e.to_string())?;

    if json {
        let items: Vec<_> = servers
            .values()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "server": s.server,
                    "description": s.description,
                    "apps": {
                        "claude": s.apps.claude,
                        "codex": s.apps.codex,
                        "gemini": s.apps.gemini,
                        "opencode": s.apps.opencode,
                        "hermes": s.apps.hermes,
                    },
                })
            })
            .collect();
        output::print_result(&items, true);
    } else {
        if servers.is_empty() {
            output::info("No MCP servers configured.");
            return Ok(());
        }

        output::header(&format!("MCP Servers ({} total):", servers.len()));
        eprintln!();
        println!(
            "  {:<24} {:<16} {:<8} {:<8} {:<8}",
            "NAME", "TYPE", "CLAUDE", "CODEX", "GEMINI"
        );
        println!("  {}", "-".repeat(70));
        for server in servers.values() {
            let server_type = if server.server.get("command").is_some() {
                "stdio"
            } else if server.server.get("url").is_some() {
                "sse"
            } else {
                "unknown"
            };
            let claude = if server.apps.claude { "✓" } else { "·" };
            let codex = if server.apps.codex { "✓" } else { "·" };
            let gemini = if server.apps.gemini { "✓" } else { "·" };
            println!(
                "  {:<24} {:<16} {:<8} {:<8} {:<8}",
                server.name, server_type, claude, codex, gemini
            );
        }
    }

    Ok(())
}

fn run_test(
    state: &open_sunstar_lib::AppState,
    server_id: Option<String>,
    command: Option<String>,
    url: Option<String>,
    json: bool,
) -> Result<(), String> {
    // Interactive select for server_id when nothing specified
    let server_id = if server_id.is_none() && command.is_none() && url.is_none() && !json {
        let servers = state
            .db
            .get_all_mcp_servers()
            .map_err(|e| e.to_string())?;
        if servers.is_empty() {
            return Err("No MCP servers configured. Use --command or --url to test directly.".to_string());
        }
        let items: Vec<String> = servers
            .values()
            .map(|s| format!("{} ({})", s.name, s.id))
            .collect();
        match output::select("Select MCP server to test", &items, false) {
            Some(idx) => servers.values().nth(idx).map(|s| s.id.clone()),
            None => None,
        }
    } else {
        server_id
    };

    // 确定测试目标
    let test_mode = if let Some(ref sid) = server_id {
        // 从数据库读取服务器配置
        let servers = state
            .db
            .get_all_mcp_servers()
            .map_err(|e| e.to_string())?;
        let server = servers
            .get(sid)
            .ok_or_else(|| format!("MCP 服务器不存在: {sid}"))?;

        if let Some(cmd) = server.server.get("command").and_then(|v| v.as_str()) {
            let args = server.server.get("args").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
            let env_vars = server.server.get("env").and_then(|v| v.as_object()).map(|o| o.iter().map(|(k,v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect());
            TestMode::Stdio {
                command: cmd.to_string(),
                args,
                env_vars,
            }
        } else if let Some(server_url) = server.server.get("url").and_then(|v| v.as_str()) {
            TestMode::Sse {
                url: server_url.to_string(),
                headers: server.server.get("headers").and_then(|v| v.as_object()).map(|o| o.iter().map(|(k,v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()),
            }
        } else {
            return Err(format!("MCP 服务器 '{sid}' 没有配置 command 或 url"));
        }
    } else if let Some(ref cmd) = command {
        TestMode::Stdio {
            command: cmd.clone(),
            args: vec![],
            env_vars: None,
        }
    } else if let Some(ref u) = url {
        TestMode::Sse {
            url: u.clone(),
            headers: None,
        }
    } else {
        return Err("请指定 --server-id、--command 或 --url".to_string());
    };

    // MCP 连接测试函数在 mcp_connection_test 模块中（私有），尚未通过 cli_api 暴露
    // 暂时输出配置信息，提示用户在 GUI 中测试
    match &test_mode {
        TestMode::Stdio {
            command,
            args,
            env_vars,
        } => {
            output::warning("MCP 连接测试功能需要完整的运行时环境。");
            output::info("请在 OpenSunstar GUI 中测试 MCP 连接。");
            let server_spec = serde_json::json!({
                "type": "stdio",
                "command": command,
                "args": args,
                "env": env_vars,
            });
            println!("服务器配置: {}", serde_json::to_string_pretty(&server_spec).unwrap_or_default());
        }
        TestMode::Sse { url, headers } => {
            output::warning("MCP 连接测试功能需要完整的运行时环境。");
            output::info("请在 OpenSunstar GUI 中测试 MCP 连接。");
            let server_spec = serde_json::json!({
                "type": "sse",
                "url": url,
                "headers": headers,
            });
            println!("服务器配置: {}", serde_json::to_string_pretty(&server_spec).unwrap_or_default());
        }
    }

    Ok(())
}

enum TestMode {
    Stdio {
        command: String,
        args: Vec<String>,
        env_vars: Option<std::collections::HashMap<String, String>>,
    },
    Sse {
        url: String,
        headers: Option<std::collections::HashMap<String, String>>,
    },
}

/// 打印 MCP 测试结果（serde_json::Value 形式）
///
/// 预期 JSON 结构:
/// ```json
/// {
///   "status": "connected" | "auth_required" | "unreachable" | "timeout" | ...,
///   "message": "...",
///   "server_info": { "name": "...", "version": "..." } | null,
///   "error_detail": "..." | null
/// }
/// ```
#[allow(dead_code)]
fn print_test_result_value(result: &serde_json::Value) {
    let status = result
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let icon = match status {
        "connected" | "Connected" => "✓",
        "auth_required" | "AuthRequired" => "⚠",
        _ => "✗",
    };

    let message = result
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown status");

    println!("{icon} Status: {message}");

    if let Some(info) = result.get("server_info") {
        if let Some(name) = info.get("name").and_then(|v| v.as_str()) {
            println!("  Server: {name}");
        }
        if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
            println!("  Version: {version}");
        }
    }

    if let Some(detail) = result.get("error_detail").and_then(|v| v.as_str()) {
        println!("  Error: {detail}");
    }
}
