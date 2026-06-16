//! User-owned skill packages under app data (`my-skills-packages/`), with manifest `my-skills-library.json`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use uuid::Uuid;

use crate::scan::{extract_skill_declared_name, preview_from_markdown};
use crate::skill_copy;
use crate::storage::app_local_dir;

const LIBRARY_FILE: &str = "my-skills-library.json";
const LIBRARY_TMP: &str = "my-skills-library.json.tmp";
const LIBRARY_BAK: &str = "my-skills-library.json.bak";
const PACKAGES_SUBDIR: &str = "my-skills-packages";
const CURRENT_VERSION: u32 = 1;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MySkillItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub path: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MySkillsLibraryFile {
    pub version: u32,
    pub items: Vec<MySkillItem>,
}

fn library_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join(LIBRARY_FILE))
}

pub fn my_skills_packages_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join(PACKAGES_SUBDIR))
}

fn default_library() -> MySkillsLibraryFile {
    MySkillsLibraryFile {
        version: CURRENT_VERSION,
        items: Vec::new(),
    }
}

fn write_library_atomic(app: &AppHandle, lib: &MySkillsLibraryFile) -> Result<(), String> {
    let path = library_path(app)?;
    let parent = path
        .parent()
        .ok_or_else(|| "无法解析「我的技能」配置目录".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let tmp = parent.join(LIBRARY_TMP);
    let bak = parent.join(LIBRARY_BAK);
    if path.exists() {
        let _ = fs::copy(&path, bak);
    }
    let json = serde_json::to_string_pretty(lib)
        .map_err(|e| format!("序列化「我的技能」清单失败：{e}"))?;
    fs::write(&tmp, json).map_err(|e| format!("写入临时文件失败：{e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("保存「我的技能」清单失败：{e}"))?;
    Ok(())
}

fn validate_and_normalize(mut lib: MySkillsLibraryFile) -> Result<MySkillsLibraryFile, String> {
    if lib.version == 0 {
        lib.version = CURRENT_VERSION;
    }
    if lib.version > CURRENT_VERSION {
        return Err(format!(
            "「我的技能」清单版本过高（{}），请升级应用。",
            lib.version
        ));
    }
    lib.version = CURRENT_VERSION;

    for item in &mut lib.items {
        item.id = item.id.trim().to_string();
        item.title = item.title.trim().to_string();
        item.description = item.description.trim().to_string();
        item.path = item.path.trim().to_string();
        item.source_path = item
            .source_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        item.source_kind = item
            .source_kind
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        if item.id.is_empty() {
            return Err("「我的技能」条目 id 为空".into());
        }
        if item.title.is_empty() {
            return Err(format!("条目 {} 缺少标题", item.id));
        }
        if item.path.is_empty() {
            return Err(format!("条目 {} 缺少路径", item.id));
        }
        if item.updated_at <= 0 {
            item.updated_at = item.created_at;
        }
    }
    Ok(lib)
}

fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    for name in ["SKILL.md", "skill.md", "CLAUDE.md", "claude.md"] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn read_skill_folder_metadata(skill_root: &Path) -> Result<(String, String), String> {
    let md_path = find_skill_md(skill_root)
        .ok_or_else(|| format!("技能文件夹内未找到 SKILL.md：{}", skill_root.display()))?;
    let content = fs::read_to_string(&md_path).unwrap_or_default();
    let title = extract_skill_declared_name(&content).unwrap_or_else(|| {
        skill_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "skill".into())
    });
    let mut desc = preview_from_markdown(&content, 160);
    if desc.is_empty() {
        desc = md_path.to_string_lossy().into_owned();
    }
    Ok((title, desc))
}

pub fn load_my_skills_library(app: &AppHandle) -> Result<MySkillsLibraryFile, String> {
    let path = library_path(app)?;
    if !path.exists() {
        return Ok(default_library());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("读取「我的技能」清单失败：{e}"))?;
    let lib: MySkillsLibraryFile =
        serde_json::from_str(&text).map_err(|e| format!("解析「我的技能」清单失败：{e}"))?;
    let mut normalized = validate_and_normalize(lib)?;

    let before = normalized.items.len();
    normalized.items.retain(|it| Path::new(&it.path).is_dir());
    if normalized.items.len() != before {
        write_library_atomic(app, &normalized)?;
    }

    Ok(normalized)
}

pub fn add_skill_to_my_library(
    app: &AppHandle,
    source_path: String,
) -> Result<MySkillItem, String> {
    let trimmed = source_path.trim().to_string();
    if trimmed.is_empty() {
        return Err("路径为空".into());
    }

    let mut lib = load_my_skills_library(app)?;
    if let Some(existing) = lib.items.iter().find(|item| {
        item.path == trimmed || item.source_path.as_deref() == Some(trimmed.as_str())
    }) {
        return Ok(existing.clone());
    }

    let pkgs = my_skills_packages_dir(app)?;
    fs::create_dir_all(&pkgs).map_err(|e| format!("创建「我的技能」目录失败：{e}"))?;

    let final_dir = skill_copy::copy_skill_package_into_parent(&pkgs, &trimmed, true, None)?;

    let (title, description) = read_skill_folder_metadata(&final_dir)?;
    let ts = now_ms();
    let entry = MySkillItem {
        id: Uuid::new_v4().to_string(),
        title,
        description,
        path: final_dir.to_string_lossy().into_owned(),
        source_path: Some(trimmed),
        source_kind: None,
        created_at: ts,
        updated_at: ts,
    };

    lib.items.push(entry.clone());
    let normalized = validate_and_normalize(lib)?;
    write_library_atomic(app, &normalized)?;

    Ok(entry)
}

fn slugify_skill_segment(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.trim().to_lowercase().chars() {
        let next = if ch.is_ascii_alphanumeric() {
            Some(ch)
        } else if ch.is_whitespace() || matches!(ch, '-' | '_' | '/' | '\\' | ':' | '.') {
            Some('-')
        } else {
            None
        };
        if let Some(c) = next {
            if c == '-' {
                if !last_dash && !out.is_empty() {
                    out.push(c);
                    last_dash = true;
                }
            } else {
                out.push(c);
                last_dash = false;
            }
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "prompt-skill".into()
    } else {
        trimmed
    }
}

fn pick_generated_skill_dir(parent: &Path, base: &str) -> PathBuf {
    let mut n = 0_u32;
    loop {
        let name = if n == 0 {
            base.to_string()
        } else {
            format!("{base}-{n}")
        };
        let path = parent.join(name);
        if !path.exists() {
            return path;
        }
        n += 1;
    }
}

fn yaml_quote(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', " ")
}

fn skill_markdown_from_prompt(
    skill_name: &str,
    title: &str,
    prompt: &str,
    output_type: &str,
    output_example: &str,
) -> String {
    let description = format!("Prompt command generated from AIControls: {title}");
    let mut md = format!(
        "---\nname: {skill_name}\ndescription: \"{}\"\n---\n\n# {title}\n\nUse this skill when the user asks for the `{skill_name}` workflow or wants to reuse this AIControls prompt.\n\n## Prompt\n\n````text\n{}\n````\n",
        yaml_quote(&description),
        prompt.trim()
    );
    md.push_str(&format!("\n## Output Type\n\n{output_type}\n"));
    if !output_example.trim().is_empty() {
        md.push_str(&format!(
            "\n## Reference Output Example\n\n````text\n{}\n````\n",
            output_example.trim()
        ));
    }
    md
}

pub fn convert_prompt_to_my_skill(
    app: &AppHandle,
    title: String,
    prompt: String,
    output_type: String,
    output_example: String,
    command_name: Option<String>,
) -> Result<MySkillItem, String> {
    let title = title.trim().to_string();
    let prompt = prompt.trim().to_string();
    let output_type = output_type.trim().to_string();
    let output_example = output_example.trim().to_string();
    if title.is_empty() {
        return Err("标题不能为空".into());
    }
    if prompt.is_empty() {
        return Err("Prompt 不能为空，无法转为 Skill".into());
    }

    let base_slug = command_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(slugify_skill_segment)
        .unwrap_or_else(|| slugify_skill_segment(&title));
    let skill_name = base_slug
        .strip_prefix("cps-")
        .or_else(|| base_slug.strip_prefix("cs-"))
        .unwrap_or(&base_slug)
        .to_string();

    let pkgs = my_skills_packages_dir(app)?;
    fs::create_dir_all(&pkgs).map_err(|e| format!("创建「我的技能」目录失败：{e}"))?;
    let final_dir = pick_generated_skill_dir(&pkgs, &skill_name);
    fs::create_dir_all(&final_dir).map_err(|e| format!("创建 Skill 目录失败：{e}"))?;

    let content =
        skill_markdown_from_prompt(&skill_name, &title, &prompt, &output_type, &output_example);
    fs::write(final_dir.join("SKILL.md"), content)
        .map_err(|e| format!("写入 SKILL.md 失败：{e}"))?;

    let (title, description) = read_skill_folder_metadata(&final_dir)?;
    let ts = now_ms();
    let entry = MySkillItem {
        id: Uuid::new_v4().to_string(),
        title,
        description,
        path: final_dir.to_string_lossy().into_owned(),
        source_path: None,
        source_kind: Some("prompt".into()),
        created_at: ts,
        updated_at: ts,
    };

    let mut lib = load_my_skills_library(app)?;
    lib.items.push(entry.clone());
    let normalized = validate_and_normalize(lib)?;
    write_library_atomic(app, &normalized)?;

    Ok(entry)
}

pub fn remove_my_skill(app: &AppHandle, id: String) -> Result<(), String> {
    let id = id.trim().to_string();
    if id.is_empty() {
        return Err("id 为空".into());
    }
    let mut lib = load_my_skills_library(app)?;
    let pos = lib
        .items
        .iter()
        .position(|x| x.id == id)
        .ok_or_else(|| "未找到该技能条目".to_string())?;
    let item = lib.items.remove(pos);
    let p = Path::new(&item.path);
    if p.exists() && p.is_dir() {
        fs::remove_dir_all(p).map_err(|e| format!("删除技能目录失败：{e}"))?;
    }
    let normalized = validate_and_normalize(lib)?;
    write_library_atomic(app, &normalized)?;
    Ok(())
}
