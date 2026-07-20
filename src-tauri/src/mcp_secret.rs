//! MCP credential protection.
//!
//! Persisted MCP server definitions must contain only Keychain references for
//! credential-bearing values. Plaintext is resolved only at an execution or
//! adapter boundary and is masked before a server definition crosses the
//! frontend boundary.

use crate::app_config::McpServer;
use crate::database::Database;
use crate::error::AppError;
use crate::keychain;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;

pub const MCP_SECRET_MASK: &str = "********";
const KEYCHAIN_REF_PREFIX: &str = "keychain://ref/";

pub fn protect_server_for_storage(
    server: &McpServer,
    existing: Option<&McpServer>,
) -> Result<McpServer, AppError> {
    let mut protected = server.clone();
    protected.server =
        hydrate_masked_spec(&server.server, existing.map(|existing| &existing.server))?;
    protect_value(
        &server.id,
        &mut protected.server,
        "$",
        SecretContext::Normal,
    )?;
    Ok(protected)
}

pub fn mask_server_for_frontend(server: &McpServer) -> McpServer {
    let mut masked = server.clone();
    mask_value(&mut masked.server);
    masked
}

pub fn resolve_spec_for_use(spec: &Value) -> Result<Value, AppError> {
    let mut resolved = spec.clone();
    resolve_value(&mut resolved)?;
    Ok(resolved)
}

pub fn hydrate_masked_spec(incoming: &Value, existing: Option<&Value>) -> Result<Value, AppError> {
    hydrate_value(incoming, existing, "$")
}

pub fn delete_server_secret_refs(server: &McpServer) -> Result<(), AppError> {
    for entry_key in collect_secret_ref_keys(&server.server) {
        keychain::delete_secret(&entry_key)?;
    }
    Ok(())
}

pub fn delete_stale_server_secret_refs(
    previous: &McpServer,
    current: &McpServer,
) -> Result<(), AppError> {
    let current_refs = collect_secret_ref_keys(&current.server);
    for entry_key in collect_secret_ref_keys(&previous.server) {
        if !current_refs.contains(&entry_key) {
            keychain::delete_secret(&entry_key)?;
        }
    }
    Ok(())
}

pub fn migrate_all_mcp_servers_if_needed(db: &Database) -> Result<usize, AppError> {
    let servers = db.get_all_mcp_servers()?;
    let mut migrated = 0;
    for server in servers.values() {
        let protected = protect_server_for_storage(server, Some(server))?;
        if protected.server != server.server {
            db.save_mcp_server(&protected)?;
            migrated += 1;
        }
    }
    Ok(migrated)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SecretContext {
    Normal,
    ForceSecret,
}

fn protect_value(
    server_id: &str,
    value: &mut Value,
    path: &str,
    context: SecretContext,
) -> Result<(), AppError> {
    match value {
        Value::Object(map) => {
            let keys = map.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                let Some(child) = map.get_mut(&key) else {
                    continue;
                };
                let child_path = format!("{path}/{}", escape_path_segment(&key));
                let normalized = normalize_key(&key);
                if normalized == "args" {
                    protect_args(server_id, child, &child_path)?;
                } else if normalized == "env" || normalized == "environment" {
                    protect_value(server_id, child, &child_path, SecretContext::ForceSecret)?;
                } else if normalized == "headers" {
                    protect_headers(server_id, child, &child_path)?;
                } else if normalized == "url" || normalized.ends_with("url") {
                    protect_url_value(server_id, child, &child_path)?;
                } else {
                    let child_context =
                        if context == SecretContext::ForceSecret || is_sensitive_key(&key) {
                            SecretContext::ForceSecret
                        } else {
                            SecretContext::Normal
                        };
                    protect_value(server_id, child, &child_path, child_context)?;
                }
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter_mut().enumerate() {
                protect_value(server_id, child, &format!("{path}/{index}"), context)?;
            }
        }
        Value::String(text) => {
            if let Some(protected_url) = protect_url_string(server_id, text, path)? {
                *text = protected_url;
            } else if context == SecretContext::ForceSecret || looks_like_secret(text) {
                *text = protect_secret_string(server_id, path, text)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn protect_headers(server_id: &str, value: &mut Value, path: &str) -> Result<(), AppError> {
    let Value::Object(headers) = value else {
        return protect_value(server_id, value, path, SecretContext::Normal);
    };
    for (name, header_value) in headers.iter_mut() {
        let context = if is_sensitive_header(name) {
            SecretContext::ForceSecret
        } else {
            SecretContext::Normal
        };
        protect_value(
            server_id,
            header_value,
            &format!("{path}/{}", escape_path_segment(name)),
            context,
        )?;
    }
    Ok(())
}

fn protect_args(server_id: &str, value: &mut Value, path: &str) -> Result<(), AppError> {
    let Value::Array(args) = value else {
        return protect_value(server_id, value, path, SecretContext::Normal);
    };
    let mut next_is_secret = false;
    let mut next_is_header = false;
    for (index, arg) in args.iter_mut().enumerate() {
        let Value::String(text) = arg else {
            continue;
        };
        let item_path = format!("{path}/{index}");
        if next_is_secret {
            *text = protect_secret_string(server_id, &item_path, text)?;
            next_is_secret = false;
            continue;
        }
        if next_is_header {
            *text = protect_header_argument(server_id, text, &item_path)?;
            next_is_header = false;
            continue;
        }

        let trimmed = text.trim();
        if trimmed == "-H" || trimmed.eq_ignore_ascii_case("--header") {
            next_is_header = true;
            continue;
        }
        if is_sensitive_flag(trimmed) {
            next_is_secret = true;
            continue;
        }
        if let Some((left, right)) = text.split_once('=') {
            if is_sensitive_key(left.trim_start_matches('-')) {
                let protected = protect_secret_string(server_id, &item_path, right)?;
                *text = format!("{left}={protected}");
                continue;
            }
            if left.eq_ignore_ascii_case("--header") || left.eq_ignore_ascii_case("-H") {
                let protected = protect_header_argument(server_id, right, &item_path)?;
                *text = format!("{left}={protected}");
                continue;
            }
        }
        if looks_like_header_argument(text) {
            *text = protect_header_argument(server_id, text, &item_path)?;
            continue;
        }
        if let Some(protected) = protect_url_fragment(server_id, text, &item_path)? {
            *text = protected;
            continue;
        }
        if looks_like_secret(text) {
            *text = protect_secret_string(server_id, &item_path, text)?;
        }
    }
    Ok(())
}

fn protect_header_argument(server_id: &str, text: &str, path: &str) -> Result<String, AppError> {
    let Some((name, value)) = text.split_once(':') else {
        return Ok(text.to_string());
    };
    if !is_sensitive_header(name.trim()) {
        return Ok(text.to_string());
    }
    let leading_ws_len = value.len() - value.trim_start().len();
    let leading_ws = &value[..leading_ws_len];
    let protected = protect_secret_string(server_id, path, value.trim_start())?;
    Ok(format!("{name}:{leading_ws}{protected}"))
}

fn protect_url_value(server_id: &str, value: &mut Value, path: &str) -> Result<(), AppError> {
    if let Value::String(text) = value {
        if let Some(protected) = protect_url_string(server_id, text, path)? {
            *text = protected;
        }
    } else {
        protect_value(server_id, value, path, SecretContext::Normal)?;
    }
    Ok(())
}

fn protect_url_fragment(
    server_id: &str,
    text: &str,
    path: &str,
) -> Result<Option<String>, AppError> {
    let Some(start) = find_url_start(text) else {
        return Ok(None);
    };
    let (prefix, url_text) = text.split_at(start);
    let Some(protected) = protect_url_string(server_id, url_text, path)? else {
        return Ok(None);
    };
    Ok(Some(format!("{prefix}{protected}")))
}

fn protect_url_string(server_id: &str, text: &str, path: &str) -> Result<Option<String>, AppError> {
    let Ok(mut url) = Url::parse(text) else {
        return Ok(None);
    };
    let pairs = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        return Ok(None);
    }
    let mut changed = false;
    let mut protected_pairs = Vec::with_capacity(pairs.len());
    for (index, (key, value)) in pairs.into_iter().enumerate() {
        if !value.is_empty()
            && (keychain::is_keychain_ref(&value)
                || is_sensitive_key(&key)
                || looks_like_secret(&value))
        {
            let protected = protect_secret_string(
                server_id,
                &format!("{path}/query/{index}/{}", escape_path_segment(&key)),
                &value,
            )?;
            changed |= protected != value;
            protected_pairs.push((key, protected));
        } else {
            protected_pairs.push((key, value));
        }
    }
    if !changed {
        return Ok(None);
    }
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in protected_pairs {
            query.append_pair(&key, &value);
        }
    }
    Ok(Some(url.into()))
}

fn protect_secret_string(server_id: &str, path: &str, value: &str) -> Result<String, AppError> {
    // A string may contain an embedded ref (URL query or `Header: value`). It
    // is already protected and must not be wrapped in a second SecretRef;
    // double-wrapping would require recursive resolution and breaks migration
    // idempotency.
    if value.is_empty() || !collect_ref_keys_from_string(value).is_empty() {
        return Ok(value.to_string());
    }
    if value == MCP_SECRET_MASK {
        return Err(AppError::InvalidInput(
            "MCP secret mask has no existing SecretRef to preserve".into(),
        ));
    }
    let entry_key = secret_entry_key(server_id, path);
    keychain::store_secret(&entry_key, value)?;
    Ok(keychain::make_keychain_ref(&entry_key))
}

fn resolve_value(value: &mut Value) -> Result<(), AppError> {
    match value {
        Value::Object(map) => {
            for child in map.values_mut() {
                resolve_value(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                resolve_value(child)?;
            }
        }
        Value::String(text) => {
            *text = transform_secret_string(text, SecretStringMode::Resolve)?;
        }
        _ => {}
    }
    Ok(())
}

fn mask_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for child in map.values_mut() {
                mask_value(child);
            }
        }
        Value::Array(values) => {
            for child in values {
                mask_value(child);
            }
        }
        Value::String(text) => {
            if let Ok(masked) = transform_secret_string(text, SecretStringMode::Mask) {
                *text = masked;
            }
        }
        _ => {}
    }
}

#[derive(Clone, Copy)]
enum SecretStringMode {
    Resolve,
    Mask,
}

fn transform_secret_string(text: &str, mode: SecretStringMode) -> Result<String, AppError> {
    let with_url = transform_url_fragment_for_output(text, mode)?;
    replace_raw_refs(&with_url, mode)
}

fn transform_url_fragment_for_output(
    text: &str,
    mode: SecretStringMode,
) -> Result<String, AppError> {
    let Some(start) = find_url_start(text) else {
        return Ok(text.to_string());
    };
    let (prefix, url_text) = text.split_at(start);
    let Ok(mut url) = Url::parse(url_text) else {
        return Ok(text.to_string());
    };
    let pairs = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        return Ok(text.to_string());
    }
    let mut changed = false;
    let mut output_pairs = Vec::with_capacity(pairs.len());
    for (key, value) in pairs {
        if keychain::is_keychain_ref(&value) {
            changed = true;
            let transformed = match mode {
                SecretStringMode::Resolve => keychain::resolve_value(&value)?,
                SecretStringMode::Mask => MCP_SECRET_MASK.to_string(),
            };
            output_pairs.push((key, transformed));
        } else {
            output_pairs.push((key, value));
        }
    }
    if !changed {
        return Ok(text.to_string());
    }
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in output_pairs {
            query.append_pair(&key, &value);
        }
    }
    Ok(format!("{prefix}{}", String::from(url)))
}

fn replace_raw_refs(text: &str, mode: SecretStringMode) -> Result<String, AppError> {
    let refs = raw_ref_tokens(text);
    if refs.is_empty() {
        return Ok(text.to_string());
    }
    let mut output = text.to_string();
    for reference in refs {
        let replacement = match mode {
            SecretStringMode::Resolve => keychain::resolve_value(&reference)?,
            SecretStringMode::Mask => MCP_SECRET_MASK.to_string(),
        };
        output = output.replacen(&reference, &replacement, 1);
    }
    Ok(output)
}

fn hydrate_value(
    incoming: &Value,
    existing: Option<&Value>,
    path: &str,
) -> Result<Value, AppError> {
    match incoming {
        Value::Object(map) => {
            let mut hydrated = serde_json::Map::new();
            for (key, child) in map {
                let existing_child = existing
                    .and_then(Value::as_object)
                    .and_then(|object| object.get(key));
                hydrated.insert(
                    key.clone(),
                    hydrate_value(
                        child,
                        existing_child,
                        &format!("{path}/{}", escape_path_segment(key)),
                    )?,
                );
            }
            Ok(Value::Object(hydrated))
        }
        Value::Array(values) => {
            let existing_values = existing.and_then(Value::as_array);
            values
                .iter()
                .enumerate()
                .map(|(index, child)| {
                    hydrate_value(
                        child,
                        existing_values.and_then(|values| values.get(index)),
                        &format!("{path}/{index}"),
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        Value::String(text) if text.contains(MCP_SECRET_MASK) => {
            let existing_text = existing.and_then(Value::as_str).ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "MCP secret mask at {path} has no existing SecretRef"
                ))
            })?;
            Ok(Value::String(hydrate_masked_string(
                text,
                existing_text,
                path,
            )?))
        }
        _ => Ok(incoming.clone()),
    }
}

fn hydrate_masked_string(incoming: &str, existing: &str, path: &str) -> Result<String, AppError> {
    if incoming == MCP_SECRET_MASK {
        if collect_ref_keys_from_string(existing).is_empty() {
            return Err(AppError::InvalidInput(format!(
                "MCP secret mask at {path} does not reference a stored secret"
            )));
        }
        return Ok(existing.to_string());
    }

    let hydrated_url = hydrate_masked_url_fragment(incoming, existing)?;
    if !hydrated_url.contains(MCP_SECRET_MASK) {
        return Ok(hydrated_url);
    }

    let references = raw_ref_tokens(existing);
    let mut output = hydrated_url;
    for reference in references {
        if !output.contains(MCP_SECRET_MASK) {
            break;
        }
        output = output.replacen(MCP_SECRET_MASK, &reference, 1);
    }
    if output.contains(MCP_SECRET_MASK) {
        return Err(AppError::InvalidInput(format!(
            "MCP secret mask at {path} could not be matched to an existing SecretRef"
        )));
    }
    Ok(output)
}

fn hydrate_masked_url_fragment(incoming: &str, existing: &str) -> Result<String, AppError> {
    let (Some(incoming_start), Some(existing_start)) =
        (find_url_start(incoming), find_url_start(existing))
    else {
        return Ok(incoming.to_string());
    };
    let (incoming_prefix, incoming_url_text) = incoming.split_at(incoming_start);
    let (_, existing_url_text) = existing.split_at(existing_start);
    let (Ok(mut incoming_url), Ok(existing_url)) =
        (Url::parse(incoming_url_text), Url::parse(existing_url_text))
    else {
        return Ok(incoming.to_string());
    };
    let incoming_pairs = incoming_url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    let existing_pairs = existing_url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    if incoming_pairs.is_empty() {
        return Ok(incoming.to_string());
    }
    let mut changed = false;
    let mut hydrated_pairs = Vec::with_capacity(incoming_pairs.len());
    for (index, (key, value)) in incoming_pairs.into_iter().enumerate() {
        if value == MCP_SECRET_MASK {
            let existing_value = existing_pairs
                .get(index)
                .filter(|(existing_key, _)| existing_key == &key)
                .map(|(_, value)| value)
                .filter(|value| keychain::is_keychain_ref(value))
                .ok_or_else(|| {
                    AppError::InvalidInput(format!(
                        "MCP URL secret mask for query parameter '{key}' has no existing SecretRef"
                    ))
                })?;
            hydrated_pairs.push((key, existing_value.clone()));
            changed = true;
        } else {
            hydrated_pairs.push((key, value));
        }
    }
    if !changed {
        return Ok(incoming.to_string());
    }
    incoming_url.set_query(None);
    {
        let mut query = incoming_url.query_pairs_mut();
        for (key, value) in hydrated_pairs {
            query.append_pair(&key, &value);
        }
    }
    Ok(format!("{incoming_prefix}{}", String::from(incoming_url)))
}

fn collect_secret_ref_keys(value: &Value) -> HashSet<String> {
    let mut refs = HashSet::new();
    collect_refs_from_value(value, &mut refs);
    refs
}

fn collect_refs_from_value(value: &Value, refs: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            for child in map.values() {
                collect_refs_from_value(child, refs);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_refs_from_value(child, refs);
            }
        }
        Value::String(text) => {
            refs.extend(collect_ref_keys_from_string(text));
        }
        _ => {}
    }
}

fn collect_ref_keys_from_string(text: &str) -> HashSet<String> {
    let mut refs = raw_ref_tokens(text)
        .into_iter()
        .filter_map(|reference| keychain::extract_ref_key(&reference).map(str::to_string))
        .collect::<HashSet<_>>();
    if let Some(start) = find_url_start(text) {
        if let Ok(url) = Url::parse(&text[start..]) {
            for (_, value) in url.query_pairs() {
                if let Some(entry_key) = keychain::extract_ref_key(&value) {
                    refs.insert(entry_key.to_string());
                }
            }
        }
    }
    refs
}

fn raw_ref_tokens(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut offset = 0;
    while let Some(relative_start) = text[offset..].find(KEYCHAIN_REF_PREFIX) {
        let start = offset + relative_start;
        let tail = &text[start..];
        let end = tail
            .char_indices()
            .skip(KEYCHAIN_REF_PREFIX.chars().count())
            .find(|(_, ch)| !ch.is_ascii_alphanumeric() && !matches!(ch, '/' | '_' | '-' | '.'))
            .map(|(index, _)| index)
            .unwrap_or(tail.len());
        refs.push(tail[..end].to_string());
        offset = start + end;
    }
    refs
}

fn find_url_start(text: &str) -> Option<usize> {
    match (text.find("https://"), text.find("http://")) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn looks_like_header_argument(value: &str) -> bool {
    value
        .split_once(':')
        .is_some_and(|(name, _)| is_sensitive_header(name.trim()))
}

fn is_sensitive_flag(value: &str) -> bool {
    if !value.starts_with('-') {
        return false;
    }
    // Inline values (for example `--endpoint=https://...?token=...`) are
    // inspected by the inline-value / URL handlers. Looking at the complete
    // argument here would mistake a sensitive value for a sensitive flag name.
    if value.contains('=') {
        return false;
    }
    let flag = value.trim_start_matches('-');
    !flag.is_empty() && is_sensitive_key(flag)
}

fn is_sensitive_header(name: &str) -> bool {
    let normalized = normalize_key(name);
    matches!(
        normalized.as_str(),
        "authorization" | "proxyauthorization" | "cookie" | "setcookie"
    ) || is_sensitive_key(name)
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    [
        "token",
        "secret",
        "password",
        "passwd",
        "apikey",
        "authorization",
        "cookie",
        "credential",
        "privatekey",
        "accesskey",
        "sessionid",
        "clientkey",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn looks_like_secret(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("bearer ")
        || lower.starts_with("basic ")
        || trimmed.starts_with("sk-")
        || trimmed.starts_with("ghp_")
        || trimmed.starts_with("github_pat_")
        || trimmed.starts_with("xoxb-")
        || trimmed.starts_with("xoxp-")
        || trimmed.starts_with("AKIA")
    {
        return true;
    }
    if trimmed.len() < 32 || trimmed.chars().any(char::is_whitespace) {
        return false;
    }
    let allowed = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~'))
        .count();
    allowed * 100 / trimmed.chars().count().max(1) >= 90
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn escape_path_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn secret_entry_key(server_id: &str, path: &str) -> String {
    let server_hash = format!("{:x}", Sha256::digest(server_id.as_bytes()));
    let path_hash = format!("{:x}", Sha256::digest(path.as_bytes()));
    format!("mcp/{server_hash}/{path_hash}")
}

#[cfg(test)]
pub(crate) fn adapter_secret_fixture(id: &str) -> (Value, Value, String) {
    let entry_key = format!("mcp-test/adapter/{id}");
    let secret = format!("adapter-secret-{id}-9f31");
    keychain::store_secret(&entry_key, &secret).expect("store adapter fixture secret");
    let reference = keychain::make_keychain_ref(&entry_key);
    (
        serde_json::json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "fixture-server", "--api-key", reference],
            "env": { "MCP_TOKEN": reference }
        }),
        serde_json::json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "fixture-server", "--api-key", secret],
            "env": { "MCP_TOKEN": secret }
        }),
        entry_key,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::McpApps;
    use crate::keychain;
    use serde_json::json;

    const ENV_SECRET: &str = "env-secret-4f2fd9a40d1a";
    const AUTH_SECRET: &str = "Bearer auth-secret-2f40c7f1";
    const COOKIE_SECRET: &str = "session=cookie-secret-9316";
    const QUERY_SECRET: &str = "query-secret-a731";
    const ARG_SECRET: &str = "arg-secret-b422";
    const EXT_SECRET: &str = "extension-secret-d309";

    fn fixture_server(id: &str) -> McpServer {
        McpServer {
            id: id.into(),
            name: "Secret Fixture".into(),
            server: json!({
                "type": "stdio",
                "command": "node",
                "args": [
                    "server.js",
                    "--api-key",
                    ARG_SECRET,
                    format!("--endpoint=https://mcp.example.test/run?access_token={QUERY_SECRET}&mode=fast"),
                    format!("Authorization: {AUTH_SECRET}")
                ],
                "env": {
                    "MCP_TOKEN": ENV_SECRET,
                    "NON_SECRET_BUT_LOCAL": "device-specific-value"
                },
                "url": format!("https://mcp.example.test/sse?token={QUERY_SECRET}&version=v1"),
                "headers": {
                    "Authorization": AUTH_SECRET,
                    "Cookie": COOKIE_SECRET,
                    "X-Trace-Id": "trace-visible"
                },
                "transportOptions": {
                    "clientSecret": EXT_SECRET,
                    "region": "eu-west-1"
                }
            }),
            apps: McpApps {
                claude: true,
                codex: true,
                gemini: true,
                opencode: true,
                hermes: true,
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        }
    }

    fn assert_no_fixture_secret(value: &Value) {
        let serialized = serde_json::to_string(value).expect("serialize fixture");
        for secret in [
            ENV_SECRET,
            AUTH_SECRET,
            COOKIE_SECRET,
            QUERY_SECRET,
            ARG_SECRET,
            EXT_SECRET,
            "device-specific-value",
        ] {
            assert!(
                !serialized.contains(secret),
                "persisted or frontend value leaked fixture secret: {secret}; value={serialized}"
            );
        }
    }

    #[test]
    fn protects_and_resolves_env_headers_query_args_and_extension_fields() {
        let original = fixture_server("secret-fixture-roundtrip");

        let protected =
            protect_server_for_storage(&original, None).expect("protect MCP fixture for storage");
        assert_no_fixture_secret(&protected.server);
        assert!(serde_json::to_string(&protected.server)
            .expect("serialize protected fixture")
            .contains("keychain"));

        let resolved = resolve_spec_for_use(&protected.server).expect("resolve MCP fixture");
        assert_eq!(resolved, original.server);
    }

    #[test]
    fn frontend_mask_contains_neither_plaintext_nor_keychain_reference() {
        let protected = protect_server_for_storage(&fixture_server("secret-fixture-mask"), None)
            .expect("protect MCP fixture");

        let masked = mask_server_for_frontend(&protected);
        assert_no_fixture_secret(&masked.server);
        let serialized = serde_json::to_string(&masked.server).expect("serialize masked fixture");
        assert!(!serialized.contains("keychain://ref/"));
        assert!(serialized.contains(MCP_SECRET_MASK));
        assert!(serialized.contains("trace-visible"));
    }

    #[test]
    fn masked_roundtrip_preserves_existing_refs_and_allows_nonsecret_edits() {
        let original = fixture_server("secret-fixture-masked-update");
        let protected = protect_server_for_storage(&original, None).expect("protect MCP fixture");
        let mut masked = mask_server_for_frontend(&protected);
        masked.server["cwd"] = json!("C:/workspace/changed");

        let reprotected = protect_server_for_storage(&masked, Some(&protected))
            .expect("preserve refs from masked update");
        assert_no_fixture_secret(&reprotected.server);

        let resolved = resolve_spec_for_use(&reprotected.server).expect("resolve masked update");
        assert_eq!(resolved["env"], original.server["env"]);
        assert_eq!(resolved["headers"], original.server["headers"]);
        assert_eq!(resolved["url"], original.server["url"]);
        assert_eq!(resolved["args"], original.server["args"]);
        assert_eq!(
            resolved["transportOptions"],
            original.server["transportOptions"]
        );
        assert_eq!(resolved["cwd"], "C:/workspace/changed");
    }

    #[test]
    fn deleting_server_removes_all_referenced_keychain_entries() {
        let protected = protect_server_for_storage(&fixture_server("secret-fixture-delete"), None)
            .expect("protect MCP fixture");
        let entry_keys = collect_secret_ref_keys(&protected.server);
        assert!(!entry_keys.is_empty());

        delete_server_secret_refs(&protected).expect("delete MCP secrets");
        for entry_key in entry_keys {
            assert_eq!(
                keychain::get_secret(&entry_key).expect("read test keychain"),
                None
            );
        }
    }

    #[test]
    fn dao_enforces_protection_masked_updates_and_secret_cleanup() {
        let db = Database::memory().expect("memory database");
        let original = fixture_server("secret-fixture-dao-gate");
        db.save_mcp_server(&original)
            .expect("DAO protects plaintext MCP fixture");

        let stored = db
            .get_all_mcp_servers()
            .expect("read stored MCP fixture")
            .shift_remove(&original.id)
            .expect("stored fixture exists");
        assert_no_fixture_secret(&stored.server);
        assert_eq!(
            resolve_spec_for_use(&stored.server).expect("resolve DAO-protected fixture"),
            original.server
        );
        let sql = db.export_sql_string().expect("export local database SQL");
        assert_no_fixture_secret(&json!(sql));

        let cookie_ref = stored.server["headers"]["Cookie"]
            .as_str()
            .and_then(keychain::extract_ref_key)
            .expect("cookie SecretRef")
            .to_string();
        let mut masked = mask_server_for_frontend(&stored);
        masked.server["cwd"] = json!("C:/workspace/dao-update");
        masked
            .server
            .get_mut("headers")
            .and_then(Value::as_object_mut)
            .expect("headers object")
            .remove("Cookie");
        db.save_mcp_server(&masked)
            .expect("DAO hydrates masked update");
        assert_eq!(
            keychain::get_secret(&cookie_ref).expect("read removed cookie secret"),
            None
        );

        let updated = db
            .get_all_mcp_servers()
            .expect("read updated MCP fixture")
            .shift_remove(&original.id)
            .expect("updated fixture exists");
        let remaining_refs = collect_secret_ref_keys(&updated.server);
        let resolved = resolve_spec_for_use(&updated.server).expect("resolve updated fixture");
        assert_eq!(resolved["cwd"], "C:/workspace/dao-update");
        assert!(resolved["headers"].get("Cookie").is_none());

        db.delete_mcp_server(&original.id)
            .expect("DAO deletes MCP fixture and secrets");
        for entry_key in remaining_refs {
            assert_eq!(
                keychain::get_secret(&entry_key).expect("read deleted DAO secret"),
                None
            );
        }
    }

    #[test]
    fn startup_migration_rewrites_legacy_plaintext_rows_idempotently() {
        let db = Database::memory().expect("memory database");
        let legacy = fixture_server("secret-fixture-migration");
        {
            let conn = db.conn.lock().expect("database lock");
            conn.execute(
                "INSERT INTO mcp_servers (
                    id, name, server_config, description, homepage, docs, tags,
                    enabled_claude, enabled_codex, enabled_gemini, enabled_opencode, enabled_hermes
                 ) VALUES (?1, ?2, ?3, NULL, NULL, NULL, '[]', 1, 1, 1, 1, 1)",
                rusqlite::params![
                    legacy.id,
                    legacy.name,
                    serde_json::to_string(&legacy.server).expect("serialize legacy fixture")
                ],
            )
            .expect("insert legacy plaintext row");
        }

        assert_eq!(
            migrate_all_mcp_servers_if_needed(&db).expect("migrate legacy MCP row"),
            1
        );
        let migrated = db
            .get_all_mcp_servers()
            .expect("read migrated MCP rows")
            .shift_remove("secret-fixture-migration")
            .expect("migrated fixture exists");
        assert_no_fixture_secret(&migrated.server);
        assert_eq!(
            resolve_spec_for_use(&migrated.server).expect("resolve migrated fixture"),
            legacy.server
        );
        assert_eq!(
            migrate_all_mcp_servers_if_needed(&db).expect("repeat migration"),
            0
        );
    }

    #[test]
    fn sync_snapshot_excludes_mcp_rows_and_preserves_local_rows_on_import() {
        let source = Database::memory().expect("source database");
        let target = Database::memory().expect("target database");
        source
            .save_mcp_server(&fixture_server("secret-fixture-sync-source"))
            .expect("save source MCP fixture");

        let mut local = fixture_server("target-local");
        local.name = "Target Local".into();
        target
            .save_mcp_server(&local)
            .expect("save target-local MCP fixture");

        let sql = source
            .export_sql_string_for_sync()
            .expect("export sync SQL");
        assert!(!sql.contains("secret-fixture-sync-source"));
        assert_no_fixture_secret(&json!(sql));

        target
            .import_sql_string_for_sync(&sql)
            .expect("import sync SQL");
        let target_servers = target.get_all_mcp_servers().expect("read target MCP rows");
        assert!(target_servers.contains_key("target-local"));
        assert!(!target_servers.contains_key("secret-fixture-sync-source"));
    }
}
