//! AI CLI discovery, version checks, installation diagnostics, and lifecycle execution.

mod discovery;
mod lifecycle;
mod version;

#[cfg(target_os = "windows")]
pub(super) const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(crate) use lifecycle::decode_command_output;
pub use lifecycle::{probe_tool_installations, run_tool_lifecycle_action, ToolInstallationReport};
pub use version::{get_tool_versions, ToolVersion, WslShellPreferenceInput};

#[cfg(test)]
mod tests;
