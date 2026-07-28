//! Team configuration: deterministic resolution of independent policy
//! sources before they are compiled into project expectations.
//!
//! Git MVP Spike 模块：
//! - domain: 领域类型（TeamWorkspace, Profile, Release, Lock, Policy）
//! - git_runner: 受限 Git 适配器（ff-only, 零 Shell 拼接）
//! - release: lock.json 生成与 SHA-256 校验
//! - requirements: 多来源策略解析（已有）
//! - security: 团队包安全扫描（已有）

pub mod assignment;
pub mod audit;
pub mod credentials;
pub mod deployment;
pub mod diff;
pub mod domain;
pub mod drift;
pub mod effective_state;
pub mod executor;
pub mod git_runner;
pub mod parser;
pub mod release;
pub mod repository;
pub mod requirements;
pub mod security;
pub mod validator;

pub use assignment::{
    generate_assignment_id, validate_assignment, AssignmentError, AssignmentStatus, TeamAssignment,
};
pub use audit::{
    generate_event_id, make_audit_event, AuditAction, TeamAuditEvent, LOCAL_DEVICE_ACTOR,
};
pub use credentials::{
    bind_credential, check_credentials_status, compute_credential_summary, credential_entry_key,
    unbind_credential, verify_credential, CredentialBindingInfo, CredentialStatus,
    CredentialSummary,
};
pub use deployment::{
    generate_deployment_plan, is_deployable, resolve_target_absolute_path,
    resolve_target_relative_path, scan_current_sha256, DeploymentAction, DeploymentPlan,
    DeploymentStep, PlanSummary, PlanWarning, WarningCode,
};
pub use diff::{
    diff_lock_vs_directory, diff_manifests, diff_two_locks, DiffAction, DiffEntry, DiffSummary,
    ReleaseDiff,
};
pub use domain::{
    AssetType, CredentialSlot, GitAbortReason, GitSafetyState, LockManifestEntry, PolicyRule,
    ProfileAsset, ReleaseLock, ReleaseStatus, RiskLevel, SourceKind, TargetApp, TeamProfile,
    TeamRelease, TeamToml, TeamWorkspace,
};
pub use drift::{
    detect_drift, execute_rollback, DriftEntry, DriftReport, DriftStatus, DriftSummary,
    RollbackReport, RollbackStepResult, RollbackSummary,
};
pub use effective_state::{
    compile_effective_config, CompilerInput, ConflictCode, EffectiveConfig, EffectiveConflict,
    EffectiveDecision, EffectiveItem, PersonalOverride, PersonalPreference, ProjectAssetInput,
    ProvenanceEntry, SourceTier,
};
pub use executor::{
    execute_deployment_plan, DeploymentReceipt, ExecuteOptions, ReceiptSummary, StepReceipt,
};
pub use git_runner::GitRunner;
pub use parser::{
    build_credential_slots, build_policies, build_profiles, parse_team_package, parse_team_toml,
    TeamTomlError,
};
pub use release::{generate_lock, validate_lock, LockValidationError};
pub use repository::{connect_team_source, ConnectError, ConnectResult, ConnectWarning};
pub use requirements::{
    resolve_requirement_sources, PolicyAction, RequirementKey, RequirementResolution,
    RequirementResolutionPlan, RequirementSource, ResolutionConflict,
};
pub use security::{
    enforce_team_package_security, validate_team_package_security, TeamPackageSecurityFinding,
    TeamPackageSecurityReport,
};
pub use validator::{
    validate_team_package, validate_team_package_dir, ValidationCode, ValidationIssue,
    ValidationOptions, ValidationReport,
};
