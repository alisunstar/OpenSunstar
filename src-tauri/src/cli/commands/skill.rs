//! `os skill` — 技能搜索与已安装列表
//!
//! 支持从多个源搜索技能，以及列出本地已安装的技能。

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub action: SkillAction,
}

#[derive(Subcommand)]
pub enum SkillAction {
    /// 搜索技能市场
    Search {
        /// 搜索关键词
        query: String,
        /// 搜索源: skills-sh|clawhub|modelscope
        #[arg(long, default_value = "skills-sh")]
        source: String,
        /// 结果数量限制
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// 列出已安装技能
    List,
}

pub fn run_with_optional_state(
    args: SkillArgs,
    state: Option<&open_sunstar_lib::AppState>,
    json: bool,
) -> Result<(), String> {
    match args.action {
        SkillAction::Search {
            query,
            source,
            limit,
        } => run_search(&query, &source, limit, json),
        SkillAction::List => {
            let state = state.ok_or_else(|| "数据库不可用，无法列出已安装技能".to_string())?;
            run_list(state, json)
        }
    }
}

fn run_search(query: &str, source: &str, limit: usize, json: bool) -> Result<(), String> {
    if source != "skills-sh" {
        output::warning(&format!("搜索源 '{source}' 暂不支持 CLI，请使用 GUI 进行搜索，或使用 --source skills-sh。"));
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let search_result = rt
        .block_on(open_sunstar_lib::SkillService::search_skills_sh(query, limit, 0))
        .map_err(|e| e.to_string())?;
    let results = serde_json::to_value(&search_result).map_err(|e| e.to_string())?;

    if json {
        output::print_result(&results, true);
    } else {
        // SkillsShSearchResult serializes to { "skills": [...], "totalCount": N, "query": "..." }
        let skills_arr = results
            .get("skills")
            .and_then(|v| v.as_array());

        match skills_arr {
            Some(arr) if !arr.is_empty() => {
                output::header(&format!(
                    "Search results for '{query}' on skills-sh ({} found):",
                    arr.len()
                ));
                eprintln!();
                for item in arr {
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let desc = item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let installs = item
                        .get("installs")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    println!(
                        "  {} {}",
                        console::style("·").green(),
                        console::style(name).green().bold()
                    );
                    if !desc.is_empty() {
                        let truncated = if desc.len() > 80 {
                            format!("{}...", &desc[..77])
                        } else {
                            desc.to_string()
                        };
                        output::dim(&format!("    {truncated}"));
                    }
                    if installs > 0 {
                        output::dim(&format!("    installs: {installs}"));
                    }
                    println!();
                }
            }
            _ => {
                output::info(&format!("No skills found for '{query}'."));
            }
        }
    }

    Ok(())
}

fn run_list(
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    let skills = state
        .db
        .get_all_installed_skills()
        .map_err(|e| e.to_string())?;

    if json {
        let items: Vec<_> = skills
            .values()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "description": s.description,
                    "directory": s.directory,
                    "apps": {
                        "claude": s.apps.claude,
                        "codex": s.apps.codex,
                        "gemini": s.apps.gemini,
                        "opencode": s.apps.opencode,
                        "hermes": s.apps.hermes,
                    },
                    "installed_at": s.installed_at,
                })
            })
            .collect();
        output::print_result(&items, true);
    } else {
        if skills.is_empty() {
            output::info("No skills installed.");
            return Ok(());
        }

        output::header(&format!("Installed Skills ({} total):", skills.len()));
        eprintln!();
        println!(
            "  {:<24} {:<12} {:<8} {:<8} {:<8}",
            "NAME", "DIRECTORY", "CLAUDE", "CODEX", "GEMINI"
        );
        println!("  {}", "-".repeat(65));
        for skill in skills.values() {
            let claude = if skill.apps.claude { "✓" } else { "·" };
            let codex = if skill.apps.codex { "✓" } else { "·" };
            let gemini = if skill.apps.gemini { "✓" } else { "·" };
            let dir = if skill.directory.is_empty() { "-" } else { skill.directory.as_str() };
            println!(
                "  {:<24} {:<12} {:<8} {:<8} {:<8}",
                skill.name, dir, claude, codex, gemini
            );
        }
    }

    Ok(())
}
