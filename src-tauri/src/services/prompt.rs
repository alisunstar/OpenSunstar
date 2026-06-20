use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::app_config::AppType;
use crate::config::write_text_file;
use crate::error::AppError;
use crate::prompt::{
    compose_prompt_fragments, Prompt, MAX_FRAGMENTS_PER_PARENT,
};
use crate::prompt_files::prompt_file_path;
use crate::services::bridge;
use crate::store::AppState;

/// 安全地获取当前 Unix 时间戳
fn get_unix_timestamp() -> Result<i64, AppError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| AppError::Message(format!("Failed to get system time: {e}")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptActivationPreview {
    pub file_path: String,
    pub current_content: String,
    pub new_content: String,
}

pub struct PromptService;

impl PromptService {
    pub fn get_prompts(
        state: &AppState,
        app: &AppType,
    ) -> Result<IndexMap<String, Prompt>, AppError> {
        state.db.get_prompts(app.as_str())
    }

    pub fn upsert_prompt(
        state: &AppState,
        app: &AppType,
        _id: &str,
        prompt: Prompt,
    ) -> Result<(), AppError> {
        Self::validate_fragment(&state, app.as_str(), &prompt)?;

        let is_enabled = prompt.enabled;
        let prompt_id = prompt.id.clone();
        let app_str = app.as_str().to_string();

        state.db.save_prompt(&app_str, &prompt)?;

        if is_enabled {
            let content = Self::resolve_effective_content(&state, &app, &prompt)?;
            let target_path = prompt_file_path(&app)?;
            write_text_file(&target_path, &content)?;
        } else {
            let prompts = state.db.get_prompts(&app_str)?;
            let any_enabled = prompts.values().any(|p| p.enabled);

            if !any_enabled {
                let target_path = prompt_file_path(&app)?;
                if target_path.exists() {
                    write_text_file(&target_path, "")?;
                }
            }
        }

        Self::try_auto_bridge_push(&state.db, &app_str, &prompt_id);

        Ok(())
    }

    fn validate_fragment(
        state: &AppState,
        app_type: &str,
        prompt: &Prompt,
    ) -> Result<(), AppError> {
        if !prompt.is_fragment {
            return Ok(());
        }
        let parent_id = prompt
            .parent_prompt_id
            .as_deref()
            .ok_or_else(|| AppError::Config("规则片段必须指定所属 Prompt".into()))?;
        let prompts = state.db.get_prompts(app_type)?;
        if !prompts.contains_key(parent_id) {
            return Err(AppError::Config(format!("父 Prompt 不存在: {parent_id}")));
        }
        if prompts.get(parent_id).map(|p| p.is_fragment).unwrap_or(false) {
            return Err(AppError::Config("父 Prompt 不能是片段".into()));
        }
        let count = state.db.count_fragments_for_parent(app_type, parent_id)?;
        let is_new = !prompts.contains_key(&prompt.id);
        if is_new && count >= MAX_FRAGMENTS_PER_PARENT {
            return Err(AppError::Config(format!(
                "单个 Prompt 最多 {MAX_FRAGMENTS_PER_PARENT} 个片段"
            )));
        }
        Ok(())
    }

    /// 解析启用时将写入文件的有效内容（组合片段或直接使用正文）
    pub fn resolve_effective_content(
        state: &AppState,
        app: &AppType,
        prompt: &Prompt,
    ) -> Result<String, AppError> {
        let app_str = app.as_str();
        let prompts = state.db.get_prompts(app_str)?;
        let fragments: Vec<Prompt> = prompts
            .values()
            .filter(|p| {
                p.is_fragment && p.parent_prompt_id.as_deref() == Some(prompt.id.as_str())
            })
            .cloned()
            .collect();

        if fragments.is_empty() {
            return Ok(prompt.content.clone());
        }

        let composed = compose_prompt_fragments(&fragments, app_str);
        if prompt.content.trim().is_empty() {
            Ok(composed)
        } else if composed.trim().is_empty() {
            Ok(prompt.content.clone())
        } else {
            Ok(format!("{}\n\n{}", prompt.content.trim(), composed))
        }
    }

    pub fn preview_prompt_activation(
        state: &AppState,
        app: &AppType,
        id: &str,
    ) -> Result<PromptActivationPreview, AppError> {
        let prompts = state.db.get_prompts(app.as_str())?;
        let prompt = prompts
            .get(id)
            .ok_or_else(|| AppError::InvalidInput(format!("提示词 {id} 不存在")))?;

        let target_path = prompt_file_path(&app)?;
        let current_content = if target_path.exists() {
            std::fs::read_to_string(&target_path).unwrap_or_default()
        } else {
            String::new()
        };
        let new_content = Self::resolve_effective_content(state, &app, prompt)?;

        Ok(PromptActivationPreview {
            file_path: target_path.display().to_string(),
            current_content,
            new_content,
        })
    }

    /// If bridge_auto_push is enabled and this prompt has downstream targets, push changes.
    fn try_auto_bridge_push(db: &crate::database::Database, app_type: &str, prompt_id: &str) {
        let auto_push = db
            .get_setting("bridge_auto_push")
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(false);

        if !auto_push {
            return;
        }

        match bridge::push_bridge_changes(db, app_type, prompt_id) {
            Ok(results) if !results.is_empty() => {
                log::info!(
                    "[bridge] Auto-pushed {} target(s) for {app_type}:{prompt_id}",
                    results.len()
                );
            }
            Ok(_) => {}
            Err(e) => {
                log::warn!("[bridge] Auto-push failed for {app_type}:{prompt_id}: {e}");
            }
        }
    }

    pub fn delete_prompt(state: &AppState, app: &AppType, id: &str) -> Result<(), AppError> {
        let prompts = state.db.get_prompts(app.as_str())?;

        if let Some(prompt) = prompts.get(id) {
            if prompt.enabled {
                return Err(AppError::InvalidInput("无法删除已启用的提示词".to_string()));
            }
        }

        state
            .db
            .delete_fragments_for_parent(app.as_str(), id)?;
        state.db.delete_prompt(app.as_str(), id)?;
        Ok(())
    }

    pub fn enable_prompt(state: &AppState, app: &AppType, id: &str) -> Result<(), AppError> {
        let target_path = prompt_file_path(&app)?;
        if target_path.exists() {
            if let Ok(live_content) = std::fs::read_to_string(&target_path) {
                if !live_content.trim().is_empty() {
                    let mut prompts = state.db.get_prompts(app.as_str())?;

                    if let Some((enabled_id, enabled_prompt)) = prompts
                        .iter_mut()
                        .find(|(_, p)| p.enabled)
                        .map(|(id, p)| (id.clone(), p))
                    {
                        let timestamp = get_unix_timestamp()?;
                        enabled_prompt.content = live_content.clone();
                        enabled_prompt.updated_at = Some(timestamp);
                        log::info!("回填 live 提示词内容到已启用项: {enabled_id}");
                        state.db.save_prompt(app.as_str(), enabled_prompt)?;
                    } else {
                        let content_exists = prompts
                            .values()
                            .any(|p| p.content.trim() == live_content.trim());
                        if !content_exists {
                            let timestamp = get_unix_timestamp()?;
                            let backup_id = format!("backup-{timestamp}");
                            let backup_prompt = Prompt {
                                id: backup_id.clone(),
                                name: format!(
                                    "原始提示词 {}",
                                    chrono::Local::now().format("%Y-%m-%d %H:%M")
                                ),
                                content: live_content,
                                description: Some("自动备份的原始提示词".to_string()),
                                enabled: false,
                                ..Default::default()
                            };
                            log::info!("回填 live 提示词内容，创建备份: {backup_id}");
                            state.db.save_prompt(app.as_str(), &backup_prompt)?;
                        }
                    }
                }
            }
        }

        let mut prompts = state.db.get_prompts(app.as_str())?;

        for prompt in prompts.values_mut() {
            prompt.enabled = false;
        }

        if let Some(prompt) = prompts.get_mut(id) {
            if prompt.is_fragment {
                return Err(AppError::InvalidInput(
                    "不能直接启用规则片段，请启用其父 Prompt".into(),
                ));
            }
            let content = Self::resolve_effective_content(state, &app, prompt)?;
            prompt.enabled = true;
            write_text_file(&target_path, &content)?;
            state.db.save_prompt(app.as_str(), prompt)?;
        } else {
            return Err(AppError::InvalidInput(format!("提示词 {id} 不存在")));
        }

        for (_, prompt) in prompts.iter() {
            state.db.save_prompt(app.as_str(), prompt)?;
        }

        Ok(())
    }

    pub fn import_from_file(state: &AppState, app: &AppType) -> Result<String, AppError> {
        let file_path = prompt_file_path(&app)?;

        if !file_path.exists() {
            return Err(AppError::Message("提示词文件不存在".to_string()));
        }

        let content =
            std::fs::read_to_string(&file_path).map_err(|e| AppError::io(&file_path, e))?;
        let timestamp = get_unix_timestamp()?;

        let id = format!("imported-{timestamp}");
        let prompt = Prompt {
            id: id.clone(),
            name: format!(
                "导入的提示词 {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            ),
            content,
            description: Some("从现有配置文件导入".to_string()),
            enabled: false,
            created_at: Some(timestamp),
            updated_at: Some(timestamp),
            ..Default::default()
        };

        Self::upsert_prompt(state, &app, &id, prompt)?;
        Ok(id)
    }

    pub fn get_current_file_content(app: AppType) -> Result<Option<String>, AppError> {
        let file_path = prompt_file_path(&app)?;
        if !file_path.exists() {
            return Ok(None);
        }
        let content =
            std::fs::read_to_string(&file_path).map_err(|e| AppError::io(&file_path, e))?;
        Ok(Some(content))
    }

    pub fn import_from_file_on_first_launch(
        state: &AppState,
        app: &AppType,
    ) -> Result<usize, AppError> {
        let existing = state.db.get_prompts(app.as_str())?;
        if !existing.is_empty() {
            return Ok(0);
        }

        let file_path = prompt_file_path(&app)?;

        if !file_path.exists() {
            return Ok(0);
        }

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("读取提示词文件失败: {file_path:?}, 错误: {e}");
                return Ok(0);
            }
        };

        if content.trim().is_empty() {
            return Ok(0);
        }

        log::info!("发现提示词文件，自动导入: {file_path:?}");

        let timestamp = get_unix_timestamp()?;
        let id = format!("auto-imported-{timestamp}");
        let prompt = Prompt {
            id: id.clone(),
            name: format!(
                "Auto-imported Prompt {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            ),
            content,
            description: Some("Automatically imported on first launch".to_string()),
            enabled: true,
            created_at: Some(timestamp),
            updated_at: Some(timestamp),
            ..Default::default()
        };

        state.db.save_prompt(app.as_str(), &prompt)?;

        log::info!("自动导入完成: {}", app.as_str());
        Ok(1)
    }
}
