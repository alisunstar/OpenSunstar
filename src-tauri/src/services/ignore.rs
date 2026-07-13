use std::collections::HashSet;
use std::path::PathBuf;

use crate::app_config::AppType;
use crate::codex_config::get_codex_auth_path;
use crate::config::{get_claude_config_dir, write_text_file};
use crate::database::ASSET_IGNORE;
use crate::error::AppError;
use crate::gemini_config::get_gemini_dir;
use crate::hermes_config::get_hermes_dir;
use crate::ignore_rule::{parse_gitignore_content, validate_ignore_pattern, IgnoreRule};
use crate::opencode_config::get_opencode_dir;
use crate::prompt_files::project_ignore_file_path;
use crate::services::marker_merge::{is_managed_ignore_file, wrap_managed_ignore};
use crate::store::AppState;

pub struct IgnoreService;

const SYNC_APPS: [AppType; 5] = [
    AppType::Claude,
    AppType::Codex,
    AppType::Gemini,
    AppType::OpenCode,
    AppType::Hermes,
];

const PROJECT_SYNC_APPS: [AppType; 5] = [
    AppType::Claude,
    AppType::Codex,
    AppType::Gemini,
    AppType::OpenCode,
    AppType::Hermes,
];

impl IgnoreService {
    pub fn get_all_rules(state: &AppState) -> Result<Vec<IgnoreRule>, AppError> {
        state.db.get_all_ignore_rules()
    }

    pub fn upsert_rule(state: &AppState, rule: IgnoreRule) -> Result<(), AppError> {
        validate_ignore_pattern(&rule.pattern).map_err(AppError::Config)?;
        let rule_id = rule.id.clone();
        state.db.save_ignore_rule(&rule)?;
        Self::sync_all_apps(state)?;
        Self::sync_project_for_rule(state, &rule_id)?;
        Ok(())
    }

    pub fn delete_rule(state: &AppState, id: &str) -> Result<bool, AppError> {
        let existed = state.db.get_all_ignore_rules()?.iter().any(|r| r.id == id);
        if !existed {
            return Ok(false);
        }
        state.db.delete_ignore_rule(id)?;
        Self::sync_all_apps(state)?;
        Self::sync_project_for_rule(state, id)?;
        Ok(true)
    }

    pub fn toggle_app(
        state: &AppState,
        rule_id: &str,
        app: AppType,
        enabled: bool,
    ) -> Result<(), AppError> {
        let mut rules = state.db.get_all_ignore_rules()?;
        if let Some(rule) = rules.iter_mut().find(|r| r.id == rule_id) {
            rule.set_enabled_for(&app, enabled);
            let snapshot = rule.clone();
            state.db.save_ignore_rule(&snapshot)?;
            Self::sync_app(state, &app)?;
            Self::sync_project_for_rule(state, rule_id)?;
        }
        Ok(())
    }

    pub fn import_from_gitignore(state: &AppState, file_path: &str) -> Result<usize, AppError> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| AppError::io(std::path::Path::new(file_path), e))?;
        let patterns = parse_gitignore_content(&content);
        if patterns.is_empty() {
            return Ok(0);
        }

        let existing = state.db.get_all_ignore_rules()?;
        let existing_patterns: std::collections::HashSet<_> =
            existing.iter().map(|r| r.pattern.clone()).collect();

        let now = chrono::Utc::now().timestamp();
        let mut added = 0usize;
        let mut sort_base = existing.len() as i32;

        for pattern in patterns {
            if existing_patterns.contains(&pattern) {
                continue;
            }
            let rule = IgnoreRule {
                id: format!("ignore-{}-{}", now, added),
                pattern,
                description: Some("从 .gitignore 导入".into()),
                enabled_claude: true,
                enabled_codex: true,
                enabled_gemini: true,
                enabled_opencode: true,
                enabled_hermes: true,
                sort_index: sort_base,
                created_at: Some(now),
            };
            sort_base += 1;
            state.db.save_ignore_rule(&rule)?;
            added += 1;
        }

        if added > 0 {
            Self::sync_all_apps(state)?;
        }
        Ok(added)
    }

    pub fn sync_all_apps(state: &AppState) -> Result<(), AppError> {
        for app in SYNC_APPS {
            Self::sync_app(state, &app)?;
        }
        Ok(())
    }

    fn sync_app(state: &AppState, app: &AppType) -> Result<(), AppError> {
        let rules = state.db.get_all_ignore_rules()?;
        let patterns: Vec<&str> = rules
            .iter()
            .filter(|r| r.is_enabled_for(app))
            .map(|r| r.pattern.as_str())
            .collect();

        let path = ignore_file_path(app)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        // 标记保护：如果文件已存在且非 OpenSunstar 管理，跳过覆盖
        if path.is_file() {
            let existing = std::fs::read_to_string(&path).unwrap_or_default();
            if !existing.is_empty() && !is_managed_ignore_file(&existing) {
                log::warn!(
                    "跳过覆盖非 OpenSunstar 管理的 ignore 文件: {}",
                    path.display()
                );
                return Ok(());
            }
        }

        let body = if patterns.is_empty() {
            String::new()
        } else {
            patterns.join("\n") + "\n"
        };
        let content = wrap_managed_ignore(&body);
        write_text_file(&path, &content)?;
        Ok(())
    }

    /// 将项目关联的 ignore 规则写回到项目根目录的 .claudeignore 等文件
    pub fn sync_project_ignore(
        state: &AppState,
        project_root: &std::path::Path,
        project_id: &str,
    ) -> Result<(), AppError> {
        let rules = state.db.get_all_ignore_rules()?;
        let links = state
            .db
            .get_project_asset_links(project_id, Some(ASSET_IGNORE))?;
        let linked_ids: HashSet<&str> = links
            .iter()
            .filter(|l| l.enabled)
            .map(|l| l.asset_id.as_str())
            .collect();

        if linked_ids.is_empty() {
            for app in &PROJECT_SYNC_APPS {
                let path = project_ignore_file_path(project_root, app)?;
                if path.is_file() {
                    // 标记保护：仅删除 OpenSunstar 管理的文件
                    let existing = std::fs::read_to_string(&path).unwrap_or_default();
                    if is_managed_ignore_file(&existing) {
                        let _ = std::fs::remove_file(&path);
                    } else {
                        log::warn!(
                            "跳过删除非 OpenSunstar 管理的 ignore 文件: {}",
                            path.display()
                        );
                    }
                }
            }
            return Ok(());
        }

        for app in &PROJECT_SYNC_APPS {
            let patterns: Vec<&str> = rules
                .iter()
                .filter(|r| linked_ids.contains(r.id.as_str()) && r.is_enabled_for(app))
                .map(|r| r.pattern.as_str())
                .collect();

            let path = project_ignore_file_path(project_root, app)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
            }

            // 标记保护：如果文件已存在且非 OpenSunstar 管理，跳过覆盖
            if path.is_file() {
                let existing = std::fs::read_to_string(&path).unwrap_or_default();
                if !existing.is_empty() && !is_managed_ignore_file(&existing) {
                    log::warn!(
                        "跳过覆盖非 OpenSunstar 管理的项目 ignore 文件: {}",
                        path.display()
                    );
                    continue;
                }
            }

            let body = if patterns.is_empty() {
                String::new()
            } else {
                patterns.join("\n") + "\n"
            };
            let content = wrap_managed_ignore(&body);
            write_text_file(&path, &content)?;
        }
        Ok(())
    }

    /// 当某条 ignore 规则变更时，同步所有关联该规则的项目级 ignore 文件
    fn sync_project_for_rule(state: &AppState, rule_id: &str) -> Result<(), AppError> {
        let projects = state.db.get_all_projects()?;
        for project in projects {
            let links = state
                .db
                .get_project_asset_links(&project.id, Some(ASSET_IGNORE))
                .unwrap_or_default();
            let is_linked = links.iter().any(|l| l.asset_id == rule_id);
            if is_linked {
                let root = std::path::PathBuf::from(&project.path);
                Self::sync_project_ignore(state, &root, &project.id)?;
            }
        }
        Ok(())
    }
}

fn ignore_file_path(app: &AppType) -> Result<PathBuf, AppError> {
    let path = match app {
        AppType::Claude => get_claude_config_dir().join(".claudeignore"),
        AppType::Codex => get_codex_auth_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".codex"))
            .join(".codexignore"),
        AppType::Gemini => get_gemini_dir().join(".geminiignore"),
        AppType::OpenCode => get_opencode_dir().join(".opencodeignore"),
        AppType::Hermes => get_hermes_dir().join(".hermesignore"),
        AppType::OpenClaw | AppType::ClaudeDesktop => {
            return Err(AppError::Config(format!("{app:?} 不支持 ignore 文件同步")));
        }
    };
    Ok(path)
}
