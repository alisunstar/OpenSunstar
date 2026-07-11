//! Transport-agnostic sync protocol layer.
//!
//! Shared by WebDAV, S3, and future transports. Artifact set: `db.sql` + `skills.zip`.

use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

use crate::error::AppError;

// Re-export archive functions for use by transport layers.
pub(crate) use super::webdav_sync::archive::{
    backup_current_skills, restore_skills_from_backup, restore_skills_zip, zip_skills_ssot,
};

// ─── Protocol constants ──────────────────────────────────────

/// Wire-format identifier stored in remote manifests.
/// Retains historic "webdav" naming for backward compatibility with existing remotes.
pub(crate) const PROTOCOL_FORMAT: &str = "OpenSunstar-webdav-sync";
pub(crate) const PROTOCOL_VERSION: u32 = 2;
pub(crate) const DB_COMPAT_VERSION: u32 = 6;
pub(crate) const LEGACY_DB_COMPAT_VERSION: u32 = 5;
pub(crate) const REMOTE_DB_SQL: &str = "db.sql";
pub(crate) const REMOTE_SKILLS_ZIP: &str = "skills.zip";
pub(crate) const REMOTE_MANIFEST: &str = "manifest.json";
pub(crate) const MAX_DEVICE_NAME_LEN: usize = 64;
pub(crate) const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_SYNC_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;

// ─── Error helpers ───────────────────────────────────────────

pub(crate) fn localized(
    key: &'static str,
    zh: impl Into<String>,
    en: impl Into<String>,
) -> AppError {
    AppError::localized(key, zh, en)
}

pub(crate) fn io_context_localized(
    _key: &'static str,
    zh: impl Into<String>,
    en: impl Into<String>,
    source: std::io::Error,
) -> AppError {
    let zh_msg = zh.into();
    let en_msg = en.into();
    AppError::IoContext {
        context: format!("{zh_msg} ({en_msg})"),
        source,
    }
}

// ─── Types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncManifest {
    pub format: String,
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_compat_version: Option<u32>,
    pub device_name: String,
    pub created_at: String,
    pub artifacts: BTreeMap<String, ArtifactMeta>,
    pub snapshot_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf_salt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArtifactMeta {
    pub sha256: String,
    pub size: u64,
}

pub(crate) struct LocalSnapshot {
    pub db_sql: Vec<u8>,
    pub skills_zip: Vec<u8>,
    pub manifest_bytes: Vec<u8>,
    pub manifest_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteLayout {
    Current,
    Legacy,
}

impl RemoteLayout {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Legacy => "legacy",
        }
    }
}

// ─── Snapshot building ───────────────────────────────────────

pub(crate) fn build_local_snapshot(
    db: &crate::database::Database,
) -> Result<LocalSnapshot, AppError> {
    // Export database to SQL string
    let sql_string = db.export_sql_string_for_sync()?;
    let db_sql_plain = sql_string.into_bytes();

    // Encrypt db.sql if sync master key is available (E2EE)
    let (db_sql, encrypted, key_id, kdf_salt) = match crate::keychain::get_sync_master_key() {
        Ok(Some(master_key)) => {
            let snapshot_salt = random_snapshot_kdf_salt();
            let derived_key = crate::keychain::derive_snapshot_key(&master_key, &snapshot_salt)?;
            let encrypted_bytes = crate::keychain::encrypt_data(&derived_key, &db_sql_plain)?;
            let kid = sha256_hex(&master_key)[..16].to_string();
            (encrypted_bytes, Some(true), Some(kid), Some(snapshot_salt))
        }
        _ => {
            log::info!("Sync master key not available, uploading db.sql in plaintext");
            (db_sql_plain, None, None, None)
        }
    };

    // Pack skills into deterministic ZIP
    let tmp = tempdir().map_err(|e| {
        io_context_localized(
            "sync.snapshot_tmpdir_failed",
            "创建快照临时目录失败",
            "Failed to create temporary directory for snapshot",
            e,
        )
    })?;
    let skills_zip_path = tmp.path().join(REMOTE_SKILLS_ZIP);
    zip_skills_ssot(&skills_zip_path)?;
    let skills_zip = fs::read(&skills_zip_path).map_err(|e| AppError::io(&skills_zip_path, e))?;

    // Build artifact map and compute hashes
    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        REMOTE_DB_SQL.to_string(),
        ArtifactMeta {
            sha256: sha256_hex(&db_sql),
            size: db_sql.len() as u64,
        },
    );
    artifacts.insert(
        REMOTE_SKILLS_ZIP.to_string(),
        ArtifactMeta {
            sha256: sha256_hex(&skills_zip),
            size: skills_zip.len() as u64,
        },
    );

    let snapshot_id = compute_snapshot_id(&artifacts);
    let manifest = SyncManifest {
        format: PROTOCOL_FORMAT.to_string(),
        version: PROTOCOL_VERSION,
        db_compat_version: Some(DB_COMPAT_VERSION),
        device_name: detect_system_device_name().unwrap_or_else(|| "Unknown Device".to_string()),
        created_at: Utc::now().to_rfc3339(),
        artifacts,
        snapshot_id,
        encrypted,
        key_id,
        kdf_salt,
    };
    let manifest_bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|e| AppError::JsonSerialize { source: e })?;
    let manifest_hash = sha256_hex(&manifest_bytes);

    Ok(LocalSnapshot {
        db_sql,
        skills_zip,
        manifest_bytes,
        manifest_hash,
    })
}

// ─── Manifest handling ───────────────────────────────────────

/// Compute a deterministic snapshot identity from artifact hashes.
///
/// BTreeMap iteration order is sorted by key, ensuring stability.
pub(crate) fn compute_snapshot_id(artifacts: &BTreeMap<String, ArtifactMeta>) -> String {
    let parts: Vec<String> = artifacts
        .iter()
        .map(|(name, meta)| format!("{}:{}", name, meta.sha256))
        .collect();
    sha256_hex(parts.join("|").as_bytes())
}

pub(crate) fn effective_db_compat_version(
    manifest: &SyncManifest,
    layout: RemoteLayout,
) -> Option<u32> {
    manifest
        .db_compat_version
        .or_else(|| (layout == RemoteLayout::Legacy).then_some(LEGACY_DB_COMPAT_VERSION))
}

pub(crate) fn validate_manifest_compat(
    manifest: &SyncManifest,
    layout: RemoteLayout,
) -> Result<(), AppError> {
    if manifest.format != PROTOCOL_FORMAT {
        return Err(localized(
            "sync.manifest_format_incompatible",
            format!("远端 manifest 格式不兼容: {}", manifest.format),
            format!(
                "Remote manifest format is incompatible: {}",
                manifest.format
            ),
        ));
    }
    if manifest.version != PROTOCOL_VERSION {
        return Err(localized(
            "sync.manifest_version_incompatible",
            format!(
                "远端 manifest 协议版本不兼容: v{} (本地 v{PROTOCOL_VERSION})",
                manifest.version
            ),
            format!(
                "Remote manifest protocol version is incompatible: v{} (local v{PROTOCOL_VERSION})",
                manifest.version
            ),
        ));
    }
    let Some(db_compat_version) = effective_db_compat_version(manifest, layout) else {
        return Err(localized(
            "sync.manifest_db_version_missing",
            "远端 manifest 缺少数据库兼容版本",
            "Remote manifest is missing the database compatibility version.",
        ));
    };
    match layout {
        RemoteLayout::Current if db_compat_version != DB_COMPAT_VERSION => {
            return Err(localized(
                "sync.manifest_db_version_incompatible",
                format!(
                    "远端数据库快照版本不兼容: db-v{db_compat_version} (本地 db-v{DB_COMPAT_VERSION})"
                ),
                format!(
                    "Remote database snapshot version is incompatible: db-v{db_compat_version} (local db-v{DB_COMPAT_VERSION})"
                ),
            ));
        }
        RemoteLayout::Legacy if db_compat_version > DB_COMPAT_VERSION => {
            return Err(localized(
                "sync.manifest_db_version_incompatible",
                format!(
                    "远端数据库快照版本不兼容: db-v{db_compat_version} (本地最高支持 db-v{DB_COMPAT_VERSION})"
                ),
                format!(
                    "Remote database snapshot version is incompatible: db-v{db_compat_version} (local supports up to db-v{DB_COMPAT_VERSION})"
                ),
            ));
        }
        _ => {}
    }
    Ok(())
}

// ─── Artifact verification ───────────────────────────────────

pub(crate) fn validate_artifact_size_limit(artifact_name: &str, size: u64) -> Result<(), AppError> {
    if size > MAX_SYNC_ARTIFACT_BYTES {
        let max_mb = MAX_SYNC_ARTIFACT_BYTES / 1024 / 1024;
        return Err(localized(
            "sync.artifact_too_large",
            format!("artifact {artifact_name} 超过下载上限（{} MB）", max_mb),
            format!(
                "Artifact {artifact_name} exceeds download limit ({} MB)",
                max_mb
            ),
        ));
    }
    Ok(())
}

/// Verify that downloaded artifact bytes match the expected size and SHA-256 hash.
pub(crate) fn verify_artifact(
    bytes: &[u8],
    artifact_name: &str,
    meta: &ArtifactMeta,
) -> Result<(), AppError> {
    // Quick size check before expensive hash
    if bytes.len() as u64 != meta.size {
        return Err(localized(
            "sync.artifact_size_mismatch",
            format!(
                "artifact {artifact_name} 大小不匹配 (expected: {}, got: {})",
                meta.size,
                bytes.len(),
            ),
            format!(
                "Artifact {artifact_name} size mismatch (expected: {}, got: {})",
                meta.size,
                bytes.len(),
            ),
        ));
    }

    let actual_hash = sha256_hex(bytes);
    if actual_hash != meta.sha256 {
        return Err(localized(
            "sync.artifact_hash_mismatch",
            format!(
                "artifact {artifact_name} SHA256 校验失败 (expected: {}..., got: {}...)",
                meta.sha256.get(..8).unwrap_or(&meta.sha256),
                actual_hash.get(..8).unwrap_or(&actual_hash),
            ),
            format!(
                "Artifact {artifact_name} SHA256 verification failed (expected: {}..., got: {}...)",
                meta.sha256.get(..8).unwrap_or(&meta.sha256),
                actual_hash.get(..8).unwrap_or(&actual_hash),
            ),
        ));
    }
    Ok(())
}

// ─── Snapshot application ────────────────────────────────────

#[allow(dead_code)]
pub(crate) fn apply_snapshot(
    db: &crate::database::Database,
    db_sql: &[u8],
    skills_zip: &[u8],
) -> Result<(), AppError> {
    apply_snapshot_with_manifest(db, db_sql, skills_zip, None)
}

/// Apply a snapshot with optional manifest for E2EE decryption.
/// If the manifest indicates encryption, decrypts db_sql before import.
pub(crate) fn apply_snapshot_with_manifest(
    db: &crate::database::Database,
    db_sql: &[u8],
    skills_zip: &[u8],
    manifest: Option<&SyncManifest>,
) -> Result<(), AppError> {
    let decrypted_sql: Vec<u8>;
    let actual_db_sql = if let Some(manifest) = manifest.filter(|m| m.encrypted == Some(true)) {
        let master_key = crate::keychain::get_sync_master_key()?.ok_or_else(|| {
            localized(
                "sync.master_key_missing",
                "同步主密钥不存在，无法解密加密的数据库快照。请确保在当前设备上配置了同步主密钥。",
                "Sync master key not found. Cannot decrypt the encrypted database snapshot. Please ensure the sync master key is configured on this device.",
            )
        })?;
        let snapshot_salt = encrypted_snapshot_kdf_salt(manifest)?;
        let derived_key = crate::keychain::derive_snapshot_key(&master_key, snapshot_salt)?;
        decrypted_sql = crate::keychain::decrypt_data(&derived_key, db_sql)?;
        &decrypted_sql
    } else {
        db_sql
    };

    let sql_str = std::str::from_utf8(actual_db_sql).map_err(|e| {
        localized(
            "sync.sql_not_utf8",
            format!("SQL 非 UTF-8: {e}"),
            format!("SQL is not valid UTF-8: {e}"),
        )
    })?;
    let skills_backup = backup_current_skills()?;

    // Replace skills first, then import database; roll back skills on DB failure.
    restore_skills_zip(skills_zip)?;

    if let Err(db_err) = db.import_sql_string_for_sync(sql_str) {
        if let Err(rollback_err) = restore_skills_from_backup(&skills_backup) {
            return Err(localized(
                "sync.db_import_and_rollback_failed",
                format!("导入数据库失败: {db_err}; 同时回滚 Skills 失败: {rollback_err}"),
                format!(
                    "Database import failed: {db_err}; skills rollback also failed: {rollback_err}"
                ),
            ));
        }
        return Err(db_err);
    }

    Ok(())
}

// ─── Utilities ───────────────────────────────────────────────

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn random_snapshot_kdf_salt() -> String {
    let mut bytes = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut bytes);
    sha256_hex(&bytes)
}

fn encrypted_snapshot_kdf_salt(manifest: &SyncManifest) -> Result<&str, AppError> {
    manifest
        .kdf_salt
        .as_deref()
        .filter(|salt| !salt.trim().is_empty())
        .ok_or_else(|| {
            localized(
                "sync.kdf_salt_missing",
                "远端加密快照缺少密钥派生盐，无法安全解密。请在源设备使用新版 OpenSunstar 重新上传同步快照。",
                "The encrypted remote snapshot is missing its key-derivation salt. Re-upload the sync snapshot from the source device with a newer OpenSunstar version.",
            )
        })
}

pub(crate) fn detect_system_device_name() -> Option<String> {
    let env_name = ["OPEN_SUNSTAR_DEVICE_NAME", "COMPUTERNAME", "HOSTNAME"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .find_map(|value| normalize_device_name(&value));

    if env_name.is_some() {
        return env_name;
    }

    let output = Command::new("hostname").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let hostname = String::from_utf8(output.stdout).ok()?;
    normalize_device_name(&hostname)
}

pub(crate) fn normalize_device_name(raw: &str) -> Option<String> {
    let compact = raw
        .chars()
        .fold(String::with_capacity(raw.len()), |mut acc, ch| {
            if ch.is_whitespace() {
                acc.push(' ');
            } else if !ch.is_control() {
                acc.push(ch);
            }
            acc
        });
    let normalized = compact.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return None;
    }

    let limited = trimmed
        .chars()
        .take(MAX_DEVICE_NAME_LEN)
        .collect::<String>();
    if limited.is_empty() {
        None
    } else {
        Some(limited)
    }
}

// ─── Sync status persistence ─────────────────────────────────

pub(crate) fn persist_sync_success_best_effort<S, F>(
    settings: &mut S,
    manifest_hash: String,
    etag: Option<String>,
    persist_fn: F,
) -> bool
where
    F: FnOnce(&mut S, String, Option<String>) -> Result<(), AppError>,
{
    match persist_fn(settings, manifest_hash, etag) {
        Ok(()) => true,
        Err(err) => {
            log::warn!("[Sync] Persist sync status failed, keep operation success: {err}");
            false
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::{tempdir, TempDir};

    fn sync_roundtrip_mutex() -> &'static Mutex<()> {
        static MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        MUTEX.get_or_init(|| Mutex::new(()))
    }

    fn prepare_sync_test_home(name: &str) -> TempDir {
        let home = tempdir().expect("create sync test home");
        std::env::set_var("OPEN_SUNSTAR_TEST_HOME", home.path());
        std::env::set_var("HOME", home.path());
        #[cfg(windows)]
        std::env::set_var("USERPROFILE", home.path());
        crate::settings::update_settings(crate::settings::AppSettings::default())
            .expect("reset settings");
        let skills_dir =
            crate::services::skill::SkillService::get_ssot_dir().expect("create skills ssot dir");
        std::fs::write(
            skills_dir.join(format!("{name}.md")),
            format!("# {name}\n\nsync test skill\n"),
        )
        .expect("write test skill");
        home
    }

    fn seeded_memory_db(marker_value: &str) -> crate::database::Database {
        let db = crate::database::Database::memory().expect("memory db");
        db.init_default_official_providers()
            .expect("seed official providers");
        db.set_setting("sync_roundtrip_marker", marker_value)
            .expect("write marker setting");
        db
    }

    fn parse_snapshot_manifest(snapshot: &LocalSnapshot) -> SyncManifest {
        serde_json::from_slice(&snapshot.manifest_bytes).expect("parse snapshot manifest")
    }

    fn assert_marker(db: &crate::database::Database, expected: &str) {
        assert_eq!(
            db.get_setting("sync_roundtrip_marker")
                .expect("read marker setting")
                .as_deref(),
            Some(expected)
        );
    }

    fn artifact(sha256: &str, size: u64) -> ArtifactMeta {
        ArtifactMeta {
            sha256: sha256.to_string(),
            size,
        }
    }

    #[test]
    fn snapshot_id_is_stable() {
        let mut artifacts = BTreeMap::new();
        artifacts.insert("db.sql".to_string(), artifact("abc123", 100));
        artifacts.insert("skills.zip".to_string(), artifact("def456", 200));

        let id1 = compute_snapshot_id(&artifacts);
        let id2 = compute_snapshot_id(&artifacts);
        assert_eq!(id1, id2);
    }

    #[test]
    fn snapshot_id_changes_with_artifacts() {
        let mut a1 = BTreeMap::new();
        a1.insert("db.sql".to_string(), artifact("hash-a", 1));

        let mut a2 = BTreeMap::new();
        a2.insert("db.sql".to_string(), artifact("hash-b", 1));

        assert_ne!(compute_snapshot_id(&a1), compute_snapshot_id(&a2));
    }

    #[test]
    fn sha256_hex_is_correct() {
        let hash = sha256_hex(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn persist_best_effort_returns_true_on_success() {
        let mut dummy = ();
        let ok = persist_sync_success_best_effort(
            &mut dummy,
            "hash".to_string(),
            Some("etag".to_string()),
            |_settings, _hash, _etag| Ok(()),
        );
        assert!(ok);
    }

    #[test]
    fn persist_best_effort_returns_false_on_error() {
        let mut dummy = ();
        let ok = persist_sync_success_best_effort(
            &mut dummy,
            "hash".to_string(),
            None,
            |_settings, _hash, _etag| Err(AppError::Config("boom".to_string())),
        );
        assert!(!ok);
    }

    fn manifest_with(format: &str, version: u32, db_compat_version: Option<u32>) -> SyncManifest {
        let mut artifacts = BTreeMap::new();
        artifacts.insert("db.sql".to_string(), artifact("abc", 1));
        artifacts.insert("skills.zip".to_string(), artifact("def", 2));
        SyncManifest {
            format: format.to_string(),
            version,
            db_compat_version,
            device_name: "My MacBook".to_string(),
            created_at: "2026-02-12T00:00:00Z".to_string(),
            artifacts,
            snapshot_id: "snap-1".to_string(),
            encrypted: None,
            key_id: None,
            kdf_salt: None,
        }
    }

    #[test]
    fn validate_manifest_compat_accepts_supported_manifest() {
        let manifest = manifest_with(PROTOCOL_FORMAT, PROTOCOL_VERSION, Some(DB_COMPAT_VERSION));
        assert!(validate_manifest_compat(&manifest, RemoteLayout::Current).is_ok());
    }

    #[test]
    fn validate_manifest_compat_rejects_wrong_format() {
        let manifest = manifest_with("other-format", PROTOCOL_VERSION, Some(DB_COMPAT_VERSION));
        assert!(validate_manifest_compat(&manifest, RemoteLayout::Current).is_err());
    }

    #[test]
    fn validate_manifest_compat_rejects_wrong_version() {
        let manifest = manifest_with(
            PROTOCOL_FORMAT,
            PROTOCOL_VERSION + 1,
            Some(DB_COMPAT_VERSION),
        );
        assert!(validate_manifest_compat(&manifest, RemoteLayout::Current).is_err());
    }

    #[test]
    fn validate_manifest_compat_accepts_legacy_manifest_without_db_compat() {
        let manifest = manifest_with(PROTOCOL_FORMAT, PROTOCOL_VERSION, None);
        assert!(validate_manifest_compat(&manifest, RemoteLayout::Legacy).is_ok());
    }

    #[test]
    fn validate_manifest_compat_rejects_current_manifest_with_wrong_db_compat() {
        let manifest = manifest_with(
            PROTOCOL_FORMAT,
            PROTOCOL_VERSION,
            Some(LEGACY_DB_COMPAT_VERSION),
        );
        assert!(validate_manifest_compat(&manifest, RemoteLayout::Current).is_err());
    }

    #[test]
    fn validate_manifest_compat_rejects_legacy_manifest_from_newer_db_generation() {
        let manifest = manifest_with(
            PROTOCOL_FORMAT,
            PROTOCOL_VERSION,
            Some(DB_COMPAT_VERSION + 1),
        );
        assert!(validate_manifest_compat(&manifest, RemoteLayout::Legacy).is_err());
    }

    #[test]
    fn effective_db_compat_version_defaults_legacy_layout_to_v5() {
        let manifest = manifest_with(PROTOCOL_FORMAT, PROTOCOL_VERSION, None);
        assert_eq!(
            effective_db_compat_version(&manifest, RemoteLayout::Legacy),
            Some(LEGACY_DB_COMPAT_VERSION)
        );
        assert_eq!(
            effective_db_compat_version(&manifest, RemoteLayout::Current),
            None
        );
    }

    #[test]
    fn encrypted_snapshot_requires_manifest_kdf_salt() {
        let mut manifest =
            manifest_with(PROTOCOL_FORMAT, PROTOCOL_VERSION, Some(DB_COMPAT_VERSION));
        manifest.encrypted = Some(true);
        let err = encrypted_snapshot_kdf_salt(&manifest)
            .expect_err("encrypted snapshot without kdf_salt should be rejected");
        assert!(err.to_string().contains("密钥派生盐") || err.to_string().contains("salt"));
    }

    #[test]
    fn encrypted_snapshot_uses_manifest_kdf_salt() {
        let mut manifest =
            manifest_with(PROTOCOL_FORMAT, PROTOCOL_VERSION, Some(DB_COMPAT_VERSION));
        manifest.encrypted = Some(true);
        manifest.kdf_salt = Some("fixed-test-salt".to_string());
        assert_eq!(
            encrypted_snapshot_kdf_salt(&manifest).expect("kdf salt"),
            "fixed-test-salt"
        );
    }

    #[test]
    fn encrypted_snapshot_roundtrip_depends_on_manifest_kdf_salt() {
        let mut manifest =
            manifest_with(PROTOCOL_FORMAT, PROTOCOL_VERSION, Some(DB_COMPAT_VERSION));
        manifest.encrypted = Some(true);
        manifest.kdf_salt = Some("fixed-test-salt".to_string());

        let master_key = vec![7u8; 32];
        let plaintext = b"CREATE TABLE test(id INTEGER);";
        let salt = encrypted_snapshot_kdf_salt(&manifest).expect("manifest salt");
        let key = crate::keychain::derive_snapshot_key(&master_key, salt).expect("derive key");
        let encrypted = crate::keychain::encrypt_data(&key, plaintext).expect("encrypt");
        let decrypted = crate::keychain::decrypt_data(&key, &encrypted).expect("decrypt");
        assert_eq!(decrypted, plaintext);

        let wrong_legacy_salt = sha256_hex(&encrypted);
        let wrong_key = crate::keychain::derive_snapshot_key(&master_key, &wrong_legacy_salt)
            .expect("derive legacy bug key");
        assert!(
            crate::keychain::decrypt_data(&wrong_key, &encrypted).is_err(),
            "ciphertext-derived salt must not be accepted for new encrypted snapshots"
        );
    }

    #[test]
    fn plaintext_snapshot_roundtrip_applies_db_and_skills() {
        let _guard = sync_roundtrip_mutex().lock().expect("lock sync test");
        let _home = prepare_sync_test_home("plaintext-roundtrip");
        crate::keychain::delete_secret("sync/master_key").expect("clear test sync key");

        let source = seeded_memory_db("plaintext-ok");
        let snapshot = build_local_snapshot(&source).expect("build plaintext snapshot");
        let manifest = parse_snapshot_manifest(&snapshot);

        assert_ne!(manifest.encrypted, Some(true));
        assert!(std::str::from_utf8(&snapshot.db_sql)
            .expect("plaintext db.sql should be utf8")
            .contains("plaintext-ok"));

        let target = seeded_memory_db("before-download");
        apply_snapshot_with_manifest(
            &target,
            &snapshot.db_sql,
            &snapshot.skills_zip,
            Some(&manifest),
        )
        .expect("apply plaintext snapshot");

        assert_marker(&target, "plaintext-ok");
    }

    #[test]
    fn encrypted_snapshot_roundtrip_applies_with_matching_master_key() {
        let _guard = sync_roundtrip_mutex().lock().expect("lock sync test");
        let _home = prepare_sync_test_home("encrypted-roundtrip");
        let master_key = vec![42u8; 32];
        crate::keychain::store_sync_master_key(&master_key).expect("store sync key");

        let source = seeded_memory_db("encrypted-ok");
        let snapshot = build_local_snapshot(&source).expect("build encrypted snapshot");
        let manifest = parse_snapshot_manifest(&snapshot);

        assert_eq!(manifest.encrypted, Some(true));
        assert!(manifest
            .kdf_salt
            .as_deref()
            .is_some_and(|salt| !salt.is_empty()));
        assert!(
            std::str::from_utf8(&snapshot.db_sql)
                .map(|sql| !sql.contains("encrypted-ok"))
                .unwrap_or(true),
            "encrypted db.sql must not expose plaintext marker"
        );

        let target = seeded_memory_db("before-download");
        apply_snapshot_with_manifest(
            &target,
            &snapshot.db_sql,
            &snapshot.skills_zip,
            Some(&manifest),
        )
        .expect("apply encrypted snapshot");

        assert_marker(&target, "encrypted-ok");
    }

    #[test]
    fn encrypted_snapshot_roundtrip_rejects_wrong_master_key() {
        let _guard = sync_roundtrip_mutex().lock().expect("lock sync test");
        let _home = prepare_sync_test_home("wrong-key-roundtrip");
        crate::keychain::store_sync_master_key(&[1u8; 32]).expect("store source sync key");

        let source = seeded_memory_db("wrong-key-secret");
        let snapshot = build_local_snapshot(&source).expect("build encrypted snapshot");
        let manifest = parse_snapshot_manifest(&snapshot);
        assert_eq!(manifest.encrypted, Some(true));

        crate::keychain::store_sync_master_key(&[2u8; 32]).expect("replace sync key");
        let target = seeded_memory_db("before-download");
        let err = apply_snapshot_with_manifest(
            &target,
            &snapshot.db_sql,
            &snapshot.skills_zip,
            Some(&manifest),
        )
        .expect_err("wrong master key must reject encrypted snapshot");

        assert!(
            err.to_string().contains("Decryption failed")
                || err.to_string().contains("解密")
                || err.to_string().contains("invalid key"),
            "unexpected error: {err}"
        );
        assert_marker(&target, "before-download");
    }

    #[test]
    fn encrypted_snapshot_roundtrip_rejects_damaged_manifest_without_kdf_salt() {
        let _guard = sync_roundtrip_mutex().lock().expect("lock sync test");
        let _home = prepare_sync_test_home("damaged-manifest-roundtrip");
        crate::keychain::store_sync_master_key(&[3u8; 32]).expect("store sync key");

        let source = seeded_memory_db("damaged-manifest-secret");
        let snapshot = build_local_snapshot(&source).expect("build encrypted snapshot");
        let mut manifest = parse_snapshot_manifest(&snapshot);
        assert_eq!(manifest.encrypted, Some(true));
        manifest.kdf_salt = None;

        let target = seeded_memory_db("before-download");
        let err = apply_snapshot_with_manifest(
            &target,
            &snapshot.db_sql,
            &snapshot.skills_zip,
            Some(&manifest),
        )
        .expect_err("damaged encrypted manifest must be rejected");

        assert!(
            err.to_string().contains("密钥派生盐") || err.to_string().contains("salt"),
            "unexpected error: {err}"
        );
        assert_marker(&target, "before-download");
    }

    #[test]
    fn normalize_device_name_returns_none_for_blank_input() {
        assert_eq!(normalize_device_name("   \n\t  "), None);
    }

    #[test]
    fn normalize_device_name_collapses_whitespace_and_drops_control_chars() {
        assert_eq!(
            normalize_device_name("  Mac\tBook \n Pro\u{0007} "),
            Some("Mac Book Pro".to_string())
        );
    }

    #[test]
    fn normalize_device_name_truncates_to_max_len() {
        let long = "a".repeat(80);
        assert_eq!(normalize_device_name(&long).map(|s| s.len()), Some(64));
    }

    #[test]
    fn manifest_serialization_uses_device_name_only() {
        let manifest = manifest_with(PROTOCOL_FORMAT, PROTOCOL_VERSION, Some(DB_COMPAT_VERSION));
        let value = serde_json::to_value(&manifest).expect("serialize manifest");
        assert!(
            value.get("deviceName").is_some(),
            "manifest should contain deviceName"
        );
        assert_eq!(
            value.get("dbCompatVersion").and_then(|v| v.as_u64()),
            Some(DB_COMPAT_VERSION as u64)
        );
        assert!(
            value.get("deviceId").is_none(),
            "manifest should not contain deviceId"
        );
    }

    #[test]
    fn validate_artifact_size_limit_rejects_oversized_artifacts() {
        let err = validate_artifact_size_limit("skills.zip", MAX_SYNC_ARTIFACT_BYTES + 1)
            .expect_err("artifact larger than limit should be rejected");
        assert!(
            err.to_string().contains("too large") || err.to_string().contains("超过"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_artifact_size_limit_accepts_limit_boundary() {
        assert!(validate_artifact_size_limit("skills.zip", MAX_SYNC_ARTIFACT_BYTES).is_ok());
    }

    #[test]
    fn verify_artifact_rejects_size_mismatch() {
        let meta = artifact("abc123", 100);
        let bytes = vec![0u8; 50];
        let err = verify_artifact(&bytes, "test.bin", &meta)
            .expect_err("size mismatch should be rejected");
        assert!(
            err.to_string().contains("mismatch") || err.to_string().contains("不匹配"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn verify_artifact_rejects_hash_mismatch() {
        let meta = ArtifactMeta {
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            size: 5,
        };
        let bytes = b"hello";
        let err = verify_artifact(bytes, "test.bin", &meta)
            .expect_err("hash mismatch should be rejected");
        assert!(
            err.to_string().contains("verification failed") || err.to_string().contains("校验失败"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn verify_artifact_accepts_matching_data() {
        let data = b"hello";
        let meta = ArtifactMeta {
            sha256: sha256_hex(data),
            size: data.len() as u64,
        };
        assert!(verify_artifact(data, "test.bin", &meta).is_ok());
    }
}
