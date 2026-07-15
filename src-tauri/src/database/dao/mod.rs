//! Data Access Object layer
//!
//! Database access operations for each domain

pub mod agent;
pub mod ai_insight;
pub mod asset_health;
pub mod command;
pub mod failover;
pub mod hook;
pub mod ignore;
pub mod mcp;
pub mod permission;
pub mod project_assets;
pub mod project_environment;
pub mod projects;
pub mod prompts;
pub mod providers;
pub mod providers_seed;
pub mod proxy;
pub mod quick_start;
pub mod sdd;
pub mod settings;
pub mod skills;
pub mod stream_check;
pub mod universal_providers;
pub mod usage_rollup;

// 所有 DAO 方法都通过 Database impl 提供，无需单独导出
// 导出 FailoverQueueItem 供外部使用
pub use failover::FailoverQueueItem;
pub use projects::{Project, ProjectConfigLink, ProjectPromptLink};
