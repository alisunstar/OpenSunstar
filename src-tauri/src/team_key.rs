//! D16 Team Key — client-side team key management
//!
//! Handles:
//! - Keychain entry key namespace: `team/{org_id}/{slot_slug}`
//! - Storing distributed team keys into OS Keychain
//! - Resolving `team-key://{slot_slug}` references to plaintext
//! - Cleanup on membership revocation
//!
//! Red line: plaintext keys ONLY exist in OS Keychain. SQLite stores only
//! `keychain://ref/` references and metadata.

use crate::database::{Database, TeamKeyLocal};
use crate::error::AppError;
use crate::keychain;

/// Prefix for team key references in team profile TOML files.
pub const TEAM_KEY_PREFIX: &str = "team-key://";

/// Build the keychain entry key for a team key slot.
///
/// Format: `team/{org_id}/{slot_slug}`
/// Stored under the single "opensunstar" keyring service.
pub fn team_key_entry_key(org_id: &str, slot_slug: &str) -> String {
    format!("team/{org_id}/{slot_slug}")
}

/// Build a `keychain://ref/` reference for a team key slot.
pub fn team_key_ref(org_id: &str, slot_slug: &str) -> String {
    keychain::make_keychain_ref(&team_key_entry_key(org_id, slot_slug))
}

/// Check if a value is a `team-key://` reference.
pub fn is_team_key_ref(value: &str) -> bool {
    value.starts_with(TEAM_KEY_PREFIX)
}

/// Extract the slot slug from a `team-key://{slot_slug}` reference.
pub fn extract_team_key_slug(value: &str) -> Option<&str> {
    value.strip_prefix(TEAM_KEY_PREFIX)
}

/// Store a team key distributed from the control plane.
///
/// Writes the plaintext to OS Keychain and upserts the reference row in SQLite.
/// The plaintext NEVER touches the database.
pub fn store_team_key(
    db: &Database,
    org_id: &str,
    slot_slug: &str,
    provider_kind: &str,
    endpoint_url: Option<&str>,
    plaintext: &str,
    version_seq: i64,
    value_sha256: &str,
    grant_id: &str,
    grant_expires: i64,
) -> Result<(), AppError> {
    let entry_key = team_key_entry_key(org_id, slot_slug);
    let keychain_ref = keychain::make_keychain_ref(&entry_key);

    // Write plaintext to OS Keychain (the ONLY place it lives)
    keychain::store_secret(&entry_key, plaintext)?;

    // Upsert reference row in SQLite (no plaintext here)
    let now_ms = chrono::Utc::now().timestamp_millis();
    db.upsert_team_key(&TeamKeyLocal {
        slot_slug: slot_slug.to_string(),
        org_id: org_id.to_string(),
        provider_kind: provider_kind.to_string(),
        endpoint_url: endpoint_url.map(|s| s.to_string()),
        keychain_ref,
        version_seq,
        value_sha256: value_sha256.to_string(),
        grant_id: grant_id.to_string(),
        grant_expires,
        last_renewed: now_ms,
        status: "active".to_string(),
    })?;

    log::info!(
        "Team key stored: slot={slot_slug} org={org_id} version={version_seq}"
    );
    Ok(())
}

/// Resolve a `team-key://{slot_slug}` reference to the actual key plaintext.
///
/// Looks up the slot in team_key_local, verifies status is active and grant
/// not expired, then reads the plaintext from OS Keychain.
pub fn resolve_team_key(db: &Database, slot_slug: &str) -> Result<String, AppError> {
    let team_key = db
        .get_team_key(slot_slug)?
        .ok_or_else(|| {
            AppError::Config(format!(
                "team-key://{slot_slug} 无法解析：本机未持有该团队 Key，请执行 team key sync"
            ))
        })?;

    if team_key.status == "revoked" {
        return Err(AppError::Config(format!(
            "team-key://{slot_slug} 已被撤销（团队 membership 已移除）"
        )));
    }

    if team_key.status == "expired" {
        return Err(AppError::Config(format!(
            "team-key://{slot_slug} 已过期（grant TTL 超时），请重新同步"
        )));
    }

    // Check grant expiry
    let now_ms = chrono::Utc::now().timestamp_millis();
    if now_ms > team_key.grant_expires {
        // Mark as expired locally
        let _ = db.update_team_key_status(slot_slug, "expired");
        return Err(AppError::Config(format!(
            "team-key://{slot_slug} grant 已过期，请执行 team key sync 续期"
        )));
    }

    // Resolve from Keychain
    let entry_key = team_key_entry_key(&team_key.org_id, slot_slug);
    keychain::get_secret(&entry_key)?
        .ok_or_else(|| {
            AppError::Config(format!(
                "team-key://{slot_slug} Keychain 条目丢失，请执行 team key sync 重新下发"
            ))
        })
}

/// Resolve a value that might be a `team-key://` reference.
///
/// If the value starts with `team-key://`, resolves it via the team_key_local
/// table + Keychain. Otherwise returns the value unchanged.
pub fn resolve_team_value(db: &Database, value: &str) -> Result<String, AppError> {
    if let Some(slot_slug) = extract_team_key_slug(value) {
        resolve_team_key(db, slot_slug)
    } else {
        Ok(value.to_string())
    }
}

/// Remove a team key from Keychain and mark as revoked in SQLite.
pub fn revoke_team_key(db: &Database, slot_slug: &str) -> Result<(), AppError> {
    if let Some(team_key) = db.get_team_key(slot_slug)? {
        let entry_key = team_key_entry_key(&team_key.org_id, slot_slug);
        keychain::delete_secret(&entry_key)?;
        db.update_team_key_status(slot_slug, "revoked")?;
        log::info!("Team key revoked: slot={slot_slug}");
    }
    Ok(())
}

/// Remove all team keys for an org (membership revocation cleanup).
pub fn revoke_all_team_keys_for_org(db: &Database, org_id: &str) -> Result<u64, AppError> {
    let keys = db.list_team_keys(org_id)?;
    for key in &keys {
        let entry_key = team_key_entry_key(org_id, &key.slot_slug);
        let _ = keychain::delete_secret(&entry_key);
    }
    let count = db.delete_team_keys_for_org(org_id)?;
    log::info!("Revoked all team keys for org={org_id}, count={count}");
    Ok(count)
}

/// Unified value resolver for team config deployment.
///
/// Handles three cases:
/// 1. `team-key://{slot_slug}` → resolves via team_key_local + Keychain
/// 2. `keychain://ref/{entry_key}` → resolves via Keychain directly
/// 3. Plain string → returned as-is
///
/// This is the single entry point for resolving credential references when
/// deploying team profiles to local CLI configurations.
pub fn resolve_any_value(db: &Database, value: &str) -> Result<String, AppError> {
    if is_team_key_ref(value) {
        let slot_slug = extract_team_key_slug(value).unwrap_or_default();
        resolve_team_key(db, slot_slug)
    } else {
        // Handles both keychain://ref/ and plain strings
        keychain::resolve_value(value)
    }
}

/// Check grant expiry for all local team keys and mark expired ones.
///
/// Called periodically (e.g., on app startup or before config deployment)
/// to ensure stale grants are flagged without a network call.
pub fn check_grant_expiry(db: &Database) -> Result<u32, AppError> {
    let keys = db.list_all_team_keys()?;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut expired_count = 0u32;

    for key in &keys {
        if key.status == "active" && now_ms > key.grant_expires {
            db.update_team_key_status(&key.slot_slug, "expired")?;
            expired_count += 1;
            log::info!(
                "Team key grant expired locally: slot={}",
                key.slot_slug
            );
        }
    }

    Ok(expired_count)
}
