use std::path::Path;

use serde_yaml::{Mapping, Value as YamlValue};

use crate::error::AppError;
use crate::hermes_config::{get_hermes_config_path, write_config_section_at_path};
use crate::services::permission_sync::PermissionLists;

pub fn sync_permissions(lists: &PermissionLists) -> Result<(), AppError> {
    sync_permissions_at_path(lists, &get_hermes_config_path())
}

pub fn sync_permissions_at_path(
    lists: &PermissionLists,
    config_path: &Path,
) -> Result<(), AppError> {
    let mut section = Mapping::new();
    let mut allow = lists.allow.clone();
    allow.extend(lists.auto_approve.clone());
    allow.sort();
    allow.dedup();

    if !allow.is_empty() {
        section.insert(
            YamlValue::String("allow".into()),
            YamlValue::Sequence(
                allow
                    .into_iter()
                    .map(|s| YamlValue::String(s))
                    .collect(),
            ),
        );
    }
    if !lists.deny.is_empty() {
        section.insert(
            YamlValue::String("deny".into()),
            YamlValue::Sequence(
                lists
                    .deny
                    .iter()
                    .map(|s| YamlValue::String(s.clone()))
                    .collect(),
            ),
        );
    }

    let value = if section.is_empty() {
        YamlValue::Null
    } else {
        YamlValue::Mapping(section)
    };

    write_config_section_at_path(config_path, "permissions", &value)
}
