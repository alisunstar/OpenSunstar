use serde::{Deserialize, Serialize};

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

pub fn parse_targets_json(targets: &str) -> Vec<String> {
    serde_json::from_str(targets).unwrap_or_else(|_| vec!["*".to_string()])
}

pub fn fragment_matches_target(fragment: &Prompt, target_app: &str) -> bool {
    if !fragment.is_fragment {
        return false;
    }
    let targets = parse_targets_json(&fragment.targets);
    targets.iter().any(|t| t == "*" || t == target_app)
}

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
}
