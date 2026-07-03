//! Marker-based Markdown section merge for managed config injection.
//!
//! OpenSunstar-owned blocks use `<!-- opensunstar:ID -->` … `<!-- /opensunstar:ID -->`.
//! Content outside managed markers (including `<!-- gentle-ai:* -->` sections) is preserved.

pub const PROMPT_SECTION_ID: &str = "managed-prompt";
/// Claude Code 跨工具 SSOT 桥接行（写入独立 marker 段）
pub const AGENTS_BRIDGE_SECTION_ID: &str = "agents-bridge";
pub const AGENTS_BRIDGE_LINE: &str = "@AGENTS.md";
/// 项目级 Command 文件首行标记（L2-04）
pub const MANAGED_COMMAND_MARKER_PREFIX: &str = "<!-- opensunstar:managed-command";

const MARKER_PREFIX: &str = "<!-- opensunstar:";
const MARKER_SUFFIX: &str = " -->";
const CLOSE_PREFIX: &str = "<!-- /opensunstar:";

fn open_marker(section_id: &str) -> String {
    format!("{MARKER_PREFIX}{section_id}{MARKER_SUFFIX}")
}

fn close_marker(section_id: &str) -> String {
    format!("{CLOSE_PREFIX}{section_id}{MARKER_SUFFIX}")
}

/// Extract inner content of a managed section, if present.
pub fn extract_markdown_section(content: &str, section_id: &str) -> Option<String> {
    let open = open_marker(section_id);
    let close = close_marker(section_id);
    let (open_idx, close_idx) = find_marker_pair(content, &open, &close)?;
    let inner_start = open_idx + open.len();
    let inner = content[inner_start..close_idx].trim_matches('\n');
    Some(inner.to_string())
}

/// Replace or append a marked section. Empty `content` removes the section only.
pub fn inject_markdown_section(existing: &str, section_id: &str, content: &str) -> String {
    let open = open_marker(section_id);
    let close = close_marker(section_id);

    let existing = strip_orphan_markers(existing, &open, &close);

    if let Some((open_idx, close_idx)) = find_marker_pair(&existing, &open, &close) {
        if content.is_empty() {
            return join_without_section(&existing, open_idx, close_idx, &close);
        }

        let before = &existing[..open_idx];
        let after = &existing[close_idx + close.len()..];
        return format!(
            "{}{}{}\n{}\n{}{}",
            before,
            open,
            "\n",
            content.trim_end_matches('\n'),
            if content.ends_with('\n') { "" } else { "\n" },
            format!("{close}{after}")
        );
    }

    if content.is_empty() {
        return existing.to_string();
    }

    let mut result = existing.to_string();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(&open);
    result.push('\n');
    result.push_str(content.trim_end_matches('\n'));
    result.push('\n');
    result.push_str(&close);
    result.push('\n');
    result
}

fn find_marker_pair<'a>(content: &'a str, open: &str, close: &str) -> Option<(usize, usize)> {
    let open_idx = content.find(open)?;
    let close_idx = content.find(close)?;
    if close_idx > open_idx {
        Some((open_idx, close_idx))
    } else {
        None
    }
}

fn join_without_section(existing: &str, open_idx: usize, close_idx: usize, close: &str) -> String {
    let mut before = existing[..open_idx].trim_end_matches('\n').to_string();
    let after = existing[close_idx + close.len()..]
        .trim_start_matches('\n')
        .to_string();

    match (before.is_empty(), after.is_empty()) {
        (true, true) => String::new(),
        (true, false) => after,
        (false, true) => {
            before.push('\n');
            before
        }
        (false, false) => {
            before.push_str("\n\n");
            before.push_str(&after);
            before.push('\n');
            before
        }
    }
}

fn strip_orphan_markers(content: &str, open: &str, close: &str) -> String {
    let mut content = content.to_string();
    loop {
        let open_idx = content.find(open);
        let close_idx = content.find(close);

        match (open_idx, close_idx) {
            (None, None) => return content,
            (None, Some(c)) => {
                content = remove_range(&content, c, c + close.len());
            }
            (Some(o), None) => {
                content = remove_range(&content, o, o + open.len());
            }
            (Some(o), Some(c)) if c < o => {
                content = remove_range(&content, c, c + close.len());
            }
            (Some(_), Some(_)) => return content,
        }
    }
}

fn remove_range(s: &str, start: usize, end: usize) -> String {
    format!("{}{}", &s[..start], &s[end..])
}

/// Wrap command body with a single-line managed marker (project-level commands dirs).
pub fn wrap_managed_command(command_id: &str, body: &str) -> String {
    format!(
        "{MANAGED_COMMAND_MARKER_PREFIX} id=\"{command_id}\" -->\n{}",
        body.trim_start()
    )
}

/// Strip the managed-command marker line if present.
pub fn strip_managed_command_marker(text: &str) -> String {
    let mut lines = text.lines();
    if lines
        .next()
        .is_some_and(|l| l.trim().starts_with(MANAGED_COMMAND_MARKER_PREFIX))
    {
        lines.collect::<Vec<_>>().join("\n")
    } else {
        text.to_string()
    }
}

pub fn is_managed_command_file(text: &str) -> bool {
    text.lines()
        .next()
        .is_some_and(|l| l.trim().starts_with(MANAGED_COMMAND_MARKER_PREFIX))
}

/// 项目级 Subagent 文件首行标记
pub const MANAGED_SUBAGENT_MARKER_PREFIX: &str = "<!-- opensunstar:managed-subagent";

pub fn wrap_managed_subagent(agent_id: &str, body: &str) -> String {
    format!(
        "{MANAGED_SUBAGENT_MARKER_PREFIX} id=\"{agent_id}\" -->\n{}",
        body.trim_start()
    )
}

pub fn wrap_managed_subagent_codex(agent_id: &str, body: &str) -> String {
    format!(
        "# opensunstar:managed-subagent id=\"{agent_id}\"\n{}",
        body.trim_start()
    )
}

pub fn strip_managed_subagent_marker(text: &str) -> String {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default().trim();
    if first.starts_with(MANAGED_SUBAGENT_MARKER_PREFIX)
        || first.starts_with("# opensunstar:managed-subagent")
    {
        lines.collect::<Vec<_>>().join("\n")
    } else {
        text.to_string()
    }
}

pub fn is_managed_subagent_file(text: &str) -> bool {
    let first = text.lines().next().unwrap_or_default().trim();
    first.starts_with(MANAGED_SUBAGENT_MARKER_PREFIX)
        || first.starts_with("# opensunstar:managed-subagent")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_roundtrip() {
        let merged = inject_markdown_section("user\n", PROMPT_SECTION_ID, "body");
        assert_eq!(
            extract_markdown_section(&merged, PROMPT_SECTION_ID).as_deref(),
            Some("body")
        );
    }

    #[test]
    fn inject_appends_when_section_missing() {
        let out = inject_markdown_section("# User rules\n", PROMPT_SECTION_ID, "OS prompt");
        assert!(out.contains("# User rules"));
        assert!(out.contains("<!-- opensunstar:managed-prompt -->"));
        assert!(out.contains("OS prompt"));
        assert!(out.contains("<!-- /opensunstar:managed-prompt -->"));
    }

    #[test]
    fn inject_replaces_existing_section() {
        let initial = inject_markdown_section("", PROMPT_SECTION_ID, "v1");
        let updated = inject_markdown_section(&initial, PROMPT_SECTION_ID, "v2");
        assert!(!updated.contains("v1"));
        assert!(updated.contains("v2"));
    }

    #[test]
    fn inject_empty_removes_section_preserves_outside() {
        let with_gentle = "# Custom\n\n<!-- gentle-ai:persona -->\nGentle\n<!-- /gentle-ai:persona -->\n";
        let with_os = inject_markdown_section(with_gentle, PROMPT_SECTION_ID, "managed");
        let cleared = inject_markdown_section(&with_os, PROMPT_SECTION_ID, "");
        assert!(!cleared.contains("opensunstar:managed-prompt"));
        assert!(cleared.contains("gentle-ai:persona"));
        assert!(cleared.contains("# Custom"));
        assert!(!cleared.contains("managed"));
    }

    #[test]
    fn preserves_gentle_ai_while_updating_os_section() {
        let base = "# Top\n\n<!-- gentle-ai:sdd -->\nSDD rules\n<!-- /gentle-ai:sdd -->\n";
        let out = inject_markdown_section(base, PROMPT_SECTION_ID, "OpenSunstar prompt");
        assert!(out.contains("gentle-ai:sdd"));
        assert!(out.contains("OpenSunstar prompt"));
        assert!(out.contains("# Top"));
    }
}
