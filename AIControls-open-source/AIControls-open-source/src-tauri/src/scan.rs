//! Scan installed agent apps and read global (non-project) skills, MCP, rules.
//!
//! **Rules** follow common 2026 layouts: project rules live under agent-specific dirs
//! (e.g. `.cursor/rules/*.mdc`, `CLAUDE.md`, `AGENTS.md`, `.trae/rules`, `.qoder/rules`, `.kiro/rules`)
//! plus legacy files (`.cursorrules`, `trae.config.jsonc`, JSON in `.qoder`/`.kiro`).
//! Global user rules: `~/.cursor/rules`, `~/.claude/rules`, etc., plus legacy JSON(C) where applicable.
//!
//! **Project Skills**: only `SKILL.md` under each agent’s conventional `skills` directory (not every
//! `SKILL.md` in the repo — excludes ad-hoc trees like `.agent/skills`).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct AgentScanResult {
    pub id: String,
    pub label: String,
    #[serde(rename = "rootPath")]
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub path: String,
    pub active: bool,
    /// AI 分类：`dev` / `office` / `creative` / `data` / `network` / `ops` / `collab`
    #[serde(default)]
    pub scenario: Option<String>,
    /// AI 生成的中文缩略介绍（<=100字）
    #[serde(default)]
    pub brief_zh: Option<String>,
    /// AI 生成的英文缩略介绍（<=100 chars）
    #[serde(default)]
    pub brief_en: Option<String>,
    /// Skill 包目录内除主 `SKILL.md` 外的其他文件名（仅当 `path` 为技能文件夹时填充）
    #[serde(default)]
    pub skill_extra_files: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInventory {
    pub skills: Vec<AssetEntry>,
    pub mcp: Vec<AssetEntry>,
    pub rules: Vec<AssetEntry>,
}

pub fn attach_scenarios(inv: &mut AgentInventory, map: &HashMap<String, String>) {
    for e in inv
        .skills
        .iter_mut()
        .chain(inv.mcp.iter_mut())
        .chain(inv.rules.iter_mut())
    {
        if let Some(s) = map.get(&e.id) {
            e.scenario = Some(s.clone());
        }
    }
}

pub fn attach_briefs(inv: &mut AgentInventory, locale: &str, map: &HashMap<String, String>) {
    for e in inv
        .skills
        .iter_mut()
        .chain(inv.mcp.iter_mut())
        .chain(inv.rules.iter_mut())
    {
        if let Some(s) = map.get(&e.id) {
            if locale == "zh" {
                e.brief_zh = Some(s.clone());
            } else {
                e.brief_en = Some(s.clone());
            }
        }
    }
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

fn stable_id(prefix: &str, path: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    prefix.hash(&mut h);
    path.to_string_lossy().hash(&mut h);
    format!("{}-{:x}", prefix, h.finish())
}

/// Stable id for a user-added agent root (persisted in `user_agents.json`).
pub fn user_agent_stable_id(canonical_root: &Path) -> String {
    stable_id("useragent", canonical_root)
}

/// macOS: `/Applications/Foo.app`
#[cfg(target_os = "macos")]
fn app_bundle_path(name: &str) -> Option<PathBuf> {
    let p = Path::new("/Applications").join(format!("{name}.app"));
    p.is_dir().then_some(p)
}

#[cfg(not(target_os = "macos"))]
fn app_bundle_path(_name: &str) -> Option<PathBuf> {
    None
}

fn first_existing_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.into_iter().find(|p| p.exists())
}

fn push_detected_agent(out: &mut Vec<AgentScanResult>, id: &str, label: &str, root: PathBuf) {
    out.push(AgentScanResult {
        id: id.into(),
        label: label.into(),
        root_path: root.to_string_lossy().into_owned(),
    });
}

pub fn detect_agents() -> Vec<AgentScanResult> {
    let home = home_dir();
    let mut out = Vec::new();

    if let Some(root) = detect_cursor_path(&home) {
        push_detected_agent(&mut out, "cursor", "Cursor", root);
    }
    if let Some(root) = detect_claude_path(&home) {
        push_detected_agent(&mut out, "claude", "Claude Code", root);
    }
    if let Some(root) = detect_codex_path(&home) {
        push_detected_agent(&mut out, "codex", "Codex", root);
    }
    if let Some(root) = detect_hermes_path(&home) {
        push_detected_agent(&mut out, "hermes", "Hermes", root);
    }
    if let Some(root) = detect_openclaw_path(&home) {
        push_detected_agent(&mut out, "openclaw", "OpenClaw", root);
    }
    if let Some(root) = detect_trae_path(&home) {
        push_detected_agent(&mut out, "trae", "Trae", root);
    }
    if let Some(root) = detect_qoder_path(&home) {
        push_detected_agent(&mut out, "qoder", "Qoder", root);
    }
    if let Some(root) = detect_kiro_path(&home) {
        push_detected_agent(&mut out, "kiro", "Kiro", root);
    }
    if let Some(root) = detect_opencode_path(&home) {
        push_detected_agent(&mut out, "opencode", "opencode", root);
    }

    out
}

fn detect_cursor_path(home: &Path) -> Option<PathBuf> {
    first_existing_path(vec![home.join(".cursor")]).or_else(|| app_bundle_path("Cursor"))
}

fn detect_claude_path(home: &Path) -> Option<PathBuf> {
    first_existing_path(vec![home.join(".claude")])
}

fn detect_codex_path(home: &Path) -> Option<PathBuf> {
    first_existing_path(vec![home.join(".codex")])
}

fn detect_hermes_path(home: &Path) -> Option<PathBuf> {
    first_existing_path(vec![home.join(".hermes")]).or_else(|| app_bundle_path("Hermes"))
}

fn detect_openclaw_path(home: &Path) -> Option<PathBuf> {
    first_existing_path(vec![home.join(".openclaw")]).or_else(|| app_bundle_path("OpenClaw"))
}

fn detect_trae_path(home: &Path) -> Option<PathBuf> {
    first_existing_path(vec![home.join(".trae")])
        .or_else(|| app_bundle_path("Trae"))
        .or_else(|| app_bundle_path("Trae CN"))
}

fn detect_qoder_path(home: &Path) -> Option<PathBuf> {
    first_existing_path(vec![home.join(".qoder"), home.join(".qoderwork")])
        .or_else(|| app_bundle_path("Qoder"))
}

fn detect_kiro_path(home: &Path) -> Option<PathBuf> {
    first_existing_path(vec![home.join(".kiro")]).or_else(|| app_bundle_path("Kiro"))
}

fn detect_opencode_path(home: &Path) -> Option<PathBuf> {
    let xdg = home.join(".config").join("opencode");
    if xdg.is_dir() { Some(xdg) } else { None }
}

fn should_skip_scan_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | ".git"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | "vendor"
            | ".cache"
            | "coverage"
            | ".svn"
            | ".hg"
    )
}

fn walk_skill_files(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if should_skip_scan_dir(name) {
                    continue;
                }
            }
            walk_skill_files(&p, depth + 1, max_depth, out);
        } else if p.file_name().and_then(|n| n.to_str()) == Some("SKILL.md") {
            out.push(p);
        }
    }
}

/// Project root: `SKILL.md` only under agent skill dirs (matches global inventory layout).
fn collect_project_skill_paths(root: &Path, out: &mut Vec<PathBuf>) {
    for rel in [
        ".cursor/skills-cursor",
        ".cursor/skills",
        ".claude/skills",
        ".codex/skills",
        ".hermes/skills",
        ".openclaw/skills",
        ".trae/skills",
        ".qoder/skills",
        ".qoderwork/skills",
        ".kiro/skills",
        ".opencode/skills",
    ] {
        let p = root.join(rel);
        if p.is_dir() {
            walk_skill_files(&p, 0, 12, out);
        }
    }
}

/// `.mdc` / `.md` rule snippets under an agent `rules` directory (not arbitrary repo Markdown).
fn walk_rules_mdc_md(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if should_skip_scan_dir(name) {
                    continue;
                }
            }
            walk_rules_mdc_md(&p, depth + 1, max_depth, out);
        } else if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name == "SKILL.md" {
                continue;
            }
            if name.ends_with(".mdc") || name.ends_with(".md") {
                out.push(p);
            }
        }
    }
}

fn push_if_file(path: PathBuf, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path);
    }
}

/// Basenames that are `*.json` on disk but editor/runtime config — not agent rule bundles.
const RULE_JSON_SHALLOW_EXCLUDE: &[&str] = &["mcp.json", "argv.json"];

/// Shallow `*.json` / `*.jsonc` in a directory (legacy agent rule bundles), excluding MCP and known non-rule JSON.
fn push_json_jsonc_in_dir_shallow(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str());
        if !matches!(ext, Some("json") | Some("jsonc")) {
            continue;
        }
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if RULE_JSON_SHALLOW_EXCLUDE.contains(&name) {
                continue;
            }
        }
        out.push(p);
    }
}

/// Project root: only recognized agent rule locations (not every `*.md` in the tree).
fn collect_project_rule_paths(root: &Path, out: &mut Vec<PathBuf>) {
    // Cursor — directory MDC (and MD); legacy `.cursorrules`
    let cursor_rules = root.join(".cursor/rules");
    if cursor_rules.is_dir() {
        walk_rules_mdc_md(&cursor_rules, 0, 8, out);
    }
    push_if_file(root.join(".cursorrules"), out);

    // Claude Code — root CLAUDE.md + `.claude/rules`
    push_if_file(root.join("CLAUDE.md"), out);
    let claude_rules = root.join(".claude/rules");
    if claude_rules.is_dir() {
        walk_rules_mdc_md(&claude_rules, 0, 8, out);
    }

    // Codex — root AGENTS.md + `.codex/rules`
    push_if_file(root.join("AGENTS.md"), out);
    let codex_rules = root.join(".codex/rules");
    if codex_rules.is_dir() {
        walk_rules_mdc_md(&codex_rules, 0, 8, out);
    }

    // Hermes — root HERMES.md + `.hermes/rules`
    push_if_file(root.join("HERMES.md"), out);
    let hermes_rules = root.join(".hermes/rules");
    if hermes_rules.is_dir() {
        walk_rules_mdc_md(&hermes_rules, 0, 8, out);
    }

    // OpenClaw — root OPENCLAW.md + `.openclaw/rules`
    push_if_file(root.join("OPENCLAW.md"), out);
    let openclaw_rules = root.join(".openclaw/rules");
    if openclaw_rules.is_dir() {
        walk_rules_mdc_md(&openclaw_rules, 0, 8, out);
    }

    // Trae — `.trae/rules` + root `trae.config.{jsonc,json}`
    let trae_rules = root.join(".trae/rules");
    if trae_rules.is_dir() {
        walk_rules_mdc_md(&trae_rules, 0, 8, out);
    }
    push_if_file(root.join("trae.config.jsonc"), out);
    push_if_file(root.join("trae.config.json"), out);

    // Qoder — `.qoder/rules`, `.qoderwork/rules`, legacy JSON next to config
    for rel in [".qoder/rules", ".qoderwork/rules"] {
        let p = root.join(rel);
        if p.is_dir() {
            walk_rules_mdc_md(&p, 0, 8, out);
        }
    }
    for q in [root.join(".qoder"), root.join(".qoderwork")] {
        if q.is_dir() {
            push_json_jsonc_in_dir_shallow(&q, out);
        }
    }

    // Kiro — `.kiro/rules` + legacy `.kiro/*.{json,jsonc}` (not nested MCP dirs)
    let kiro_rules = root.join(".kiro/rules");
    if kiro_rules.is_dir() {
        walk_rules_mdc_md(&kiro_rules, 0, 8, out);
    }
    let kiro_home = root.join(".kiro");
    if kiro_home.is_dir() {
        push_json_jsonc_in_dir_shallow(&kiro_home, out);
    }

    // opencode — `.opencode/rules` + root opencode.json / opencode.jsonc
    let opencode_rules = root.join(".opencode/rules");
    if opencode_rules.is_dir() {
        walk_rules_mdc_md(&opencode_rules, 0, 8, out);
    }
    push_if_file(root.join("opencode.json"), out);
    push_if_file(root.join("opencode.jsonc"), out);
}

fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    paths.sort();
    paths.dedup();
}

fn collect_skill_files(root: &Path, out: &mut Vec<PathBuf>) {
    if root.is_dir() {
        walk_skill_files(root, 0, 12, out);
    }
}

fn collect_rule_files(root: &Path, out: &mut Vec<PathBuf>) {
    if root.is_dir() {
        walk_rules_mdc_md(root, 0, 12, out);
    }
}

fn read_preview(path: &Path, max: usize) -> String {
    fs::read_to_string(path)
        .map(|s| preview_from_markdown(&s, max))
        .unwrap_or_default()
}

/// Matches `SkillDetailPanel` / Cursor skill convention: prefer YAML `description:` in frontmatter,
/// else first non-empty line of the Markdown body (often `# Title`).
pub(crate) fn preview_from_markdown(s: &str, max: usize) -> String {
    let t = s.trim();
    let (fm_opt, body): (Option<&str>, &str) = if t.starts_with("---") {
        if let Some(rest) = t.strip_prefix("---") {
            if let Some(end) = rest.find("\n---") {
                (Some(rest[..end].trim()), rest[end + 4..].trim())
            } else {
                (None, t)
            }
        } else {
            (None, t)
        }
    } else {
        (None, t)
    };

    if let Some(fm) = fm_opt {
        if let Some(desc) = extract_description_from_yaml_like(fm, true) {
            let desc = desc.trim();
            if !desc.is_empty() {
                return truncate_chars(desc, max);
            }
        }
    }

    // Cursor-style skills: `* * *` + `## name:` + `description: ...` before first ATX H1 (`# `).
    if let Some(desc) = description_in_pseudo_header(t) {
        let desc = desc.trim();
        if !desc.is_empty() {
            return truncate_chars(desc, max);
        }
    }

    let one_line = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    truncate_chars(one_line.trim(), max)
}

/// `description:` inline scalar, or YAML block scalar (`|`, `>-`, …) until H1 / sibling map key.
fn extract_description_from_yaml_like(text: &str, stop_on_yaml_map_key: bool) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        let Some(after_desc) = trimmed.strip_prefix("description:") else {
            i += 1;
            continue;
        };
        let rest = after_desc.trim_start();

        if yaml_block_scalar_starts(rest) {
            i += 1;
            return collect_description_block_scalar(&lines, i, stop_on_yaml_map_key);
        }

        let val = rest.trim();
        if !val.is_empty() {
            return Some(unquote_yaml_scalar(val));
        }
        i += 1;
    }
    None
}

fn yaml_block_scalar_starts(rest: &str) -> bool {
    let s = rest.trim();
    let Some(first) = s.chars().next() else {
        return false;
    };
    first == '|' || first == '>'
}

/// Fold block lines into one line (YAML folded style approximation).
fn collect_description_block_scalar(
    lines: &[&str],
    mut i: usize,
    stop_on_yaml_map_key: bool,
) -> Option<String> {
    let mut buf: Vec<&str> = Vec::new();
    while i < lines.len() {
        let line = lines[i];
        if is_atx_h1_line(line) {
            break;
        }
        if stop_on_yaml_map_key && line_looks_like_yaml_map_key(line) {
            break;
        }
        buf.push(line);
        i += 1;
    }
    while buf.last().is_some_and(|l| l.trim().is_empty()) {
        buf.pop();
    }
    if buf.is_empty() {
        return None;
    }
    let folded = buf
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ");
    if folded.is_empty() {
        None
    } else {
        Some(folded)
    }
}

/// Short `key:` / `key: token` lines after `description:` block (YAML frontmatter siblings).
fn line_looks_like_yaml_map_key(line: &str) -> bool {
    let t = line.trim_start();
    if t.is_empty() || is_atx_h1_line(line) {
        return false;
    }
    let Some(colon_idx) = t.find(':') else {
        return false;
    };
    let key = &t[..colon_idx];
    if key.is_empty() {
        return false;
    }
    let mut key_chars = key.chars();
    let Some(first) = key_chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return false;
    }
    let after = t[colon_idx + 1..].trim();
    if after.is_empty() {
        return true;
    }
    if after.starts_with('"') || after.starts_with('\'') {
        return true;
    }
    after.split_whitespace().count() == 1
}

/// ATX H1: line starts with `#` but not `##` (after leading whitespace).
fn is_atx_h1_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('#') && !t.starts_with("##")
}

/// Lines before the first H1, capped — matches skills that use `* * *` / `## name:` instead of `---` YAML.
fn description_in_pseudo_header(content: &str) -> Option<String> {
    const MAX_HEADER_LINES: usize = 80;
    let mut header_lines = Vec::new();
    for line in content.lines().take(MAX_HEADER_LINES) {
        if is_atx_h1_line(line) {
            break;
        }
        header_lines.push(line);
    }
    extract_description_from_yaml_like(&header_lines.join("\n"), false)
}

fn unquote_yaml_scalar(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].replace("\\\"", "\"")
    } else if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

/// 与 SKILL 内 `name:` 对比用：小写、空白与下划线归一为 `-`、压连续 `-`。
fn skill_name_slug(s: &str) -> String {
    let t = s.trim().to_lowercase();
    let mut out = String::with_capacity(t.len());
    let mut prev_dash = true;
    for ch in t.chars() {
        let c = if ch.is_whitespace() || ch == '_' {
            '-'
        } else {
            ch
        };
        if c == '-' {
            if !prev_dash {
                out.push('-');
                prev_dash = true;
            }
        } else {
            out.push(c);
            prev_dash = false;
        }
    }
    out.trim_matches('-').to_string()
}

pub(crate) fn skills_container_dir(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "skills" | "skills-cursor" | "skill"
    )
}

/// `SKILL.md` 所在目录名与正文声明的 `name:` 一致（或正文中未声明 `name`）时视为技能包根目录。
pub(crate) fn folder_matches_skill_name(folder: &str, declared_name: Option<&str>) -> bool {
    match declared_name {
        None => true,
        Some(d) => skill_name_slug(folder) == skill_name_slug(d),
    }
}

fn extract_yaml_scalar_key(text: &str, key: &str, stop_on_yaml_map_key: bool) -> Option<String> {
    let prefix = format!("{key}:");
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        let Some(after_key) = trimmed.strip_prefix(&prefix) else {
            i += 1;
            continue;
        };
        let rest = after_key.trim_start();
        if yaml_block_scalar_starts(rest) {
            i += 1;
            return collect_description_block_scalar(&lines, i, stop_on_yaml_map_key);
        }
        let val = rest.trim();
        if !val.is_empty() {
            return Some(unquote_yaml_scalar(val));
        }
        i += 1;
    }
    None
}

fn header_lines_before_h1(content: &str, max_lines: usize) -> Vec<&str> {
    let mut header_lines = Vec::new();
    for line in content.lines().take(max_lines) {
        if is_atx_h1_line(line) {
            break;
        }
        header_lines.push(line);
    }
    header_lines
}

/// YAML frontmatter 或首个 `#` 标题前的伪头中的 `name:`。
pub(crate) fn extract_skill_declared_name(markdown: &str) -> Option<String> {
    let t = markdown.trim_start();
    let (fm_opt, body_after_fm): (Option<&str>, &str) = if t.starts_with("---") {
        if let Some(rest) = t.strip_prefix("---") {
            if let Some(end) = rest.find("\n---") {
                (Some(rest[..end].trim()), rest[end + 4..].trim())
            } else {
                (None, t)
            }
        } else {
            (None, t)
        }
    } else {
        (None, t)
    };

    if let Some(fm) = fm_opt {
        if let Some(n) = extract_yaml_scalar_key(fm, "name", true) {
            let n = n.trim();
            if !n.is_empty() {
                return Some(n.to_string());
            }
        }
    }

    let header = header_lines_before_h1(body_after_fm, 80).join("\n");
    extract_yaml_scalar_key(&header, "name", false)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 技能目录内除扫描到的主 `SKILL.md` 外的普通文件（不含子目录），排序后返回。
fn collect_skill_sibling_files(dir: &Path, primary_md: &Path) -> Option<Vec<String>> {
    let Ok(rd) = fs::read_dir(dir) else {
        return None;
    };
    let mut names: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() || path == primary_md {
                return None;
            }
            let fname = e.file_name().to_string_lossy().into_owned();
            if fname == ".DS_Store" {
                return None;
            }
            Some(fname)
        })
        .collect();
    names.sort();
    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

fn push_skills_from_paths(mut paths: Vec<PathBuf>, list: &mut Vec<AssetEntry>) {
    paths.sort();
    for p in paths {
        let Some(dir) = p.parent() else {
            continue;
        };
        let title = dir
            .file_name()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|| "skill".into());

        let content = fs::read_to_string(&p).unwrap_or_default();
        let desc = preview_from_markdown(&content, 160);
        let desc = if desc.is_empty() {
            p.to_string_lossy().into_owned()
        } else {
            desc
        };

        let declared = extract_skill_declared_name(&content);
        let use_skill_dir = dir.is_dir()
            && !skills_container_dir(&title)
            && folder_matches_skill_name(&title, declared.as_deref());

        let path_str = if use_skill_dir {
            dir.to_string_lossy().into_owned()
        } else {
            p.to_string_lossy().into_owned()
        };

        let skill_extra_files = if use_skill_dir {
            collect_skill_sibling_files(dir, &p)
        } else {
            None
        };

        list.push(AssetEntry {
            id: stable_id("skill", &p),
            kind: "skill".into(),
            title,
            description: desc,
            path: path_str,
            active: true,
            scenario: None,
            brief_zh: None,
            brief_en: None,
            skill_extra_files,
        });
    }
}

fn push_skills_from_roots(roots: &[PathBuf], list: &mut Vec<AssetEntry>) {
    let mut paths = Vec::new();
    for r in roots {
        collect_skill_files(r, &mut paths);
    }
    push_skills_from_paths(paths, list);
}

pub fn push_skills_from_roots_public(roots: &[PathBuf], list: &mut Vec<AssetEntry>) {
    push_skills_from_roots(roots, list);
}

fn push_skills_from_project_root(root: &Path, list: &mut Vec<AssetEntry>) {
    let mut paths = Vec::new();
    collect_project_skill_paths(root, &mut paths);
    dedupe_paths(&mut paths);
    push_skills_from_paths(paths, list);
}

fn push_prompt_commands_from_roots(roots: &[PathBuf], list: &mut Vec<AssetEntry>) {
    let mut paths = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with("cp-") && name.ends_with(".md") {
                paths.push(p);
            }
        }
    }
    paths.sort();
    for p in paths {
        let title = p
            .file_stem()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|| "cp-prompt".into());
        let desc = read_preview(&p, 160);
        let desc = if desc.is_empty() {
            p.to_string_lossy().into_owned()
        } else {
            desc
        };
        list.push(AssetEntry {
            id: stable_id("prompt", &p),
            kind: "prompt".into(),
            title,
            description: desc,
            path: p.to_string_lossy().into_owned(),
            active: true,
            scenario: None,
            brief_zh: None,
            brief_en: None,
            skill_extra_files: None,
        });
    }
}

fn push_rules_from_paths(mut paths: Vec<PathBuf>, list: &mut Vec<AssetEntry>) {
    paths.sort();
    for p in paths {
        let title = p
            .file_stem()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|| "rule".into());
        let desc = read_preview(&p, 160);
        let desc = if desc.is_empty() {
            p.to_string_lossy().into_owned()
        } else {
            desc
        };
        list.push(AssetEntry {
            id: stable_id("rule", &p),
            kind: "rule".into(),
            title,
            description: desc,
            path: p.to_string_lossy().into_owned(),
            active: true,
            scenario: None,
            brief_zh: None,
            brief_en: None,
            skill_extra_files: None,
        });
    }
}

fn push_rules_from_roots(roots: &[PathBuf], list: &mut Vec<AssetEntry>) {
    let mut paths = Vec::new();
    for r in roots {
        collect_rule_files(r, &mut paths);
    }
    push_rules_from_paths(paths, list);
}

fn push_rules_from_project_root(root: &Path, list: &mut Vec<AssetEntry>) {
    let mut paths = Vec::new();
    collect_project_rule_paths(root, &mut paths);
    dedupe_paths(&mut paths);
    push_rules_from_paths(paths, list);
}

fn parse_mcp_object_at(
    map: &serde_json::Map<String, Value>,
    source_json: Option<&Path>,
    list: &mut Vec<AssetEntry>,
) {
    for (name, cfg) in map {
        let desc = match cfg {
            Value::Object(o) => {
                let cmd = o.get("command").map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Array(arr) => arr.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    _ => String::new(),
                }).unwrap_or_default();
                let args = o
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let url = o.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if !url.is_empty() {
                    format!("url: {url}")
                } else if !cmd.is_empty() {
                    format!("{cmd} {args}").trim().to_string()
                } else {
                    cfg.to_string()
                }
            }
            _ => cfg.to_string(),
        };
        let id_key = match source_json {
            Some(p) => format!("{}|{}", p.to_string_lossy(), name),
            None => name.clone(),
        };
        list.push(AssetEntry {
            id: stable_id("mcp", Path::new(&id_key)),
            kind: "mcp".into(),
            title: name.clone(),
            description: desc,
            path: source_json
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("mcp:{name}")),
            active: true,
            scenario: None,
            brief_zh: None,
            brief_en: None,
            skill_extra_files: None,
        });
    }
}

fn parse_mcp_file(path: &Path, list: &mut Vec<AssetEntry>) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if let Some(m) = v.get("mcpServers").and_then(|x| x.as_object()) {
        parse_mcp_object_at(m, Some(path), list);
        return;
    }
    if let Some(m) = v.get("servers").and_then(|x| x.as_object()) {
        parse_mcp_object_at(m, Some(path), list);
    }
}

fn parse_mcp_json_files(paths: &[PathBuf], list: &mut Vec<AssetEntry>) {
    for p in paths {
        if p.is_file() {
            parse_mcp_file(p, list);
        }
    }
}

fn merge_mcp_from_json_files(paths: &[PathBuf], list: &mut Vec<AssetEntry>) {
    for p in paths {
        if !p.is_file() {
            continue;
        }
        if let Ok(text) = fs::read_to_string(p) {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                merge_mcp_from_json_value(&v, list);
            }
        }
    }
}

fn parse_mcp_toml_files(paths: &[PathBuf], list: &mut Vec<AssetEntry>) {
    for p in paths {
        if p.is_file() {
            parse_mcp_toml_file(p, list);
        }
    }
}

fn toml_value_field_str<'a>(table: &'a toml::Table, key: &str) -> &'a str {
    table.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn parse_toml_mcp_object_at(
    map: &toml::Table,
    source_toml: Option<&Path>,
    list: &mut Vec<AssetEntry>,
) {
    for (name, cfg) in map {
        let desc = match cfg {
            toml::Value::Table(o) => {
                let cmd = toml_value_field_str(o, "command");
                let args = o
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let url = toml_value_field_str(o, "url");
                if !url.is_empty() {
                    format!("url: {url}")
                } else if !cmd.is_empty() {
                    format!("{cmd} {args}").trim().to_string()
                } else {
                    cfg.to_string()
                }
            }
            _ => cfg.to_string(),
        };
        let id_key = match source_toml {
            Some(p) => format!("{}|{}", p.to_string_lossy(), name),
            None => name.clone(),
        };
        list.push(AssetEntry {
            id: stable_id("mcp", Path::new(&id_key)),
            kind: "mcp".into(),
            title: name.clone(),
            description: desc,
            path: source_toml
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("mcp:{name}")),
            active: true,
            scenario: None,
            brief_zh: None,
            brief_en: None,
            skill_extra_files: None,
        });
    }
}

fn parse_mcp_toml_file(path: &Path, list: &mut Vec<AssetEntry>) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let Ok(v) = toml::from_str::<toml::Value>(&text) else {
        return;
    };
    if let Some(m) = v.get("mcp_servers").and_then(|x| x.as_table()) {
        parse_toml_mcp_object_at(m, Some(path), list);
    }
    if let Some(m) = v.get("mcpServers").and_then(|x| x.as_table()) {
        parse_toml_mcp_object_at(m, Some(path), list);
    }
}

fn merge_mcp_from_json_value(v: &Value, list: &mut Vec<AssetEntry>) {
    merge_mcp_from_json_value_at(v, None, list);
}

fn merge_mcp_from_json_value_at(v: &Value, source_json: Option<&Path>, list: &mut Vec<AssetEntry>) {
    if let Some(m) = v.get("mcpServers").and_then(|x| x.as_object()) {
        parse_mcp_object_at(m, source_json, list);
    }
    if let Some(m) = v.get("mcp").and_then(|x| x.as_object()) {
        parse_mcp_object_at(m, source_json, list);
    }
}

fn walk_json_for_mcp(dir: &Path, depth: usize, max_depth: usize, list: &mut Vec<AssetEntry>) {
    if depth > max_depth || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if should_skip_scan_dir(name) {
                    continue;
                }
            }
            walk_json_for_mcp(&p, depth + 1, max_depth, list);
        } else if p.extension().and_then(|e| e.to_str()) == Some("json") {
            let Ok(text) = fs::read_to_string(&p) else {
                continue;
            };
            let Ok(v) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            if p.file_name().and_then(|n| n.to_str()) == Some("mcp.json") {
                parse_mcp_file(&p, list);
            } else {
                merge_mcp_from_json_value_at(&v, Some(&p), list);
            }
        }
    }
}

/// Walks `root`: `SKILL.md` only under conventional agent `skills/` dirs (see `collect_project_skill_paths`),
/// agent rules (see `collect_project_rule_paths`), and MCP from JSON (`mcp.json`, settings with MCP keys, etc.).
/// Global inventory under a user-chosen dot-folder root (e.g. `~/.mytool`):
/// `skills/`, `commands/`, `rules/`, plus `mcp.json` / `settings.json` / `config.toml` at the root.
pub fn global_inventory_at_agent_root(root: &Path) -> Result<AgentInventory, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("无法解析路径: {e}"))?;
    if !root.is_dir() {
        return Err("所选路径不是文件夹".into());
    }

    let mut skills = Vec::new();
    let mut mcp = Vec::new();
    let mut rules = Vec::new();

    push_skills_from_roots(&[root.join("skills")], &mut skills);
    push_prompt_commands_from_roots(&[root.join("commands")], &mut skills);
    parse_mcp_json_files(&[root.join("mcp.json")], &mut mcp);
    merge_mcp_from_json_files(&[root.join("settings.json")], &mut mcp);
    parse_mcp_toml_files(&[root.join("config.toml")], &mut mcp);
    let mut rule_paths = Vec::new();
    let rules_dir = root.join("rules");
    if rules_dir.is_dir() {
        walk_rules_mdc_md(&rules_dir, 0, 12, &mut rule_paths);
    }
    dedupe_paths(&mut rule_paths);
    push_rules_from_paths(rule_paths, &mut rules);

    dedupe_mcp(&mut mcp);

    Ok(AgentInventory { skills, mcp, rules })
}

pub fn scan_project_directory(root: &Path) -> Result<AgentInventory, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("无法解析路径: {e}"))?;
    if !root.is_dir() {
        return Err("所选路径不是文件夹".into());
    }

    let mut skills = Vec::new();
    let mut mcp = Vec::new();
    let mut rules = Vec::new();

    push_skills_from_project_root(&root, &mut skills);
    walk_json_for_mcp(&root, 0, 16, &mut mcp);
    let codex_project_config = root.join(".codex/config.toml");
    if codex_project_config.is_file() {
        parse_mcp_toml_file(&codex_project_config, &mut mcp);
    }
    for rel in [".hermes", ".openclaw"] {
        let agent_root = root.join(rel);
        parse_mcp_json_files(&[agent_root.join("mcp.json")], &mut mcp);
        merge_mcp_from_json_files(&[agent_root.join("settings.json")], &mut mcp);
        parse_mcp_toml_files(&[agent_root.join("config.toml")], &mut mcp);
    }
    push_rules_from_project_root(&root, &mut rules);

    dedupe_mcp(&mut mcp);

    Ok(AgentInventory { skills, mcp, rules })
}

pub fn global_inventory(agent_id: &str) -> Result<AgentInventory, String> {
    let home = home_dir();
    let mut skills = Vec::new();
    let mut mcp = Vec::new();
    let mut rules = Vec::new();

    match agent_id {
        "cursor" => {
            let roots_skill = vec![
                home.join(".cursor/skills-cursor"),
                home.join(".cursor/skills"),
            ];
            push_skills_from_roots(&roots_skill, &mut skills);
            push_prompt_commands_from_roots(&[home.join(".cursor/commands")], &mut skills);
            let mcp_path = home.join(".cursor/mcp.json");
            if mcp_path.is_file() {
                parse_mcp_file(&mcp_path, &mut mcp);
            }
            push_rules_from_roots(&[home.join(".cursor/rules")], &mut rules);
        }
        "claude" => {
            push_skills_from_roots(&[home.join(".claude/skills")], &mut skills);
            push_prompt_commands_from_roots(&[home.join(".claude/commands")], &mut skills);
            let settings = home.join(".claude/settings.json");
            if settings.is_file() {
                if let Ok(text) = fs::read_to_string(&settings) {
                    if let Ok(v) = serde_json::from_str::<Value>(&text) {
                        merge_mcp_from_json_value(&v, &mut mcp);
                    }
                }
            }
            let local = home.join(".claude/settings.local.json");
            if local.is_file() {
                if let Ok(text) = fs::read_to_string(&local) {
                    if let Ok(v) = serde_json::from_str::<Value>(&text) {
                        merge_mcp_from_json_value(&v, &mut mcp);
                    }
                }
            }
            let root_json = home.join(".claude.json");
            if root_json.is_file() {
                if let Ok(text) = fs::read_to_string(&root_json) {
                    if let Ok(v) = serde_json::from_str::<Value>(&text) {
                        merge_mcp_from_json_value(&v, &mut mcp);
                    }
                }
            }
            push_rules_from_roots(&[home.join(".claude/rules")], &mut rules);
        }
        "codex" => {
            push_skills_from_roots(&[home.join(".codex/skills")], &mut skills);
            push_prompt_commands_from_roots(&[home.join(".codex/prompts")], &mut skills);
            let config = home.join(".codex/config.toml");
            if config.is_file() {
                parse_mcp_toml_file(&config, &mut mcp);
            }
            let mut codex_paths = Vec::new();
            push_if_file(home.join(".codex/AGENTS.md"), &mut codex_paths);
            let codex_rules = home.join(".codex/rules");
            if codex_rules.is_dir() {
                walk_rules_mdc_md(&codex_rules, 0, 12, &mut codex_paths);
            }
            dedupe_paths(&mut codex_paths);
            push_rules_from_paths(codex_paths, &mut rules);
        }
        "hermes" => {
            push_skills_from_roots(&[home.join(".hermes/skills")], &mut skills);
            push_prompt_commands_from_roots(&[home.join(".hermes/commands")], &mut skills);
            let hermes_home = home.join(".hermes");
            parse_mcp_json_files(&[hermes_home.join("mcp.json")], &mut mcp);
            merge_mcp_from_json_files(&[hermes_home.join("settings.json")], &mut mcp);
            parse_mcp_toml_files(&[hermes_home.join("config.toml")], &mut mcp);
            let mut hermes_paths = Vec::new();
            push_if_file(hermes_home.join("HERMES.md"), &mut hermes_paths);
            let hermes_rules = hermes_home.join("rules");
            if hermes_rules.is_dir() {
                walk_rules_mdc_md(&hermes_rules, 0, 12, &mut hermes_paths);
            }
            dedupe_paths(&mut hermes_paths);
            push_rules_from_paths(hermes_paths, &mut rules);
        }
        "openclaw" => {
            push_skills_from_roots(&[home.join(".openclaw/skills")], &mut skills);
            push_prompt_commands_from_roots(&[home.join(".openclaw/commands")], &mut skills);
            let openclaw_home = home.join(".openclaw");
            parse_mcp_json_files(&[openclaw_home.join("mcp.json")], &mut mcp);
            merge_mcp_from_json_files(&[openclaw_home.join("settings.json")], &mut mcp);
            parse_mcp_toml_files(&[openclaw_home.join("config.toml")], &mut mcp);
            let mut openclaw_paths = Vec::new();
            push_if_file(openclaw_home.join("OPENCLAW.md"), &mut openclaw_paths);
            let openclaw_rules = openclaw_home.join("rules");
            if openclaw_rules.is_dir() {
                walk_rules_mdc_md(&openclaw_rules, 0, 12, &mut openclaw_paths);
            }
            dedupe_paths(&mut openclaw_paths);
            push_rules_from_paths(openclaw_paths, &mut rules);
        }
        "trae" => {
            push_skills_from_roots(&[home.join(".trae/skills")], &mut skills);
            push_prompt_commands_from_roots(&[home.join(".trae/commands")], &mut skills);
            for name in [".trae/mcp.json", ".cursor/mcp.json"] {
                let p = home.join(name);
                if p.is_file() {
                    parse_mcp_file(&p, &mut mcp);
                }
            }
            #[cfg(target_os = "macos")]
            {
                let asupport = home.join("Library/Application Support/Trae/User/mcp.json");
                if asupport.is_file() {
                    parse_mcp_file(&asupport, &mut mcp);
                }
            }
            let mut tr_paths = Vec::new();
            let tr = home.join(".trae/rules");
            if tr.is_dir() {
                walk_rules_mdc_md(&tr, 0, 12, &mut tr_paths);
            }
            push_if_file(home.join(".trae/trae.config.jsonc"), &mut tr_paths);
            push_if_file(home.join(".trae/trae.config.json"), &mut tr_paths);
            let tr_home = home.join(".trae");
            if tr_home.is_dir() {
                push_json_jsonc_in_dir_shallow(&tr_home, &mut tr_paths);
            }
            dedupe_paths(&mut tr_paths);
            push_rules_from_paths(tr_paths, &mut rules);
        }
        "qoder" => {
            push_skills_from_roots(
                &[home.join(".qoder/skills"), home.join(".qoderwork/skills")],
                &mut skills,
            );
            push_prompt_commands_from_roots(
                &[
                    home.join(".qoder/commands"),
                    home.join(".qoderwork/commands"),
                ],
                &mut skills,
            );
            for rel in [".qoder/mcp.json", ".qoderwork/mcp.json"] {
                let p = home.join(rel);
                if p.is_file() {
                    parse_mcp_file(&p, &mut mcp);
                }
            }
            for rel in [".qoder/settings.json", ".qoderwork/settings.json"] {
                let p = home.join(rel);
                if p.is_file() {
                    if let Ok(text) = fs::read_to_string(&p) {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            merge_mcp_from_json_value(&v, &mut mcp);
                        }
                    }
                }
            }
            let mut q_paths = Vec::new();
            for r in [home.join(".qoder/rules"), home.join(".qoderwork/rules")] {
                if r.is_dir() {
                    walk_rules_mdc_md(&r, 0, 12, &mut q_paths);
                }
            }
            for q in [home.join(".qoder"), home.join(".qoderwork")] {
                if q.is_dir() {
                    push_json_jsonc_in_dir_shallow(&q, &mut q_paths);
                }
            }
            dedupe_paths(&mut q_paths);
            push_rules_from_paths(q_paths, &mut rules);
        }
        "kiro" => {
            push_skills_from_roots(&[home.join(".kiro/skills")], &mut skills);
            push_prompt_commands_from_roots(&[home.join(".kiro/commands")], &mut skills);
            let mut k_paths = Vec::new();
            let kr = home.join(".kiro/rules");
            if kr.is_dir() {
                walk_rules_mdc_md(&kr, 0, 12, &mut k_paths);
            }
            let kiro_home = home.join(".kiro");
            if kiro_home.is_dir() {
                push_json_jsonc_in_dir_shallow(&kiro_home, &mut k_paths);
            }
            dedupe_paths(&mut k_paths);
            push_rules_from_paths(k_paths, &mut rules);
        }
        "opencode" => {
            push_skills_from_roots(
                &[
                    home.join(".config/opencode/skills"),
                    home.join(".config/opencode/skill"),
                ],
                &mut skills,
            );
            let config = home.join(".config/opencode/opencode.json");
            if config.is_file() {
                merge_mcp_from_json_files(&[config], &mut mcp);
            }
        }
        _ => return Err(format!("unknown agent: {agent_id}")),
    }

    dedupe_mcp(&mut mcp);

    Ok(AgentInventory { skills, mcp, rules })
}

fn dedupe_mcp(items: &mut Vec<AssetEntry>) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut i = 0;
    while i < items.len() {
        let key = items[i].title.clone();
        if seen.contains_key(&key) {
            items.remove(i);
        } else {
            seen.insert(key, i);
            i += 1;
        }
    }
}

fn walk_doc_files(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    candidates: &[&str],
    out: &mut Vec<(PathBuf, String)>,
) {
    if depth > max_depth || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if should_skip_scan_dir(name) {
                    continue;
                }
            }
            walk_doc_files(&p, depth + 1, max_depth, candidates, out);
        } else if p.is_file() {
            let fname_os = p.file_name().map(|n| n.to_string_lossy().to_string());
            if let Some(ref fname) = fname_os {
                if candidates.contains(&fname.as_str()) {
                    out.push((p, fname.clone()));
                }
            }
        }
    }
}

/// Read a documentation file (SKILL.md, README.md, etc.) from a given path.
/// If `path` is a directory, searches for known doc files within it.
/// If `path` is a file, reads it directly.
/// Returns `(filename, content)`.
pub fn read_skill_document(path: &Path) -> Result<(String, String), String> {
    if path.is_dir() {
        let candidates = [
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
            "OPENCODE.md",
            "opencode.md",
            "README.md",
            "readme.md",
        ];

        // First pass: exact match at root of directory
        for name in &candidates {
            let file_path = path.join(name);
            if file_path.is_file() {
                let content =
                    fs::read_to_string(&file_path).map_err(|e| format!("读取文件失败: {e}"))?;
                return Ok((name.to_string(), content));
            }
        }

        // Second pass: recursive search (max_depth=4) — matches skills-manager behavior
        let mut found = Vec::new();
        walk_doc_files(path, 0, 4, &candidates, &mut found);
        // Sort so SKILL.md is preferred over README.md if both exist
        found.sort_by(|a, b| {
            let a_idx = candidates.iter().position(|c| *c == a.1).unwrap_or(99);
            let b_idx = candidates.iter().position(|c| *c == b.1).unwrap_or(99);
            a_idx.cmp(&b_idx)
        });
        if let Some((file_path, fname)) = found.into_iter().next() {
            let content =
                fs::read_to_string(&file_path).map_err(|e| format!("读取文件失败: {e}"))?;
            return Ok((fname, content));
        }

        Err("未找到文档文件 (SKILL.md / README.md)".to_string())
    } else if path.is_file() {
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let content = fs::read_to_string(path).map_err(|e| format!("读取文件失败: {e}"))?;
        Ok((fname, content))
    } else {
        Err("路径不存在".to_string())
    }
}
