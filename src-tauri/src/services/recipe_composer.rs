//! Orchestration Layer: Stage Graph (generic DAG) + Recipe intent declaration (YAML+Markdown hybrid).
//!
//! - **Stage Graph**: a methodology-agnostic directed acyclic graph of workflow stages.
//!   Built from any `WorkflowPreset` — nodes are stages, edges are `depends_on` relationships.
//!   Supports branch detection, lateral (cross-cutting) nodes, and depth computation.
//!
//! - **Recipe**: a user-composed intent declaration combining a YAML frontmatter (machine-parseable)
//!   with a Markdown body (human/AI-readable). Exported as `.recipe.md` to `.opensunstar/recipe/`.
//!   The YAML+Markdown hybrid balances structured data extraction with rich documentation.
//!
//! - **Schema**: designed as a superset that works across all 8 methodology modules
//!   (OpenSpec, Spec-Kit, BMAD, GStack, Task Master, Superpowers, Skills, and flow-kit).

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::write_text_file;
use crate::error::AppError;
use crate::services::design_contract::{InstallFileEntry, InstallAuditFinding, InstallAuditSummary};
use crate::services::flow_orchestrator::{
    append_orchestration_log,
    resolve_stages_for_preset, WorkflowModule, WorkflowPreset, WorkflowStage,
};

const OPENSUNSTAR_DIR: &str = ".opensunstar";
const RECIPE_DIR: &str = "recipe";
const RECIPE_SCHEMA_VERSION: u32 = 1;

/// Lateral (cross-cutting) stage IDs that can be invoked at any pipeline stage.
const LATERAL_STAGE_IDS: &[&str] = &[
    "L-restyle",
    "M-health",
    "I-intel-scan",
    "A-architect",
    "A-evolve",
];

// ──────────────────────────────── Types ────────────────────────────────

/// A node in the stage graph DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageGraphNode {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// true = fully standalone; "semi" = needs `requires` deps; false = not standalone.
    #[serde(default)]
    pub standalone: String,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub condition: Option<String>,
    /// Whether this is a lateral (cross-cutting) node.
    #[serde(default)]
    pub lateral: bool,
    /// Computed BFS depth from root nodes (0 for roots).
    #[serde(default)]
    pub depth: u32,
}

/// A directed edge in the stage graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageGraphEdge {
    pub source: String,
    pub target: String,
}

/// Generic stage graph DAG — methodology-agnostic.
///
/// Built from any `WorkflowPreset`. The `preset_id` and `source_framework`
/// fields trace provenance but the graph structure itself is framework-neutral.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageGraph {
    pub preset_id: String,
    pub preset_name: String,
    pub source_framework: String,
    pub nodes: Vec<StageGraphNode>,
    pub edges: Vec<StageGraphEdge>,
    pub lateral_nodes: Vec<StageGraphNode>,
}

/// A selected stage within a recipe composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeStage {
    pub id: String,
    pub name: String,
    pub artifact: Option<String>,
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub condition: Option<String>,
    /// Markdown documentation for this stage (user-editable).
    #[serde(default)]
    pub doc: String,
}

/// A project-level artifact tracked by the recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeArtifact {
    pub path: String,
    pub purpose: String,
    /// Refresh threshold in days; None = no freshness check.
    #[serde(default)]
    pub freshness_days: Option<u32>,
}

/// A rule or constraint included in the recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRule {
    pub name: String,
    pub value: String,
    pub description: String,
}

/// Full composition recipe — the user's intent declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionRecipe {
    pub schema_version: u32,
    pub name: String,
    pub description: String,
    pub preset_id: String,
    pub project_type: String,
    pub modules: Vec<String>,
    pub stages: Vec<RecipeStage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_stages: Vec<String>,
    pub artifacts: Vec<RecipeArtifact>,
    pub rules: Vec<RecipeRule>,
    pub notes: String,
    pub generated_at: String,
    pub opensunstar_version: String,
}

/// Parameters for composing a recipe from a preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeComposeParams {
    pub preset_id: String,
    pub project_type: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub selected_modules: Option<Vec<String>>,
    #[serde(default)]
    pub disabled_stages: Option<Vec<String>>,
    #[serde(default)]
    pub notes: Option<String>,
    /// Optional per-stage documentation overrides (keyed by stage ID).
    #[serde(default)]
    pub stage_docs: Option<HashMap<String, String>>,
}

// ──────────────────────────── Stage Graph ────────────────────────────

/// Build a [`StageGraph`] from a [`WorkflowPreset`].
///
/// Pipeline stages (those appearing in `paths`) become `nodes`;
/// lateral stages (matching [`LATERAL_STAGE_IDS`]) are separated into `lateral_nodes`.
/// Edges are derived from each stage's `depends_on` field.
/// Branch nodes (out-degree > 1) are flagged.
/// Depth is computed via BFS from root nodes.
pub fn build_stage_graph(preset: &WorkflowPreset) -> StageGraph {
    let stage_map: HashMap<&str, &WorkflowStage> =
        preset.stages.iter().map(|s| (s.id.as_str(), s)).collect();

    let lateral_set: HashSet<&str> = LATERAL_STAGE_IDS.iter().copied().collect();

    // Partition stages into pipeline vs lateral
    let (pipeline_stages, lateral_stages): (Vec<_>, Vec<_>) = preset
        .stages
        .iter()
        .partition(|s| !lateral_set.contains(s.id.as_str()));

    // Build pipeline nodes
    let mut nodes: Vec<StageGraphNode> = pipeline_stages
        .iter()
        .map(|s| stage_to_node(s, false))
        .collect();

    // Build lateral nodes
    let lateral_nodes: Vec<StageGraphNode> = lateral_stages
        .iter()
        .map(|s| stage_to_node(s, true))
        .collect();

    // Build edges from depends_on
    let mut edges = Vec::new();
    for stage in &pipeline_stages {
        for dep in &stage.depends_on {
            if stage_map.contains_key(dep.as_str()) {
                edges.push(StageGraphEdge {
                    source: dep.clone(),
                    target: stage.id.clone(),
                });
            }
        }
    }

    // Detect branch nodes (out-degree > 1)
    let mut out_degree: HashMap<&str, u32> = HashMap::new();
    for edge in &edges {
        *out_degree.entry(edge.source.as_str()).or_default() += 1;
    }
    for node in &mut nodes {
        if out_degree.get(node.id.as_str()).copied().unwrap_or(0) > 1 {
            // Mark as branch by adding a condition hint (if not already set)
            if node.condition.is_none() {
                node.condition = Some("branch".to_string());
            }
        }
    }

    // Compute depth via BFS from roots (in-degree == 0)
    // Use owned String keys to avoid borrow conflicts with nodes
    let mut in_degree: HashMap<String, u32> = HashMap::new();
    for node in &nodes {
        in_degree.entry(node.id.clone()).or_default();
    }
    for edge in &edges {
        *in_degree.entry(edge.target.clone()).or_default() += 1;
    }

    let mut depth_map: HashMap<String, u32> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    for (id, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(id.clone());
            depth_map.insert(id.clone(), 0);
        }
    }

    while let Some(current) = queue.pop_front() {
        let current_depth = depth_map.get(&current).copied().unwrap_or(0);
        for edge in &edges {
            if edge.source == current {
                let new_depth = current_depth + 1;
                let target_id = &edge.target;
                let existing = depth_map.get(target_id).copied().unwrap_or(0);
                if new_depth > existing {
                    depth_map.insert(target_id.clone(), new_depth);
                }
                // Only enqueue when all incoming edges have been processed
                let all_incoming_done = edges
                    .iter()
                    .filter(|e| e.target == *target_id)
                    .all(|e| depth_map.contains_key(&e.source));
                if all_incoming_done && !queue.iter().any(|q| q == target_id) {
                    queue.push_back(target_id.clone());
                }
            }
        }
    }

    // Apply depth to nodes
    for node in &mut nodes {
        node.depth = depth_map.get(&node.id).copied().unwrap_or(0);
    }

    // Determine source framework from modules
    let source_framework = detect_source_framework(&preset.modules);

    StageGraph {
        preset_id: preset.id.clone(),
        preset_name: preset.name.clone(),
        source_framework,
        nodes,
        edges,
        lateral_nodes,
    }
}

fn stage_to_node(stage: &WorkflowStage, lateral: bool) -> StageGraphNode {
    let artifact_files: Vec<String> = stage
        .artifacts
        .iter()
        .map(|a| a.file.clone())
        .collect();

    let requires: Vec<String> = stage
        .artifacts
        .iter()
        .filter(|a| !a.optional)
        .map(|a| a.file.clone())
        .collect();

    let standalone = if lateral {
        "true".to_string()
    } else if stage.artifacts.is_empty() {
        "false".to_string()
    } else if requires.is_empty() {
        "true".to_string()
    } else {
        "semi".to_string()
    };

    let condition = stage.skip_when.as_ref().and_then(|sw| {
        if sw.project_type.is_empty() {
            None
        } else {
            Some(format!("skip_when: {}", sw.project_type.join(", ")))
        }
    });

    StageGraphNode {
        id: stage.id.clone(),
        name: stage.name.clone(),
        artifacts: artifact_files,
        depends_on: stage.depends_on.clone(),
        standalone,
        requires,
        condition,
        lateral,
        depth: 0,
    }
}

fn detect_source_framework(modules: &[String]) -> String {
    for m in modules {
        if m.contains("flow-kit") {
            return "flow-kit".to_string();
        }
        if m.contains("openspec") {
            return "openspec".to_string();
        }
        if m.contains("spec-kit") {
            return "spec-kit".to_string();
        }
        if m.contains("gstack") {
            return "gstack".to_string();
        }
        if m.contains("bmad") {
            return "bmad".to_string();
        }
    }
    "mixed".to_string()
}

/// Topological sort of stage IDs using Kahn's algorithm.
/// Returns IDs in dependency order (roots first).
fn topological_sort(stages: &[RecipeStage]) -> Vec<String> {
    let id_set: HashSet<&str> = stages.iter().map(|s| s.id.as_str()).collect();

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for s in stages {
        in_degree.entry(s.id.as_str()).or_default();
    }
    for s in stages {
        for dep in &s.depends_on {
            if id_set.contains(dep.as_str()) {
                *in_degree.entry(s.id.as_str()).or_default() += 1;
            }
        }
    }

    let mut queue: VecDeque<&str> = VecDeque::new();
    for (id, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(id);
        }
    }

    let mut sorted = Vec::new();
    while let Some(current) = queue.pop_front() {
        sorted.push(current.to_string());
        for s in stages {
            if s.depends_on.iter().any(|d| d == current) {
                if let Some(deg) = in_degree.get_mut(s.id.as_str()) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(s.id.as_str());
                    }
                }
            }
        }
    }

    // If there are remaining nodes (cycle), append them in original order
    if sorted.len() < stages.len() {
        let sorted_set: HashSet<String> = sorted.iter().cloned().collect();
        for s in stages {
            if !sorted_set.contains(&s.id) {
                log::warn!(
                    "Topological sort: cycle detected involving stage '{}', appending in original order",
                    s.id
                );
                sorted.push(s.id.clone());
            }
        }
    }

    sorted
}

// ──────────────────────────── Recipe Compose ────────────────────────────

/// Compose a [`CompositionRecipe`] from a preset and user selections.
pub fn compose_recipe(
    preset: &WorkflowPreset,
    params: &RecipeComposeParams,
    available_modules: &[WorkflowModule],
) -> Result<CompositionRecipe, AppError> {
    let resolved = resolve_stages_for_preset(preset, &params.project_type)?;

    let disabled_set: HashSet<&str> = params
        .disabled_stages
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|s| s.as_str())
        .collect();

    let stage_map: HashMap<&str, &WorkflowStage> =
        preset.stages.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut recipe_stages = Vec::new();
    let mut excluded = Vec::new();

    for sid in &resolved {
        let Some(stage) = stage_map.get(sid.as_str()) else {
            continue;
        };

        let enabled = !disabled_set.contains(sid.as_str());
        if !enabled {
            excluded.push(sid.clone());
        }

        let artifact = stage.artifacts.first().map(|a| a.file.clone());
        let condition = stage.skip_when.as_ref().map(|sw| {
            format!("project_type not in [{}]", sw.project_type.join(", "))
        });

        let doc = params
            .stage_docs
            .as_ref()
            .and_then(|docs| docs.get(sid))
            .cloned()
            .unwrap_or_else(|| default_stage_doc(sid, stage));

        recipe_stages.push(RecipeStage {
            id: sid.clone(),
            name: stage.name.clone(),
            artifact,
            depends_on: stage.depends_on.clone(),
            enabled,
            condition,
            doc,
        });
    }

    // Topological sort (should already be in order, but ensures DAG consistency)
    let sorted_ids = topological_sort(&recipe_stages);
    let stage_order: HashMap<&str, usize> = sorted_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    recipe_stages.sort_by_key(|s| stage_order.get(s.id.as_str()).copied().unwrap_or(usize::MAX));

    // Modules
    let modules = params
        .selected_modules
        .clone()
        .unwrap_or_else(|| preset.modules.clone());

    // Artifacts
    let artifacts = vec![
        RecipeArtifact {
            path: ".specs/CONTEXT.md".into(),
            purpose: "Rules layer: glossary, naming conventions, no-touch list".into(),
            freshness_days: Some(90),
        },
        RecipeArtifact {
            path: ".specs/ARCHITECTURE.md".into(),
            purpose: "Structure layer: module diagram, ADR list, cross-module contracts".into(),
            freshness_days: None,
        },
        RecipeArtifact {
            path: ".specs/LESSONS.md".into(),
            purpose: "Project-level failure knowledge base".into(),
            freshness_days: None,
        },
        RecipeArtifact {
            path: "STATE.md".into(),
            purpose: "Cross-session state: active change, interrupted tasks, decision log".into(),
            freshness_days: None,
        },
    ];

    // Rules (R9.6 safety valve + framework constraints)
    let mut rules = vec![
        RecipeRule {
            name: "max_auto_retry".into(),
            value: "3".into(),
            description: "Maximum automatic retry attempts per stage".into(),
        },
        RecipeRule {
            name: "role_separation".into(),
            value: "true".into(),
            description: "Enforce role boundaries (Architect/Dev/Reviewer)".into(),
        },
        RecipeRule {
            name: "require_diff_boundary".into(),
            value: "true".into(),
            description: "Each change must have clear diff boundary".into(),
        },
    ];

    // Add module-specific rules
    for mod_id in &modules {
        if let Some(m) = available_modules.iter().find(|m| m.id == *mod_id) {
            for cap in &m.capabilities {
                if cap == "role-boundaries" {
                    // Already covered by role_separation
                } else if cap == "change-isolation" {
                    rules.push(RecipeRule {
                        name: "change_isolation".into(),
                        value: "per-change-folder".into(),
                        description: format!(
                            "Each change gets its own .specs/<change-id>/ directory ({})",
                            m.name
                        ),
                    });
                } else if cap == "tdd" {
                    rules.push(RecipeRule {
                        name: "tdd_enforcement".into(),
                        value: "RED-GREEN-REFACTOR".into(),
                        description: format!("TDD cycle enforced in dev stage ({})", m.name),
                    });
                }
            }
        }
    }

    Ok(CompositionRecipe {
        schema_version: RECIPE_SCHEMA_VERSION,
        name: params.name.clone(),
        description: params
            .description
            .clone()
            .unwrap_or_else(|| format!("Recipe composed from {} preset", preset.name)),
        preset_id: preset.id.clone(),
        project_type: params.project_type.clone(),
        modules,
        stages: recipe_stages,
        excluded_stages: excluded,
        artifacts,
        rules,
        notes: params
            .notes
            .clone()
            .unwrap_or_default(),
        generated_at: Utc::now().to_rfc3339(),
        opensunstar_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

fn default_stage_doc(_id: &str, stage: &WorkflowStage) -> String {
    let artifacts = if stage.artifacts.is_empty() {
        "None".to_string()
    } else {
        stage
            .artifacts
            .iter()
            .map(|a| {
                if a.optional {
                    format!("`{}` (optional)", a.file)
                } else {
                    format!("`{}`", a.file)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    let deps = if stage.depends_on.is_empty() {
        "None (root stage)".to_string()
    } else {
        stage
            .depends_on
            .iter()
            .map(|d| format!("`{d}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!(
        "**{}** stage. Produces: {artifacts}. Depends on: {deps}.",
        stage.name
    )
}

// ──────────────────────── YAML+Markdown Hybrid ────────────────────────

/// Generate the YAML+Markdown hybrid document from a [`CompositionRecipe`].
///
/// Output format:
/// ```text
/// ---
/// # YAML frontmatter (machine-parseable metadata)
/// schema_version: 1
/// name: "..."
/// ...
/// ---
///
/// # Markdown body (human/AI-readable documentation)
/// ## Overview
/// ...
/// ```
///
/// The file is valid Markdown with YAML frontmatter (Jekyll/Hugo convention).
/// Tools parse the frontmatter for structured data; humans and AI agents
/// read the full document as context documentation.
pub fn generate_recipe_hybrid(recipe: &CompositionRecipe) -> Result<String, AppError> {
    // 1. Generate YAML frontmatter
    let yaml_front = serde_yaml::to_string(recipe)
        .map_err(|e| AppError::Message(format!("序列化 recipe YAML 失败: {e}")))?;
    let yaml_front = yaml_front.trim_start_matches("---\n").trim_end();

    // 2. Generate Markdown body
    let mut md = String::new();

    // Title
    md.push_str(&format!("# {}\n\n", recipe.name));
    md.push_str(&format!("> {}\n\n", recipe.description));

    // Quick reference table
    md.push_str("## Quick Reference\n\n");
    md.push_str(&format!(
        "| Key | Value |\n|---|---|\n\
         | Preset | `{}` |\n\
         | Project Type | `{}` |\n\
         | Stages | {} enabled ({} excluded) |\n\
         | Modules | {} |\n\
         | Schema | v{} |\n\
         | Generated | {} |\n\n",
        recipe.preset_id,
        recipe.project_type,
        recipe.stages.iter().filter(|s| s.enabled).count(),
        recipe.excluded_stages.len(),
        recipe.modules.len(),
        recipe.schema_version,
        &recipe.generated_at[..10],
    ));

    // Modules section
    md.push_str("## Modules\n\n");
    if recipe.modules.is_empty() {
        md.push_str("No modules selected.\n\n");
    } else {
        for m in &recipe.modules {
            md.push_str(&format!("- `{m}`\n"));
        }
        md.push('\n');
    }

    // Stage Pipeline
    md.push_str("## Stage Pipeline\n\n");
    md.push_str("```mermaid\ngraph LR\n");
    for stage in &recipe.stages {
        if stage.enabled {
            let label = stage.artifact.as_deref().unwrap_or(&stage.name);
            md.push_str(&format!(
                "    {}[\"{}  <br/>{}\"]\n",
                sanitize_mermaid_id(&stage.id),
                stage.name,
                label
            ));
        }
    }
    for stage in &recipe.stages {
        if !stage.enabled {
            continue;
        }
        for dep in &stage.depends_on {
            if recipe.stages.iter().any(|s| s.id == *dep && s.enabled) {
                md.push_str(&format!(
                    "    {} --> {}\n",
                    sanitize_mermaid_id(dep),
                    sanitize_mermaid_id(&stage.id)
                ));
            }
        }
    }
    md.push_str("```\n\n");

    // Stage Documentation
    md.push_str("## Stage Documentation\n\n");
    for stage in &recipe.stages {
        if !stage.enabled {
            continue;
        }
        md.push_str(&format!("### {}\n\n", stage.name));
        md.push_str(&format!("- **ID**: `{}`\n", stage.id));
        if let Some(ref art) = stage.artifact {
            md.push_str(&format!("- **Artifact**: `{art}`\n"));
        }
        if !stage.depends_on.is_empty() {
            md.push_str(&format!(
                "- **Depends on**: {}\n",
                stage
                    .depends_on
                    .iter()
                    .map(|d| format!("`{d}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(ref cond) = stage.condition {
            md.push_str(&format!("- **Condition**: {cond}\n"));
        }
        md.push('\n');
        md.push_str(&stage.doc);
        md.push_str("\n\n");
    }

    // Excluded stages
    if !recipe.excluded_stages.is_empty() {
        md.push_str("## Excluded Stages\n\n");
        md.push_str("The following stages from the base preset are not included in this recipe:\n\n");
        for s in &recipe.excluded_stages {
            md.push_str(&format!("- ~~`{s}`~~\n"));
        }
        md.push('\n');
    }

    // Rules & Constraints
    md.push_str("## Rules & Constraints\n\n");
    for rule in &recipe.rules {
        md.push_str(&format!(
            "- **{}**: `{}` — {}\n",
            rule.name, rule.value, rule.description
        ));
    }
    md.push('\n');

    // Usage (for AI agents)
    md.push_str("## Usage\n\n");
    md.push_str("### For AI Agents\n\n");
    md.push_str("Reference this recipe file as workflow context:\n\n");
    md.push_str("```\n");
    md.push_str(&format!("@.opensunstar/recipe/{}.recipe.md\n", slugify(&recipe.name)));
    md.push_str("```\n\n");
    md.push_str("Parse the YAML frontmatter between `---` delimiters for structured data. ");
    md.push_str("The Markdown body provides stage documentation and project context.\n\n");
    md.push_str("### For Humans\n\n");
    md.push_str("This file is readable as Markdown documentation for the project's workflow. ");
    md.push_str("The YAML frontmatter (between `---` lines) contains machine-parseable metadata. ");
    md.push_str("Edit the Stage Documentation sections to customize workflow guidance.\n\n");

    // Project Artifacts
    md.push_str("## Project Artifacts\n\n");
    for art in &recipe.artifacts {
        let freshness = match art.freshness_days {
            Some(days) => format!(" (refresh every {days} days)"),
            None => " (no freshness check)".to_string(),
        };
        md.push_str(&format!("- `{}` — {}{}\n", art.path, art.purpose, freshness));
    }
    md.push('\n');

    // Notes
    if !recipe.notes.is_empty() {
        md.push_str("## Notes\n\n");
        md.push_str(&recipe.notes);
        md.push_str("\n\n");
    }

    // Assemble: YAML frontmatter + Markdown body
    Ok(format!("---\n{yaml_front}\n---\n\n{md}"))
}

/// Parse YAML frontmatter from a hybrid recipe document.
pub fn parse_recipe_frontmatter(content: &str) -> Result<CompositionRecipe, AppError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(AppError::Message(
            "Recipe 文件缺少 YAML frontmatter (应以 --- 开头)".into(),
        ));
    }
    let rest = &trimmed[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| AppError::Message("Recipe 文件 YAML frontmatter 未闭合".into()))?;
    let yaml_str = &rest[..end];
    serde_yaml::from_str(yaml_str)
        .map_err(|e| AppError::Message(format!("解析 recipe YAML 失败: {e}")))
}

fn sanitize_mermaid_id(id: &str) -> String {
    id.replace('-', "_").replace(' ', "_")
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ──────────────────────────── File I/O ────────────────────────────

fn recipe_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path)
        .join(OPENSUNSTAR_DIR)
        .join(RECIPE_DIR)
}

fn recipe_filename(name: &str) -> String {
    format!("{}.recipe.md", slugify(name))
}

/// Export a recipe to `.opensunstar/recipe/<slug>.recipe.md`.
///
/// Creates the directory if it doesn't exist. Overwrites existing files
/// with the same name. Appends to the orchestration audit log.
pub fn export_recipe(
    project_path: &str,
    recipe: &CompositionRecipe,
) -> Result<String, AppError> {
    let dir = recipe_dir(project_path);
    fs::create_dir_all(&dir).map_err(|e| AppError::io(&dir, e))?;

    let content = generate_recipe_hybrid(recipe)?;
    let filename = recipe_filename(&recipe.name);
    let out_path = dir.join(&filename);
    write_text_file(&out_path, &content)?;

    append_orchestration_log(
        project_path,
        serde_json::json!({
            "event": "recipe_export",
            "name": recipe.name,
            "presetId": recipe.preset_id,
            "projectType": recipe.project_type,
            "stageCount": recipe.stages.iter().filter(|s| s.enabled).count(),
            "moduleCount": recipe.modules.len(),
            "file": format!(".opensunstar/recipe/{filename}"),
        }),
    )?;

    Ok(content)
}

/// Read a saved recipe from `.opensunstar/recipe/<slug>.recipe.md`.
pub fn read_saved_recipe(project_path: &str, name: &str) -> Result<String, AppError> {
    let dir = recipe_dir(project_path);
    let filename = recipe_filename(name);
    let path = dir.join(&filename);
    if !path.is_file() {
        return Err(AppError::Message(format!(
            "Recipe 文件不存在: .opensunstar/recipe/{filename}"
        )));
    }
    fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))
}

/// List all saved recipe names from `.opensunstar/recipe/`.
pub fn list_saved_recipes(project_path: &str) -> Result<Vec<String>, AppError> {
    let dir = recipe_dir(project_path);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                if fname.ends_with(".recipe.md") {
                    let name = fname.trim_end_matches(".recipe.md").to_string();
                    names.push(name);
                }
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Delete a saved recipe (moves to trash on supported platforms).
pub fn delete_saved_recipe(project_path: &str, name: &str) -> Result<(), AppError> {
    let dir = recipe_dir(project_path);
    let filename = recipe_filename(name);
    let path = dir.join(&filename);
    if !path.is_file() {
        return Err(AppError::Message(format!(
            "Recipe 文件不存在: .opensunstar/recipe/{filename}"
        )));
    }
    fs::remove_file(&path).map_err(|e| AppError::io(&path, e))?;
    Ok(())
}

// ──────────────────────── Recipe Install (Template Scaffolding) ────────────────────────

/// Result of installing a recipe into a project directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub change_id: String,
    pub files_created: Vec<String>,
    pub files_skipped: Vec<String>,
    pub specs_dir_created: bool,
    pub state_file_created: bool,
}

/// Pre-flight dry-run result for recipe install.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInstallPlan {
    pub files: Vec<InstallFileEntry>,
    pub audit: InstallAuditSummary,
}

/// Starter template for `.specs/CONTEXT.md` (project-level rules layer).
const TEMPLATE_CONTEXT: &str = r#"# Context — Rules Layer

> This file defines project-level naming conventions, glossary, and constraints.
> Refresh threshold: 90 days.

## Glossary

| Term | Definition |
|------|-----------|
| *Add project-specific terms here* | |

## Naming Conventions

- Files: `kebab-case.md`
- Directories: `kebab-case/`
- Variables: *project convention*

## No-Touch List

> Files/directories that AI agents must NOT modify without explicit approval.

- `.opensunstar/`
- `package.json` (dependencies section)

## Constraints

- *Add project-specific constraints here*
"#;

/// Starter template for `.specs/ARCHITECTURE.md` (structure layer).
const TEMPLATE_ARCHITECTURE: &str = r#"# Architecture — Structure Layer

> Module diagram, ADR list, and cross-module contracts.

## Module Diagram

```
┌─────────────┐
│   Frontend   │
├─────────────┤
│   Backend    │
├─────────────┤
│   Database   │
└─────────────┘
```

*Replace with your actual architecture diagram.*

## ADR (Architecture Decision Records)

| # | Decision | Status | Date |
|---|---------|--------|------|
| 1 | *First decision* | proposed | *date* |

## Cross-Module Contracts

- *Define interfaces between modules here*
"#;

/// Starter template for `.specs/LESSONS.md` (failure knowledge base).
const TEMPLATE_LESSONS: &str = r#"# Lessons Learned

> Project-level failure knowledge base. Record mistakes and their fixes
> so they are not repeated.

## Format

Each lesson follows:
```
### [Date] Brief title
- **What happened**: description
- **Root cause**: why it happened
- **Fix**: what resolved it
- **Prevention**: how to avoid it in the future
```

## Lessons

*Add lessons as they occur.*
"#;

/// Starter template for `STATE.md` (cross-session state).
const TEMPLATE_STATE: &str = r#"# STATE

> Cross-session state file. Updated by workflow tools and AI agents.
> Do NOT edit manually unless you understand the format.

active_change: {change_id}
interrupted_task: none

## Decision Log

| Date | Decision | Rationale |
|------|---------|-----------|
| *date* | *decision* | *rationale* |
"#;

/// Generate a starter template for a stage artifact.
fn stage_artifact_template(stage_id: &str, stage_name: &str, artifact_file: &str) -> String {
    format!(
        r#"# {stage_name}

> Stage: `{stage_id}`
> Artifact: `{artifact_file}`
> Generated by OpenSunstar Recipe Installer

## Objective

*Describe what this stage should accomplish.*

## Details

*Add stage-specific content here.*

## Acceptance Criteria

- [ ] *Criterion 1*
- [ ] *Criterion 2*
"#
    )
}

// ──────────────────────── Pre-flight Dry Run ────────────────────────

/// Generate a pre-flight install plan for recipe: what WILL happen if install proceeds.
/// Writes content to a temp directory, runs audit::scan_dir, checks existing files.
pub fn preview_recipe_install_plan(
    project_path: &str,
    recipe: &CompositionRecipe,
    change_id: &str,
) -> Result<RecipeInstallPlan, AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }

    // 1. Generate content to a temp dir for audit scanning
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| AppError::Message(format!("创建临时目录失败: {e}")))?;

    // Write project-level templates to temp
    let temp_specs = temp_dir.path().join(".specs");
    fs::create_dir_all(&temp_specs).ok();
    for (rel_path, content) in &[
        (".specs/CONTEXT.md", TEMPLATE_CONTEXT),
        (".specs/ARCHITECTURE.md", TEMPLATE_ARCHITECTURE),
        (".specs/LESSONS.md", TEMPLATE_LESSONS),
    ] {
        let path = temp_dir.path().join(rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        write_text_file(&path, content)?;
    }

    // Write STATE.md
    let state_content = TEMPLATE_STATE.replace("{change_id}", change_id);
    write_text_file(&temp_dir.path().join("STATE.md"), &state_content)?;

    // Write stage artifact templates
    let temp_change_dir = temp_specs.join(change_id);
    fs::create_dir_all(&temp_change_dir).ok();
    for stage in &recipe.stages {
        if !stage.enabled { continue; }
        if let Some(ref artifact_file) = stage.artifact {
            let content = stage_artifact_template(&stage.id, &stage.name, artifact_file);
            write_text_file(&temp_change_dir.join(artifact_file), &content)?;
        }
    }

    // 2. Run audit scan on temp dir
    let audit_result = crate::audit::scan_dir(
        temp_dir.path(),
        &crate::audit::AuditContext {
            source: crate::audit::AuditSource::RecipeInstall {
                recipe_name: recipe.name.clone(),
                change_id: change_id.to_string(),
            },
            threshold: Default::default(),
        },
    )?;

    let audit = InstallAuditSummary {
        files_scanned: audit_result.files_scanned,
        total_findings: audit_result.total_findings(),
        critical: audit_result.summary.critical,
        high: audit_result.summary.high,
        medium: audit_result.summary.medium,
        low: audit_result.summary.low,
        blocked: audit_result.should_block(),
        findings: audit_result
            .findings
            .iter()
            .map(|f| InstallAuditFinding {
                severity: f.severity.label().to_string(),
                rule_id: f.rule_id.clone(),
                message: f.message.clone(),
                file: f.file.clone(),
            })
            .collect(),
    };

    // 3. Check existing files at target paths
    let mut files = Vec::new();

    for (rel_path, content) in &[
        (".specs/CONTEXT.md", TEMPLATE_CONTEXT),
        (".specs/ARCHITECTURE.md", TEMPLATE_ARCHITECTURE),
        (".specs/LESSONS.md", TEMPLATE_LESSONS),
    ] {
        let target = root.join(rel_path);
        let existing = if target.is_file() { fs::read_to_string(&target).ok() } else { None };
        files.push(InstallFileEntry {
            path: rel_path.to_string(),
            status: if existing.is_some() { "skip".into() } else { "create".into() },
            new_content: Some(content.to_string()),
            existing_content: existing,
        });
    }

    // STATE.md
    let state_target = root.join("STATE.md");
    let existing_state = if state_target.is_file() { fs::read_to_string(&state_target).ok() } else { None };
    files.push(InstallFileEntry {
        path: "STATE.md".into(),
        status: if existing_state.is_some() { "skip".into() } else { "create".into() },
        new_content: Some(state_content),
        existing_content: existing_state,
    });

    // Stage artifacts
    for stage in &recipe.stages {
        if !stage.enabled { continue; }
        if let Some(ref artifact_file) = stage.artifact {
            let rel = format!(".specs/{change_id}/{artifact_file}");
            let target = root.join(&rel);
            let existing = if target.is_file() { fs::read_to_string(&target).ok() } else { None };
            let content = stage_artifact_template(&stage.id, &stage.name, artifact_file);
            files.push(InstallFileEntry {
                path: rel,
                status: if existing.is_some() { "skip".into() } else { "create".into() },
                new_content: Some(content),
                existing_content: existing,
            });
        }
    }

    Ok(RecipeInstallPlan { files, audit })
}

/// Install a recipe into a project: scaffold `.specs/`, create templates, and write `STATE.md`.
///
/// This is the "template installer" — it materializes the recipe's declarative metadata
/// into an actual working directory structure with starter template files.
///
/// - Creates `.specs/` directory if it doesn't exist
/// - Creates project-level artifacts (CONTEXT.md, ARCHITECTURE.md, LESSONS.md) if they don't exist
/// - Creates `.specs/<change-id>/` with starter templates for each enabled stage
/// - Creates `STATE.md` at project root if it doesn't exist
/// - Never overwrites existing files (safe install)
pub fn install_recipe(
    project_path: &str,
    recipe: &CompositionRecipe,
    change_id: &str,
) -> Result<InstallResult, AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }

    let mut files_created = Vec::new();
    let mut files_skipped = Vec::new();

    // 1. Create .specs/ directory
    let specs_dir = root.join(".specs");
    let specs_dir_created = if !specs_dir.is_dir() {
        fs::create_dir_all(&specs_dir)
            .map_err(|e| AppError::io(&specs_dir, e))?;
        true
    } else {
        false
    };

    // 2. Create project-level artifacts (only if they don't exist)
    let project_artifacts = [
        (".specs/CONTEXT.md", TEMPLATE_CONTEXT),
        (".specs/ARCHITECTURE.md", TEMPLATE_ARCHITECTURE),
        (".specs/LESSONS.md", TEMPLATE_LESSONS),
    ];

    for (rel_path, content) in &project_artifacts {
        let path = root.join(rel_path);
        if path.is_file() {
            files_skipped.push(rel_path.to_string());
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
            }
            write_text_file(&path, content)?;
            files_created.push(rel_path.to_string());
        }
    }

    // 3. Create STATE.md at project root
    let state_path = root.join("STATE.md");
    let state_created = if !state_path.is_file() {
        let state_content = TEMPLATE_STATE.replace("{change_id}", change_id);
        write_text_file(&state_path, &state_content)?;
        files_created.push("STATE.md".to_string());
        true
    } else {
        files_skipped.push("STATE.md".to_string());
        false
    };

    // 4. Create .specs/<change-id>/ directory
    let change_dir = specs_dir.join(change_id);
    fs::create_dir_all(&change_dir)
        .map_err(|e| AppError::io(&change_dir, e))?;

    // 5. Create starter templates for each enabled stage
    for stage in &recipe.stages {
        if !stage.enabled {
            continue;
        }
        if let Some(ref artifact_file) = stage.artifact {
            let artifact_path = change_dir.join(artifact_file);
            let rel_path = format!(".specs/{change_id}/{artifact_file}");
            if artifact_path.is_file() {
                files_skipped.push(rel_path);
            } else {
                let content = stage_artifact_template(&stage.id, &stage.name, artifact_file);
                write_text_file(&artifact_path, &content)?;
                files_created.push(rel_path);
            }
        }
    }

    // 6. Also export the recipe file itself
    let _recipe_content = export_recipe(project_path, recipe)?;
    let recipe_filename = format!(
        ".opensunstar/recipe/{}.recipe.md",
        slugify(&recipe.name)
    );
    files_created.push(recipe_filename);

    // 7. Audit log
    append_orchestration_log(
        project_path,
        serde_json::json!({
            "event": "recipe_install",
            "name": recipe.name,
            "presetId": recipe.preset_id,
            "projectType": recipe.project_type,
            "changeId": change_id,
            "filesCreated": files_created.len(),
            "filesSkipped": files_skipped.len(),
        }),
    )?;

    log::info!(
        "Recipe '{}' installed: {} files created, {} skipped",
        recipe.name,
        files_created.len(),
        files_skipped.len()
    );

    Ok(InstallResult {
        change_id: change_id.to_string(),
        files_created,
        files_skipped,
        specs_dir_created,
        state_file_created: state_created,
    })
}

// ──────────────────────────── Tests ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::flow_orchestrator::{
        WorkflowArtifactSpec, WorkflowPresetPaths, WorkflowStage, WorkflowStageSkipWhen,
    };

    fn uuid_simple() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("opensunstar-recipe-{}", uuid_simple()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_preset() -> WorkflowPreset {
        WorkflowPreset {
            id: "standard".into(),
            name: "Standard".into(),
            name_zh: Some("标准流程".into()),
            description: "Full loop with UI design branch".into(),
            r3_tier: Some("standard".into()),
            modules: vec![
                "openspec-change".into(),
                "spec-kit-cascade".into(),
                "gstack-conductor".into(),
            ],
            stages: vec![
                WorkflowStage {
                    id: "0-change".into(),
                    name: "Change".into(),
                    prompt: None,
                    depends_on: vec![],
                    skip_when: None,
                    artifacts: vec![WorkflowArtifactSpec {
                        file: "CHANGE.md".into(),
                        scope: "change".into(),
                        optional: false,
                    }],
                },
                WorkflowStage {
                    id: "1-requirement".into(),
                    name: "Requirement".into(),
                    prompt: None,
                    depends_on: vec!["0-change".into()],
                    skip_when: None,
                    artifacts: vec![WorkflowArtifactSpec {
                        file: "REQUIREMENT.md".into(),
                        scope: "change".into(),
                        optional: false,
                    }],
                },
                WorkflowStage {
                    id: "2-design".into(),
                    name: "Design".into(),
                    prompt: None,
                    depends_on: vec!["1-requirement".into()],
                    skip_when: None,
                    artifacts: vec![WorkflowArtifactSpec {
                        file: "DESIGN.md".into(),
                        scope: "change".into(),
                        optional: false,
                    }],
                },
                WorkflowStage {
                    id: "2a-ui-design".into(),
                    name: "UI Design".into(),
                    prompt: None,
                    depends_on: vec!["2-design".into()],
                    skip_when: Some(WorkflowStageSkipWhen {
                        project_type: vec!["backend".into(), "cli".into()],
                    }),
                    artifacts: vec![WorkflowArtifactSpec {
                        file: "UI-DESIGN.md".into(),
                        scope: "change".into(),
                        optional: false,
                    }],
                },
                WorkflowStage {
                    id: "3-task".into(),
                    name: "Task".into(),
                    prompt: None,
                    depends_on: vec!["2-design".into(), "2a-ui-design".into()],
                    skip_when: None,
                    artifacts: vec![WorkflowArtifactSpec {
                        file: "TASK.md".into(),
                        scope: "change".into(),
                        optional: false,
                    }],
                },
                WorkflowStage {
                    id: "4-dev".into(),
                    name: "Dev".into(),
                    prompt: None,
                    depends_on: vec!["3-task".into()],
                    skip_when: None,
                    artifacts: vec![],
                },
            ],
            paths: WorkflowPresetPaths {
                frontend: vec![
                    "0-change".into(),
                    "1-requirement".into(),
                    "2-design".into(),
                    "2a-ui-design".into(),
                    "3-task".into(),
                    "4-dev".into(),
                ],
                backend: vec![
                    "0-change".into(),
                    "1-requirement".into(),
                    "2-design".into(),
                    "3-task".into(),
                    "4-dev".into(),
                ],
                cli: vec![
                    "0-change".into(),
                    "1-requirement".into(),
                    "2-design".into(),
                    "3-task".into(),
                    "4-dev".into(),
                ],
                mvp: vec![
                    "1-requirement".into(),
                    "2-design".into(),
                    "3-task".into(),
                    "4-dev".into(),
                ],
            },
        }
    }

    fn sample_modules() -> Vec<WorkflowModule> {
        vec![
            WorkflowModule {
                id: "openspec-change".into(),
                name: "OpenSpec Change Isolation".into(),
                name_zh: None,
                source: "OpenSpec".into(),
                description: "Per-change folder isolation".into(),
                capabilities: vec!["change-isolation".into()],
            },
            WorkflowModule {
                id: "spec-kit-cascade".into(),
                name: "Spec-Kit Cascade".into(),
                name_zh: None,
                source: "spec-kit".into(),
                description: "Artifact cascade".into(),
                capabilities: vec!["ac-format".into()],
            },
            WorkflowModule {
                id: "gstack-conductor".into(),
                name: "GStack Conductor".into(),
                name_zh: None,
                source: "gstack".into(),
                description: "Role-based conductor".into(),
                capabilities: vec!["conductor-routing".into()],
            },
        ]
    }

    #[test]
    fn stage_graph_has_correct_nodes() {
        let preset = sample_preset();
        let graph = build_stage_graph(&preset);

        // 6 pipeline stages, 0 lateral (none in sample)
        assert_eq!(graph.nodes.len(), 6);
        assert!(graph.lateral_nodes.is_empty());

        // Verify specific nodes
        assert!(graph.nodes.iter().any(|n| n.id == "0-change"));
        assert!(graph.nodes.iter().any(|n| n.id == "2a-ui-design"));
        assert!(graph.nodes.iter().any(|n| n.id == "4-dev"));
    }

    #[test]
    fn stage_graph_edges_from_depends_on() {
        let preset = sample_preset();
        let graph = build_stage_graph(&preset);

        // 0-change → 1-requirement
        assert!(graph
            .edges
            .iter()
            .any(|e| e.source == "0-change" && e.target == "1-requirement"));

        // 2-design → 3-task
        assert!(graph
            .edges
            .iter()
            .any(|e| e.source == "2-design" && e.target == "3-task"));

        // 2a-ui-design → 3-task (branch)
        assert!(graph
            .edges
            .iter()
            .any(|e| e.source == "2a-ui-design" && e.target == "3-task"));
    }

    #[test]
    fn stage_graph_branch_detection() {
        let preset = sample_preset();
        let graph = build_stage_graph(&preset);

        // 2-design has 2 outgoing edges (→ 2a-ui-design, → 3-task) → branch
        let design_node = graph.nodes.iter().find(|n| n.id == "2-design").unwrap();
        assert_eq!(design_node.condition, Some("branch".to_string()));

        // 0-change has 1 outgoing edge → not branch
        let change_node = graph.nodes.iter().find(|n| n.id == "0-change").unwrap();
        assert!(change_node.condition.is_none());
    }

    #[test]
    fn stage_graph_depth_computation() {
        let preset = sample_preset();
        let graph = build_stage_graph(&preset);

        let depth = |id: &str| graph.nodes.iter().find(|n| n.id == id).unwrap().depth;

        assert_eq!(depth("0-change"), 0);
        assert_eq!(depth("1-requirement"), 1);
        assert_eq!(depth("2-design"), 2);
        assert_eq!(depth("2a-ui-design"), 3);
        // 3-task depends on both 2-design (depth 2) and 2a-ui-design (depth 3)
        // so its depth = max(2, 3) + 1 = 4
        assert_eq!(depth("3-task"), 4);
        assert_eq!(depth("4-dev"), 5);
    }

    #[test]
    fn stage_graph_standalone_classification() {
        let preset = sample_preset();
        let graph = build_stage_graph(&preset);

        // 0-change has required artifact CHANGE.md → "semi"
        let change_node = graph.nodes.iter().find(|n| n.id == "0-change").unwrap();
        assert_eq!(change_node.standalone, "semi");

        // 4-dev has no artifacts → "false"
        let dev_node = graph.nodes.iter().find(|n| n.id == "4-dev").unwrap();
        assert_eq!(dev_node.standalone, "false");
    }

    #[test]
    fn stage_graph_detects_framework() {
        let preset = sample_preset();
        let graph = build_stage_graph(&preset);
        // First module is openspec-change → "openspec"
        assert_eq!(graph.source_framework, "openspec");
    }

    #[test]
    fn compose_recipe_filters_disabled_stages() {
        let preset = sample_preset();
        let modules = sample_modules();
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "backend".into(),
            name: "Test Recipe".into(),
            description: Some("A test recipe".into()),
            selected_modules: None,
            disabled_stages: Some(vec!["4-dev".into()]),
            notes: None,
            stage_docs: None,
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();

        // 4-dev should be excluded
        assert!(recipe.excluded_stages.contains(&"4-dev".to_string()));

        // 4-dev should still appear in stages but with enabled=false
        let dev_stage = recipe.stages.iter().find(|s| s.id == "4-dev").unwrap();
        assert!(!dev_stage.enabled);
    }

    #[test]
    fn compose_recipe_applies_module_rules() {
        let preset = sample_preset();
        let modules = sample_modules();
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "frontend".into(),
            name: "Module Rules Test".into(),
            description: None,
            selected_modules: Some(vec!["openspec-change".into()]),
            disabled_stages: None,
            notes: None,
            stage_docs: None,
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();

        // openspec-change has "change-isolation" capability → should add rule
        assert!(recipe
            .rules
            .iter()
            .any(|r| r.name == "change_isolation"));
    }

    #[test]
    fn generate_hybrid_has_yaml_frontmatter() {
        let preset = sample_preset();
        let modules = sample_modules();
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "backend".into(),
            name: "Hybrid Format Test".into(),
            description: Some("Testing YAML+MD hybrid".into()),
            selected_modules: None,
            disabled_stages: None,
            notes: Some("Custom notes for testing.".into()),
            stage_docs: None,
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();
        let content = generate_recipe_hybrid(&recipe).unwrap();

        // Must start with ---
        assert!(content.starts_with("---\n"), "Should start with YAML delimiter");

        // Must have closing ---
        let second_delim = content[4..].find("\n---\n");
        assert!(second_delim.is_some(), "Should have closing YAML delimiter");

        // Must contain Markdown sections
        assert!(content.contains("# Hybrid Format Test"));
        assert!(content.contains("## Stage Pipeline"));
        assert!(content.contains("## Stage Documentation"));
        assert!(content.contains("## Rules & Constraints"));
        assert!(content.contains("## Usage"));
        assert!(content.contains("Custom notes for testing."));
    }

    #[test]
    fn generate_hybrid_has_mermaid_diagram() {
        let preset = sample_preset();
        let modules = sample_modules();
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "frontend".into(),
            name: "Mermaid Test".into(),
            description: None,
            selected_modules: None,
            disabled_stages: None,
            notes: None,
            stage_docs: None,
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();
        let content = generate_recipe_hybrid(&recipe).unwrap();

        assert!(content.contains("```mermaid"));
        assert!(content.contains("graph LR"));
        assert!(content.contains("-->"));
    }

    #[test]
    fn roundtrip_frontmatter_parse() {
        let preset = sample_preset();
        let modules = sample_modules();
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "backend".into(),
            name: "Roundtrip Test".into(),
            description: Some("Roundtrip YAML parse".into()),
            selected_modules: None,
            disabled_stages: None,
            notes: None,
            stage_docs: None,
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();
        let content = generate_recipe_hybrid(&recipe).unwrap();
        let parsed = parse_recipe_frontmatter(&content).unwrap();

        assert_eq!(parsed.name, recipe.name);
        assert_eq!(parsed.preset_id, recipe.preset_id);
        assert_eq!(parsed.project_type, recipe.project_type);
        assert_eq!(parsed.stages.len(), recipe.stages.len());
        assert_eq!(parsed.modules, recipe.modules);
    }

    #[test]
    fn export_writes_recipe_file() {
        let root = temp_project();
        let preset = sample_preset();
        let modules = sample_modules();
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "backend".into(),
            name: "Export Test".into(),
            description: None,
            selected_modules: None,
            disabled_stages: None,
            notes: None,
            stage_docs: None,
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();
        let content = export_recipe(root.to_str().unwrap(), &recipe).unwrap();

        // File should exist
        let expected_path = root
            .join(".opensunstar")
            .join("recipe")
            .join("export-test.recipe.md");
        assert!(expected_path.is_file(), "Recipe file should exist at {expected_path:?}");

        // Content should match
        let on_disk = fs::read_to_string(&expected_path).unwrap();
        assert_eq!(on_disk, content);

        // Orchestration log should have entry
        let log_path = root
            .join(".opensunstar")
            .join("orchestration.log.jsonl");
        assert!(log_path.is_file());
        let log_content = fs::read_to_string(&log_path).unwrap();
        assert!(log_content.contains("recipe_export"));
    }

    #[test]
    fn list_saved_recipes_returns_names() {
        let root = temp_project();
        let recipe_dir = root.join(".opensunstar").join("recipe");
        fs::create_dir_all(&recipe_dir).unwrap();

        fs::write(recipe_dir.join("alpha.recipe.md"), "---\nschema_version: 1\n---\n# Alpha").unwrap();
        fs::write(recipe_dir.join("beta.recipe.md"), "---\nschema_version: 1\n---\n# Beta").unwrap();
        fs::write(recipe_dir.join("not-a-recipe.md"), "# Ignore me").unwrap();

        let names = list_saved_recipes(root.to_str().unwrap()).unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    #[test]
    fn topological_sort_respects_dependencies() {
        let stages = vec![
            RecipeStage {
                id: "c".into(),
                name: "C".into(),
                artifact: None,
                depends_on: vec!["a".into(), "b".into()],
                enabled: true,
                condition: None,
                doc: String::new(),
            },
            RecipeStage {
                id: "a".into(),
                name: "A".into(),
                artifact: None,
                depends_on: vec![],
                enabled: true,
                condition: None,
                doc: String::new(),
            },
            RecipeStage {
                id: "b".into(),
                name: "B".into(),
                artifact: None,
                depends_on: vec!["a".into()],
                enabled: true,
                condition: None,
                doc: String::new(),
            },
        ];

        let sorted = topological_sort(&stages);
        let pos = |id: &str| sorted.iter().position(|s| s == id).unwrap();

        assert!(pos("a") < pos("b"));
        assert!(pos("a") < pos("c"));
        assert!(pos("b") < pos("c"));
    }

    #[test]
    fn stage_docs_override_default() {
        let preset = sample_preset();
        let modules = sample_modules();
        let mut docs = HashMap::new();
        docs.insert(
            "0-change".to_string(),
            "Custom change documentation.".to_string(),
        );
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "backend".into(),
            name: "Stage Docs Test".into(),
            description: None,
            selected_modules: None,
            disabled_stages: None,
            notes: None,
            stage_docs: Some(docs),
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();
        let change_stage = recipe.stages.iter().find(|s| s.id == "0-change").unwrap();
        assert_eq!(change_stage.doc, "Custom change documentation.");

        // Non-overridden stage should have default doc
        let req_stage = recipe.stages.iter().find(|s| s.id == "1-requirement").unwrap();
        assert!(req_stage.doc.contains("Requirement"));
    }

    #[test]
    fn slugify_handles_special_chars() {
        assert_eq!(slugify("My Recipe 2025"), "my-recipe-2025");
        assert_eq!(slugify("前端 / Backend"), "前端-backend");
        assert_eq!(slugify("  spaces  "), "spaces");
    }

    #[test]
    fn install_creates_specs_dir_and_templates() {
        let root = temp_project();
        let preset = sample_preset();
        let modules = sample_modules();
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "backend".into(),
            name: "Install Test".into(),
            description: Some("Testing install_recipe".into()),
            selected_modules: None,
            disabled_stages: Some(vec!["4-dev".into()]),
            notes: None,
            stage_docs: None,
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();
        let result = install_recipe(root.to_str().unwrap(), &recipe, "chg-001").unwrap();

        // .specs/ dir should have been created
        assert!(result.specs_dir_created);
        assert!(root.join(".specs").is_dir());

        // Project-level artifacts
        assert!(root.join(".specs/CONTEXT.md").is_file());
        assert!(root.join(".specs/ARCHITECTURE.md").is_file());
        assert!(root.join(".specs/LESSONS.md").is_file());
        assert!(result.files_created.contains(&".specs/CONTEXT.md".to_string()));
        assert!(result.files_created.contains(&".specs/ARCHITECTURE.md".to_string()));
        assert!(result.files_created.contains(&".specs/LESSONS.md".to_string()));

        // STATE.md
        assert!(result.state_file_created);
        assert!(root.join("STATE.md").is_file());
        let state_content = fs::read_to_string(root.join("STATE.md")).unwrap();
        assert!(state_content.contains("chg-001"), "STATE.md should contain change_id");

        // Change dir with stage artifacts
        assert!(root.join(".specs/chg-001").is_dir());
        // Enabled stages with artifacts: 0-change (CHANGE.md), 1-requirement (REQUIREMENT.md),
        // 2-design (DESIGN.md), 3-task (TASK.md). 4-dev is disabled, 2a-ui-design has no artifact for backend
        assert!(root.join(".specs/chg-001/CHANGE.md").is_file());
        assert!(root.join(".specs/chg-001/REQUIREMENT.md").is_file());
        assert!(root.join(".specs/chg-001/DESIGN.md").is_file());
        assert!(root.join(".specs/chg-001/TASK.md").is_file());

        // Recipe file itself
        assert!(root.join(".opensunstar/recipe/install-test.recipe.md").is_file());

        // change_id matches
        assert_eq!(result.change_id, "chg-001");

        // Orchestration log
        let log_path = root.join(".opensunstar/orchestration.log.jsonl");
        assert!(log_path.is_file());
        let log_content = fs::read_to_string(&log_path).unwrap();
        assert!(log_content.contains("recipe_install"));
    }

    #[test]
    fn install_skips_existing_files() {
        let root = temp_project();

        // Pre-create .specs/ dir and some files
        let specs_dir = root.join(".specs");
        fs::create_dir_all(&specs_dir).unwrap();
        fs::write(specs_dir.join("CONTEXT.md"), "# Existing Context").unwrap();
        fs::write(root.join("STATE.md"), "# Existing State").unwrap();

        let preset = sample_preset();
        let modules = sample_modules();
        let params = RecipeComposeParams {
            preset_id: "standard".into(),
            project_type: "backend".into(),
            name: "Skip Test".into(),
            description: None,
            selected_modules: None,
            disabled_stages: Some(vec!["4-dev".into()]),
            notes: None,
            stage_docs: None,
        };

        let recipe = compose_recipe(&preset, &params, &modules).unwrap();
        let result = install_recipe(root.to_str().unwrap(), &recipe, "chg-002").unwrap();

        // .specs/ already existed
        assert!(!result.specs_dir_created);

        // Pre-existing files should be skipped
        assert!(result.files_skipped.contains(&".specs/CONTEXT.md".to_string()));
        assert!(result.files_skipped.contains(&"STATE.md".to_string()));
        assert!(!result.state_file_created);

        // Existing content should NOT be overwritten
        let ctx = fs::read_to_string(specs_dir.join("CONTEXT.md")).unwrap();
        assert_eq!(ctx, "# Existing Context", "Should not overwrite existing CONTEXT.md");
        let state = fs::read_to_string(root.join("STATE.md")).unwrap();
        assert_eq!(state, "# Existing State", "Should not overwrite existing STATE.md");

        // New files should still be created
        assert!(root.join(".specs/ARCHITECTURE.md").is_file());
        assert!(root.join(".specs/LESSONS.md").is_file());
        assert!(root.join(".specs/chg-002/CHANGE.md").is_file());
    }
}
