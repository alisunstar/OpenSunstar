pub mod claude;
pub mod codex;
pub mod gemini;
pub mod hermes;
pub mod openclaw;
pub mod opencode;
pub mod shared;

pub use shared::{StatusOutcome, WriteOutcome, MANAGED_MARKER};

pub const ALL_TOOLS: &[&str] = &[
    "claude-code",
    "codex",
    "gemini-cli",
    "opencode",
    "openclaw",
    "hermes",
];

pub const PHASE1_TOOLS: &[&str] = ALL_TOOLS;

pub fn tool_paths(tool: &str) -> Result<Vec<std::path::PathBuf>, crate::error::AppError> {
    match tool {
        "claude-code" => Ok(claude::paths()),
        "codex" => Ok(codex::paths()),
        "gemini-cli" => Ok(gemini::paths()),
        "opencode" => Ok(opencode::paths()),
        "openclaw" => Ok(openclaw::paths()),
        "hermes" => Ok(hermes::paths()),
        other => Err(crate::error::AppError::Message(format!(
            "未知工具: {other}"
        ))),
    }
}

pub fn tool_status(tool: &str) -> Result<StatusOutcome, crate::error::AppError> {
    match tool {
        "claude-code" => claude::status(),
        "codex" => codex::status(),
        "gemini-cli" => gemini::status(),
        "opencode" => opencode::status(),
        "openclaw" => openclaw::status(),
        "hermes" => hermes::status(),
        other => Err(crate::error::AppError::Message(format!(
            "未知工具: {other}"
        ))),
    }
}
