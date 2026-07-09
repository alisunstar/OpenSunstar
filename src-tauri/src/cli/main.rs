//! OpenSunstar CLI (`os`) — 治理与编排命令行工具
//!
//! 链接 `open_sunstar_lib` crate，复用 services 层，零重构验证楔子。
//! 架构路径: A+ ([[bin]] + AppState::new(db) 直调)

use clap::{Parser, Subcommand};
use std::process;

mod commands;
mod output;
#[cfg(feature = "tui")]
mod tui;

#[derive(Parser)]
#[command(
    name = "os",
    version,
    about = "OpenSunstar CLI — AI 编程助手治理与编排工具",
    long_about = "os 是 OpenSunstar 的治理与编排 CLI。\n\
                  让 AI 编程工作流的「就绪度、漂移、阶段门禁」可脚本化、可进 CI、可被 Agent 自愈。\n\n\
                  不带子命令时启动全屏 TUI 仪表盘（需要 --features tui 编译）。\n\n\
                  完整文档: https://github.com/alisunstar/OpenSunstar"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// 输出 JSON 格式（机器消费）
    #[arg(long, global = true)]
    pub json: bool,

    /// 操作超时（秒），防止 Agent 会话无限挂起
    #[arg(long, global = true)]
    pub timeout: Option<u64>,

    /// 禁用 TUI 仪表盘（强制 CLI 模式）
    #[arg(long, global = true)]
    pub no_tui: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 显示版本信息
    Version,

    /// 环境诊断：检查数据库、工具安装、配置目录
    Doctor(commands::doctor::DoctorArgs),

    /// 配置漂移治理（检查/修复）
    Drift(commands::drift::DriftArgs),

    /// Agent 就绪度评分
    Readiness(commands::readiness::ReadinessArgs),

    /// 工作流编排
    Flow(commands::flow::FlowArgs),

    /// Recipe 管理
    Recipe(commands::recipe::RecipeArgs),

    /// 设计合约
    Design(commands::design::DesignArgs),

    /// 项目蓝图/配置快照
    Profile(commands::profile::ProfileArgs),

    /// 项目管理
    Project(commands::project::ProjectArgs),

    /// 资产管理 (kubectl 风格)
    Asset(commands::asset::AssetArgs),

    /// MCP 连接测试
    Mcp(commands::mcp_cmd::McpArgs),

    /// 技能搜索
    Skill(commands::skill::SkillArgs),

    /// 供应商管理
    Provider(commands::provider::ProviderArgs),

    /// 跨设备同步
    Sync(commands::sync::SyncArgs),

    /// 配置导入/导出
    Config(commands::config::ConfigArgs),
}

/// CLI 错误：携带可选 hint（Agent-Native 契约）
struct CliError {
    message: String,
    hint: Option<&'static str>,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<String> for CliError {
    fn from(message: String) -> Self {
        CliError { message, hint: None }
    }
}

/// 初始化数据库连接，CLI 友好错误处理（携带 hint）
fn init_database() -> Result<open_sunstar_lib::Database, CliError> {
    open_sunstar_lib::cli_api::cli_ensure_database().map_err(|e| CliError {
        message: format!("数据库初始化失败: {e}"),
        hint: Some("运行 os config bootstrap 手动初始化，或检查 ~/.OpenSunstar 目录权限"),
    })
}

/// 初始化 AppState（数据库 + 服务层），CLI 友好错误处理
fn init_state() -> Result<open_sunstar_lib::AppState, CliError> {
    let db = init_database()?;
    Ok(open_sunstar_lib::AppState::new(std::sync::Arc::new(db)))
}

/// 初始化 AppState，失败时打印带 hint 的错误并退出
fn init_state_or_exit(json: bool) -> open_sunstar_lib::AppState {
    match init_state() {
        Ok(s) => s,
        Err(e) => {
            output::print_error_with_hint(&e.message, json, e.hint);
            process::exit(3);
        }
    }
}

/// 命令分发：将 Cli 执行结果统一为 Result<(), String>
fn run_command(command: Commands, json: bool) -> Result<(), String> {
    let result = match command {
        // ── 不需要数据库的命令 ──
        Commands::Version => commands::version::run(json),
        Commands::Doctor(args) => commands::doctor::run(args, json),
        Commands::Flow(args) => commands::flow::run(args, json),
        Commands::Recipe(args) => commands::recipe::run(args, json),
        Commands::Design(args) => commands::design::run(args, json),

        // ── 需要数据库的命令 ──
        Commands::Drift(args) => {
            let state = init_state_or_exit(json);
            commands::drift::run(args, &state, json)
        }
        Commands::Readiness(args) => {
            let state = init_state_or_exit(json);
            commands::readiness::run(args, &state, json)
        }
        Commands::Profile(args) => {
            let state = init_state_or_exit(json);
            commands::profile::run(args, &state, json)
        }
        Commands::Project(args) => {
            let state = init_state_or_exit(json);
            commands::project::run(args, &state, json)
        }
        Commands::Asset(args) => {
            let state = init_state_or_exit(json);
            commands::asset::run(args, &state, json)
        }
        Commands::Mcp(args) => {
            let state = init_state_or_exit(json);
            commands::mcp_cmd::run(args, &state, json)
        }
        Commands::Sync(args) => {
            let state = init_state_or_exit(json);
            commands::sync::run(args, &state, json)
        }

        // ── 部分子命令需要数据库的命令 ──

        // skill: Search 不需要 DB，List 需要 DB
        Commands::Skill(args) => {
            let needs_db = matches!(args.action, commands::skill::SkillAction::List);
            if needs_db {
                let state = init_state_or_exit(json);
                commands::skill::run_with_optional_state(args, Some(&state), json)
            } else {
                // Search: try to init DB but don't fail if unavailable
                let state = init_state().ok();
                commands::skill::run_with_optional_state(args, state.as_ref(), json)
            }
        }

        // provider: List/Switch 需要 DB，Verify 不需要
        Commands::Provider(args) => {
            let needs_db = !matches!(args.action, commands::provider::ProviderAction::Verify { .. });
            if needs_db {
                let state = init_state_or_exit(json);
                commands::provider::run(args, Some(&state), json)
            } else {
                commands::provider::run(args, None, json)
            }
        }

        // config: Path/Bootstrap 不需要 DB，Export/Import 需要
        Commands::Config(args) => {
            let needs_db = !matches!(
                args.action,
                commands::config::ConfigAction::Path
                    | commands::config::ConfigAction::Bootstrap { .. }
            );
            if needs_db {
                let state = init_state_or_exit(json);
                commands::config::run(args, Some(&state), json)
            } else {
                commands::config::run(args, None, json)
            }
        }
    };

    result
}

/// 带超时的命令执行（通过 channel + 子线程实现）
fn run_with_timeout<F>(f: F, timeout: std::time::Duration) -> Result<Result<(), String>, String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = f();
        let _ = tx.send(result);
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => Ok(result),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(format!("操作超时 ({}s)", timeout.as_secs()))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("操作线程异常退出".to_string())
        }
    }
}

fn main() {
    let cli = Cli::parse();

    // 初始化最小日志（stderr，不写文件）
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .format_timestamp(None)
        .init();

    let json = cli.json;
    let timeout = cli.timeout;
    let no_tui = cli.no_tui;
    let command = cli.command;

    // ── TUI Dashboard: 无子命令 + TTY + tui feature ──
    if command.is_none() && !no_tui && !json {
        #[cfg(feature = "tui")]
        {
            use std::io::IsTerminal;
            if std::io::stdin().is_terminal() {
                let state = match init_state() {
                    Ok(s) => s,
                    Err(e) => {
                        output::print_error_with_hint(&e.message, false, e.hint);
                        process::exit(3);
                    }
                };
                if let Err(e) = tui::run_dashboard(&state) {
                    eprintln!("TUI error: {e}");
                    process::exit(3);
                }
                return;
            }
        }

        // No TUI: print help hint
        eprintln!("{}", console::style("OpenSunstar CLI — AI 编程助手治理与编排工具").cyan().bold());
        eprintln!();
        #[cfg(not(feature = "tui"))]
        eprintln!("  TUI 仪表盘未启用。使用 {} 编译以启用全屏仪表盘。",
            console::style("cargo build --features tui").yellow());
        #[cfg(feature = "tui")]
        eprintln!("  非交互终端，请使用子命令。运行 {} 查看帮助。",
            console::style("os --help").yellow());
        eprintln!();
        eprintln!("  常用命令:");
        eprintln!("    {}  检查配置漂移", console::style("os drift check").green());
        eprintln!("    {}  Agent 就绪度评分", console::style("os readiness score").green());
        eprintln!("    {}  环境诊断", console::style("os doctor").green());
        eprintln!("    {}  项目全景状态", console::style("os project status").green());
        eprintln!();
        return;
    }

    // ── CLI 模式 ──
    let command = match command {
        Some(c) => c,
        None => {
            // json mode or --no-tui without subcommand
            if json {
                output::print_error("No subcommand provided. Use os --help.", json);
            } else {
                eprintln!("No subcommand provided. Use os --help.");
            }
            process::exit(3);
        }
    };

    let final_result: Result<(), String> = if let Some(timeout_secs) = timeout {
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);
        match run_with_timeout(
            move || run_command(command, json),
            timeout_duration,
        ) {
            Ok(inner) => inner,
            Err(timeout_err) => Err(timeout_err),
        }
    } else {
        run_command(command, json)
    };

    if let Err(e) = final_result {
        output::print_error(&e, json);
        process::exit(3);
    }
}
