//! Team configuration Spike: deterministic resolution of independent policy
//! sources before they are compiled into project expectations.

pub mod requirements;
pub mod security;

pub use requirements::{
    resolve_requirement_sources, PolicyAction, RequirementKey, RequirementResolution,
    RequirementResolutionPlan, RequirementSource, ResolutionConflict,
};
pub use security::{
    enforce_team_package_security, validate_team_package_security, TeamPackageSecurityFinding,
    TeamPackageSecurityReport,
};
