use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::openclaw_config::{read_openclaw_config, write_root_section};
use crate::services::permission_sync::PermissionLists;

pub fn sync_permissions(lists: &PermissionLists) -> Result<(), AppError> {
    let mut config = read_openclaw_config()?;
    let root = config
        .as_object_mut()
        .ok_or_else(|| AppError::Config("OpenClaw 配置根节点不是对象".into()))?;

    let agents = root
        .entry("agents".to_string())
        .or_insert_with(|| json!({}));
    let agents_obj = agents
        .as_object_mut()
        .ok_or_else(|| AppError::Config("OpenClaw agents 节点无效".into()))?;

    let defaults = agents_obj
        .entry("defaults".to_string())
        .or_insert_with(|| json!({}));
    let defaults_obj = defaults
        .as_object_mut()
        .ok_or_else(|| AppError::Config("OpenClaw agents.defaults 节点无效".into()))?;

    let mut allow = lists.allow.clone();
    allow.extend(lists.auto_approve.clone());
    allow.sort();
    allow.dedup();

    let tools = json!({
        "allow": allow,
        "deny": lists.deny
    });

    if allow.is_empty() && lists.deny.is_empty() {
        defaults_obj.remove("tools");
    } else {
        defaults_obj.insert("tools".to_string(), tools);
    }

    let agents_value = root
        .get("agents")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    write_root_section("agents", &agents_value).map(|_| ())
}
