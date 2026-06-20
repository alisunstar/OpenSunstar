use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::hook::{validate_hook_event_type, validate_timeout, Hook};
use crate::services::claude_settings::ClaudeSettingsMerger;
use crate::store::AppState;

pub struct HookService;

impl HookService {
    pub fn get_all_hooks(state: &AppState) -> Result<Vec<Hook>, AppError> {
        state.db.get_all_hooks()
    }

    pub fn upsert_hook(state: &AppState, hook: Hook) -> Result<(), AppError> {
        validate_hook_event_type(&hook.event_type).map_err(AppError::Config)?;
        validate_timeout(hook.timeout_seconds).map_err(AppError::Config)?;
        state.db.save_hook(&hook)?;
        Self::sync_hooks_to_claude(state)
    }

    pub fn delete_hook(state: &AppState, id: &str) -> Result<bool, AppError> {
        let existed = state.db.get_all_hooks()?.iter().any(|h| h.id == id);
        if !existed {
            return Ok(false);
        }
        state.db.delete_hook(id)?;
        Self::sync_hooks_to_claude(state)?;
        Ok(true)
    }

    pub fn sync_hooks_to_claude(state: &AppState) -> Result<(), AppError> {
        let hooks = state
            .db
            .get_all_hooks()?
            .into_iter()
            .filter(|h| h.enabled_claude)
            .collect::<Vec<_>>();

        let mut hooks_map: Map<String, Value> = Map::new();
        for hook in hooks {
            let entry = json!({
                "matcher": hook.tool_pattern,
                "hooks": [{
                    "type": "command",
                    "command": hook.hook_command,
                    "timeout": hook.timeout_seconds
                }]
            });
            hooks_map
                .entry(hook.event_type.clone())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .expect("hooks array")
                .push(entry);
        }

        ClaudeSettingsMerger::update_field("hooks", Value::Object(hooks_map))
    }
}
