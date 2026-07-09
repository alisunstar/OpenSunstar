//! `os sync` — 跨设备同步
//!
//! 提供简化的 CLI 同步能力（数据库导出/导入）。
//! 完整同步（WebDAV/S3 自动同步）需要 GUI 应用。

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub action: SyncAction,
}

#[derive(Subcommand)]
pub enum SyncAction {
    /// 推送数据到远端（导出 SQL 供同步）
    Push {
        /// 同步后端: webdav|s3
        #[arg(long, default_value = "webdav")]
        backend: String,
        /// 跳过确认
        #[arg(long)]
        yes: bool,
    },
    /// 从远端拉取数据（提示：完整同步需要 GUI）
    Pull {
        /// 同步后端: webdav|s3
        #[arg(long, default_value = "webdav")]
        backend: String,
        /// 跳过确认
        #[arg(long)]
        yes: bool,
    },
    /// 查看同步状态
    Status,
}

pub fn run(
    args: SyncArgs,
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    match args.action {
        SyncAction::Push { backend, yes } => run_push(state, &backend, yes, json),
        SyncAction::Pull { backend, yes } => run_pull(&backend, yes, json),
        SyncAction::Status => run_status(state, json),
    }
}

fn run_push(
    state: &open_sunstar_lib::AppState,
    backend: &str,
    yes: bool,
    json: bool,
) -> Result<(), String> {
    // Interactive confirmation (unless --yes or --json)
    output::header("Sync Push");
    output::info(&format!("Backend: {backend}"));
    output::info(&format!(
        "Will export database SQL to sync-export/ directory for backend '{backend}'."
    ));
    output::dim("Use the GUI for full automatic sync (WebDAV/S3).");
    if !output::confirm("确认执行?", yes || json, false) {
        output::info("已取消。");
        return Ok(());
    }

    // CLI 简化版：导出 SQL 到标准输出或临时文件，提示用户使用 GUI 进行完整同步
    let sql = state
        .db
        .export_sql_string_for_sync()
        .map_err(|e| format!("导出同步 SQL 失败: {e}"))?;

    let config_dir = open_sunstar_lib::get_app_config_dir();
    let sync_dir = config_dir.join("sync-export");
    std::fs::create_dir_all(&sync_dir)
        .map_err(|e| format!("创建同步导出目录失败: {e}"))?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("sync_{backend}_{timestamp}.sql");
    let output_path = sync_dir.join(&filename);

    std::fs::write(&output_path, sql.as_bytes())
        .map_err(|e| format!("写入同步文件失败: {e}"))?;

    if json {
        let result = serde_json::json!({
            "exported": true,
            "backend": backend,
            "path": output_path.display().to_string(),
            "size_bytes": std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0),
            "note": "This is a simplified CLI export. Use the GUI for full WebDAV/S3 sync.",
        });
        output::print_result(&result, true);
    } else {
        output::success(&format!("Sync data exported for backend '{backend}'"));
        println!("  File: {}", output_path.display());
        println!(
            "  Size: {} bytes",
            std::fs::metadata(&output_path)
                .map(|m| m.len())
                .unwrap_or(0)
        );
        println!();
        println!(
            "  Note: This is a simplified CLI export."
        );
        println!(
            "  For full automatic sync (WebDAV/S3), use the OpenSunstar GUI."
        );
    }

    Ok(())
}

fn run_pull(backend: &str, yes: bool, json: bool) -> Result<(), String> {
    // Interactive confirmation (unless --yes or --json)
    output::header("Sync Pull");
    output::info(&format!("Backend: {backend}"));
    output::info(&format!(
        "Will attempt to pull sync data from backend '{backend}'."
    ));
    output::dim("Note: Full pull requires the GUI application.");
    if !output::confirm("确认执行?", yes || json, false) {
        output::info("已取消。");
        return Ok(());
    }

    // CLI 无法直接拉取远端（需要 AppHandle + settings）
    if json {
        let result = serde_json::json!({
            "supported": false,
            "backend": backend,
            "message": "CLI pull is not supported. Use the OpenSunstar GUI for full sync pull.",
        });
        output::print_result(&result, true);
    } else {
        output::error_msg(&format!(
            "CLI pull is not supported for backend '{backend}'."
        ));
        println!();
        println!("  The pull operation requires WebDAV/S3 credentials and settings");
        println!("  that are managed through the GUI application.");
        println!();
        println!("  To pull sync data via CLI:");
        println!("  1. Download the sync SQL file from your sync backend manually");
        println!("  2. Run: os config import --input <downloaded-file.sql>");
    }

    Ok(())
}

fn run_status(
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    // 读取同步相关 settings
    let webdav_url = state
        .db
        .get_setting("webdav_sync_url")
        .map_err(|e| e.to_string())?;
    let s3_bucket = state
        .db
        .get_setting("s3_sync_bucket")
        .map_err(|e| e.to_string())?;
    let auto_sync_enabled = state
        .db
        .get_setting("auto_sync_enabled")
        .map_err(|e| e.to_string())?
        .map(|v| v == "true")
        .unwrap_or(false);

    if json {
        let result = serde_json::json!({
            "auto_sync_enabled": auto_sync_enabled,
            "webdav": {
                "configured": webdav_url.is_some(),
                "url": webdav_url,
            },
            "s3": {
                "configured": s3_bucket.is_some(),
                "bucket": s3_bucket,
            },
        });
        output::print_result(&result, true);
    } else {
        println!("Sync Status:\n");
        println!(
            "  Auto sync: {}",
            if auto_sync_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!(
            "  WebDAV:    {}",
            if webdav_url.is_some() {
                "configured"
            } else {
                "not configured"
            }
        );
        if let Some(ref url) = webdav_url {
            println!("             url: {url}");
        }
        println!(
            "  S3:        {}",
            if s3_bucket.is_some() {
                "configured"
            } else {
                "not configured"
            }
        );
        if let Some(ref bucket) = s3_bucket {
            println!("             bucket: {bucket}");
        }
    }

    Ok(())
}
