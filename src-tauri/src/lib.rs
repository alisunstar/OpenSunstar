// 顶层私有 `agent` 模块与 `pub use commands::*;`（其中含 `pub mod agent`）同名，
// 刻意保留私有模块遮蔽 glob 再导出，可见性语义不变。
#[allow(hidden_glob_reexports)]
mod agent;
mod app;
mod app_config;
mod app_store;
mod audit;
mod auto_launch;
mod claude_desktop_config;
mod claude_mcp;
mod claude_plugin;
mod codex_config;
mod codex_history_migration;
mod command;
mod commands;
mod config;
mod database;
mod deeplink;
mod error;
mod gemini_config;
mod grok_config;
mod gemini_mcp;
pub mod hermes_config;
mod hook;
mod ignore_rule;
mod init_status;
pub mod keychain;
mod lightweight;
#[cfg(target_os = "linux")]
mod linux_fix;
mod mcp;
mod mcp_connection_test;
mod mcp_registry;
mod mcp_secret;
mod mcp_smithery;
mod openclaw_config;
mod opencode_config;
mod panic_hook;
pub mod product_auth;
mod prompt;
mod prompt_files;
mod provider;
mod provider_defaults;
mod provider_keychain;
mod proxy;
mod services;
mod session_manager;
mod settings;
mod store;
pub mod team_config;
pub mod team_key;
mod tool_permission;

mod ai;
mod project_metrics;
mod tray;
mod usage_events;
mod usage_script;

pub mod cli_api;

pub use app_config::{AppType, InstalledSkill, McpApps, McpServer, MultiAppConfig, SkillApps};
pub use codex_config::{get_codex_auth_path, get_codex_config_path, write_codex_live_atomic};
pub use commands::open_provider_terminal;
pub use commands::*;
pub use config::{
    get_app_config_dir, get_claude_mcp_path, get_claude_settings_path, read_json_file,
};
pub use database::Database;
pub use deeplink::{import_provider_from_deeplink, parse_deeplink_url, DeepLinkImportRequest};
pub use error::AppError;
pub use mcp::{
    import_from_claude, import_from_codex, import_from_gemini, remove_server_from_claude,
    remove_server_from_codex, remove_server_from_gemini, sync_enabled_to_claude,
    sync_enabled_to_codex, sync_enabled_to_gemini, sync_single_server_to_claude,
    sync_single_server_to_codex, sync_single_server_to_gemini,
};
pub use provider::{Provider, ProviderMeta};
pub use services::project_wiki;
pub use services::knowledge_routing;
pub use services::rd_validate;
pub use services::{
    simple_connect,
    skill::{migrate_skills_to_ssot, ImportSkillSelection},
    ConfigService, EndpointLatency, McpService, PromptService, ProviderService, ProxyService,
    SkillService, SpeedtestService,
};
pub use settings::{update_settings, AppSettings};
pub use store::AppState;

// CLI 需要的 AI 治理类型
pub use ai::asset_effective_state::{
    EffectiveItemState, EffectiveScanContext, EffectiveScanResult, RepairAssetDriftResult,
    RepairProjectDriftResult,
};
pub use ai::asset_health::{get_project_asset_health, AssetHealthPlan, AssetHealthRecord};
pub use ai::types::AgentReadinessItem;

// CLI Phase B/C 类型重导出（供 `os` CLI 二进制直接引用，无需访问私有模块）
pub use cli_api::ProjectContext;
pub use database::Project;
pub use database::ProjectAllAssetCounts;
pub use services::blueprint::{Blueprint, BlueprintApplyPreview, BlueprintLinkAction};
pub use services::design_contract::{
    DesignColors, DesignContract, DesignContractParams, DesignElevation, DesignGuardrail,
    DesignInstallPlan, DesignInstallResult, DesignShapes, DesignSpacing, DesignTypography,
    ImportResult as DesignImportResult, InstallAuditFinding, InstallAuditSummary, InstallFileEntry,
};
pub use services::flow_orchestrator::{
    FlowConfig, FlowConfigGate, FlowConfigRules, FlowConfigStage, SpecsChangeIndex,
    SpecsWorkflowIndex, StageGateResult, WorkflowModule, WorkflowPreset, WorkflowPresetPaths,
    WorkflowPresetSummary, WorkflowProfile, WorkflowStage, WorkflowStageSkipWhen,
};
pub use services::orchestration_plan::{
    OrchestrationReceipt, OrchestrationStepReceipt, OrchestrationVerification,
};
pub use services::project_environment::{
    ProjectEnvironmentApplyPreview, ProjectEnvironmentApplyReceipt, ProjectEnvironmentDiff,
    ProjectEnvironmentDimension, ProjectEnvironmentSnapshotDto, ProjectEnvironmentVerification,
};
pub use services::provider::{VerifyKeyResult, VerifyProtocol};
pub use services::recipe_composer::{
    CompositionRecipe, InstallResult as RecipeInstallResult, RecipeArtifact, RecipeComposeParams,
    RecipeInstallPlan, RecipeRule, RecipeStage, StageGraph, StageGraphEdge, StageGraphNode,
};
pub use services::sdd::{SddDescriptorSummary, SddDetectionResult, SignalMatch};

pub(crate) use app::remove_tray_icon_before_exit;
pub use app::{
    cleanup_before_exit, destroy_single_instance_lock, restart_process,
    save_window_state_before_exit,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run();
}
