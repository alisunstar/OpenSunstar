//! 团队配置领域模型 — Git MVP Spike
//!
//! 定义团队工作空间、Profile、Release、Lock 等核心类型。
//! 设计原则（第二版 §4.1）：
//! - 声明式：团队发布"想要什么"，适配器决定如何写入
//! - 版本化：设备只应用已验证、不可变的发布版本
//! - 可解释：每个有效值都能追溯到来源
//! - 密钥分离：配置描述、凭证要求和凭证值是三个不同对象

use serde::{Deserialize, Serialize};

// ─── 来源类型 ──────────────────────────────────────────────────────────────────

/// 团队配置的来源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// 本地 Git 仓库
    Git,
    /// 本地目录（无版本控制）
    LocalDir,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::LocalDir => "local_dir",
        }
    }
}

// ─── 团队工作空间 ──────────────────────────────────────────────────────────────

/// 团队工作空间：一个连接到 Git 仓库或本地目录的团队配置源
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamWorkspace {
    pub workspace_id: String,
    pub name: String,
    pub source_kind: SourceKind,
    /// Git remote URL 或本地路径
    pub source_path: String,
    /// 当前跟踪的分支（Git 模式）
    pub branch: Option<String>,
    /// 最近一次成功 fetch/pull 的 commit SHA
    pub last_synced_commit: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

// ─── Profile ───────────────────────────────────────────────────────────────────

/// 目标 AI 工具标识
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum TargetApp {
    ClaudeCode,
    Codex,
    Cursor,
    Windsurf,
    /// 其他工具（扩展用）
    Other(String),
}

impl TargetApp {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ClaudeCode => "claude_code",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::Windsurf => "windsurf",
            Self::Other(s) => s,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "claude_code" | "claude" => Self::ClaudeCode,
            "codex" => Self::Codex,
            "cursor" => Self::Cursor,
            "windsurf" => Self::Windsurf,
            other => Self::Other(other.to_string()),
        }
    }
}

/// 8 类受管资产类型（第二版 §5）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Prompt,
    Rule,
    Skill,
    Ignore,
    Permission,
    Mcp,
    Command,
    Hook,
    Subagent,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::Rule => "rule",
            Self::Skill => "skill",
            Self::Ignore => "ignore",
            Self::Permission => "permission",
            Self::Mcp => "mcp",
            Self::Command => "command",
            Self::Hook => "hook",
            Self::Subagent => "subagent",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "prompt" => Some(Self::Prompt),
            "rule" => Some(Self::Rule),
            "skill" => Some(Self::Skill),
            "ignore" => Some(Self::Ignore),
            "permission" => Some(Self::Permission),
            "mcp" => Some(Self::Mcp),
            "command" => Some(Self::Command),
            "hook" => Some(Self::Hook),
            "subagent" => Some(Self::Subagent),
            _ => None,
        }
    }
}

/// Profile 中的单个资产条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAsset {
    pub asset_type: AssetType,
    pub asset_id: String,
    /// 资产内容或引用路径（相对于 team 包根目录）
    pub content_ref: String,
    /// 内容 SHA-256（发布时计算）
    pub content_sha256: Option<String>,
    /// 适用的目标工具（None = 全部）
    pub target_apps: Option<Vec<TargetApp>>,
    /// 风险等级（由安全扫描标注）
    pub risk_level: Option<RiskLevel>,
}

/// 资产风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    /// 需要显式信任（如 MCP 服务器、可执行 Hook）
    RequiresTrust,
}

/// 团队 Profile：一组命名的资产配置集合
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamProfile {
    pub profile_id: String,
    pub name: String,
    pub description: Option<String>,
    /// Profile 包含的资产列表
    pub assets: Vec<ProfileAsset>,
    /// 凭证槽位声明（不含明文值）
    pub credential_slots: Vec<CredentialSlot>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 凭证槽位声明（第二版 §9：配置描述与凭证值分离）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSlot {
    pub slot_id: String,
    /// 凭证类型：oauth, api_key, token
    pub kind: String,
    /// 供应商标识（如 github, openai）
    pub provider: Option<String>,
    /// 人类可读描述
    pub description: Option<String>,
    /// 是否必需（false = 可选，缺失时降级而非阻断）
    pub required: bool,
}

// ─── Policy ────────────────────────────────────────────────────────────────────

/// 团队策略规则
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRule {
    pub rule_id: String,
    pub asset_type: AssetType,
    /// 匹配的资产 ID 模式（精确匹配或 "*" 通配）
    pub asset_pattern: String,
    pub action: super::requirements::PolicyAction,
    /// 适用的目标工具（None = 全部）
    pub target_apps: Option<Vec<TargetApp>>,
    /// 约束条件 JSON（如版本范围、路径限制）
    pub constraint_json: String,
    /// 规则说明
    pub reason: Option<String>,
}

// ─── Release & Lock ────────────────────────────────────────────────────────────

/// 团队配置发布：不可变的版本化快照
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamRelease {
    pub release_id: String,
    pub workspace_id: String,
    /// 语义化版本号或递增序号
    pub version_label: String,
    /// 发布包含的 Profile IDs
    pub profile_ids: Vec<String>,
    /// 发布包含的策略规则
    pub policies: Vec<PolicyRule>,
    /// 源 commit SHA（Git 模式）
    pub source_commit: Option<String>,
    /// 发布者
    pub published_by: String,
    pub published_at: i64,
    /// 发布状态
    pub status: ReleaseStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStatus {
    /// 草稿（未发布）
    Draft,
    /// 已发布（可被设备拉取）
    Published,
    /// 已撤回
    Retracted,
}

/// lock.json：不可变发布清单（Spike 验证项 3）
///
/// 包含发布中每个文件的 SHA-256 摘要，用于：
/// - 完整性校验（篡改检测）
/// - 增量同步（只拉取变更文件）
/// - 兼容性矩阵（哪些工具版本支持此发布）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseLock {
    /// lock 格式版本
    pub lock_version: u32,
    pub release_id: String,
    pub version_label: String,
    /// 源 commit SHA
    pub source_commit: Option<String>,
    /// 生成时间戳（Unix ms）
    pub generated_at: i64,
    /// 文件清单：路径 → SHA-256
    pub manifests: Vec<LockManifestEntry>,
    /// 整个 lock 的确定性摘要（自引用，生成时最后计算）
    pub lock_sha256: String,
    /// 兼容性矩阵
    pub compatibility: Option<CompatibilityMatrix>,
}

/// lock.json 中的单个文件条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockManifestEntry {
    /// 相对于 team 包根目录的路径
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    /// 资产类型标注（可选，用于快速过滤）
    pub asset_type: Option<AssetType>,
}

/// 兼容性矩阵：声明此发布支持的工具及版本范围
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityMatrix {
    pub entries: Vec<CompatibilityEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityEntry {
    pub target_app: TargetApp,
    /// 最低兼容版本（语义化版本字符串）
    pub min_version: Option<String>,
    /// 最高兼容版本（None = 无上限）
    pub max_version: Option<String>,
}

// ─── Git 安全状态 ──────────────────────────────────────────────────────────────

/// Git 工作树安全状态（Spike 验证项 2）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSafetyState {
    /// 是否为 Git 仓库
    pub is_git_repo: bool,
    /// 工作树是否干净（无未提交变更）
    pub is_clean: bool,
    /// 当前分支
    pub current_branch: Option<String>,
    /// HEAD commit SHA
    pub head_commit: Option<String>,
    /// 远程 URL
    pub remote_url: Option<String>,
    /// 是否可 fast-forward（本地落后于远程）
    pub can_fast_forward: bool,
    /// 是否分叉（本地和远程各有独立提交）
    pub is_diverged: bool,
    /// 阻断原因（如果任何安全检查失败）
    pub abort_reason: Option<GitAbortReason>,
}

impl GitSafetyState {
    /// 是否允许执行 pull --ff-only
    pub fn can_pull(&self) -> bool {
        self.is_git_repo && self.is_clean && !self.is_diverged && self.abort_reason.is_none()
    }
}

/// Git 操作阻断原因
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitAbortReason {
    /// 工作树有未提交变更
    DirtyWorktree,
    /// 本地与远程分叉
    Diverged,
    /// 存在未解决的合并冲突
    MergeConflict,
    /// 凭证错误（无法访问远程）
    CredentialError,
    /// 不是 Git 仓库
    NotARepo,
    /// 其他错误
    Other(String),
}

// ─── team.toml 解析结构 ────────────────────────────────────────────────────────

/// team.toml 的顶层结构（Spike 解析用）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamToml {
    /// 团队包元数据
    #[serde(default)]
    pub team: TeamTomlMeta,
    /// Profile 定义
    #[serde(default)]
    pub profiles: Vec<TomlProfile>,
    /// 策略规则
    #[serde(default)]
    pub policies: Vec<TomlPolicyRule>,
    /// 凭证槽位声明
    #[serde(default)]
    pub credential_slots: Vec<TomlCredentialSlot>,
    /// 绑定（凭证 → 环境变量映射）
    #[serde(default)]
    pub bindings: Option<toml::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TeamTomlMeta {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// 兼容性声明
    #[serde(default)]
    pub compatibility: Vec<TomlCompatibility>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TomlProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// 资产引用列表
    #[serde(default)]
    pub assets: Vec<TomlAssetRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TomlAssetRef {
    #[serde(rename = "type")]
    pub asset_type: String,
    pub id: String,
    /// 文件路径（相对于 team 包根目录）
    pub path: Option<String>,
    /// 内联内容（与 path 二选一）
    pub content: Option<String>,
    /// 适用工具
    #[serde(default)]
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TomlPolicyRule {
    #[serde(rename = "type")]
    pub asset_type: String,
    /// 资产 ID 模式
    pub pattern: String,
    /// required | recommended | denied
    pub action: String,
    pub reason: Option<String>,
    #[serde(default)]
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TomlCredentialSlot {
    pub id: String,
    pub kind: String,
    pub provider: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TomlCompatibility {
    pub app: String,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
}
