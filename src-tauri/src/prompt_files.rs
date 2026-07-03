use std::path::PathBuf;

use crate::app_config::AppType;
use crate::codex_config::get_codex_auth_path;
use crate::config::get_claude_settings_path;
use crate::error::AppError;
use crate::gemini_config::get_gemini_dir;
use crate::openclaw_config::get_openclaw_dir;
use crate::opencode_config::get_opencode_dir;

/// 返回指定应用所使用的提示词文件路径。
pub fn prompt_file_path(app: &AppType) -> Result<PathBuf, AppError> {
    if matches!(app, AppType::ClaudeDesktop) {
        return Err(AppError::localized(
            "claude_desktop.prompts_unsupported",
            "Claude Desktop 暂不支持 Prompts",
            "Claude Desktop does not support Prompts",
        ));
    }

    let base_dir: PathBuf = match app {
        AppType::Claude => get_base_dir_with_fallback(get_claude_settings_path(), ".claude")?,
        AppType::Codex => get_base_dir_with_fallback(get_codex_auth_path(), ".codex")?,
        AppType::Gemini => get_gemini_dir(),
        AppType::OpenCode => get_opencode_dir(),
        AppType::OpenClaw => get_openclaw_dir(),
        AppType::Hermes => crate::hermes_config::get_hermes_dir(),
        AppType::ClaudeDesktop => unreachable!("handled above"),
    };

    let filename = match app {
        AppType::Claude => "CLAUDE.md",
        AppType::Codex => "AGENTS.md",
        AppType::Gemini => "GEMINI.md",
        AppType::OpenCode | AppType::OpenClaw | AppType::Hermes => "AGENTS.md",
        AppType::ClaudeDesktop => unreachable!("handled above"),
    };

    Ok(base_dir.join(filename))
}

/// 项目根目录下的 Prompt 文件名（各 CLI 约定）
pub fn project_prompt_filename(app: &AppType) -> Result<&'static str, AppError> {
    if matches!(app, AppType::ClaudeDesktop) {
        return Err(AppError::localized(
            "claude_desktop.prompts_unsupported",
            "Claude Desktop 暂不支持 Prompts",
            "Claude Desktop does not support Prompts",
        ));
    }
    Ok(match app {
        AppType::Claude => "CLAUDE.md",
        AppType::Codex => "AGENTS.md",
        AppType::Gemini => "GEMINI.md",
        AppType::OpenCode | AppType::OpenClaw | AppType::Hermes => "AGENTS.md",
        AppType::ClaudeDesktop => unreachable!(),
    })
}

/// 项目根目录下的 Prompt 文件路径
pub fn project_prompt_file_path(project_root: &std::path::Path, app: &AppType) -> Result<PathBuf, AppError> {
    Ok(project_root.join(project_prompt_filename(app)?))
}

/// 项目根 `.mcp.json`（Claude Code 项目级 MCP）
pub fn project_mcp_json_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".mcp.json")
}

/// 项目级 Command 清单（L2-04 写回追踪）
pub fn project_command_manifest_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".opensunstar").join("command-manifest.json")
}

/// 各 CLI 在项目根下的配置目录名（`.claude`、`.codex` …）
pub fn project_cli_dot_dir(app: &AppType) -> Result<&'static str, AppError> {
    Ok(match app {
        AppType::Claude => ".claude",
        AppType::Codex => ".codex",
        AppType::Gemini => ".gemini",
        AppType::OpenCode => ".opencode",
        AppType::Hermes => ".hermes",
        AppType::OpenClaw | AppType::ClaudeDesktop => {
            return Err(AppError::Config(format!("{app:?} 不支持项目级 Commands 目录")));
        }
    })
}

/// 项目级 Commands 目录，如 `{project}/.claude/commands`
pub fn project_commands_dir(
    project_root: &std::path::Path,
    app: &AppType,
) -> Result<PathBuf, AppError> {
    Ok(project_root
        .join(project_cli_dot_dir(app)?)
        .join("commands"))
}

/// 项目级单个 Command 文件路径
pub fn project_command_file_path(
    project_root: &std::path::Path,
    app: &AppType,
    command_name: &str,
) -> Result<PathBuf, AppError> {
    crate::command::validate_command_name(command_name).map_err(AppError::Config)?;
    Ok(project_commands_dir(project_root, app)?.join(format!("{command_name}.md")))
}

/// 项目级 Agents 目录，如 `{project}/.claude/agents`
pub fn project_agents_dir(project_root: &std::path::Path, app: &AppType) -> Result<PathBuf, AppError> {
    Ok(project_root
        .join(project_cli_dot_dir(app)?)
        .join("agents"))
}

/// 项目级单个 Subagent 文件路径
pub fn project_agent_file_path(
    project_root: &std::path::Path,
    app: &AppType,
    agent_name: &str,
) -> Result<PathBuf, AppError> {
    crate::agent::validate_agent_name(agent_name).map_err(AppError::Config)?;
    let ext = if matches!(app, AppType::Codex) {
        "toml"
    } else {
        "md"
    };
    Ok(project_agents_dir(project_root, app)?.join(format!("{agent_name}.{ext}")))
}

/// 项目级 Skills 目录，如 `{project}/.claude/skills`
pub fn project_skills_dir(project_root: &std::path::Path, app: &AppType) -> Result<PathBuf, AppError> {
    Ok(project_root
        .join(project_cli_dot_dir(app)?)
        .join("skills"))
}

/// 项目级 Claude `settings.json`
pub fn project_claude_settings_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".claude").join("settings.json")
}

/// 项目级 Codex `config.toml`
pub fn project_codex_config_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".codex").join("config.toml")
}

/// 项目级 Gemini `settings.json`
pub fn project_gemini_settings_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".gemini").join("settings.json")
}

/// 项目级 Hermes `config.yaml`
pub fn project_hermes_config_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".hermes").join("config.yaml")
}

/// 项目级 OpenCode `opencode.json`
pub fn project_opencode_config_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".opencode").join("opencode.json")
}

/// 项目级 Subagent 清单（写回追踪）
pub fn project_subagent_manifest_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".opensunstar").join("subagent-manifest.json")
}

/// 项目级 Skill 清单（写回追踪）
pub fn project_skill_manifest_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".opensunstar").join("skill-manifest.json")
}

/// 项目级 Ignore 文件路径，如 {project}/.claudeignore
pub fn project_ignore_file_path(
    project_root: &std::path::Path,
    app: &AppType,
) -> Result<PathBuf, AppError> {
    let filename = match app {
        AppType::Claude => ".claudeignore",
        AppType::Codex => ".codexignore",
        AppType::Gemini => ".geminiignore",
        AppType::OpenCode => ".opencodeignore",
        AppType::Hermes => ".hermesignore",
        AppType::OpenClaw | AppType::ClaudeDesktop => {
            return Err(AppError::Config(format!(
                "{app:?} 不支持项目级 ignore 文件同步"
            )));
        }
    };
    Ok(project_root.join(filename))
}

fn get_base_dir_with_fallback(
    primary_path: PathBuf,
    fallback_dir: &str,
) -> Result<PathBuf, AppError> {
    primary_path
        .parent()
        .map(|p| p.to_path_buf())
        .or_else(|| dirs::home_dir().map(|h| h.join(fallback_dir)))
        .ok_or_else(|| {
            AppError::localized(
                "home_dir_not_found",
                format!("无法确定 {fallback_dir} 配置目录：用户主目录不存在"),
                format!("Cannot determine {fallback_dir} config directory: user home not found"),
            )
        })
}
