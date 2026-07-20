use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_targets")]
    pub targets: String,
    #[serde(default = "default_globs")]
    pub globs: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub is_fragment: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_prompt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
}

fn default_targets() -> String {
    r#"["*"]"#.to_string()
}

fn default_globs() -> String {
    "[]".to_string()
}

pub const MAX_FRAGMENTS_PER_PARENT: usize = 50;

/// Maximum files to scan when checking globs against a project tree.
const MAX_GLOB_SCAN_FILES: usize = 10_000;

/// Maximum directory depth when scanning for glob matches.
const MAX_GLOB_SCAN_DEPTH: usize = 8;

pub fn parse_targets_json(targets: &str) -> Vec<String> {
    serde_json::from_str(targets).unwrap_or_else(|_| vec!["*".to_string()])
}

/// Parse the `globs` JSON field into a vector of glob pattern strings.
/// Returns an empty vec for invalid JSON or `"[]"`.
pub fn parse_globs_json(globs: &str) -> Vec<String> {
    serde_json::from_str(globs).unwrap_or_default()
}

pub fn fragment_matches_target(fragment: &Prompt, target_app: &str) -> bool {
    if !fragment.is_fragment {
        return false;
    }
    let targets = parse_targets_json(&fragment.targets);
    targets.iter().any(|t| t == "*" || t == target_app)
}

/// Check whether a fragment's globs match a specific relative file path.
/// Fragments with empty globs always match (universal scope).
pub fn fragment_matches_file(fragment: &Prompt, relative_path: &str) -> bool {
    let globs = parse_globs_json(&fragment.globs);
    if globs.is_empty() {
        return true;
    }
    let path = relative_path.replace('\\', "/");
    globs.iter().any(|pattern| {
        globset::GlobBuilder::new(pattern)
            .literal_separator(false)
            .build()
            .ok()
            .and_then(|g| g.compile_matcher().is_match(&path).then_some(true))
            .is_some()
    })
}

/// Check whether a fragment's globs match at least one file under the project root.
/// Returns `true` if globs are empty (universal), or if any scanned file matches.
pub fn fragment_globs_match_project(fragment: &Prompt, project_root: &Path) -> bool {
    let globs = parse_globs_json(&fragment.globs);
    if globs.is_empty() {
        return true;
    }

    let matchers: Vec<globset::GlobMatcher> = globs
        .iter()
        .filter_map(|p| {
            globset::GlobBuilder::new(p)
                .literal_separator(false)
                .build()
                .ok()
                .map(|g| g.compile_matcher())
        })
        .collect();

    if matchers.is_empty() {
        return true;
    }

    let mut count = 0usize;
    walk_for_glob_match(project_root, project_root, &matchers, 0, &mut count)
}

/// Recursively walk the project tree looking for glob matches.
/// Skips hidden directories, common build artefacts, and honours depth/count limits.
fn walk_for_glob_match(
    root: &Path,
    dir: &Path,
    matchers: &[globset::GlobMatcher],
    depth: usize,
    count: &mut usize,
) -> bool {
    if depth > MAX_GLOB_SCAN_DEPTH || *count >= MAX_GLOB_SCAN_FILES {
        return false;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        if *count >= MAX_GLOB_SCAN_FILES {
            return false;
        }

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden, node_modules, build artefacts
        if name_str.starts_with('.')
            || name_str == "node_modules"
            || name_str == "target"
            || name_str == "dist"
            || name_str == "build"
        {
            continue;
        }

        if ft.is_file() {
            *count += 1;
            if let Ok(rel) = entry.path().strip_prefix(root) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if matchers.iter().any(|m| m.is_match(&rel_str)) {
                    return true;
                }
            }
        } else if ft.is_dir() {
            if walk_for_glob_match(root, &entry.path(), matchers, depth + 1, count) {
                return true;
            }
        }
    }

    false
}

/// Original composition — filter by target app, sort by priority, concatenate.
/// Backward-compatible; does not consider globs.
pub fn compose_prompt_fragments(fragments: &[Prompt], target_app: &str) -> String {
    let mut matching: Vec<&Prompt> = fragments
        .iter()
        .filter(|f| fragment_matches_target(f, target_app))
        .collect();
    matching.sort_by(|a, b| b.priority.cmp(&a.priority));
    matching
        .iter()
        .map(|f| f.content.as_str())
        .filter(|c| !c.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Composition with optional file-path context.
/// When `file_path` is `Some`, fragments with non-empty globs are additionally
/// filtered to those whose globs match the given relative path.
/// Fragments with empty globs always pass (universal scope).
pub fn compose_prompt_fragments_with_context(
    fragments: &[Prompt],
    target_app: &str,
    file_path: Option<&str>,
) -> String {
    let mut matching: Vec<&Prompt> = fragments
        .iter()
        .filter(|f| {
            fragment_matches_target(f, target_app)
                && file_path.map_or(true, |p| fragment_matches_file(f, p))
        })
        .collect();
    matching.sort_by(|a, b| b.priority.cmp(&a.priority));
    matching
        .iter()
        .map(|f| f.content.as_str())
        .filter(|c| !c.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Project-level composition: includes fragments whose target matches AND
/// whose globs (if any) match at least one file under the project tree.
/// This is the entry point for project sync — fragments scoped to files that
/// don't exist in the project are excluded.
pub fn compose_prompt_fragments_for_project(
    fragments: &[Prompt],
    target_app: &str,
    project_root: &Path,
) -> String {
    let mut matching: Vec<&Prompt> = fragments
        .iter()
        .filter(|f| {
            fragment_matches_target(f, target_app)
                && fragment_globs_match_project(f, project_root)
        })
        .collect();
    matching.sort_by(|a, b| b.priority.cmp(&a.priority));
    matching
        .iter()
        .map(|f| f.content.as_str())
        .filter(|c| !c.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            content: String::new(),
            description: None,
            enabled: false,
            targets: default_targets(),
            globs: default_globs(),
            priority: 0,
            is_fragment: false,
            parent_prompt_id: None,
            created_at: None,
            updated_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(id: &str, content: &str, targets: &str, priority: i32) -> Prompt {
        Prompt {
            id: id.into(),
            name: id.into(),
            content: content.into(),
            enabled: false,
            targets: targets.into(),
            is_fragment: true,
            priority,
            ..Default::default()
        }
    }

    fn frag_with_globs(
        id: &str,
        content: &str,
        targets: &str,
        globs: &str,
        priority: i32,
    ) -> Prompt {
        let mut f = frag(id, content, targets, priority);
        f.globs = globs.into();
        f
    }

    #[test]
    fn compose_filters_by_target_and_priority() {
        let fragments = vec![
            frag("a", "A", r#"["claude"]"#, 1),
            frag("b", "B", r#"["gemini"]"#, 10),
            frag("c", "C", r#"["*"]"#, 5),
        ];
        assert_eq!(compose_prompt_fragments(&fragments, "claude"), "C\n\nA");
        assert_eq!(compose_prompt_fragments(&fragments, "gemini"), "B\n\nC");
    }

    #[test]
    fn parse_globs_handles_valid_and_invalid() {
        assert_eq!(parse_globs_json(r#"["src/**/*.ts"]"#), vec!["src/**/*.ts"]);
        assert_eq!(
            parse_globs_json(r#"["*.ts","*.tsx"]"#),
            vec!["*.ts", "*.tsx"]
        );
        let empty: Vec<String> = vec![];
        assert_eq!(parse_globs_json("[]"), empty);
        assert_eq!(parse_globs_json("invalid"), empty);
        assert_eq!(parse_globs_json(""), empty);
    }

    #[test]
    fn fragment_matches_file_universal_when_no_globs() {
        let f = frag("a", "A", r#"["*"]"#, 1);
        assert!(fragment_matches_file(&f, "anything.ts"));
        assert!(fragment_matches_file(&f, "deep/path/file.rs"));
    }

    #[test]
    fn fragment_matches_file_with_glob_patterns() {
        let f = frag_with_globs("a", "A", r#"["*"]"#, r#"["src/**/*.ts"]"#, 1);
        assert!(fragment_matches_file(&f, "src/index.ts"));
        assert!(fragment_matches_file(&f, "src/components/App.ts"));
        assert!(!fragment_matches_file(&f, "src/index.rs"));
        assert!(!fragment_matches_file(&f, "test/index.ts"));
    }

    #[test]
    fn fragment_matches_file_multiple_globs() {
        let f = frag_with_globs(
            "a",
            "A",
            r#"["*"]"#,
            r#"["src/**/*.ts","lib/**/*.tsx"]"#,
            1,
        );
        assert!(fragment_matches_file(&f, "src/index.ts"));
        assert!(fragment_matches_file(&f, "lib/component.tsx"));
        assert!(!fragment_matches_file(&f, "src/style.css"));
    }

    #[test]
    fn compose_with_context_filters_by_glob() {
        let fragments = vec![
            frag_with_globs("ts", "TS rules", r#"["*"]"#, r#"["**/*.ts"]"#, 10),
            frag("universal", "Universal", r#"["*"]"#, 5),
            frag_with_globs("rs", "RS rules", r#"["*"]"#, r#"["**/*.rs"]"#, 8),
        ];

        // With TS file context: ts + universal, no rs
        let result = compose_prompt_fragments_with_context(
            &fragments,
            "claude",
            Some("src/index.ts"),
        );
        assert_eq!(result, "TS rules\n\nUniversal");

        // With RS file context: rs + universal, no ts
        let result = compose_prompt_fragments_with_context(
            &fragments,
            "claude",
            Some("src/lib.rs"),
        );
        assert_eq!(result, "RS rules\n\nUniversal");

        // Without file context: all match (backward-compatible)
        let result = compose_prompt_fragments_with_context(&fragments, "claude", None);
        assert_eq!(result, "TS rules\n\nRS rules\n\nUniversal");
    }

    #[test]
    fn compose_for_project_includes_universal_fragments() {
        let fragments = vec![
            frag("universal", "Universal", r#"["*"]"#, 5),
            frag_with_globs("ts", "TS rules", r#"["*"]"#, r#"["**/*.ts"]"#, 10),
        ];

        // Use a temp dir with a .ts file
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("index.ts"), "").unwrap();

        let result =
            compose_prompt_fragments_for_project(&fragments, "claude", tmp.path());
        assert_eq!(result, "TS rules\n\nUniversal");
    }

    #[test]
    fn compose_for_project_excludes_nonmatching_globs() {
        let fragments = vec![
            frag("universal", "Universal", r#"["*"]"#, 5),
            frag_with_globs("py", "PY rules", r#"["*"]"#, r#"["**/*.py"]"#, 10),
        ];

        // Temp dir with only .ts files — PY fragment should be excluded
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("index.ts"), "").unwrap();

        let result =
            compose_prompt_fragments_for_project(&fragments, "claude", tmp.path());
        assert_eq!(result, "Universal");
    }
}
