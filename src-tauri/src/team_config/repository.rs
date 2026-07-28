//! 团队配置源连接管理（Local Alpha L1）
//!
//! 职责：
//! - 接受本地目录或 Git 仓库路径，检测来源类型
//! - 验证 team.toml 存在且可解析
//! - Git 模式下执行安全状态检查（复用 GitRunner）
//! - 返回连接结果（工作空间元数据 + 解析后的 team.toml）
//!
//! 设计约束（第二版 §4.4）：
//! - 不自动 clone、stash、rebase 或修改 Git 配置
//! - 连接是只读操作，不写入任何文件
//! - Git dirty/diverged 不阻断连接（只阻断后续 pull），但会标注状态

use std::path::{Path, PathBuf};

use super::domain::{SourceKind, TeamToml, TeamWorkspace};
use super::git_runner::GitRunner;
use super::parser::{parse_team_toml, TeamTomlError};

/// 连接源时的验证结果
#[derive(Debug, Clone)]
pub struct ConnectResult {
    /// 构建的工作空间对象
    pub workspace: TeamWorkspace,
    /// 解析后的 team.toml
    pub team_toml: TeamToml,
    /// Git 安全状态（Git 模式；LocalDir 模式为 None）
    pub git_safety: Option<super::domain::GitSafetyState>,
    /// 连接期间的非致命警告
    pub warnings: Vec<ConnectWarning>,
}

/// 连接期间的非致命警告
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectWarning {
    /// Git 工作树不干净（不阻断连接，但阻断后续 pull）
    DirtyWorktree,
    /// 本地与远程分叉
    Diverged,
    /// 无远程 origin（Git 模式但无远程）
    NoRemote,
    /// team.toml 缺少版本号
    MissingVersion,
    /// team.toml 缺少团队名称
    MissingName,
}

impl std::fmt::Display for ConnectWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DirtyWorktree => write!(
                f,
                "git worktree is not clean; pull will be blocked until committed"
            ),
            Self::Diverged => write!(
                f,
                "local branch has diverged from remote; pull will be blocked"
            ),
            Self::NoRemote => write!(f, "no remote 'origin' configured; sync unavailable"),
            Self::MissingVersion => write!(f, "team.toml [team].version is not set"),
            Self::MissingName => write!(f, "team.toml [team].name is not set"),
        }
    }
}

/// 连接错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectError {
    /// 路径不存在
    PathNotFound(String),
    /// 路径不是目录
    NotADirectory(String),
    /// team.toml 不存在
    TeamTomlNotFound(String),
    /// team.toml 解析失败
    TeamTomlParseError(String),
    /// 读取 team.toml 文件失败
    TeamTomlReadError(String),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathNotFound(p) => write!(f, "connect_path_not_found: {p}"),
            Self::NotADirectory(p) => write!(f, "connect_not_a_directory: {p}"),
            Self::TeamTomlNotFound(p) => {
                write!(
                    f,
                    "connect_team_toml_not_found: {p}/team.toml does not exist"
                )
            }
            Self::TeamTomlParseError(msg) => write!(f, "connect_team_toml_parse_error: {msg}"),
            Self::TeamTomlReadError(msg) => write!(f, "connect_team_toml_read_error: {msg}"),
        }
    }
}

impl std::error::Error for ConnectError {}

impl From<TeamTomlError> for ConnectError {
    fn from(e: TeamTomlError) -> Self {
        Self::TeamTomlParseError(e.to_string())
    }
}

/// 连接到团队配置源
///
/// 这是 Local Alpha 的入口函数。执行以下步骤：
/// 1. 验证路径存在且为目录
/// 2. 检测 team.toml 是否存在
/// 3. 判断来源类型（Git / LocalDir）
/// 4. Git 模式下获取安全状态
/// 5. 解析 team.toml
/// 6. 构建 TeamWorkspace + 收集警告
pub fn connect_team_source(path: &Path) -> Result<ConnectResult, ConnectError> {
    // 1. 验证路径
    if !path.exists() {
        return Err(ConnectError::PathNotFound(path.display().to_string()));
    }
    if !path.is_dir() {
        return Err(ConnectError::NotADirectory(path.display().to_string()));
    }

    // 2. 检测 team.toml
    let team_toml_path = path.join("team.toml");
    if !team_toml_path.exists() {
        return Err(ConnectError::TeamTomlNotFound(path.display().to_string()));
    }

    // 3. 判断来源类型
    let source_kind = detect_source_kind(path);

    // 4. Git 模式下获取安全状态
    let git_safety = if source_kind == SourceKind::Git {
        let runner = GitRunner::new(path);
        Some(runner.safety_state())
    } else {
        None
    };

    // 5. 解析 team.toml
    let content = std::fs::read_to_string(&team_toml_path)
        .map_err(|e| ConnectError::TeamTomlReadError(e.to_string()))?;
    let team_toml = parse_team_toml(&content)?;

    // 6. 收集警告
    let mut warnings = Vec::new();
    collect_warnings(&team_toml, &git_safety, &mut warnings);

    // 7. 构建 workspace
    let now = chrono::Utc::now().timestamp_millis();
    let workspace_id = generate_workspace_id(path);

    let (branch, last_commit, _remote_url) = if let Some(state) = &git_safety {
        (
            state.current_branch.clone(),
            state.head_commit.clone(),
            state.remote_url.clone(),
        )
    } else {
        (None, None, None)
    };

    let workspace = TeamWorkspace {
        workspace_id,
        name: team_toml
            .team
            .name
            .clone()
            .unwrap_or_else(|| "Unnamed Team".to_string()),
        source_kind,
        source_path: path.to_string_lossy().replace('\\', "/"),
        branch,
        last_synced_commit: last_commit,
        created_at: now,
        updated_at: now,
    };

    // remote_url 不存入 workspace（避免冗余），但警告中已覆盖 NoRemote

    Ok(ConnectResult {
        workspace,
        team_toml,
        git_safety,
        warnings,
    })
}

/// 检测来源类型：是否为 Git 仓库
fn detect_source_kind(path: &Path) -> SourceKind {
    // 检查 .git 目录或文件（worktree 模式下 .git 是文件）
    let git_marker = path.join(".git");
    if git_marker.exists() {
        SourceKind::Git
    } else {
        // 也检查是否在 Git 工作树内（子目录场景）
        let runner = GitRunner::new(path);
        let state = runner.safety_state();
        if state.is_git_repo {
            SourceKind::Git
        } else {
            SourceKind::LocalDir
        }
    }
}

/// 收集非致命警告
fn collect_warnings(
    team_toml: &TeamToml,
    git_safety: &Option<super::domain::GitSafetyState>,
    warnings: &mut Vec<ConnectWarning>,
) {
    // team.toml 元数据警告
    if team_toml.team.name.is_none() {
        warnings.push(ConnectWarning::MissingName);
    }
    if team_toml.team.version.is_none() {
        warnings.push(ConnectWarning::MissingVersion);
    }

    // Git 状态警告
    if let Some(state) = git_safety {
        if !state.is_clean {
            warnings.push(ConnectWarning::DirtyWorktree);
        }
        if state.is_diverged {
            warnings.push(ConnectWarning::Diverged);
        }
        if state.remote_url.is_none() {
            warnings.push(ConnectWarning::NoRemote);
        }
    }
}

/// 生成工作空间 ID（基于路径的确定性标识）
fn generate_workspace_id(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let hash = Sha256::digest(canonical.to_string_lossy().as_bytes());
    let hex: String = hash[..8].iter().map(|b| format!("{b:02x}")).collect();
    format!("ws_{hex}")
}

/// 获取团队包的 team.toml 路径（如果存在）
pub fn find_team_toml(root: &Path) -> Option<PathBuf> {
    let candidate = root.join("team.toml");
    if candidate.exists() && candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const MINIMAL_TEAM_TOML: &str = r#"
[team]
name = "Test Team"
version = "1.0.0"
"#;

    fn setup_local_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("team.toml"), MINIMAL_TEAM_TOML).expect("write");
        dir
    }

    fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("team.toml"), MINIMAL_TEAM_TOML).expect("write");
        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .expect("git");
        };
        run(&["init"]);
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["add", "."]);
        run(&["commit", "-m", "init"]);
        dir
    }

    #[test]
    fn connects_to_local_directory() {
        let dir = setup_local_dir();
        let result = connect_team_source(dir.path()).expect("connect");

        assert_eq!(result.workspace.source_kind, SourceKind::LocalDir);
        assert_eq!(result.workspace.name, "Test Team");
        assert!(result.git_safety.is_none());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn connects_to_git_repository() {
        let dir = setup_git_repo();
        let result = connect_team_source(dir.path()).expect("connect");

        assert_eq!(result.workspace.source_kind, SourceKind::Git);
        assert!(result.git_safety.is_some());
        let safety = result.git_safety.unwrap();
        assert!(safety.is_git_repo);
        assert!(safety.is_clean);
        assert!(safety.head_commit.is_some());
    }

    #[test]
    fn warns_on_dirty_git_worktree() {
        let dir = setup_git_repo();
        fs::write(dir.path().join("dirty.txt"), "uncommitted").expect("dirty");

        let result = connect_team_source(dir.path()).expect("connect");
        assert!(result.warnings.contains(&ConnectWarning::DirtyWorktree));
    }

    #[test]
    fn warns_on_missing_name_and_version() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("team.toml"), "[team]\n").expect("write");

        let result = connect_team_source(dir.path()).expect("connect");
        assert!(result.warnings.contains(&ConnectWarning::MissingName));
        assert!(result.warnings.contains(&ConnectWarning::MissingVersion));
        assert_eq!(result.workspace.name, "Unnamed Team");
    }

    #[test]
    fn rejects_nonexistent_path() {
        let result = connect_team_source(Path::new("/nonexistent/path/xyz"));
        assert!(matches!(result, Err(ConnectError::PathNotFound(_))));
    }

    #[test]
    fn rejects_missing_team_toml() {
        let dir = tempfile::tempdir().expect("temp dir");
        let result = connect_team_source(dir.path());
        assert!(matches!(result, Err(ConnectError::TeamTomlNotFound(_))));
    }

    #[test]
    fn rejects_invalid_team_toml() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("team.toml"), "invalid [[[ toml").expect("write");
        let result = connect_team_source(dir.path());
        assert!(matches!(result, Err(ConnectError::TeamTomlParseError(_))));
    }

    #[test]
    fn workspace_id_is_deterministic() {
        let dir = setup_local_dir();
        let id1 = generate_workspace_id(dir.path());
        let id2 = generate_workspace_id(dir.path());
        assert_eq!(id1, id2);
        assert!(id1.starts_with("ws_"));
    }

    #[test]
    fn warns_no_remote_on_git_without_origin() {
        let dir = setup_git_repo();
        let result = connect_team_source(dir.path()).expect("connect");
        // 本地 init 的仓库没有 remote
        assert!(result.warnings.contains(&ConnectWarning::NoRemote));
    }
}
