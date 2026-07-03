use crate::app_config::AppType;
use crate::error::AppError;
use crate::hook::{validate_hook_event_type, validate_timeout, Hook};
use crate::services::hook_sync;
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
        Self::sync_all_apps(state)
    }

    pub fn delete_hook(state: &AppState, id: &str) -> Result<bool, AppError> {
        let existed = state.db.get_all_hooks()?.iter().any(|h| h.id == id);
        if !existed {
            return Ok(false);
        }
        state.db.delete_hook(id)?;
        Self::sync_all_apps(state)?;
        Ok(true)
    }

    pub fn toggle_app(
        state: &AppState,
        hook_id: &str,
        app: AppType,
        enabled: bool,
    ) -> Result<(), AppError> {
        let mut hooks = state.db.get_all_hooks()?;
        if let Some(hook) = hooks.iter_mut().find(|h| h.id == hook_id) {
            hook.set_enabled_for(&app, enabled);
            let snapshot = hook.clone();
            state.db.save_hook(&snapshot)?;
            hook_sync::sync_app(state, &app)?;
        }
        Ok(())
    }

    pub fn sync_all_apps(state: &AppState) -> Result<(), AppError> {
        hook_sync::sync_all_apps(state)
    }

    /// Backward-compatible alias for Claude-only callers.
    pub fn sync_hooks_to_claude(state: &AppState) -> Result<(), AppError> {
        hook_sync::sync_app(state, &AppType::Claude)
    }
}
