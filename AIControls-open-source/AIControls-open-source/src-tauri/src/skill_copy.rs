//! Copy a skill **package** (folder with `SKILL.md` + siblings/subdirs, or loose `SKILL.md` + optional same-name folder)
//! into an allowed global or project agent `skills` parent directory only.

use std::fs;
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::scan::{extract_skill_declared_name, skills_container_dir};

fn home_dir_buf() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

/// Same order as `global_inventory` / `collect_project_skill_paths` (bucket_index).
pub fn global_skill_parent_dirs(agent_id: &str) -> Result<Vec<PathBuf>, String> {
    let home = home_dir_buf();
    Ok(match agent_id {
        "cursor" => vec![
            home.join(".cursor/skills-cursor"),
            home.join(".cursor/skills"),
        ],
        "claude" => vec![home.join(".claude/skills")],
        "codex" => vec![home.join(".codex/skills")],
        "hermes" => vec![home.join(".hermes/skills")],
        "openclaw" => vec![home.join(".openclaw/skills")],
        "trae" => vec![home.join(".trae/skills")],
        "qoder" => vec![home.join(".qoder/skills"), home.join(".qoderwork/skills")],
        "kiro" => vec![home.join(".kiro/skills")],
        "opencode" => vec![home.join(".config/opencode/skills")],
        _ => return Err(format!("未知 agent: {agent_id}")),
    })
}

/// Global `skills` parent dirs: built-in agents or a single `…/skills` under a user-added root.
pub fn resolve_global_skill_buckets(
    app: Option<&AppHandle>,
    agent_id: &str,
) -> Result<Vec<PathBuf>, String> {
    if let Some(app_handle) = app {
        if let Ok(Some(root)) = crate::storage::user_agent_root_for_id(app_handle, agent_id) {
            return Ok(vec![root.join("skills")]);
        }
    }
    global_skill_parent_dirs(agent_id)
}

/// e.g. `<project>/.cursor/skills` → marker `<project>/.cursor`
fn bucket_agent_marker_path(project_root: &Path, bucket_dest: &Path) -> Option<PathBuf> {
    let rel = bucket_dest.strip_prefix(project_root).ok()?;
    let mut it = rel.components();
    let first = it.next()?;
    Some(project_root.join(first.as_os_str()))
}

fn bucket_agent_marker_exists(project_root: &Path, bucket_dest: &Path) -> bool {
    bucket_agent_marker_path(project_root, bucket_dest)
        .map(|p| p.is_dir())
        .unwrap_or(false)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleProjectSkillBucket {
    pub agent_id: String,
    pub bucket_index: usize,
}

/// 仅返回「项目根下已存在对应 Agent 目录」时的复制桶（如仅有 `.cursor`/`.claude` 则不会列出 Trae/Qoder 等）。
pub fn list_visible_project_skill_buckets(
    project_root: &str,
) -> Result<Vec<VisibleProjectSkillBucket>, String> {
    let root = Path::new(project_root.trim())
        .canonicalize()
        .map_err(|e| format!("无法解析项目路径: {e}"))?;
    if !root.is_dir() {
        return Err("项目路径不是文件夹".into());
    }
    let mut out = Vec::new();
    for agent_id in [
        "cursor", "claude", "codex", "hermes", "openclaw", "trae", "qoder", "kiro",
        "opencode",
    ] {
        let buckets = project_skill_parent_dirs(&root, agent_id)?;
        for (idx, bucket_path) in buckets.iter().enumerate() {
            if bucket_agent_marker_exists(&root, bucket_path) {
                out.push(VisibleProjectSkillBucket {
                    agent_id: agent_id.to_string(),
                    bucket_index: idx,
                });
            }
        }
    }
    Ok(out)
}

/// Project-relative skill roots for `agent_id` (bucket_index matches this slice).
pub fn project_skill_parent_dirs(
    project_root: &Path,
    agent_id: &str,
) -> Result<Vec<PathBuf>, String> {
    let root = project_root
        .canonicalize()
        .map_err(|e| format!("无法解析项目根目录: {e}"))?;
    if !root.is_dir() {
        return Err("项目根目录不是文件夹".into());
    }
    let rels: &[&str] = match agent_id {
        "cursor" => &[".cursor/skills-cursor", ".cursor/skills"],
        "claude" => &[".claude/skills"],
        "codex" => &[".codex/skills"],
        "hermes" => &[".hermes/skills"],
        "openclaw" => &[".openclaw/skills"],
        "trae" => &[".trae/skills"],
        "qoder" => &[".qoder/skills", ".qoderwork/skills"],
        "kiro" => &[".kiro/skills"],
        "opencode" => &[".opencode/skills"],
        _ => return Err(format!("未知 agent: {agent_id}")),
    };
    Ok(rels.iter().map(|r| root.join(r)).collect())
}

fn sanitize_folder_segment(name: &str) -> String {
    let t = name.trim();
    if t.is_empty() {
        return String::new();
    }
    let mut s = String::with_capacity(t.len());
    for ch in t.chars() {
        match ch {
            '/' | '\\' | ':' | '\0' => s.push('-'),
            c if c.is_control() => s.push('-'),
            c => s.push(c),
        }
    }
    let s = s.trim_matches('.').trim().to_string();
    if s.is_empty() {
        "skill".into()
    } else {
        s
    }
}

fn pick_dest_dir(dest_parent: &Path, base: &str, use_suffix: bool) -> Result<PathBuf, String> {
    let base = sanitize_folder_segment(base);
    if base.is_empty() {
        return Err("无效的文件夹名".into());
    }
    if !use_suffix {
        let p = dest_parent.join(&base);
        if p.exists() {
            return Err(format!("目标已存在: {}", p.display()));
        }
        return Ok(p);
    }
    let mut n = 0_u32;
    loop {
        let name = if n == 0 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        let p = dest_parent.join(&name);
        if !p.exists() {
            return Ok(p);
        }
        n += 1;
        if n > 10_000 {
            return Err("无法分配不冲突的目标目录名".into());
        }
    }
}

fn prefixed_folder_base_name(base: &str, prefix: Option<&str>) -> String {
    let base = sanitize_folder_segment(base);
    let Some(prefix) = prefix.map(str::trim).filter(|s| !s.is_empty()) else {
        return base;
    };
    let prefix = sanitize_folder_segment(prefix);
    if prefix.is_empty() || base.starts_with(&prefix) {
        base
    } else {
        format!("{prefix}{base}")
    }
}

fn copy_tree_merge_contents(from: &Path, to: &Path) -> Result<(), String> {
    if !from.is_dir() {
        return Err(format!("源不是目录: {}", from.display()));
    }
    fs::create_dir_all(to).map_err(|e| format!("创建目录失败 {e}: {}", to.display()))?;
    for ent in fs::read_dir(from).map_err(|e| format!("读取目录失败 {e}: {}", from.display()))?
    {
        let ent = ent.map_err(|e| format!("读取目录项失败: {e}"))?;
        let fp = ent.path();
        let tp = to.join(ent.file_name());
        let ty = ent
            .file_type()
            .map_err(|e| format!("读取文件类型失败 {e}: {}", fp.display()))?;
        if ty.is_dir() {
            copy_tree_merge_contents(&fp, &tp)?;
        } else if ty.is_file() {
            if let Some(parent) = tp.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("{e}"))?;
            }
            fs::copy(&fp, &tp)
                .map_err(|e| format!("复制文件失败 {e}: {} → {}", fp.display(), tp.display()))?;
        }
    }
    Ok(())
}

fn find_primary_skill_doc(dir: &Path) -> Option<PathBuf> {
    for name in [
        "SKILL.md",
        "skill.md",
        "CLAUDE.md",
        "claude.md",
        "AGENTS.md",
        "agents.md",
        "HERMES.md",
        "hermes.md",
        "OPENCLAW.md",
        "openclaw.md",
    ] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn slugify_skill_name(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "skill".into()
    } else {
        out
    }
}

fn ensure_skill_doc_name_prefix(
    skill_dir: &Path,
    folder_prefix: Option<&str>,
) -> Result<(), String> {
    let Some(prefix) = folder_prefix.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let Some(doc) = find_primary_skill_doc(skill_dir) else {
        return Ok(());
    };
    let text = fs::read_to_string(&doc).unwrap_or_default();
    let desired_name = skill_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(slugify_skill_name)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{prefix}skill"));

    let mut lines: Vec<String> = text.lines().map(ToString::to_string).collect();
    if lines.first().is_some_and(|line| line.trim() == "---") {
        if let Some(end_idx) = lines
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(idx, line)| (line.trim() == "---").then_some(idx))
        {
            for line in lines.iter_mut().take(end_idx).skip(1) {
                if line.trim_start().starts_with("name:") {
                    *line = format!("name: {desired_name}");
                    let mut next = lines.join("\n");
                    next.push('\n');
                    fs::write(&doc, next).map_err(|e| format!("更新 Skill name 失败: {e}"))?;
                    return Ok(());
                }
            }
            lines.insert(1, format!("name: {desired_name}"));
            let mut next = lines.join("\n");
            next.push('\n');
            fs::write(&doc, next).map_err(|e| format!("更新 Skill name 失败: {e}"))?;
            return Ok(());
        }
    }

    let next = format!("---\nname: {desired_name}\n---\n\n{text}");
    fs::write(&doc, next).map_err(|e| format!("更新 Skill name 失败: {e}"))?;
    Ok(())
}

enum SkillCopySource {
    /// Recursively copy everything under this directory into a new folder under dest parent.
    Directory {
        root: PathBuf,
        folder_base_name: String,
    },
    /// `SKILL.md` (or variant) sits directly under a `skills` container; materialize `dest_parent/<name>/`.
    LooseMarkdown {
        skill_md: PathBuf,
        dest_folder_name: String,
    },
}

fn resolve_skill_copy_source(path: &Path) -> Result<SkillCopySource, String> {
    let path = if path.exists() {
        path.canonicalize()
            .map_err(|e| format!("无法解析路径: {e}"))?
    } else {
        return Err("路径不存在".into());
    };

    if path.is_dir() {
        let folder_base_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "skill".into());
        return Ok(SkillCopySource::Directory {
            root: path,
            folder_base_name,
        });
    }

    let parent = path
        .parent()
        .ok_or_else(|| "无法解析技能文件父目录".to_string())?
        .to_path_buf();
    let parent_name = parent
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let content = fs::read_to_string(&path).unwrap_or_default();
    let declared = extract_skill_declared_name(&content);
    let in_container = skills_container_dir(&parent_name);

    if parent.is_dir() && !in_container {
        return Ok(SkillCopySource::Directory {
            root: parent,
            folder_base_name: parent_name,
        });
    }

    if in_container {
        let raw_name = declared
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| path.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("skill");
        let dest_folder_name = sanitize_folder_segment(raw_name);
        return Ok(SkillCopySource::LooseMarkdown {
            skill_md: path,
            dest_folder_name,
        });
    }

    Err("无法解析技能包（仅支持技能目录或位于 skills 根下的 SKILL 文件）".into())
}

fn path_starts_with_canonical(child: &Path, prefix: &Path) -> bool {
    let Ok(c) = child.canonicalize() else {
        return false;
    };
    let Ok(p) = prefix.canonicalize() else {
        return false;
    };
    c.starts_with(&p)
}

fn resolve_dest_parent(
    kind: &str,
    agent_id: &str,
    bucket_index: usize,
    project_root: Option<&Path>,
    app: Option<&AppHandle>,
) -> Result<PathBuf, String> {
    let dest_parent = match kind {
        "global" => {
            let buckets = resolve_global_skill_buckets(app, agent_id)?;
            buckets
                .get(bucket_index)
                .cloned()
                .ok_or_else(|| "bucket_index 越界".to_string())?
        }
        "project" => {
            let root = project_root.ok_or_else(|| "project 模式需要 project_root".to_string())?;
            let buckets = project_skill_parent_dirs(root, agent_id)?;
            buckets
                .get(bucket_index)
                .cloned()
                .ok_or_else(|| "bucket_index 越界".to_string())?
        }
        _ => return Err(format!("未知 kind: {kind}")),
    };
    Ok(dest_parent)
}

/// `on_conflict_suffix`: true → `name`, `name-2`, … ; false → error if exists.
pub fn perform_copy_with_options(
    app: Option<&AppHandle>,
    source_path: &str,
    kind: &str,
    agent_id: &str,
    bucket_index: usize,
    project_root: Option<&str>,
    on_conflict_suffix: bool,
    folder_name_prefix: Option<&str>,
) -> Result<String, String> {
    let root_path = project_root.map(Path::new);
    let dest_parent_uncanon = resolve_dest_parent(kind, agent_id, bucket_index, root_path, app)?;
    let guard = if kind == "project" { root_path } else { None };
    let final_dir = copy_skill_package_into_parent_with_options(
        &dest_parent_uncanon,
        source_path,
        on_conflict_suffix,
        guard,
        folder_name_prefix,
    )?;
    Ok(final_dir.to_string_lossy().into_owned())
}

/// Copy a skill package into an existing canonical destination parent (e.g. app-managed「我的技能」目录)。
pub fn copy_skill_package_into_parent(
    dest_parent_uncanon: &Path,
    source_path: &str,
    on_conflict_suffix: bool,
    project_root_for_guard: Option<&Path>,
) -> Result<PathBuf, String> {
    copy_skill_package_into_parent_with_options(
        dest_parent_uncanon,
        source_path,
        on_conflict_suffix,
        project_root_for_guard,
        None,
    )
}

pub fn copy_skill_package_into_parent_with_options(
    dest_parent_uncanon: &Path,
    source_path: &str,
    on_conflict_suffix: bool,
    project_root_for_guard: Option<&Path>,
    folder_name_prefix: Option<&str>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(dest_parent_uncanon).map_err(|e| format!("无法创建目标目录: {e}"))?;

    let dest_parent = dest_parent_uncanon
        .canonicalize()
        .map_err(|e| format!("无法解析目标目录: {e}"))?;

    if let Some(root_raw) = project_root_for_guard {
        let root = root_raw
            .canonicalize()
            .map_err(|e| format!("无法解析项目根目录: {e}"))?;
        if !dest_parent.starts_with(&root) {
            return Err("目标 skills 目录必须位于所选项目根之下".into());
        }
    }

    finish_skill_copy_under_dest(
        &dest_parent,
        source_path.trim(),
        on_conflict_suffix,
        folder_name_prefix,
    )
}

fn finish_skill_copy_under_dest(
    dest_parent: &Path,
    source_path: &str,
    on_conflict_suffix: bool,
    folder_name_prefix: Option<&str>,
) -> Result<PathBuf, String> {
    let source = Path::new(source_path.trim());
    let source_kind = resolve_skill_copy_source(source)?;

    match source_kind {
        SkillCopySource::Directory {
            root,
            folder_base_name,
        } => {
            if path_starts_with_canonical(dest_parent, &root) {
                return Err("不能复制到该技能包自身目录内部".into());
            }
            let folder_base_name = prefixed_folder_base_name(&folder_base_name, folder_name_prefix);
            let dest_dir = pick_dest_dir(dest_parent, &folder_base_name, on_conflict_suffix)?;
            copy_tree_merge_contents(&root, &dest_dir)?;
            ensure_skill_doc_name_prefix(&dest_dir, folder_name_prefix)?;
            Ok(dest_dir)
        }
        SkillCopySource::LooseMarkdown {
            skill_md,
            dest_folder_name,
        } => {
            let parent = skill_md
                .parent()
                .ok_or_else(|| "无效路径".to_string())?
                .to_path_buf();
            if path_starts_with_canonical(dest_parent, &parent) {
                return Err("不能复制到源文件所在目录内部".into());
            }
            let dest_folder_name = prefixed_folder_base_name(&dest_folder_name, folder_name_prefix);
            let dest_dir = pick_dest_dir(dest_parent, &dest_folder_name, on_conflict_suffix)?;
            fs::create_dir_all(&dest_dir).map_err(|e| format!("{e}"))?;
            let fname = skill_md
                .file_name()
                .ok_or_else(|| "无效文件名".to_string())?;
            fs::copy(&skill_md, dest_dir.join(fname))
                .map_err(|e| format!("复制 SKILL 文件失败: {e}"))?;

            let sibling = parent.join(&dest_folder_name);
            if sibling.is_dir() {
                let can_dest = dest_dir.canonicalize().map_err(|e| format!("{e}"))?;
                let can_sib = sibling.canonicalize().map_err(|e| format!("{e}"))?;
                if can_sib != can_dest {
                    copy_tree_merge_contents(&sibling, &dest_dir)?;
                }
            }

            ensure_skill_doc_name_prefix(&dest_dir, folder_name_prefix)?;
            Ok(dest_dir)
        }
    }
}

/// 仅删除**技能包文件夹**（`remove_dir_all`）。`skills` 根下的散装 `SKILL.md` 只能复制，不提供整夹删除。
pub fn perform_delete_skill(source_path: &str) -> Result<(), String> {
    let trimmed = source_path.trim();
    if trimmed.is_empty() {
        return Err("路径为空".into());
    }
    let sk = resolve_skill_copy_source(Path::new(trimmed))?;
    match sk {
        SkillCopySource::Directory { root, .. } => {
            fs::remove_dir_all(&root)
                .map_err(|e| format!("删除技能目录失败 ({e}): {}", root.to_string_lossy()))?;
        }
        SkillCopySource::LooseMarkdown { .. } => {
            return Err(
                "当前技能为 skills 目录下的散装 SKILL.md，未形成技能文件夹；请整理为文件夹技能包后再删除，或在访达中手动删除该文件。"
                    .into(),
            );
        }
    }
    Ok(())
}
