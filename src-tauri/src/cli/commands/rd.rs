//! `os rd` — RD 过程资产层工具（P1-4 对账确定性段）
//!
//! 确定性、无 LLM、无外部数据源（K1/K2/K3）。

use clap::{Args, Subcommand};
use std::process;

#[derive(Args)]
pub struct RdArgs {
    #[command(subcommand)]
    pub action: RdAction,
}

#[derive(Subcommand)]
pub enum RdAction {
    /// 实现校验确定性段：IMPLEMENTATION-CHECK 五状态 schema 校验 + git diff 统计
    Validate {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 变更 ID
        #[arg(long)]
        change_id: String,
    },
}

pub fn run(args: RdArgs, json: bool) -> Result<(), String> {
    match args.action {
        RdAction::Validate {
            project_path,
            change_id,
        } => {
            let report = open_sunstar_lib::rd_validate::validate_implementation_check(
                &project_path,
                &change_id,
            )?;
            let valid = report.schema_valid;
            if json {
                crate::output::print_result(&report, true);
            } else {
                crate::output::header("RD Validate (deterministic)");
                println!("  Change: {}", report.change_id);
                println!("  Rows:   {}", report.rows);
                for (k, v) in &report.counts {
                    println!("    {k}: {v}");
                }
                if !report.diff_stats.is_empty() {
                    println!("  Diff (top {}):", report.diff_stats.len());
                    for d in &report.diff_stats {
                        println!("    +{} -{} {}", d.added, d.deleted, d.file);
                    }
                }
                if let Some(note) = &report.diff_note {
                    crate::output::dim(&format!("  note: {note}"));
                }
                if valid {
                    crate::output::success("schema 校验通过");
                } else {
                    println!("  Issues:");
                    for i in &report.issues {
                        println!("    ! {i}");
                    }
                }
            }
            if !valid {
                process::exit(1);
            }
            Ok(())
        }
    }
}
