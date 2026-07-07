//! Design Contract: machine-readable frontend design system configuration.
//!
//! Inspired by Google DESIGN.md, Meta Astryx, shadcn/ui registry, and Ant Design's
//! AI-ready toolchain. This module provides:
//!
//! - **DesignContract** data model (colors, typography, spacing, elevation, shapes, components, guardrails)
//! - **8 built-in brand templates** (Vercel, Apple, Stripe, Linear, Notion, GitHub, shadcn, Neutral)
//! - **DESIGN.md generation** (Google DESIGN.md spec compatible: YAML frontmatter + Markdown body)
//! - **W3C DTCG JSON export** (Design Tokens Community Group format for toolchain interop)
//! - **Import from URL/file** (parse existing DESIGN.md back into DesignContract)
//! - **Install to project** (write DESIGN.md to project root, archive in `.opensunstar/contract/`)

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::write_text_file;
use crate::error::AppError;
use crate::services::flow_orchestrator::append_orchestration_log;

const OPENSUNSTAR_DIR: &str = ".opensunstar";
const CONTRACT_DIR: &str = "contract";
const CONTRACT_SCHEMA_VERSION: u32 = 1;

// ──────────────────────────────── Types ────────────────────────────────

/// A complete frontend design system configuration (design contract).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignContract {
    pub schema_version: u32,
    pub name: String,
    pub description: Option<String>,
    pub colors: DesignColors,
    pub typography: DesignTypography,
    pub spacing: DesignSpacing,
    pub elevation: DesignElevation,
    pub shapes: DesignShapes,
    pub components: Vec<ComponentSpec>,
    pub guardrails: Vec<DesignGuardrail>,
    pub source_template: Option<String>,
    pub generated_at: String,
    pub opensunstar_version: String,
}

/// Color system tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignColors {
    pub primary: String,
    pub primary_hover: String,
    pub background: String,
    pub surface: String,
    pub text_primary: String,
    pub text_muted: String,
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub border: String,
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

/// Typography system tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignTypography {
    pub font_family_base: String,
    pub font_family_heading: String,
    pub font_family_mono: String,
    pub font_weights: Vec<u32>,
    pub size_scale: Vec<FontSize>,
}

/// Single font size entry in the scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontSize {
    pub name: String,
    pub size: String,
    pub line_height: String,
}

/// Spacing system tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSpacing {
    pub base_unit: u32,
    pub scale: Vec<u32>,
}

/// Elevation / shadow system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignElevation {
    pub levels: Vec<ShadowLevel>,
}

/// Single shadow level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowLevel {
    pub name: String,
    pub value: String,
}

/// Shape / border-radius system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignShapes {
    pub border_radius: HashMap<String, String>,
}

/// Component specification within the design contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentSpec {
    pub name: String,
    pub description: String,
    pub variants: Vec<String>,
    pub sizes: Vec<String>,
    pub rules: Vec<String>,
}

/// Design guardrail (Do's and Don'ts).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignGuardrail {
    pub rule: String,
    pub category: String,
    pub severity: String, // must | should | must_not | should_not
}

/// Parameters for composing a design contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignContractParams {
    pub template_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub colors: Option<DesignColors>,
    pub typography: Option<DesignTypography>,
    pub spacing: Option<DesignSpacing>,
    pub elevation: Option<DesignElevation>,
    pub shapes: Option<DesignShapes>,
    pub components: Option<Vec<ComponentSpec>>,
    pub guardrails: Option<Vec<DesignGuardrail>>,
}

/// Result of importing a DESIGN.md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub contract: DesignContract,
    pub source: String,
    pub warnings: Vec<String>,
}

/// Result of installing a design contract into a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignInstallResult {
    pub files_created: Vec<String>,
    pub files_skipped: Vec<String>,
    pub design_md_created: bool,
    pub dtchg_json_created: bool,
}

/// Per-file entry in an install plan (pre-flight dry run).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallFileEntry {
    pub path: String,
    pub status: String,
    pub new_content: Option<String>,
    pub existing_content: Option<String>,
}

/// Audit finding summary for install plan (serializable subset).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallAuditFinding {
    pub severity: String,
    pub rule_id: String,
    pub message: String,
    pub file: String,
}

/// Audit summary for install plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallAuditSummary {
    pub files_scanned: usize,
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub blocked: bool,
    pub findings: Vec<InstallAuditFinding>,
}

/// Pre-flight dry-run result: what WILL happen if install proceeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignInstallPlan {
    pub files: Vec<InstallFileEntry>,
    pub audit: InstallAuditSummary,
}

// ──────────────────────── Built-in Brand Templates ────────────────────────

/// List all built-in brand template IDs and names.
pub fn list_design_templates() -> Vec<(String, String)> {
    vec![
        ("vercel".into(), "Vercel".into()),
        ("apple".into(), "Apple".into()),
        ("stripe".into(), "Stripe".into()),
        ("linear".into(), "Linear".into()),
        ("notion".into(), "Notion".into()),
        ("github".into(), "GitHub".into()),
        ("shadcn".into(), "shadcn/ui".into()),
        ("neutral".into(), "Neutral (Blank)".into()),
    ]
}

/// Get a built-in template by ID.
pub fn get_design_template(id: &str) -> Option<DesignContract> {
    match id {
        "vercel" => Some(template_vercel()),
        "apple" => Some(template_apple()),
        "stripe" => Some(template_stripe()),
        "linear" => Some(template_linear()),
        "notion" => Some(template_notion()),
        "github" => Some(template_github()),
        "shadcn" => Some(template_shadcn()),
        "neutral" => Some(template_neutral()),
        _ => None,
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn default_contract(name: &str, template_id: &str) -> DesignContract {
    DesignContract {
        schema_version: CONTRACT_SCHEMA_VERSION,
        name: name.to_string(),
        description: None,
        colors: DesignColors {
            primary: "#000000".into(),
            primary_hover: "#333333".into(),
            background: "#ffffff".into(),
            surface: "#fafafa".into(),
            text_primary: "#171717".into(),
            text_muted: "#737373".into(),
            accent: "#0070f3".into(),
            success: "#0070f3".into(),
            warning: "#f5a623".into(),
            error: "#e00".into(),
            border: "#eaeaea".into(),
            custom: HashMap::new(),
        },
        typography: DesignTypography {
            font_family_base: "Inter, -apple-system, BlinkMacSystemFont, sans-serif".into(),
            font_family_heading: "Inter, sans-serif".into(),
            font_family_mono: "JetBrains Mono, monospace".into(),
            font_weights: vec![400, 500, 600, 700],
            size_scale: vec![
                FontSize { name: "xs".into(), size: "12px".into(), line_height: "16px".into() },
                FontSize { name: "sm".into(), size: "14px".into(), line_height: "20px".into() },
                FontSize { name: "base".into(), size: "16px".into(), line_height: "24px".into() },
                FontSize { name: "lg".into(), size: "18px".into(), line_height: "28px".into() },
                FontSize { name: "xl".into(), size: "20px".into(), line_height: "28px".into() },
                FontSize { name: "2xl".into(), size: "24px".into(), line_height: "32px".into() },
                FontSize { name: "3xl".into(), size: "30px".into(), line_height: "36px".into() },
                FontSize { name: "4xl".into(), size: "36px".into(), line_height: "40px".into() },
            ],
        },
        spacing: DesignSpacing {
            base_unit: 4,
            scale: vec![0, 1, 2, 3, 4, 6, 8, 12, 16, 24],
        },
        elevation: DesignElevation {
            levels: vec![
                ShadowLevel { name: "sm".into(), value: "0 1px 2px rgba(0,0,0,0.05)".into() },
                ShadowLevel { name: "md".into(), value: "0 4px 6px rgba(0,0,0,0.07)".into() },
                ShadowLevel { name: "lg".into(), value: "0 10px 15px rgba(0,0,0,0.1)".into() },
                ShadowLevel { name: "xl".into(), value: "0 20px 25px rgba(0,0,0,0.15)".into() },
            ],
        },
        shapes: DesignShapes {
            border_radius: {
                let mut m = HashMap::new();
                m.insert("none".into(), "0".into());
                m.insert("sm".into(), "4px".into());
                m.insert("md".into(), "8px".into());
                m.insert("lg".into(), "12px".into());
                m.insert("full".into(), "9999px".into());
                m
            },
        },
        components: default_components(),
        guardrails: default_guardrails(),
        source_template: Some(template_id.to_string()),
        generated_at: now_iso(),
        opensunstar_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn default_components() -> Vec<ComponentSpec> {
    vec![
        ComponentSpec {
            name: "Button".into(),
            description: "Interactive action trigger".into(),
            variants: vec!["primary".into(), "secondary".into(), "ghost".into(), "destructive".into()],
            sizes: vec!["sm".into(), "md".into(), "lg".into()],
            rules: vec![
                "Always use semantic color tokens, never hardcode hex values".into(),
                "Use appropriate variant for context (primary for main CTA)".into(),
            ],
        },
        ComponentSpec {
            name: "Card".into(),
            description: "Content container with elevation and border".into(),
            variants: vec!["default".into(), "outlined".into(), "elevated".into()],
            sizes: vec![],
            rules: vec![
                "Use surface color token for background".into(),
                "Maintain consistent padding using spacing scale".into(),
            ],
        },
        ComponentSpec {
            name: "Input".into(),
            description: "Text input field".into(),
            variants: vec!["default".into(), "error".into(), "disabled".into()],
            sizes: vec!["sm".into(), "md".into(), "lg".into()],
            rules: vec![
                "Always include a label".into(),
                "Use border token for outline".into(),
            ],
        },
        ComponentSpec {
            name: "Dialog".into(),
            description: "Modal overlay for focused interactions".into(),
            variants: vec!["default".into(), "alert".into()],
            sizes: vec!["sm".into(), "md".into(), "lg".into()],
            rules: vec![
                "Never use raw HTML modal — always use the component".into(),
                "Include close button and backdrop click dismissal".into(),
            ],
        },
    ]
}

fn default_guardrails() -> Vec<DesignGuardrail> {
    vec![
        DesignGuardrail { rule: "Use design tokens for all colors, spacing, and typography".into(), category: "general".into(), severity: "must".into() },
        DesignGuardrail { rule: "Use semantic component names, never raw HTML elements for interactive UI".into(), category: "component".into(), severity: "must".into() },
        DesignGuardrail { rule: "Use inline styles".into(), category: "general".into(), severity: "must_not".into() },
        DesignGuardrail { rule: "Use magic numbers for spacing or sizing".into(), category: "spacing".into(), severity: "must_not".into() },
        DesignGuardrail { rule: "Prefer composition over customization".into(), category: "component".into(), severity: "should".into() },
        DesignGuardrail { rule: "Maintain WCAG AA contrast ratios (4.5:1 for text)".into(), category: "color".into(), severity: "should".into() },
    ]
}

fn template_vercel() -> DesignContract {
    let mut c = default_contract("Vercel Style", "vercel");
    c.description = Some("Vercel's minimalist black-and-white design language. Geist font. Near-zero color palette.".into());
    c.colors = DesignColors {
        primary: "#171717".into(),
        primary_hover: "#000000".into(),
        background: "#ffffff".into(),
        surface: "#fafafa".into(),
        text_primary: "#171717".into(),
        text_muted: "#666666".into(),
        accent: "#0070f3".into(),
        success: "#0070f3".into(),
        warning: "#f5a623".into(),
        error: "#ee0000".into(),
        border: "#eaeaea".into(),
        custom: HashMap::new(),
    };
    c.typography.font_family_base = "Geist, Geist Sans, -apple-system, BlinkMacSystemFont, sans-serif".into();
    c.typography.font_family_heading = "Geist, Geist Sans, sans-serif".into();
    c.typography.font_family_mono = "Geist Mono, monospace".into();
    c
}

fn template_apple() -> DesignContract {
    let mut c = default_contract("Apple Style", "apple");
    c.description = Some("Apple's human interface guidelines. SF Pro font. Generous whitespace. Rounded cards.".into());
    c.colors = DesignColors {
        primary: "#0071e3".into(),
        primary_hover: "#0077ed".into(),
        background: "#ffffff".into(),
        surface: "#f5f5f7".into(),
        text_primary: "#1d1d1f".into(),
        text_muted: "#86868b".into(),
        accent: "#0071e3".into(),
        success: "#34c759".into(),
        warning: "#ff9f0a".into(),
        error: "#ff3b30".into(),
        border: "#d2d2d7".into(),
        custom: HashMap::new(),
    };
    c.typography.font_family_base = "-apple-system, BlinkMacSystemFont, 'SF Pro Display', 'SF Pro Text', 'Helvetica Neue', sans-serif".into();
    c.typography.font_family_heading = "-apple-system, BlinkMacSystemFont, 'SF Pro Display', sans-serif".into();
    c.typography.font_family_mono = "'SF Mono', SFMono-Regular, Menlo, monospace".into();
    c.shapes.border_radius.insert("sm".into(), "6px".into());
    c.shapes.border_radius.insert("md".into(), "12px".into());
    c.shapes.border_radius.insert("lg".into(), "18px".into());
    c
}

fn template_stripe() -> DesignContract {
    let mut c = default_contract("Stripe Style", "stripe");
    c.description = Some("Stripe's gradient-rich design. Inter font. Purple-blue accent. Refined shadows.".into());
    c.colors = DesignColors {
        primary: "#635bff".into(),
        primary_hover: "#7a73ff".into(),
        background: "#ffffff".into(),
        surface: "#f6f9fc".into(),
        text_primary: "#0a2540".into(),
        text_muted: "#425466".into(),
        accent: "#635bff".into(),
        success: "#24b47e".into(),
        warning: "#f3a616".into(),
        error: "#df1b41".into(),
        border: "#e6ebf1".into(),
        custom: HashMap::new(),
    };
    c.typography.font_family_base = "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif".into();
    c.typography.font_family_heading = "Inter, sans-serif".into();
    c
}

fn template_linear() -> DesignContract {
    let mut c = default_contract("Linear Style", "linear");
    c.description = Some("Linear's dark-first design. Inter font. Violet-blue accent. Minimal chrome.".into());
    c.colors = DesignColors {
        primary: "#5e6ad2".into(),
        primary_hover: "#6e7ae2".into(),
        background: "#0e0e10".into(),
        surface: "#1c1c1f".into(),
        text_primary: "#f4f4f5".into(),
        text_muted: "#7c7c82".into(),
        accent: "#5e6ad2".into(),
        success: "#26c281".into(),
        warning: "#f2c94c".into(),
        error: "#eb5757".into(),
        border: "#2b2b2f".into(),
        custom: HashMap::new(),
    };
    c.typography.font_family_base = "Inter, -apple-system, BlinkMacSystemFont, sans-serif".into();
    c.typography.font_family_heading = "Inter, sans-serif".into();
    c
}

fn template_notion() -> DesignContract {
    let mut c = default_contract("Notion Style", "notion");
    c.description = Some("Notion's clean workspace aesthetic. Serif headings. Minimal color. Generous whitespace.".into());
    c.colors = DesignColors {
        primary: "#2383e2".into(),
        primary_hover: "#1b6ec2".into(),
        background: "#ffffff".into(),
        surface: "#f7f6f3".into(),
        text_primary: "#37352f".into(),
        text_muted: "#9b9a97".into(),
        accent: "#2383e2".into(),
        success: "#4daa57".into(),
        warning: "#d9730d".into(),
        error: "#e03e3e".into(),
        border: "#e9e9e7".into(),
        custom: HashMap::new(),
    };
    c.typography.font_family_base = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, sans-serif".into();
    c.typography.font_family_heading = "'Georgia', 'Cambria', 'Times New Roman', serif".into();
    c
}

fn template_github() -> DesignContract {
    let mut c = default_contract("GitHub Style", "github");
    c.description = Some("GitHub's Primer design system. System font stack. Utility-first. Pragmatic.".into());
    c.colors = DesignColors {
        primary: "#1f883d".into(),
        primary_hover: "#2ea043".into(),
        background: "#ffffff".into(),
        surface: "#f6f8fa".into(),
        text_primary: "#1f2328".into(),
        text_muted: "#656d76".into(),
        accent: "#0969da".into(),
        success: "#1a7f37".into(),
        warning: "#9a6700".into(),
        error: "#cf222e".into(),
        border: "#d0d7de".into(),
        custom: HashMap::new(),
    };
    c.typography.font_family_base = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif".into();
    c.typography.font_family_heading = "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif".into();
    c.typography.font_family_mono = "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace".into();
    c
}

fn template_shadcn() -> DesignContract {
    let mut c = default_contract("shadcn/ui Style", "shadcn");
    c.description = Some("shadcn/ui's neutral design language. Inter font. CSS variable-based theming. Minimal rounded corners.".into());
    c.colors = DesignColors {
        primary: "#18181b".into(),
        primary_hover: "#27272a".into(),
        background: "#ffffff".into(),
        surface: "#f4f4f5".into(),
        text_primary: "#09090b".into(),
        text_muted: "#71717a".into(),
        accent: "#f4f4f5".into(),
        success: "#22c55e".into(),
        warning: "#eab308".into(),
        error: "#ef4444".into(),
        border: "#e4e4e7".into(),
        custom: HashMap::new(),
    };
    c.typography.font_family_base = "Inter, ui-sans-serif, system-ui, sans-serif".into();
    c.typography.font_family_heading = "Inter, sans-serif".into();
    c.shapes.border_radius.insert("sm".into(), "6px".into());
    c.shapes.border_radius.insert("md".into(), "8px".into());
    c.shapes.border_radius.insert("lg".into(), "10px".into());
    c
}

fn template_neutral() -> DesignContract {
    default_contract("Neutral (Blank)", "neutral")
}

// ──────────────────────── Compose ────────────────────────

/// Compose a design contract from parameters, optionally starting from a built-in template.
pub fn compose_design_contract(params: &DesignContractParams) -> Result<DesignContract, AppError> {
    let base = match &params.template_id {
        Some(id) => get_design_template(id).unwrap_or_else(|| default_contract(&params.name, "custom")),
        None => default_contract(&params.name, "custom"),
    };

    Ok(DesignContract {
        schema_version: CONTRACT_SCHEMA_VERSION,
        name: params.name.clone(),
        description: params.description.clone().or(base.description),
        colors: params.colors.clone().unwrap_or(base.colors),
        typography: params.typography.clone().unwrap_or(base.typography),
        spacing: params.spacing.clone().unwrap_or(base.spacing),
        elevation: params.elevation.clone().unwrap_or(base.elevation),
        shapes: params.shapes.clone().unwrap_or(base.shapes),
        components: params.components.clone().unwrap_or(base.components),
        guardrails: params.guardrails.clone().unwrap_or(base.guardrails),
        source_template: params.template_id.clone().or(base.source_template),
        generated_at: now_iso(),
        opensunstar_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// ──────────────────────── Generate DESIGN.md ────────────────────────

/// Generate a standard DESIGN.md file (Google DESIGN.md spec compatible).
pub fn generate_design_md(contract: &DesignContract) -> Result<String, AppError> {
    let mut out = String::new();

    // YAML frontmatter
    out.push_str("---\n");
    out.push_str(&format!("name: \"{}\"\n", contract.name));
    out.push_str(&format!("version: \"{}.0.0\"\n", contract.schema_version));
    if let Some(ref desc) = contract.description {
        out.push_str(&format!("description: \"{}\"\n", desc));
    }
    if let Some(ref tpl) = contract.source_template {
        out.push_str(&format!("source_template: \"{}\"\n", tpl));
    }
    out.push_str(&format!("generated: \"{}\"\n", contract.generated_at));
    out.push_str(&format!("generator: \"OpenSunstar {}\"\n", contract.opensunstar_version));
    out.push_str("---\n\n");

    // Title
    out.push_str(&format!("# {}\n\n", contract.name));
    if let Some(ref desc) = contract.description {
        out.push_str(&format!("> {}\n\n", desc));
    }

    // Colors
    out.push_str("## Colors\n\n");
    out.push_str("| Token | Value | Usage |\n");
    out.push_str("|-------|-------|-------|\n");
    out.push_str(&format!("| primary | {} | Primary actions, links |\n", contract.colors.primary));
    out.push_str(&format!("| primary-hover | {} | Hover state |\n", contract.colors.primary_hover));
    out.push_str(&format!("| background | {} | Page background |\n", contract.colors.background));
    out.push_str(&format!("| surface | {} | Card / panel background |\n", contract.colors.surface));
    out.push_str(&format!("| text-primary | {} | Primary text |\n", contract.colors.text_primary));
    out.push_str(&format!("| text-muted | {} | Secondary text |\n", contract.colors.text_muted));
    out.push_str(&format!("| accent | {} | Accent / highlight |\n", contract.colors.accent));
    out.push_str(&format!("| success | {} | Success state |\n", contract.colors.success));
    out.push_str(&format!("| warning | {} | Warning state |\n", contract.colors.warning));
    out.push_str(&format!("| error | {} | Error state |\n", contract.colors.error));
    out.push_str(&format!("| border | {} | Borders and dividers |\n", contract.colors.border));
    for (k, v) in &contract.colors.custom {
        out.push_str(&format!("| {} | {} | Custom token |\n", k, v));
    }
    out.push('\n');

    // Typography
    out.push_str("## Typography\n\n");
    out.push_str(&format!("- **Base**: {}\n", contract.typography.font_family_base));
    out.push_str(&format!("- **Heading**: {}\n", contract.typography.font_family_heading));
    out.push_str(&format!("- **Mono**: {}\n", contract.typography.font_family_mono));
    out.push_str(&format!("- **Weights**: {}\n",
        contract.typography.font_weights.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(", ")));
    out.push_str("- **Size scale**:\n");
    for fs in &contract.typography.size_scale {
        out.push_str(&format!("  - {}: {} / {} (line-height)\n", fs.name, fs.size, fs.line_height));
    }
    out.push('\n');

    // Layout & Spacing
    out.push_str("## Layout & Spacing\n\n");
    out.push_str(&format!("- Base unit: {}px\n", contract.spacing.base_unit));
    out.push_str(&format!("- Scale: {}\n",
        contract.spacing.scale.iter().map(|s| format!("{} ({})", s, s * contract.spacing.base_unit)).collect::<Vec<_>>().join(" · ")));
    out.push('\n');

    // Elevation & Depth
    out.push_str("## Elevation & Depth\n\n");
    for level in &contract.elevation.levels {
        out.push_str(&format!("- **{}**: `{}`\n", level.name, level.value));
    }
    out.push('\n');

    // Shapes
    out.push_str("## Shapes\n\n");
    let mut radii: Vec<_> = contract.shapes.border_radius.iter().collect();
    radii.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (name, value) in &radii {
        out.push_str(&format!("- **{}**: {}\n", name, value));
    }
    out.push('\n');

    // Components
    if !contract.components.is_empty() {
        out.push_str("## Components\n\n");
        for comp in &contract.components {
            out.push_str(&format!("### {}\n\n", comp.name));
            out.push_str(&format!("{}\n\n", comp.description));
            if !comp.variants.is_empty() {
                out.push_str(&format!("- Variants: {}\n", comp.variants.join(", ")));
            }
            if !comp.sizes.is_empty() {
                out.push_str(&format!("- Sizes: {}\n", comp.sizes.join(", ")));
            }
            for rule in &comp.rules {
                out.push_str(&format!("- Rule: {}\n", rule));
            }
            out.push('\n');
        }
    }

    // Do's and Don'ts
    if !contract.guardrails.is_empty() {
        out.push_str("## Do's and Don'ts\n\n");
        for g in &contract.guardrails {
            let prefix = match g.severity.as_str() {
                "must" => "**MUST**",
                "should" => "**SHOULD**",
                "must_not" => "**MUST NOT**",
                "should_not" => "**SHOULD NOT**",
                _ => "**RULE**",
            };
            out.push_str(&format!("- {} {}\n", prefix, g.rule));
        }
        out.push('\n');
    }

    Ok(out)
}

// ──────────────────────── Generate DTCG JSON ────────────────────────

/// Generate W3C Design Tokens Community Group JSON format.
pub fn generate_dtchg_json(contract: &DesignContract) -> Result<String, AppError> {
    let mut root = serde_json::Map::new();

    // Colors
    let mut colors = serde_json::Map::new();
    let add_color = |m: &mut serde_json::Map<String, serde_json::Value>, k: &str, v: &str| {
        m.insert(k.into(), serde_json::json!({ "$value": v, "$type": "color" }));
    };
    add_color(&mut colors, "primary", &contract.colors.primary);
    add_color(&mut colors, "primary-hover", &contract.colors.primary_hover);
    add_color(&mut colors, "background", &contract.colors.background);
    add_color(&mut colors, "surface", &contract.colors.surface);
    add_color(&mut colors, "text-primary", &contract.colors.text_primary);
    add_color(&mut colors, "text-muted", &contract.colors.text_muted);
    add_color(&mut colors, "accent", &contract.colors.accent);
    add_color(&mut colors, "success", &contract.colors.success);
    add_color(&mut colors, "warning", &contract.colors.warning);
    add_color(&mut colors, "error", &contract.colors.error);
    add_color(&mut colors, "border", &contract.colors.border);
    for (k, v) in &contract.colors.custom {
        add_color(&mut colors, k, v);
    }
    root.insert("color".into(), serde_json::Value::Object(colors));

    // Typography
    let mut typo = serde_json::Map::new();
    typo.insert("base".into(), serde_json::json!({ "$value": contract.typography.font_family_base, "$type": "fontFamily" }));
    typo.insert("heading".into(), serde_json::json!({ "$value": contract.typography.font_family_heading, "$type": "fontFamily" }));
    typo.insert("mono".into(), serde_json::json!({ "$value": contract.typography.font_family_mono, "$type": "fontFamily" }));
    root.insert("fontFamily".into(), serde_json::Value::Object(typo));

    // Font sizes
    let mut sizes = serde_json::Map::new();
    for fs in &contract.typography.size_scale {
        sizes.insert(fs.name.clone(), serde_json::json!({
            "$value": fs.size,
            "$type": "dimension",
            "$description": format!("line-height: {}", fs.line_height)
        }));
    }
    root.insert("fontSize".into(), serde_json::Value::Object(sizes));

    // Spacing
    let mut spacing = serde_json::Map::new();
    for &s in &contract.spacing.scale {
        let px = s * contract.spacing.base_unit;
        let key = format!("space-{}", s);
        spacing.insert(key, serde_json::json!({ "$value": format!("{}px", px), "$type": "dimension" }));
    }
    root.insert("spacing".into(), serde_json::Value::Object(spacing));

    // Border radius
    let mut radii = serde_json::Map::new();
    for (k, v) in &contract.shapes.border_radius {
        radii.insert(k.clone(), serde_json::json!({ "$value": v, "$type": "dimension" }));
    }
    root.insert("borderRadius".into(), serde_json::Value::Object(radii));

    // Shadows
    let mut shadows = serde_json::Map::new();
    for level in &contract.elevation.levels {
        shadows.insert(level.name.clone(), serde_json::json!({ "$value": level.value, "$type": "shadow" }));
    }
    root.insert("shadow".into(), serde_json::Value::Object(shadows));

    serde_json::to_string_pretty(&serde_json::Value::Object(root))
        .map_err(|e| AppError::Message(format!("DTCG JSON 序列化失败: {e}")))
}

// ──────────────────────── Parse DESIGN.md ────────────────────────

/// Parse a DESIGN.md file content back into a DesignContract.
/// Extracts YAML frontmatter tokens and Markdown body guardrails/components.
pub fn parse_design_md(content: &str) -> Result<(DesignContract, Vec<String>), AppError> {
    let mut warnings: Vec<String> = Vec::new();

    // Extract YAML frontmatter
    let (yaml_str, body) = if content.starts_with("---\n") {
        if let Some(end) = content[4..].find("\n---") {
            (&content[4..4 + end], &content[4 + end + 4..])
        } else {
            warnings.push("No closing YAML delimiter found".into());
            ("", content)
        }
    } else {
        warnings.push("No YAML frontmatter found".into());
        ("", content)
    };

    // Parse basic YAML fields (name, source_template)
    let name = extract_yaml_field(yaml_str, "name").unwrap_or_else(|| "Imported Design".into());
    let source_template = extract_yaml_field(yaml_str, "source_template");
    let description = extract_yaml_field(yaml_str, "description");

    // Try to extract colors from markdown table
    let colors = extract_colors_from_md(body).unwrap_or_else(|| {
        warnings.push("Could not extract colors from Markdown body".into());
        template_neutral().colors
    });

    // Try to extract typography
    let typography = extract_typography_from_md(body).unwrap_or_else(|| {
        warnings.push("Could not extract typography from Markdown body".into());
        template_neutral().typography
    });

    // Try to extract spacing
    let spacing = extract_spacing_from_md(body).unwrap_or_else(|| {
        warnings.push("Could not extract spacing from Markdown body".into());
        template_neutral().spacing
    });

    // Try to extract shapes
    let shapes = extract_shapes_from_md(body).unwrap_or_else(|| {
        template_neutral().shapes
    });

    // Try to extract elevation
    let elevation = extract_elevation_from_md(body).unwrap_or_else(|| {
        template_neutral().elevation
    });

    // Extract guardrails from Do's and Don'ts section
    let guardrails = extract_guardrails_from_md(body);

    Ok((DesignContract {
        schema_version: CONTRACT_SCHEMA_VERSION,
        name,
        description,
        colors,
        typography,
        spacing,
        elevation,
        shapes,
        components: default_components(),
        guardrails,
        source_template,
        generated_at: now_iso(),
        opensunstar_version: env!("CARGO_PKG_VERSION").to_string(),
    }, warnings))
}

fn extract_yaml_field(yaml: &str, field: &str) -> Option<String> {
    for line in yaml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(field) {
            if let Some(rest) = rest.strip_prefix(':') {
                let val = rest.trim().trim_matches('"').trim_matches('\'');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn extract_colors_from_md(body: &str) -> Option<DesignColors> {
    let mut colors = DesignColors {
        primary: "#000000".into(),
        primary_hover: "#333333".into(),
        background: "#ffffff".into(),
        surface: "#fafafa".into(),
        text_primary: "#171717".into(),
        text_muted: "#737373".into(),
        accent: "#0070f3".into(),
        success: "#22c55e".into(),
        warning: "#eab308".into(),
        error: "#ef4444".into(),
        border: "#e4e4e7".into(),
        custom: HashMap::new(),
    };

    let mut found_any = false;
    for line in body.lines() {
        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if parts.len() >= 3 {
            let token = parts.get(1).unwrap_or(&"").to_lowercase();
            let value = parts.get(2).unwrap_or(&"").trim();
            if value.starts_with('#') && (value.len() == 7 || value.len() == 4) {
                match token.as_str() {
                    "primary" => { colors.primary = value.into(); found_any = true; }
                    s if s.contains("primary") && s.contains("hover") => { colors.primary_hover = value.into(); }
                    "background" => { colors.background = value.into(); found_any = true; }
                    "surface" => { colors.surface = value.into(); }
                    "text-primary" | "text_primary" => { colors.text_primary = value.into(); found_any = true; }
                    "text-muted" | "text_muted" => { colors.text_muted = value.into(); }
                    "accent" => { colors.accent = value.into(); }
                    "success" => { colors.success = value.into(); }
                    "warning" => { colors.warning = value.into(); }
                    "error" => { colors.error = value.into(); }
                    "border" => { colors.border = value.into(); }
                    _ => { colors.custom.insert(token, value.into()); }
                }
            }
        }
    }

    if found_any { Some(colors) } else { None }
}

/// Check if a line is a Markdown table separator (e.g., `|---|---|`).
fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && trimmed.starts_with('|')
        && trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
}

/// Extract lines belonging to a specific Markdown section (between `## heading` and next `##`).
fn md_section_lines<'a>(body: &'a str, heading_contains: &str) -> Vec<&'a str> {
    let mut lines = Vec::new();
    let mut in_section = false;
    let needle = heading_contains.to_lowercase();
    for line in body.lines() {
        if line.starts_with("## ") {
            if line.to_lowercase().contains(&needle) {
                in_section = true;
                continue;
            } else if in_section {
                break;
            }
        }
        if in_section {
            lines.push(line);
        }
    }
    lines
}

/// Greatest common divisor (for inferring spacing base unit from px values).
fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn extract_typography_from_md(body: &str) -> Option<DesignTypography> {
    let mut font_family_base: Option<String> = None;
    let mut font_family_heading: Option<String> = None;
    let mut font_family_mono: Option<String> = None;
    let mut font_weights: Vec<u32> = Vec::new();
    let mut size_scale: Vec<FontSize> = Vec::new();
    let mut in_size_scale = false;

    for line in body.lines() {
        let trimmed = line.trim();

        // A new section header resets size-scale tracking
        if trimmed.starts_with("## ") && !trimmed.to_lowercase().contains("typography") {
            in_size_scale = false;
        }

        // ── List-based format (generated by generate_design_md) ──

        // "- **Base**: Inter, sans-serif"
        if let Some(rest) = trimmed.strip_prefix("- **Base**:") {
            font_family_base = Some(rest.trim().to_string());
            in_size_scale = false;
            continue;
        }
        // "- **Heading**: Inter, sans-serif"
        if let Some(rest) = trimmed.strip_prefix("- **Heading**:") {
            font_family_heading = Some(rest.trim().to_string());
            in_size_scale = false;
            continue;
        }
        // "- **Mono**: JetBrains Mono, monospace"
        if let Some(rest) = trimmed.strip_prefix("- **Mono**:") {
            font_family_mono = Some(rest.trim().to_string());
            in_size_scale = false;
            continue;
        }
        // "- **Weights**: 400, 500, 600, 700"
        if let Some(rest) = trimmed.strip_prefix("- **Weights**:") {
            font_weights = rest
                .split(',')
                .filter_map(|s| s.trim().parse::<u32>().ok())
                .collect();
            in_size_scale = false;
            continue;
        }
        // "- **Size scale**:"
        if trimmed.contains("**Size scale**") {
            in_size_scale = true;
            continue;
        }
        // Size scale entries: "  - xs: 12px / 16px (line-height)"
        if in_size_scale {
            if let Some(rest) = trimmed.strip_prefix("- ") {
                if let Some((name, rest)) = rest.split_once(':') {
                    let rest = rest.trim();
                    if let Some((size_part, lh_part)) = rest.split_once('/') {
                        let size = size_part.trim().to_string();
                        let line_height = lh_part
                            .trim()
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_string();
                        if !size.is_empty() && !line_height.is_empty() {
                            size_scale.push(FontSize {
                                name: name.trim().to_string(),
                                size,
                                line_height,
                            });
                        }
                    }
                }
                continue;
            }
            // Non-list, non-empty line ends the size-scale block
            if !trimmed.is_empty() && !trimmed.starts_with('-') {
                in_size_scale = false;
            }
        }

        // ── Table-based format: | token | value | ──
        if trimmed.starts_with('|') && !is_table_separator(trimmed) {
            let parts: Vec<&str> = trimmed.split('|').collect();
            if parts.len() >= 3 {
                let token = parts[1].trim().to_lowercase();
                let value = parts[2].trim();
                // Skip header rows
                if token.contains("token") || token.contains("property") || token.contains("name") {
                    continue;
                }
                match token.as_str() {
                    "font-family-base" | "fontfamilybase" | "base-font" | "font-base" => {
                        if font_family_base.is_none() {
                            font_family_base = Some(value.to_string());
                        }
                    }
                    "font-family-heading" | "fontfamilyheading" | "heading-font" | "font-heading" => {
                        if font_family_heading.is_none() {
                            font_family_heading = Some(value.to_string());
                        }
                    }
                    "font-family-mono" | "fontfamilymono" | "mono-font" | "font-mono" => {
                        if font_family_mono.is_none() {
                            font_family_mono = Some(value.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if font_family_base.is_some() || !size_scale.is_empty() {
        Some(DesignTypography {
            font_family_base: font_family_base.unwrap_or_else(|| "Inter, sans-serif".into()),
            font_family_heading: font_family_heading.unwrap_or_else(|| "Inter, sans-serif".into()),
            font_family_mono: font_family_mono.unwrap_or_else(|| "monospace".into()),
            font_weights: if font_weights.is_empty() {
                vec![400, 500, 600, 700]
            } else {
                font_weights
            },
            size_scale,
        })
    } else {
        None
    }
}

fn extract_spacing_from_md(body: &str) -> Option<DesignSpacing> {
    let mut base_unit: Option<u32> = None;
    let mut scale: Option<Vec<u32>> = None;
    let mut table_px_values: Vec<u32> = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();

        // ── List-based format ──

        // "- Base unit: 4px"
        if let Some(rest) = trimmed.strip_prefix("- Base unit:") {
            let num_str: String = rest.trim().chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_str.parse::<u32>() {
                base_unit = Some(n);
            }
        }

        // "- Scale: 0 (0) · 1 (4) · 2 (8) · 3 (12) · ..."
        if let Some(rest) = trimmed.strip_prefix("- Scale:") {
            let multipliers: Vec<u32> = rest
                .split('\u{00b7}') // middle-dot '·'
                .filter_map(|part| {
                    let num_str: String =
                        part.trim().chars().take_while(|c| c.is_ascii_digit()).collect();
                    num_str.parse::<u32>().ok()
                })
                .collect();
            if !multipliers.is_empty() {
                scale = Some(multipliers);
            }
        }

        // ── Table-based format: | space-N | Npx | ──
        if trimmed.starts_with('|') && !is_table_separator(trimmed) {
            let parts: Vec<&str> = trimmed.split('|').collect();
            if parts.len() >= 3 {
                let token = parts[1].trim().to_lowercase();
                let value = parts[2].trim();
                if (token.starts_with("space-") || token.starts_with("spacing-"))
                    && !token.contains("token")
                {
                    let num_str: String =
                        value.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if let Ok(px) = num_str.parse::<u32>() {
                        table_px_values.push(px);
                    }
                }
            }
        }
    }

    // If we only have table data, infer base_unit and multipliers
    if scale.is_none() && !table_px_values.is_empty() {
        let non_zero: Vec<u32> = table_px_values.iter().copied().filter(|&v| v > 0).collect();
        let bu = if !non_zero.is_empty() {
            non_zero.iter().copied().reduce(gcd).unwrap_or(4)
        } else {
            4
        };
        let multipliers: Vec<u32> = table_px_values
            .iter()
            .map(|&px| if bu > 0 { px / bu } else { px })
            .collect();
        base_unit = Some(bu);
        scale = Some(multipliers);
    }

    if base_unit.is_some() || scale.is_some() {
        Some(DesignSpacing {
            base_unit: base_unit.unwrap_or(4),
            scale: scale.unwrap_or_else(|| vec![0, 1, 2, 3, 4, 6, 8, 12, 16, 24]),
        })
    } else {
        None
    }
}

fn extract_shapes_from_md(body: &str) -> Option<DesignShapes> {
    let mut border_radius: HashMap<String, String> = HashMap::new();

    // ── Section-based list format (generated by generate_design_md) ──
    let section_lines = md_section_lines(body, "shape");
    for line in &section_lines {
        let trimmed = line.trim();

        // "- **sm**: 4px"
        if let Some(rest) = trimmed.strip_prefix("- **") {
            if let Some((name, value)) = rest.split_once("**:") {
                let name = name.trim().to_string();
                let value = value.trim().trim_matches('`').to_string();
                if !name.is_empty() && !value.is_empty() {
                    border_radius.insert(name, value);
                }
            }
        }

        // Table-based: | sm | 4px |
        if trimmed.starts_with('|') && !is_table_separator(trimmed) {
            let parts: Vec<&str> = trimmed.split('|').collect();
            if parts.len() >= 3 {
                let token = parts[1].trim().to_lowercase();
                let value = parts[2].trim().trim_matches('`').to_string();
                if token.contains("token") || token.contains("name") || token.contains("property") {
                    continue;
                }
                let is_dimension = value.ends_with("px")
                    || value.ends_with("rem")
                    || value.ends_with("em")
                    || value == "0"
                    || value == "full"
                    || value == "none";
                let is_radius_token = ["none", "sm", "md", "lg", "xl", "2xl", "full", "pill"]
                    .contains(&token.as_str())
                    || token.contains("radius");
                if is_dimension && is_radius_token {
                    let clean_name = token
                        .strip_prefix("radius-")
                        .unwrap_or(&token)
                        .to_string();
                    border_radius.insert(clean_name, value);
                }
            }
        }
    }

    // ── Fallback: scan whole body for table rows with radius tokens ──
    if border_radius.is_empty() {
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('|') && !is_table_separator(trimmed) {
                let parts: Vec<&str> = trimmed.split('|').collect();
                if parts.len() >= 3 {
                    let token = parts[1].trim().to_lowercase();
                    let value = parts[2].trim().to_string();
                    if token.contains("radius") || token.contains("border-radius") {
                        let clean_name = token
                            .strip_prefix("border-radius-")
                            .or_else(|| token.strip_prefix("radius-"))
                            .unwrap_or(&token)
                            .to_string();
                        if !clean_name.is_empty() && !value.is_empty() {
                            border_radius.insert(clean_name, value);
                        }
                    }
                }
            }
        }
    }

    if border_radius.is_empty() {
        None
    } else {
        Some(DesignShapes { border_radius })
    }
}

fn extract_elevation_from_md(body: &str) -> Option<DesignElevation> {
    let mut levels: Vec<ShadowLevel> = Vec::new();

    // ── Section-based parsing (generated by generate_design_md) ──
    let section_lines = md_section_lines(body, "elevation");
    for line in &section_lines {
        let trimmed = line.trim();

        // "- **sm**: `0 1px 2px rgba(0,0,0,0.05)`"
        if let Some(rest) = trimmed.strip_prefix("- **") {
            if let Some((name, value)) = rest.split_once("**:") {
                let name = name.trim().to_string();
                let value = value.trim().trim_matches('`').to_string();
                if !name.is_empty() && !value.is_empty() {
                    levels.push(ShadowLevel { name, value });
                }
            }
        }

        // Table-based: | sm | 0 1px 2px rgba(0,0,0,0.05) |
        if trimmed.starts_with('|') && !is_table_separator(trimmed) {
            let parts: Vec<&str> = trimmed.split('|').collect();
            if parts.len() >= 3 {
                let token = parts[1].trim().to_lowercase();
                let value = parts[2].trim().trim_matches('`').to_string();
                if token.contains("token")
                    || token.contains("level")
                    || token.contains("name")
                    || value.is_empty()
                    || value.to_lowercase().contains("value")
                {
                    continue;
                }
                let looks_like_shadow = value.contains("rgba")
                    || value.contains("hsl")
                    || value.contains("px")
                    || value.contains("inset")
                    || value.starts_with("0 ");
                if looks_like_shadow {
                    levels.push(ShadowLevel {
                        name: token,
                        value,
                    });
                }
            }
        }
    }

    // ── Fallback: scan whole body for shadow table rows ──
    if levels.is_empty() {
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('|') && !is_table_separator(trimmed) {
                let parts: Vec<&str> = trimmed.split('|').collect();
                if parts.len() >= 3 {
                    let token = parts[1].trim().to_lowercase();
                    let value = parts[2].trim().trim_matches('`').to_string();
                    if (token.contains("shadow") || token.contains("elevation"))
                        && !token.contains("token")
                        && !value.is_empty()
                    {
                        let clean_name = token
                            .strip_prefix("shadow-")
                            .or_else(|| token.strip_prefix("elevation-"))
                            .unwrap_or(&token)
                            .to_string();
                        levels.push(ShadowLevel {
                            name: clean_name,
                            value,
                        });
                    }
                }
            }
        }
    }

    if levels.is_empty() {
        None
    } else {
        Some(DesignElevation { levels })
    }
}

fn extract_guardrails_from_md(body: &str) -> Vec<DesignGuardrail> {
    let mut guardrails = Vec::new();
    let mut in_section = false;

    for line in body.lines() {
        if line.starts_with("## Do's and Don'ts") || line.starts_with("## Do's & Don'ts") {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## ") {
            break;
        }
        if in_section && line.starts_with("- ") {
            let content = &line[2..];
            let (severity, rule) = if let Some(rest) = content.strip_prefix("**MUST NOT** ") {
                ("must_not", rest)
            } else if let Some(rest) = content.strip_prefix("**SHOULD NOT** ") {
                ("should_not", rest)
            } else if let Some(rest) = content.strip_prefix("**MUST** ") {
                ("must", rest)
            } else if let Some(rest) = content.strip_prefix("**SHOULD** ") {
                ("should", rest)
            } else {
                ("should", content)
            };
            guardrails.push(DesignGuardrail {
                rule: rule.to_string(),
                category: "general".into(),
                severity: severity.into(),
            });
        }
    }

    guardrails
}

// ──────────────────────── Import from URL ────────────────────────

/// Import a DESIGN.md from a URL by fetching its content.
/// Note: Actual HTTP fetch is done in the command layer; this function parses the content.
pub fn import_design_from_content(content: &str, source: &str) -> Result<ImportResult, AppError> {
    let (contract, warnings) = parse_design_md(content)?;
    Ok(ImportResult {
        contract,
        source: source.to_string(),
        warnings,
    })
}

/// Import a DESIGN.md from a local file path.
pub fn import_design_from_file(file_path: &str) -> Result<ImportResult, AppError> {
    let path = PathBuf::from(file_path);
    if !path.is_file() {
        return Err(AppError::Message(format!("文件不存在: {file_path}")));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| AppError::io(&path, e))?;
    let (contract, warnings) = parse_design_md(&content)?;
    Ok(ImportResult {
        contract,
        source: file_path.to_string(),
        warnings,
    })
}

// ──────────────────────── Export ────────────────────────

/// Export a design contract: write DESIGN.md and archive to `.opensunstar/contract/`.
pub fn export_design_contract(
    project_path: &str,
    contract: &DesignContract,
) -> Result<String, AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!("项目路径不存在: {project_path}")));
    }

    // Generate DESIGN.md content
    let md_content = generate_design_md(contract)?;

    // Write to .opensunstar/contract/<slug>.contract.md (archive)
    let contract_dir = root.join(OPENSUNSTAR_DIR).join(CONTRACT_DIR);
    fs::create_dir_all(&contract_dir)
        .map_err(|e| AppError::io(&contract_dir, e))?;

    let slug = contract.name.to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "");
    let archive_path = contract_dir.join(format!("{slug}.contract.md"));
    write_text_file(&archive_path, &md_content)?;

    // Also write DESIGN.md to project root
    let design_md_path = root.join("DESIGN.md");
    write_text_file(&design_md_path, &md_content)?;

    // Audit log
    append_orchestration_log(
        project_path,
        serde_json::json!({
            "event": "design_contract_export",
            "name": contract.name,
            "sourceTemplate": contract.source_template,
        }),
    )?;

    log::info!("Design contract '{}' exported to {}", contract.name, design_md_path.display());
    Ok(md_content)
}

// ──────────────────────── Pre-flight Dry Run ────────────────────────

/// Generate a pre-flight install plan: what WILL happen if install proceeds.
/// Writes content to a temp directory, runs audit::scan_dir, checks existing files.
pub fn preview_install_plan(
    project_path: &str,
    contract: &DesignContract,
) -> Result<DesignInstallPlan, AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!("项目路径不存在: {project_path}")));
    }

    // 1. Generate content
    let design_md_content = generate_design_md(contract)?;
    let dtcg_content = generate_dtchg_json(contract)?;

    // 2. Write to temp dir and run audit scan
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| AppError::Message(format!("创建临时目录失败: {e}")))?;
    let temp_design_md = temp_dir.path().join("DESIGN.md");
    let temp_dtcg = temp_dir.path().join("design-tokens.json");
    write_text_file(&temp_design_md, &design_md_content)?;
    write_text_file(&temp_dtcg, &dtcg_content)?;

    let audit_result = crate::audit::scan_dir(
        temp_dir.path(),
        &crate::audit::AuditContext {
            source: crate::audit::AuditSource::DesignContractInstall {
                contract_name: contract.name.clone(),
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

    let design_md_path = root.join("DESIGN.md");
    let existing_design_md = if design_md_path.is_file() {
        fs::read_to_string(&design_md_path).ok()
    } else {
        None
    };
    files.push(InstallFileEntry {
        path: "DESIGN.md".into(),
        status: if existing_design_md.is_some() { "skip".into() } else { "create".into() },
        new_content: Some(design_md_content),
        existing_content: existing_design_md,
    });

    let dtcg_path = root.join("design-tokens.json");
    let existing_dtcg = if dtcg_path.is_file() {
        fs::read_to_string(&dtcg_path).ok()
    } else {
        None
    };
    files.push(InstallFileEntry {
        path: "design-tokens.json".into(),
        status: if existing_dtcg.is_some() { "skip".into() } else { "create".into() },
        new_content: Some(dtcg_content),
        existing_content: existing_dtcg,
    });

    // Archive file (always create if not exists)
    let slug = contract.name.to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "");
    let archive_rel = format!(".opensunstar/contract/{slug}.contract.md");
    let archive_path = root.join(&archive_rel);
    let archive_exists = archive_path.is_file();
    files.push(InstallFileEntry {
        path: archive_rel,
        status: if archive_exists { "skip".into() } else { "create".into() },
        new_content: None,
        existing_content: None,
    });

    Ok(DesignInstallPlan { files, audit })
}

// ──────────────────────── Install ────────────────────────

/// Install a design contract into a project: write DESIGN.md + DTCG JSON.
/// Never overwrites existing files (safe install).
pub fn install_design_contract(
    project_path: &str,
    contract: &DesignContract,
) -> Result<DesignInstallResult, AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!("项目路径不存在: {project_path}")));
    }

    let mut files_created = Vec::new();
    let mut files_skipped = Vec::new();

    // 1. DESIGN.md
    let design_md_path = root.join("DESIGN.md");
    let design_md_created = if !design_md_path.is_file() {
        let content = generate_design_md(contract)?;
        write_text_file(&design_md_path, &content)?;
        files_created.push("DESIGN.md".into());
        true
    } else {
        files_skipped.push("DESIGN.md".into());
        false
    };

    // 2. design-tokens.json (DTCG format)
    let dtcg_path = root.join("design-tokens.json");
    let dtchg_json_created = if !dtcg_path.is_file() {
        let content = generate_dtchg_json(contract)?;
        write_text_file(&dtcg_path, &content)?;
        files_created.push("design-tokens.json".into());
        true
    } else {
        files_skipped.push("design-tokens.json".into());
        false
    };

    // 3. Archive in .opensunstar/contract/
    let contract_dir = root.join(OPENSUNSTAR_DIR).join(CONTRACT_DIR);
    fs::create_dir_all(&contract_dir)
        .map_err(|e| AppError::io(&contract_dir, e))?;

    let slug = contract.name.to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "");
    let archive_path = contract_dir.join(format!("{slug}.contract.md"));
    if !archive_path.is_file() {
        let content = generate_design_md(contract)?;
        write_text_file(&archive_path, &content)?;
        let rel = format!(".opensunstar/contract/{slug}.contract.md");
        files_created.push(rel);
    }

    // Audit log
    append_orchestration_log(
        project_path,
        serde_json::json!({
            "event": "design_contract_install",
            "name": contract.name,
            "filesCreated": files_created.len(),
            "filesSkipped": files_skipped.len(),
        }),
    )?;

    log::info!(
        "Design contract '{}' installed: {} created, {} skipped",
        contract.name,
        files_created.len(),
        files_skipped.len()
    );

    Ok(DesignInstallResult {
        files_created,
        files_skipped,
        design_md_created,
        dtchg_json_created,
    })
}

// ──────────────────────────── Tests ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid_simple() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("opensunstar-design-{}", uuid_simple()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn list_templates_returns_eight() {
        let templates = list_design_templates();
        assert_eq!(templates.len(), 8);
        assert!(templates.iter().any(|(id, _)| id == "vercel"));
        assert!(templates.iter().any(|(id, _)| id == "neutral"));
    }

    #[test]
    fn get_template_returns_valid_contract() {
        let contract = get_design_template("vercel").unwrap();
        assert_eq!(contract.name, "Vercel Style");
        assert_eq!(contract.colors.primary, "#171717");
        assert!(contract.typography.font_family_base.contains("Geist"));
    }

    #[test]
    fn get_template_unknown_returns_none() {
        assert!(get_design_template("nonexistent").is_none());
    }

    #[test]
    fn compose_from_template() {
        let params = DesignContractParams {
            template_id: Some("stripe".into()),
            name: "My Stripe Project".into(),
            description: Some("Custom description".into()),
            colors: None,
            typography: None,
            spacing: None,
            elevation: None,
            shapes: None,
            components: None,
            guardrails: None,
        };
        let contract = compose_design_contract(&params).unwrap();
        assert_eq!(contract.name, "My Stripe Project");
        assert_eq!(contract.colors.primary, "#635bff"); // Stripe's primary color
        assert_eq!(contract.description, Some("Custom description".into()));
    }

    #[test]
    fn compose_custom_overrides_template() {
        let params = DesignContractParams {
            template_id: Some("vercel".into()),
            name: "Custom".into(),
            description: None,
            colors: Some(DesignColors {
                primary: "#ff0000".into(),
                primary_hover: "#cc0000".into(),
                background: "#111111".into(),
                surface: "#222222".into(),
                text_primary: "#ffffff".into(),
                text_muted: "#aaaaaa".into(),
                accent: "#ff0000".into(),
                success: "#00ff00".into(),
                warning: "#ffff00".into(),
                error: "#ff0000".into(),
                border: "#333333".into(),
                custom: HashMap::new(),
            }),
            typography: None,
            spacing: None,
            elevation: None,
            shapes: None,
            components: None,
            guardrails: None,
        };
        let contract = compose_design_contract(&params).unwrap();
        assert_eq!(contract.colors.primary, "#ff0000");
        // Typography should come from Vercel template
        assert!(contract.typography.font_family_base.contains("Geist"));
    }

    #[test]
    fn generate_design_md_has_frontmatter() {
        let contract = get_design_template("vercel").unwrap();
        let md = generate_design_md(&contract).unwrap();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("\n---\n"));
        assert!(md.contains("# Vercel Style"));
        assert!(md.contains("## Colors"));
        assert!(md.contains("## Typography"));
        assert!(md.contains("## Layout & Spacing"));
        assert!(md.contains("## Elevation & Depth"));
        assert!(md.contains("## Shapes"));
        assert!(md.contains("## Components"));
        assert!(md.contains("## Do's and Don'ts"));
    }

    #[test]
    fn generate_dtchg_json_valid() {
        let contract = get_design_template("apple").unwrap();
        let json_str = generate_dtchg_json(&contract).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.get("color").is_some());
        assert!(parsed.get("fontFamily").is_some());
        assert!(parsed.get("spacing").is_some());
        assert!(parsed.get("borderRadius").is_some());
        assert!(parsed.get("shadow").is_some());

        // Check a specific color
        let primary = parsed["color"]["primary"]["$value"].as_str().unwrap();
        assert_eq!(primary, "#0071e3");
    }

    #[test]
    fn roundtrip_design_md_parse() {
        let original = get_design_template("stripe").unwrap();
        let md = generate_design_md(&original).unwrap();
        let (parsed, _warnings) = parse_design_md(&md).unwrap();

        assert_eq!(parsed.name, "Stripe Style");
        // Colors should round-trip
        assert_eq!(parsed.colors.primary, "#635bff");
        assert_eq!(parsed.colors.text_primary, "#0a2540");
    }

    #[test]
    fn export_writes_files() {
        let root = temp_project();
        let contract = get_design_template("linear").unwrap();
        let content = export_design_contract(root.to_str().unwrap(), &contract).unwrap();

        assert!(root.join("DESIGN.md").is_file());
        assert!(root.join(".opensunstar/contract/linear-style.contract.md").is_file());

        let on_disk = fs::read_to_string(root.join("DESIGN.md")).unwrap();
        assert_eq!(on_disk, content);
    }

    #[test]
    fn install_creates_design_md_and_dtcg() {
        let root = temp_project();
        let contract = get_design_template("notion").unwrap();
        let result = install_design_contract(root.to_str().unwrap(), &contract).unwrap();

        assert!(result.design_md_created);
        assert!(result.dtchg_json_created);
        assert!(root.join("DESIGN.md").is_file());
        assert!(root.join("design-tokens.json").is_file());
        assert!(result.files_created.contains(&"DESIGN.md".to_string()));
        assert!(result.files_created.contains(&"design-tokens.json".to_string()));
    }

    #[test]
    fn install_skips_existing_files() {
        let root = temp_project();
        // Pre-create DESIGN.md
        fs::write(root.join("DESIGN.md"), "# Existing").unwrap();

        let contract = get_design_template("github").unwrap();
        let result = install_design_contract(root.to_str().unwrap(), &contract).unwrap();

        assert!(!result.design_md_created);
        assert!(result.files_skipped.contains(&"DESIGN.md".to_string()));
        // Should not overwrite
        let content = fs::read_to_string(root.join("DESIGN.md")).unwrap();
        assert_eq!(content, "# Existing");
        // DTCG should still be created
        assert!(result.dtchg_json_created);
    }

    #[test]
    fn import_from_file_works() {
        let root = temp_project();
        let contract = get_design_template("vercel").unwrap();
        let md = generate_design_md(&contract).unwrap();
        let file_path = root.join("test-design.md");
        fs::write(&file_path, &md).unwrap();

        let result = import_design_from_file(file_path.to_str().unwrap()).unwrap();
        assert_eq!(result.contract.name, "Vercel Style");
        assert_eq!(result.contract.colors.primary, "#171717");
    }

    #[test]
    fn guardrails_extracted_from_md() {
        let md = "## Do's and Don'ts\n\n- **MUST** use design tokens\n- **MUST NOT** use inline styles\n- **SHOULD** prefer composition\n";
        let guardrails = extract_guardrails_from_md(md);
        assert_eq!(guardrails.len(), 3);
        assert_eq!(guardrails[0].severity, "must");
        assert_eq!(guardrails[1].severity, "must_not");
        assert_eq!(guardrails[2].severity, "should");
    }
}
