//! `os doctor` — 环境诊断：数据库、工具安装、配置目录

use std::path::PathBuf;

use crate::output;

#[derive(clap::Args)]
pub struct DoctorArgs {
    /// 初始化数据目录与数据库（等同 os config bootstrap）
    #[arg(long)]
    pub init: bool,
}

#[derive(serde::Serialize)]
struct DoctorReport {
    status: String,
    config_dir: String,
    database: DatabaseStatus,
    tools: Vec<ToolStatus>,
}

#[derive(serde::Serialize)]
struct DatabaseStatus {
    path: String,
    exists: bool,
    schema_version: i32,
    readable: bool,
}

#[derive(serde::Serialize)]
struct ToolStatus {
    name: String,
    installed: bool,
    version: Option<String>,
    error: Option<String>,
}

pub fn run(args: DoctorArgs, json: bool) -> Result<(), String> {
    if args.init {
        return run_init(json);
    }

    let config_dir = open_sunstar_lib::get_app_config_dir();
    let db_path = config_dir.join("OpenSunstar.db");

    // 1. 数据库检查
    let db_status = check_database(&db_path);

    // 2. 工具安装检查
    let tools = check_tools();

    // 3. 汇总
    let all_ok = db_status.exists && db_status.readable && tools.iter().any(|t| t.installed);
    let status = if all_ok { "ok" } else { "issues" };

    let report = DoctorReport {
        status: status.to_string(),
        config_dir: config_dir.display().to_string(),
        database: db_status,
        tools,
    };

    if json {
        output::print_result(&report, true);
    } else {
        print_human_report(&report);
    }

    Ok(())
}

fn check_database(db_path: &PathBuf) -> DatabaseStatus {
    let exists = db_path.exists();
    let readable = if exists {
        // 尝试初始化数据库（含 schema 迁移）
        open_sunstar_lib::Database::init().is_ok()
    } else {
        false
    };

    let schema_version = if readable {
        open_sunstar_lib::get_build_info().schema_version
    } else {
        -1
    };

    DatabaseStatus {
        path: db_path.display().to_string(),
        exists,
        schema_version,
        readable,
    }
}

fn check_tools() -> Vec<ToolStatus> {
    let tools = ["claude", "codex", "gemini", "opencode", "openclaw", "hermes"];
    let term = console::Term::stderr();

    let results: Vec<_> = tools
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let _ = term.write_str(&format!(
                "\r{}",
                console::style(format!("Checking tools... ({}/{})", i + 1, tools.len())).dim()
            ));
            let version = get_tool_version(name);
            let installed = version.is_some();
            ToolStatus {
                name: name.to_string(),
                installed,
                version,
                error: None,
            }
        })
        .collect();

    let _ = term.clear_line();
    let _ = term.write_str("\r");
    results
}

fn get_tool_version(tool: &str) -> Option<String> {
    use std::process::Command;

    let output = Command::new(tool).arg("--version").output().ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Some(stdout.trim().to_string())
    } else {
        None
    }
}

fn print_human_report(report: &DoctorReport) {
    output::header("OpenSunstar Doctor Report");
    eprintln!();

    // 状态
    let status_msg = format!("Status: {}", report.status);
    if report.status == "ok" {
        output::success(&status_msg);
    } else {
        output::warning(&status_msg);
    }
    eprintln!();

    // 配置目录
    output::info(&format!("Config directory: {}", report.config_dir));

    // 数据库
    let db = &report.database;
    let db_detail = if db.readable {
        format!("v{}, readable", db.schema_version)
    } else if db.exists {
        "exists but not readable".to_string()
    } else {
        "not found".to_string()
    };
    let db_msg = format!("Database: {} ({})", db.path, db_detail);
    if db.readable {
        output::success(&db_msg);
    } else {
        output::error_msg(&db_msg);
    }

    // 工具
    eprintln!();
    output::header("AI Tools:");
    for tool in &report.tools {
        let detail = if let Some(ref v) = tool.version {
            format!(" ({v})")
        } else {
            " (not installed)".to_string()
        };
        let msg = format!("{}{}", tool.name, detail);
        if tool.installed {
            output::success(&msg);
        } else {
            output::dim(&msg);
        }
    }
}

fn run_init(json: bool) -> Result<(), String> {
    let result = open_sunstar_lib::cli_api::cli_bootstrap_database()?;
    if json {
        output::print_result(&result, true);
    } else {
        if result.created {
            output::success("已创建 OpenSunstar 数据库。");
        } else {
            output::success("OpenSunstar 数据库已就绪。");
        }
        println!("  Config: {}", result.config_dir);
        println!("  DB:     {}", result.db_path);
    }
    Ok(())
}
