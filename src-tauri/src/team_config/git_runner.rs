//! 受限 Git 命令适配器（第二版 §4.4）
//!
//! 安全约束：
//! - 仅参数化调用 `git` 可执行文件，**零 Shell 拼接**
//! - 允许：status, rev-parse, remote get-url, fetch, pull --ff-only
//! - 仅在干净工作树且可 fast-forward 时执行 pull
//! - 脏工作树、冲突、分叉、凭证错误 → 立即中止并引导到外部 Git 工具
//! - 不自动 stash、rebase、force push 或修改全局 Git 配置

use std::path::Path;
use std::process::Command;

use super::domain::{GitAbortReason, GitSafetyState};

/// Git 命令执行结果
#[derive(Debug)]
pub struct GitOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// 受限 Git Runner：所有 Git 操作的唯一入口
pub struct GitRunner {
    /// 工作目录（team 包所在路径）
    work_dir: std::path::PathBuf,
}

impl GitRunner {
    pub fn new(work_dir: impl AsRef<Path>) -> Self {
        Self {
            work_dir: work_dir.as_ref().to_path_buf(),
        }
    }

    // ─── 安全状态检测 ──────────────────────────────────────────────────────────

    /// 获取完整的 Git 安全状态（所有后续操作的前置检查）
    pub fn safety_state(&self) -> GitSafetyState {
        // 1. 是否为 Git 仓库
        if !self.is_git_repo() {
            return GitSafetyState {
                is_git_repo: false,
                is_clean: false,
                current_branch: None,
                head_commit: None,
                remote_url: None,
                can_fast_forward: false,
                is_diverged: false,
                abort_reason: Some(GitAbortReason::NotARepo),
            };
        }

        let current_branch = self.current_branch();
        let head_commit = self.head_commit();
        let remote_url = self.remote_url();
        let is_clean = self.is_worktree_clean();
        let has_conflict = self.has_merge_conflict();

        // 分叉检测：fetch 后比较 local vs remote
        let (can_ff, is_diverged) = if remote_url.is_some() && current_branch.is_some() {
            self.check_divergence(current_branch.as_deref().unwrap_or("main"))
        } else {
            (false, false)
        };

        // 确定阻断原因
        let abort_reason = if has_conflict {
            Some(GitAbortReason::MergeConflict)
        } else if !is_clean {
            Some(GitAbortReason::DirtyWorktree)
        } else if is_diverged {
            Some(GitAbortReason::Diverged)
        } else {
            None
        };

        GitSafetyState {
            is_git_repo: true,
            is_clean,
            current_branch,
            head_commit,
            remote_url,
            can_fast_forward: can_ff,
            is_diverged,
            abort_reason,
        }
    }

    // ─── 允许的 Git 操作 ───────────────────────────────────────────────────────

    /// fetch 远程（只读操作，安全）
    pub fn fetch(&self) -> Result<GitOutput, GitAbortReason> {
        let output = self.run_git(&["fetch", "--quiet"])?;
        if !output.success {
            // 检查是否为凭证错误
            if is_credential_error(&output.stderr) {
                return Err(GitAbortReason::CredentialError);
            }
            return Err(GitAbortReason::Other(output.stderr.clone()));
        }
        Ok(output)
    }

    /// pull --ff-only（仅在安全状态允许时执行）
    ///
    /// 前置条件：
    /// - 工作树干净
    /// - 无合并冲突
    /// - 未分叉
    /// - 可 fast-forward
    pub fn pull_ff_only(&self) -> Result<GitOutput, GitAbortReason> {
        let state = self.safety_state();
        if !state.can_pull() {
            return Err(state.abort_reason.unwrap_or(GitAbortReason::Other(
                "cannot pull: safety check failed".into(),
            )));
        }
        if !state.can_fast_forward {
            return Err(GitAbortReason::Other(
                "already up to date or cannot fast-forward".into(),
            ));
        }

        let output = self.run_git(&["pull", "--ff-only", "--quiet"])?;
        if !output.success {
            if is_credential_error(&output.stderr) {
                return Err(GitAbortReason::CredentialError);
            }
            return Err(GitAbortReason::Other(output.stderr.clone()));
        }
        Ok(output)
    }

    /// 获取指定文件的最新内容（从 HEAD）
    pub fn show_file(&self, relative_path: &str) -> Result<String, GitAbortReason> {
        let output = self.run_git(&["show", &format!("HEAD:{relative_path}")])?;
        if !output.success {
            return Err(GitAbortReason::Other(format!(
                "git show HEAD:{relative_path} failed: {}",
                output.stderr
            )));
        }
        Ok(output.stdout)
    }

    /// 列出仓库中指定目录下的所有文件
    pub fn list_files(&self, prefix: &str) -> Result<Vec<String>, GitAbortReason> {
        let path_spec = if prefix.is_empty() { "." } else { prefix };
        let output = self.run_git(&["ls-tree", "-r", "--name-only", "HEAD", path_spec])?;
        if !output.success {
            return Err(GitAbortReason::Other(output.stderr.clone()));
        }
        Ok(output
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect())
    }

    // ─── 内部方法（参数化调用，零 Shell 拼接） ─────────────────────────────────

    fn run_git(&self, args: &[&str]) -> Result<GitOutput, GitAbortReason> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.work_dir)
            .output()
            .map_err(|e| GitAbortReason::Other(format!("git 执行失败: {e}")))?;

        Ok(GitOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn is_git_repo(&self) -> bool {
        self.run_git(&["rev-parse", "--is-inside-work-tree"])
            .map(|o| o.success && o.stdout.trim() == "true")
            .unwrap_or(false)
    }

    fn current_branch(&self) -> Option<String> {
        self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .ok()
            .filter(|o| o.success)
            .map(|o| o.stdout.trim().to_string())
            .filter(|b| b != "HEAD") // detached HEAD
    }

    fn head_commit(&self) -> Option<String> {
        self.run_git(&["rev-parse", "HEAD"])
            .ok()
            .filter(|o| o.success)
            .map(|o| o.stdout.trim().to_string())
    }

    fn remote_url(&self) -> Option<String> {
        self.run_git(&["remote", "get-url", "origin"])
            .ok()
            .filter(|o| o.success)
            .map(|o| o.stdout.trim().to_string())
    }

    fn is_worktree_clean(&self) -> bool {
        self.run_git(&["status", "--porcelain"])
            .map(|o| o.success && o.stdout.trim().is_empty())
            .unwrap_or(false)
    }

    fn has_merge_conflict(&self) -> bool {
        // 检查 .git/MERGE_HEAD 是否存在（表示正在合并中）
        self.run_git(&["rev-parse", "--verify", "MERGE_HEAD"])
            .map(|o| o.success)
            .unwrap_or(false)
    }

    /// 检查本地与远程的分叉状态
    /// 返回 (can_fast_forward, is_diverged)
    fn check_divergence(&self, branch: &str) -> (bool, bool) {
        let tracking = format!("origin/{branch}");

        // 获取本地领先/落后的 commit 数
        let output = match self.run_git(&[
            "rev-list",
            "--left-right",
            "--count",
            &format!("HEAD...{tracking}"),
        ]) {
            Ok(o) if o.success => o,
            _ => return (false, false), // 无法比较（可能无远程分支）
        };

        // 输出格式: "ahead\tbehind"
        let parts: Vec<&str> = output.stdout.trim().split('\t').collect();
        if parts.len() != 2 {
            return (false, false);
        }

        let ahead: u32 = parts[0].parse().unwrap_or(0);
        let behind: u32 = parts[1].parse().unwrap_or(0);

        let can_ff = ahead == 0 && behind > 0;
        let is_diverged = ahead > 0 && behind > 0;
        (can_ff, is_diverged)
    }
}

/// 检测 Git 错误输出中的凭证问题
fn is_credential_error(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("authentication failed")
        || lower.contains("could not read username")
        || lower.contains("permission denied")
        || lower.contains("invalid credentials")
        || lower.contains("terminal prompts disabled")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        let runner = GitRunner::new(dir.path());
        runner.run_git(&["init"]).expect("git init");
        runner
            .run_git(&["config", "user.email", "test@test.com"])
            .expect("config email");
        runner
            .run_git(&["config", "user.name", "Test"])
            .expect("config name");
        // 创建初始 commit
        fs::write(dir.path().join("team.toml"), "[team]\nname = \"test\"").expect("write");
        runner.run_git(&["add", "."]).expect("add");
        runner.run_git(&["commit", "-m", "init"]).expect("commit");
        dir
    }

    #[test]
    fn detects_clean_repo_state() {
        let dir = setup_git_repo();
        let runner = GitRunner::new(dir.path());
        let state = runner.safety_state();

        assert!(state.is_git_repo);
        assert!(state.is_clean);
        assert!(state.head_commit.is_some());
        assert!(state.abort_reason.is_none());
    }

    #[test]
    fn detects_dirty_worktree() {
        let dir = setup_git_repo();
        // 制造脏工作树
        fs::write(dir.path().join("dirty.txt"), "uncommitted").expect("write");
        let runner = GitRunner::new(dir.path());
        let state = runner.safety_state();

        assert!(!state.is_clean);
        assert_eq!(state.abort_reason, Some(GitAbortReason::DirtyWorktree));
        assert!(!state.can_pull());
    }

    #[test]
    fn detects_non_git_directory() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let runner = GitRunner::new(dir.path());
        let state = runner.safety_state();

        assert!(!state.is_git_repo);
        assert_eq!(state.abort_reason, Some(GitAbortReason::NotARepo));
    }

    #[test]
    fn pull_refused_on_dirty_worktree() {
        let dir = setup_git_repo();
        fs::write(dir.path().join("dirty.txt"), "uncommitted").expect("write");
        let runner = GitRunner::new(dir.path());

        let result = runner.pull_ff_only();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), GitAbortReason::DirtyWorktree);
    }

    #[test]
    fn show_file_reads_from_head() {
        let dir = setup_git_repo();
        let runner = GitRunner::new(dir.path());

        let content = runner.show_file("team.toml").expect("show file");
        assert!(content.contains("[team]"));
    }

    #[test]
    fn list_files_returns_tracked_files() {
        let dir = setup_git_repo();
        let runner = GitRunner::new(dir.path());

        let files = runner.list_files("").expect("list files");
        assert!(files.contains(&"team.toml".to_string()));
    }
}
