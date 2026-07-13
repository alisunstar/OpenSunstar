//! 项目看板后端：代码统计（tokei）+ Git 分析 + 文件浏览

use serde::Serialize;
use std::path::Path;
use std::process::Command;
use tokei::{Config, Languages};

// ── tokei 代码行数统计 ─────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct CodeLineResult {
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub files: usize,
    pub languages: Vec<LanguageStat>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LanguageStat {
    pub language: String,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub files: usize,
}

fn common_excluded() -> Vec<&'static str> {
    vec![
        "node_modules",
        "target",
        ".git",
        "dist",
        "build",
        "out",
        ".next",
        ".nuxt",
        "vendor",
        "Pods",
        ".gradle",
        ".idea",
        ".vscode",
        ".cache",
        "__pycache__",
        ".venv",
        "venv",
        "env",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        "coverage",
        ".terraform",
        ".serverless",
        ".parcel-cache",
        ".DS_Store",
        "DerivedData",
    ]
}

pub fn count_code_lines(root: &Path) -> Result<CodeLineResult, String> {
    if !root.exists() {
        return Err("路径不存在".into());
    }
    if !root.is_dir() {
        return Err("路径不是文件夹".into());
    }

    let path_str = root.to_string_lossy().to_string();
    let paths = &[path_str.as_str()];
    let excluded = common_excluded();
    let config = Config::default();

    let mut languages = Languages::new();
    languages.get_statistics(paths, &excluded, &config);

    let mut total_code = 0usize;
    let mut total_comments = 0usize;
    let mut total_blanks = 0usize;
    let mut total_files = 0usize;
    let mut lang_stats: Vec<LanguageStat> = Vec::new();

    for (lang_type, language) in &languages {
        let code = language.code;
        let comments = language.comments;
        let blanks = language.blanks;
        let files = language.reports.len();
        if code == 0 && comments == 0 && blanks == 0 {
            continue;
        }
        total_code += code;
        total_comments += comments;
        total_blanks += blanks;
        total_files += files;
        lang_stats.push(LanguageStat {
            language: lang_type.name().to_string(),
            code_lines: code,
            comment_lines: comments,
            blank_lines: blanks,
            files,
        });
    }

    lang_stats.sort_by(|a, b| b.code_lines.cmp(&a.code_lines));

    Ok(CodeLineResult {
        total_lines: total_code + total_comments + total_blanks,
        code_lines: total_code,
        comment_lines: total_comments,
        blank_lines: total_blanks,
        files: total_files,
        languages: lang_stats,
    })
}

// ── package.json 版本读取 ───────────────────────

pub fn read_package_version(root: &Path) -> Result<Option<String>, String> {
    let pkg_path = root.join("package.json");
    if !pkg_path.is_file() {
        return Ok(None);
    }
    let content =
        std::fs::read_to_string(&pkg_path).map_err(|e| format!("读取 package.json 失败: {e}"))?;
    let val: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析失败: {e}"))?;
    let version = val.get("version").and_then(|v| v.as_str()).map(|v| {
        if v.starts_with('v') {
            v.to_string()
        } else {
            format!("v{v}")
        }
    });
    Ok(version)
}

// ── Git CLI 辅助 ───────────────────────────────

fn git_command_output(dir: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
}

// ── Git 仓库信息 ───────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct ProjectGitInfo {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub branches: Vec<String>,
    pub remote_url: Option<String>,
    pub remote_name: Option<String>,
    pub last_commit_hash: Option<String>,
    pub last_commit_message: Option<String>,
    pub last_commit_author: Option<String>,
    pub last_commit_date: Option<String>,
}

pub fn detect_git_info(root: &Path) -> Result<ProjectGitInfo, String> {
    let git_dir = root.join(".git");
    if !git_dir.is_dir() {
        return Ok(ProjectGitInfo {
            is_repo: false,
            branch: None,
            branches: vec![],
            remote_url: None,
            remote_name: None,
            last_commit_hash: None,
            last_commit_message: None,
            last_commit_author: None,
            last_commit_date: None,
        });
    }

    let branch = git_command_output(root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string());

    let branches_raw =
        git_command_output(root, &["branch", "--format=%(refname:short)"]).unwrap_or_default();
    let branches: Vec<String> = branches_raw
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let remote_url =
        git_command_output(root, &["remote", "get-url", "origin"]).map(|s| s.trim().to_string());

    let remote_name = remote_url.as_ref().map(|_| "origin".to_string());

    let last_hash =
        git_command_output(root, &["log", "-1", "--format=%H"]).map(|s| s.trim().to_string());

    let last_msg =
        git_command_output(root, &["log", "-1", "--format=%s"]).map(|s| s.trim().to_string());

    let last_author =
        git_command_output(root, &["log", "-1", "--format=%an"]).map(|s| s.trim().to_string());

    let last_date =
        git_command_output(root, &["log", "-1", "--format=%ci"]).map(|s| s.trim().to_string());

    Ok(ProjectGitInfo {
        is_repo: true,
        branch,
        branches,
        remote_url,
        remote_name,
        last_commit_hash: last_hash,
        last_commit_message: last_msg,
        last_commit_author: last_author,
        last_commit_date: last_date,
    })
}

// ── Git 活跃度 ────────────────────────────────

pub fn git_commit_count_last_n_days(root: &Path, days: u32) -> u32 {
    if !root.join(".git").is_dir() {
        return 0;
    }
    let since = format!("--since={}.days", days);
    git_command_output(root, &["log", "--oneline", &since])
        .map(|s| s.lines().count() as u32)
        .unwrap_or(0)
}

pub fn git_weekly_commit_counts(root: &Path) -> Vec<u32> {
    if !root.join(".git").is_dir() {
        return vec![0u32; 12];
    }
    let mut weeks = Vec::with_capacity(12);
    for w in (0..12).rev() {
        let since = format!("--since={}.weeks", w + 1);
        let until = format!("--until={}.weeks", w);
        let count = git_command_output(root, &["log", "--oneline", &since, &until])
            .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count() as u32)
            .unwrap_or(0);
        weeks.push(count);
    }
    weeks
}

// ── Git 贡献者 ────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct Contributor {
    pub name: String,
    pub email: String,
    pub commits: u32,
}

pub fn git_contributors(root: &Path) -> Vec<Contributor> {
    if !root.join(".git").is_dir() {
        return vec![];
    }
    let output = match git_command_output(root, &["shortlog", "-sne", "HEAD"]) {
        Some(s) => s,
        None => return vec![],
    };
    let mut list = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(tab_pos) = line.find('\t') else {
            continue;
        };
        let count_str = line[..tab_pos].trim();
        let info = &line[tab_pos + 1..];
        let commits: u32 = count_str.parse().unwrap_or(0);
        let (name, email) = if let Some(lt) = info.rfind('<') {
            let name_part = info[..lt].trim().to_string();
            let email_part = if let Some(gt) = info.rfind('>') {
                info[lt + 1..gt].trim().to_string()
            } else {
                info[lt + 1..].trim().to_string()
            };
            (name_part, email_part)
        } else {
            (info.to_string(), String::new())
        };
        list.push(Contributor {
            name,
            email,
            commits,
        });
    }
    list
}

// ── 文件夹浏览 ────────────────────────────────

#[cfg(target_os = "windows")]
pub fn reveal_path_in_folder(path: &str) -> Result<(), String> {
    Command::new("explorer")
        .arg(format!("/select,{}", path))
        .spawn()
        .map_err(|e| format!("打开文件夹失败: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn reveal_path_in_folder(path: &str) -> Result<(), String> {
    Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn()
        .map_err(|e| format!("打开文件夹失败: {e}"))?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn reveal_path_in_folder(path: &str) -> Result<(), String> {
    // Try to open the parent directory
    let parent = Path::new(path).parent().unwrap_or(Path::new(path));
    Command::new("xdg-open")
        .arg(parent)
        .spawn()
        .map_err(|e| format!("打开文件夹失败: {e}"))?;
    Ok(())
}
