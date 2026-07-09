//! `os config` — 配置导入/导出
//!
//! 支持数据库 SQL dump 的导出、导入，以及查看配置目录路径。

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// 导出配置（数据库 SQL dump）
    Export {
        /// 输出文件路径
        #[arg(long)]
        output: String,
    },
    /// 导入配置（从 SQL dump 恢复）
    Import {
        /// 输入文件路径
        #[arg(long)]
        input: String,
        /// 仅预览变更，不执行
        #[arg(long)]
        dry_run: bool,
        /// 跳过确认
        #[arg(long)]
        yes: bool,
    },
    /// 显示配置目录路径
    Path,
    /// 初始化数据目录与数据库（无需 GUI）
    Bootstrap {
        /// 跳过确认
        #[arg(long)]
        yes: bool,
    },
}

pub fn run(
    args: ConfigArgs,
    state: Option<&open_sunstar_lib::AppState>,
    json: bool,
) -> Result<(), String> {
    match args.action {
        ConfigAction::Export { output } => {
            let state = state.ok_or_else(|| "数据库不可用".to_string())?;
            run_export(state, &output, json)
        }
        ConfigAction::Import { input, dry_run, yes } => {
            let state = state.ok_or_else(|| "数据库不可用".to_string())?;
            run_import(state, &input, dry_run, yes, json)
        }
        ConfigAction::Path => run_path(json),
        ConfigAction::Bootstrap { yes } => run_bootstrap(yes, json),
    }
}

fn run_export(
    state: &open_sunstar_lib::AppState,
    output_path: &str,
    json: bool,
) -> Result<(), String> {
    let target = std::path::Path::new(output_path);

    state
        .db
        .export_sql(target)
        .map_err(|e| format!("导出失败: {e}"))?;

    let size = std::fs::metadata(target)
        .map(|m| m.len())
        .unwrap_or(0);

    if json {
        let result = serde_json::json!({
            "exported": true,
            "path": output_path,
            "size_bytes": size,
        });
        output::print_result(&result, true);
    } else {
        output::success(&format!("Configuration exported to: {output_path}"));
        println!("  Size: {size} bytes");
    }

    Ok(())
}

fn run_import(
    state: &open_sunstar_lib::AppState,
    input: &str,
    dry_run: bool,
    yes: bool,
    json: bool,
) -> Result<(), String> {
    let source = std::path::Path::new(input);
    if !source.exists() {
        return Err(format!("文件不存在: {input}"));
    }

    let file_size = std::fs::metadata(source)
        .map(|m| m.len())
        .unwrap_or(0);

    // Dry-run: show what would happen and exit
    if dry_run {
        if json {
            let result = serde_json::json!({
                "dry_run": true,
                "source": input,
                "size_bytes": file_size,
                "message": format!("Would import SQL from {input}, replacing current database. A backup will be created first."),
            });
            output::print_result(&result, true);
        } else {
            println!("Config Import Plan (dry-run)\n");
            println!("  Source: {input}");
            println!("  Size:   {file_size} bytes");
            println!();
            println!("Would import SQL from {input}, replacing current database.");
            println!("A backup will be created first.");
        }

        return Ok(());
    }

    // 交互确认（除非 --yes）
    if !yes && !json {
        output::warning(&format!(
            "WARNING: This will replace ALL current configuration with data from: {input}"
        ));
        output::info("A safety backup will be created before import.");
    }
    if !output::confirm("Proceed with import?", yes || json, false) {
        output::info("Aborted.");
        return Ok(());
    }

    let backup_id = state
        .db
        .import_sql(source)
        .map_err(|e| format!("导入失败: {e}"))?;

    if json {
        let result = serde_json::json!({
            "imported": true,
            "source": input,
            "safety_backup_id": backup_id,
        });
        output::print_result(&result, true);
    } else {
        output::success(&format!("Configuration imported from: {input}"));
        if !backup_id.is_empty() {
            println!("  Safety backup: {backup_id}");
        }
    }

    Ok(())
}

fn run_path(json: bool) -> Result<(), String> {
    let config_dir = open_sunstar_lib::get_app_config_dir();

    if json {
        let result = serde_json::json!({
            "config_dir": config_dir.display().to_string(),
        });
        output::print_result(&result, true);
    } else {
        println!("{}", config_dir.display());
    }

    Ok(())
}

fn run_bootstrap(yes: bool, json: bool) -> Result<(), String> {
    let config_dir = open_sunstar_lib::get_app_config_dir();
    let db_path = config_dir.join("OpenSunstar.db");
    let exists = db_path.exists();

    if exists && !json {
        output::info(&format!("数据库已存在: {}", db_path.display()));
        output::info("将补全官方预设与默认技能源（幂等）。");
    }

    if !output::confirm("初始化 OpenSunstar 数据目录?", yes || json, true) {
        output::info("已取消。");
        return Ok(());
    }

    let result = open_sunstar_lib::cli_api::cli_bootstrap_database()?;

    if json {
        output::print_result(&result, true);
    } else {
        if result.created {
            output::success("已创建 OpenSunstar 数据库。");
        } else {
            output::success("OpenSunstar 数据库已就绪。");
        }
        println!("  Config:  {}", result.config_dir);
        println!("  DB:      {}", result.db_path);
        if !result.imported_apps.is_empty() {
            println!("  Imported live config: {}", result.imported_apps.join(", "));
        }
        if result.seeded_official_providers > 0 {
            println!(
                "  Seeded official providers: {}",
                result.seeded_official_providers
            );
        }
        if result.seeded_skill_repos > 0 {
            println!("  Seeded skill repos: {}", result.seeded_skill_repos);
        }
    }

    Ok(())
}
