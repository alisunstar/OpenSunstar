//! Team configuration Spike: deterministic resolution of independent policy
//! sources before they are compiled into project expectations.

pub mod requirements;

pub use requirements::{
    resolve_requirement_sources, PolicyAction, RequirementKey, RequirementResolution,
    RequirementResolutionPlan, RequirementSource, ResolutionConflict,
};
