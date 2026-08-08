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
///
/// W-2 修复：存储前校验 sha256(plaintext) == value_sha256，检测 MITM 篡改。
#[allow(clippy::too_many_arguments)]
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
    // I-6: 拒绝空 plaintext
    if plaintext.is_empty() {
        return Err(AppError::Config(format!(
            "team key slot={slot_slug} 下发值为空，拒绝存储"
        )));
    }

    // W-2: 完整性校验（防 MITM 替换）
    if !value_sha256.is_empty() {
        let computed = crate::team_config::release::sha256_of_content(plaintext.as_bytes());
        if computed != value_sha256 {
            return Err(AppError::Config(format!(
                "team key slot={slot_slug} 完整性校验失败: 期望 {value_sha256}, 实际 {computed}"
            )));
        }
    }

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

    log::info!("Team key stored: slot={slot_slug} org={org_id} version={version_seq}");
    Ok(())
}

/// Resolve a `team-key://{slot_slug}` reference to the actual key plaintext.
///
/// Looks up the slot in team_key_local, verifies status is active and grant
/// not expired, then reads the plaintext from OS Keychain.
pub fn resolve_team_key(db: &Database, slot_slug: &str) -> Result<String, AppError> {
    let team_key = db.get_team_key(slot_slug)?.ok_or_else(|| {
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
    keychain::get_secret(&entry_key)?.ok_or_else(|| {
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
/// 2. `keychain://ref/team/{org_id}/{slot_slug}` → routes through team key governance
/// 3. `keychain://ref/{other}` → resolves via Keychain directly
/// 4. Plain string → returned as-is
///
/// C-2 修复：`keychain://ref/team/` 命名空间不再绕过 status/grant 检查。
pub fn resolve_any_value(db: &Database, value: &str) -> Result<String, AppError> {
    if is_team_key_ref(value) {
        let slot_slug = extract_team_key_slug(value).unwrap_or_default();
        resolve_team_key(db, slot_slug)
    } else if let Some(entry_key) = keychain::extract_ref_key(value) {
        // C-2: team/ 命名空间必须走治理路径
        if let Some(rest) = entry_key.strip_prefix("team/") {
            // entry_key = "team/{org_id}/{slot_slug}" → 提取 slot_slug
            if let Some(slot_slug) = rest.rsplit('/').next() {
                return resolve_team_key(db, slot_slug);
            }
        }
        // 非 team 命名空间：直接解析
        keychain::resolve_value(value)
    } else {
        Ok(value.to_string())
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
            log::info!("Team key grant expired locally: slot={}", key.slot_slug);
        }
    }

    Ok(expired_count)
}

// ─── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::memory().expect("创建内存数据库")
    }

    /// 未来时间戳（grant 未过期）
    fn future_expiry() -> i64 {
        chrono::Utc::now().timestamp_millis() + 72 * 3600 * 1000 // +72h
    }

    /// 过去时间戳（grant 已过期）
    fn past_expiry() -> i64 {
        chrono::Utc::now().timestamp_millis() - 1000 // -1s
    }

    fn store_test_key(db: &Database, org: &str, slot: &str, plaintext: &str, expiry: i64) {
        store_team_key(
            db,
            org,
            slot,
            "api_key",
            Some("https://api.example.com"),
            plaintext,
            1,
            "",
            "grant_001",
            expiry,
        )
        .expect("store_team_key 应成功");
    }

    // ─── 格式与解析 ────────────────────────────────────────────────────────────

    #[test]
    fn entry_key_format() {
        assert_eq!(
            team_key_entry_key("org_abc", "openrouter-main"),
            "team/org_abc/openrouter-main"
        );
    }

    #[test]
    fn ref_format() {
        let r = team_key_ref("org_abc", "claude-team");
        assert_eq!(r, "keychain://ref/team/org_abc/claude-team");
    }

    #[test]
    fn is_team_key_ref_detection() {
        assert!(is_team_key_ref("team-key://openrouter-main"));
        assert!(!is_team_key_ref("keychain://ref/foo"));
        assert!(!is_team_key_ref("sk-plain-value"));
        assert!(!is_team_key_ref(""));
    }

    #[test]
    fn extract_slug_from_ref() {
        assert_eq!(
            extract_team_key_slug("team-key://openrouter-main"),
            Some("openrouter-main")
        );
        assert_eq!(extract_team_key_slug("team-key://"), Some(""));
        assert_eq!(extract_team_key_slug("not-a-ref"), None);
    }

    // ─── 存储 + 解析闭环 ───────────────────────────────────────────────────────

    #[test]
    fn store_and_resolve_roundtrip() {
        let db = test_db();
        store_test_key(&db, "org_1", "my-slot", "sk-secret-12345", future_expiry());

        let resolved = resolve_team_key(&db, "my-slot").unwrap();
        assert_eq!(resolved, "sk-secret-12345");
    }

    #[test]
    fn store_upserts_version() {
        let db = test_db();
        store_test_key(&db, "org_1", "slot-v", "old-key", future_expiry());

        // 模拟轮换：version_seq + 1，新 plaintext
        store_team_key(
            &db,
            "org_1",
            "slot-v",
            "api_key",
            None,
            "new-key-rotated",
            2,
            "",
            "grant_002",
            future_expiry(),
        )
        .unwrap();

        let resolved = resolve_team_key(&db, "slot-v").unwrap();
        assert_eq!(resolved, "new-key-rotated");

        // DB 中 version_seq 应更新
        let row = db.get_team_key("slot-v").unwrap().unwrap();
        assert_eq!(row.version_seq, 2);
        assert_eq!(row.grant_id, "grant_002");
    }

    #[test]
    fn plaintext_never_in_database() {
        let db = test_db();
        store_test_key(
            &db,
            "org_1",
            "secret-slot",
            "sk-SUPER-SECRET",
            future_expiry(),
        );

        // DB 行中不应包含明文
        let row = db.get_team_key("secret-slot").unwrap().unwrap();
        assert!(row.keychain_ref.starts_with("keychain://ref/"));
        assert!(!row.keychain_ref.contains("sk-SUPER-SECRET"));
        assert!(!row.value_sha256.contains("sk-SUPER-SECRET"));
        assert!(!row.grant_id.contains("sk-SUPER-SECRET"));
    }

    // ─── 状态拒绝 ──────────────────────────────────────────────────────────────

    #[test]
    fn resolve_revoked_key_fails() {
        let db = test_db();
        store_test_key(&db, "org_1", "revoked-slot", "sk-val", future_expiry());
        db.update_team_key_status("revoked-slot", "revoked")
            .unwrap();

        let err = resolve_team_key(&db, "revoked-slot").unwrap_err();
        assert!(err.to_string().contains("已被撤销"));
    }

    #[test]
    fn resolve_expired_key_fails() {
        let db = test_db();
        store_test_key(&db, "org_1", "expired-slot", "sk-val", future_expiry());
        db.update_team_key_status("expired-slot", "expired")
            .unwrap();

        let err = resolve_team_key(&db, "expired-slot").unwrap_err();
        assert!(err.to_string().contains("已过期"));
    }

    #[test]
    fn resolve_grant_timeout_auto_expires() {
        let db = test_db();
        // 存储时 grant 已过期
        store_test_key(&db, "org_1", "timeout-slot", "sk-val", past_expiry());

        let err = resolve_team_key(&db, "timeout-slot").unwrap_err();
        assert!(err.to_string().contains("grant 已过期"));

        // 应自动标记为 expired
        let row = db.get_team_key("timeout-slot").unwrap().unwrap();
        assert_eq!(row.status, "expired");
    }

    #[test]
    fn resolve_nonexistent_key_fails() {
        let db = test_db();
        let err = resolve_team_key(&db, "no-such-slot").unwrap_err();
        assert!(err.to_string().contains("本机未持有"));
    }

    // ─── resolve_team_value ────────────────────────────────────────────────────

    #[test]
    fn resolve_team_value_passthrough_plain() {
        let db = test_db();
        let val = resolve_team_value(&db, "sk-plain-key").unwrap();
        assert_eq!(val, "sk-plain-key");
    }

    #[test]
    fn resolve_team_value_resolves_ref() {
        let db = test_db();
        store_test_key(&db, "org_x", "router", "sk-router-key", future_expiry());

        let val = resolve_team_value(&db, "team-key://router").unwrap();
        assert_eq!(val, "sk-router-key");
    }

    // ─── 撤销 ──────────────────────────────────────────────────────────────────

    #[test]
    fn revoke_single_key() {
        let db = test_db();
        store_test_key(&db, "org_1", "to-revoke", "sk-val", future_expiry());

        revoke_team_key(&db, "to-revoke").unwrap();

        // 状态变为 revoked
        let row = db.get_team_key("to-revoke").unwrap().unwrap();
        assert_eq!(row.status, "revoked");

        // Keychain 条目已删除 → resolve 失败
        let err = resolve_team_key(&db, "to-revoke").unwrap_err();
        assert!(err.to_string().contains("已被撤销"));
    }

    #[test]
    fn revoke_nonexistent_is_noop() {
        let db = test_db();
        // 不应报错
        revoke_team_key(&db, "ghost-slot").unwrap();
    }

    #[test]
    fn revoke_all_for_org_batch() {
        let db = test_db();
        store_test_key(&db, "org_leaving", "key-a", "val-a", future_expiry());
        store_test_key(&db, "org_leaving", "key-b", "val-b", future_expiry());
        store_test_key(&db, "org_staying", "key-c", "val-c", future_expiry());

        let count = revoke_all_team_keys_for_org(&db, "org_leaving").unwrap();
        assert_eq!(count, 2);

        // org_leaving 的 key 已删除
        assert!(db.get_team_key("key-a").unwrap().is_none());
        assert!(db.get_team_key("key-b").unwrap().is_none());

        // org_staying 不受影响
        let row = db.get_team_key("key-c").unwrap().unwrap();
        assert_eq!(row.status, "active");
        assert_eq!(resolve_team_key(&db, "key-c").unwrap(), "val-c");
    }

    // ─── check_grant_expiry ────────────────────────────────────────────────────

    #[test]
    fn check_grant_expiry_marks_stale() {
        let db = test_db();
        store_test_key(&db, "org_1", "fresh", "val-1", future_expiry());
        store_test_key(&db, "org_1", "stale", "val-2", past_expiry());
        store_test_key(&db, "org_2", "also-stale", "val-3", past_expiry());

        let expired = check_grant_expiry(&db).unwrap();
        assert_eq!(expired, 2);

        assert_eq!(db.get_team_key("fresh").unwrap().unwrap().status, "active");
        assert_eq!(db.get_team_key("stale").unwrap().unwrap().status, "expired");
        assert_eq!(
            db.get_team_key("also-stale").unwrap().unwrap().status,
            "expired"
        );
    }

    #[test]
    fn check_grant_expiry_skips_already_revoked() {
        let db = test_db();
        store_test_key(&db, "org_1", "revoked-stale", "val", past_expiry());
        db.update_team_key_status("revoked-stale", "revoked")
            .unwrap();

        let expired = check_grant_expiry(&db).unwrap();
        assert_eq!(expired, 0); // revoked 不算 expired
        assert_eq!(
            db.get_team_key("revoked-stale").unwrap().unwrap().status,
            "revoked"
        );
    }

    // ─── resolve_any_value 统一入口 ────────────────────────────────────────────

    #[test]
    fn resolve_any_value_team_key() {
        let db = test_db();
        store_test_key(&db, "org_u", "unified", "sk-unified", future_expiry());

        let val = resolve_any_value(&db, "team-key://unified").unwrap();
        assert_eq!(val, "sk-unified");
    }

    #[test]
    fn resolve_any_value_plain_string() {
        let db = test_db();
        let val = resolve_any_value(&db, "just-a-plain-value").unwrap();
        assert_eq!(val, "just-a-plain-value");
    }

    #[test]
    fn resolve_any_value_keychain_ref() {
        let db = test_db();
        // 直接写入 keychain 一个条目
        keychain::store_secret("test/unified-ref", "sk-from-keychain").unwrap();

        let val = resolve_any_value(&db, "keychain://ref/test/unified-ref").unwrap();
        assert_eq!(val, "sk-from-keychain");
    }

    // ─── W-2: SHA-256 完整性校验 ───────────────────────────────────────────────

    #[test]
    fn store_with_valid_sha256_succeeds() {
        let db = test_db();
        let plaintext = "sk-verified-key";
        let sha = crate::team_config::release::sha256_of_content(plaintext.as_bytes());

        store_team_key(
            &db,
            "org_1",
            "verified-slot",
            "api_key",
            None,
            plaintext,
            1,
            &sha,
            "grant_v",
            future_expiry(),
        )
        .unwrap();

        assert_eq!(resolve_team_key(&db, "verified-slot").unwrap(), plaintext);
    }

    #[test]
    fn store_with_mismatched_sha256_rejected() {
        let db = test_db();
        let result = store_team_key(
            &db,
            "org_1",
            "bad-hash-slot",
            "api_key",
            None,
            "sk-actual-value",
            1,
            "wrong_hash_value",
            "grant_x",
            future_expiry(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("完整性校验失败"));
    }

    #[test]
    fn store_empty_plaintext_rejected() {
        let db = test_db();
        let result = store_team_key(
            &db,
            "org_1",
            "empty-slot",
            "api_key",
            None,
            "",
            1,
            "",
            "grant_y",
            future_expiry(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("下发值为空"));
    }

    // ─── C-2: 治理绕过阻断 ─────────────────────────────────────────────────────

    #[test]
    fn resolve_any_value_team_ref_respects_revoked() {
        let db = test_db();
        store_test_key(&db, "org_sec", "guarded", "sk-guarded", future_expiry());

        // 通过 keychain://ref/team/ 路径也应受治理
        let ref_path = "keychain://ref/team/org_sec/guarded";
        assert_eq!(resolve_any_value(&db, ref_path).unwrap(), "sk-guarded");

        // 撤销后，两条路径都应失败
        revoke_team_key(&db, "guarded").unwrap();
        assert!(resolve_any_value(&db, "team-key://guarded").is_err());
        assert!(resolve_any_value(&db, ref_path).is_err());
    }
}
