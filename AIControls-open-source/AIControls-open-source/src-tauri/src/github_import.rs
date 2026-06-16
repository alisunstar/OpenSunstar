use std::fs;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::my_skills_library;
use crate::scan::extract_skill_declared_name;
use crate::skill_copy;
use tauri::AppHandle;

fn finalize_github_import_dest(
    app: &AppHandle,
    source_dir: &Path,
    dest_kind: &str,
    agent_id: &str,
    bucket_index: usize,
    project_root: Option<&str>,
    on_conflict_suffix: bool,
) -> Result<String, String> {
    if dest_kind == "myLibrary" {
        let entry = my_skills_library::add_skill_to_my_library(
            app,
            source_dir.to_string_lossy().into_owned(),
        )?;
        Ok(entry.path)
    } else {
        skill_copy::perform_copy_with_options(
            Some(app),
            source_dir.to_string_lossy().as_ref(),
            dest_kind,
            agent_id,
            bucket_index,
            project_root,
            on_conflict_suffix,
            None,
        )
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubSkillCandidate {
    pub id: String,
    pub path: String,
    pub title: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubSkillDetectionResult {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub base_path: Option<String>,
    pub skills: Vec<GithubSkillCandidate>,
}

#[derive(Deserialize)]
struct GithubTreeResponse {
    tree: Vec<GithubTreeEntry>,
    truncated: Option<bool>,
}

#[derive(Deserialize)]
struct GithubTreeEntry {
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
}

fn split_non_empty(input: &str) -> Vec<String> {
    input
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_github_repo_url(input: &str) -> Result<(String, String, String, Option<String>), String> {
    let t = input.trim().trim_end_matches('/');
    let no_scheme = if let Some(rest) = t.strip_prefix("https://") {
        rest
    } else if let Some(rest) = t.strip_prefix("http://") {
        rest
    } else {
        t
    };
    let host_prefix = "github.com/";
    let rest = if let Some(idx) = no_scheme.to_ascii_lowercase().find(host_prefix) {
        &no_scheme[idx + host_prefix.len()..]
    } else {
        return Err("仅支持 github.com 仓库链接".into());
    };

    let parts = split_non_empty(rest);
    if parts.len() < 2 {
        return Err("仓库地址格式无效，应类似 https://github.com/<owner>/<repo>".into());
    }
    let owner = parts[0].clone();
    let repo = parts[1].trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        return Err("仓库地址格式无效：owner/repo 不能为空".into());
    }

    let mut branch = "HEAD".to_string();
    let mut base_path = None;
    if parts.len() >= 4 && parts[2] == "tree" {
        branch = parts[3].clone();
        if parts.len() > 4 {
            let p = parts[4..].join("/");
            if !p.is_empty() {
                base_path = Some(p);
            }
        }
    }
    Ok((owner, repo, branch, base_path))
}

fn github_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("AIControls"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers
}

async fn fetch_repo_tree(
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Vec<GithubTreeEntry>, String> {
    let branch_enc = urlencoding::encode(branch);
    let url =
        format!("https://api.github.com/repos/{owner}/{repo}/git/trees/{branch_enc}?recursive=1");
    let cli = reqwest::Client::new();
    let resp = cli
        .get(url)
        .headers(github_headers())
        .send()
        .await
        .map_err(|e| format!("请求 GitHub 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "读取仓库目录失败（HTTP {}）",
            resp.status().as_u16()
        ));
    }
    let data = resp
        .json::<GithubTreeResponse>()
        .await
        .map_err(|e| format!("解析 GitHub 响应失败: {e}"))?;
    if data.truncated.unwrap_or(false) {
        return Err("仓库文件过多，GitHub 目录响应被截断，暂不支持该仓库".into());
    }
    Ok(data.tree)
}

fn path_matches_base(path: &str, base_path: Option<&str>) -> bool {
    let Some(base) = base_path else {
        return true;
    };
    let b = base.trim_matches('/');
    path == b || path.starts_with(&format!("{b}/"))
}

fn detect_skill_paths(
    tree: &[GithubTreeEntry],
    base_path: Option<&str>,
) -> Vec<GithubSkillCandidate> {
    let mut out: Vec<GithubSkillCandidate> = Vec::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    for ent in tree {
        if ent.entry_type != "blob" {
            continue;
        }
        if !path_matches_base(&ent.path, base_path) {
            continue;
        }
        let lower = ent.path.to_ascii_lowercase();
        if !(lower == "skill.md" || lower.ends_with("/skill.md")) {
            continue;
        }
        let dir = ent
            .path
            .rsplit_once('/')
            .map(|(p, _)| p.to_string())
            .unwrap_or_else(|| "".to_string());
        if !seen.insert(dir.clone()) {
            continue;
        }
        let title = if dir.is_empty() {
            "仓库根目录".to_string()
        } else {
            dir.rsplit('/').next().unwrap_or("Skill").to_string()
        };
        out.push(GithubSkillCandidate {
            id: if dir.is_empty() {
                ".".into()
            } else {
                dir.clone()
            },
            path: if dir.is_empty() { ".".into() } else { dir },
            title,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

async fn fetch_file_content(
    owner: &str,
    repo: &str,
    branch: &str,
    file_path: &str,
) -> Result<Vec<u8>, String> {
    let path_enc = urlencoding::encode(file_path);
    let branch_enc = urlencoding::encode(branch);
    let url =
        format!("https://api.github.com/repos/{owner}/{repo}/contents/{path_enc}?ref={branch_enc}");
    let cli = reqwest::Client::new();
    let resp = cli
        .get(url)
        .headers(github_headers())
        .send()
        .await
        .map_err(|e| format!("读取文件失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "读取文件失败（HTTP {}）：{}",
            resp.status().as_u16(),
            file_path
        ));
    }
    let v = resp
        .json::<Value>()
        .await
        .map_err(|e| format!("解析文件响应失败: {e}"))?;
    let encoding = v
        .get("encoding")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if encoding != "base64" {
        return Err(format!("不支持的文件编码：{file_path}"));
    }
    let content = v
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("文件内容为空：{file_path}"))?;
    let compact = content.replace('\n', "");
    STANDARD
        .decode(compact)
        .map_err(|e| format!("解码文件失败（{file_path}）: {e}"))
}

pub async fn detect_github_repo_skills(
    repo_url: &str,
) -> Result<GithubSkillDetectionResult, String> {
    let (owner, repo, branch, base_path) = parse_github_repo_url(repo_url)?;
    let tree = fetch_repo_tree(&owner, &repo, &branch).await?;
    let skills = detect_skill_paths(&tree, base_path.as_deref());
    Ok(GithubSkillDetectionResult {
        owner,
        repo,
        branch,
        base_path,
        skills,
    })
}

pub async fn import_github_skill_to_destination(
    app: &AppHandle,
    repo_url: &str,
    skill_path: &str,
    dest_kind: &str,
    agent_id: &str,
    bucket_index: usize,
    project_root: Option<&str>,
    on_conflict_suffix: bool,
) -> Result<String, String> {
    let (owner, repo, branch, _) = parse_github_repo_url(repo_url)?;
    let tree = fetch_repo_tree(&owner, &repo, &branch).await?;
    let chosen = if skill_path.trim() == "." {
        "".to_string()
    } else {
        skill_path.trim_matches('/').to_string()
    };
    if !tree.iter().any(|e| {
        if e.entry_type != "blob" {
            return false;
        }
        let lower = e.path.to_ascii_lowercase();
        if !(lower == "skill.md" || lower.ends_with("/skill.md")) {
            return false;
        }
        if chosen.is_empty() {
            !e.path.contains('/')
        } else {
            e.path == format!("{chosen}/SKILL.md")
                || e.path == format!("{chosen}/skill.md")
                || e.path.starts_with(&format!("{chosen}/")) && lower.ends_with("/skill.md")
        }
    }) {
        return Err("未找到所选 Skill 目录中的 SKILL.md".into());
    }

    let files: Vec<String> = tree
        .iter()
        .filter(|e| e.entry_type == "blob")
        .map(|e| e.path.clone())
        .filter(|p| {
            if chosen.is_empty() {
                !p.contains('/')
            } else {
                p.starts_with(&format!("{chosen}/"))
            }
        })
        .collect();
    if files.is_empty() {
        return Err("所选 Skill 路径下没有可导入文件".into());
    }

    let preferred_folder_name = if chosen.is_empty() {
        repo.clone()
    } else {
        chosen
            .rsplit('/')
            .next()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("skill")
            .to_string()
    };
    let tmp_root = std::env::temp_dir().join(format!("aicontrols-gh-import-{}", Uuid::new_v4()));
    let source_dir = tmp_root.join(&preferred_folder_name);
    fs::create_dir_all(&source_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;

    let mut skill_md_text: Option<String> = None;
    for file in &files {
        let rel = if chosen.is_empty() {
            file.clone()
        } else {
            file.trim_start_matches(&format!("{chosen}/")).to_string()
        };
        if rel.is_empty() {
            continue;
        }
        let bytes = fetch_file_content(&owner, &repo, &branch, file).await?;
        let out_path: PathBuf = source_dir.join(&rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }
        fs::write(&out_path, bytes).map_err(|e| format!("写入临时文件失败: {e}"))?;
        if rel.eq_ignore_ascii_case("SKILL.md") {
            if let Ok(text) = fs::read_to_string(&out_path) {
                skill_md_text = Some(text);
            }
        }
    }

    if let Some(name) = skill_md_text
        .as_deref()
        .and_then(extract_skill_declared_name)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        let renamed_dir = tmp_root.join(name);
        if renamed_dir != source_dir {
            fs::rename(&source_dir, &renamed_dir)
                .map_err(|e| format!("设置技能目录名失败: {e}"))?;
            let result = finalize_github_import_dest(
                app,
                &renamed_dir,
                dest_kind,
                agent_id,
                bucket_index,
                project_root,
                on_conflict_suffix,
            );
            let _ = fs::remove_dir_all(&tmp_root);
            return result;
        }
    }

    let result = finalize_github_import_dest(
        app,
        &source_dir,
        dest_kind,
        agent_id,
        bucket_index,
        project_root,
        on_conflict_suffix,
    );
    let _ = fs::remove_dir_all(&tmp_root);
    result
}
