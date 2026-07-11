//! OS Keychain abstraction layer for secure credential storage.
//!
//! Uses the `keyring` crate to store secrets in:
//! - macOS: Keychain Access
//! - Windows: Credential Manager
//! - Linux: libsecret (GNOME Keyring / KDE Wallet)
//!
//! Fallback: If the platform keychain is unavailable (e.g. headless Linux),
//! secrets are stored in an AES-256-GCM encrypted file at ~/.OpenSunstar/keystore.enc

#![cfg_attr(test, allow(dead_code))]

use crate::config::get_app_config_dir;
use crate::error::AppError;
use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

const SERVICE_NAME: &str = "opensunstar";
const KEYCHAIN_REF_PREFIX: &str = "keychain://ref/";
const FALLBACK_SALT: &[u8] = b"opensunstar-fallback-keystore-v1";

static KEYCHAIN_AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
static FALLBACK_STORE: std::sync::OnceLock<Mutex<FallbackStore>> = std::sync::OnceLock::new();

/// In-memory write-through cache for platform keychain secrets.
/// Bridges the Windows Credential Manager DPAPI propagation delay where
/// `set_password` returns Ok but `get_password` returns NoEntry for a short window.
/// Also avoids redundant OS calls for frequently-read keys within the same session.
static KEYCHAIN_CACHE: std::sync::OnceLock<Mutex<HashMap<String, String>>> =
    std::sync::OnceLock::new();

#[cfg(test)]
static TEST_SECRET_STORE: std::sync::OnceLock<Mutex<HashMap<String, String>>> =
    std::sync::OnceLock::new();

#[cfg(test)]
fn test_secret_store() -> &'static Mutex<HashMap<String, String>> {
    TEST_SECRET_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn keychain_cache() -> &'static Mutex<HashMap<String, String>> {
    KEYCHAIN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

struct FallbackStore {
    entries: HashMap<String, String>,
    dirty: bool,
}

/// Check if a value is a keychain reference placeholder
pub fn is_keychain_ref(value: &str) -> bool {
    value.starts_with(KEYCHAIN_REF_PREFIX)
}

/// Extract the entry key from a keychain reference
pub fn extract_ref_key(value: &str) -> Option<&str> {
    value.strip_prefix(KEYCHAIN_REF_PREFIX)
}

/// Build a keychain reference placeholder for storage in DB
pub fn make_keychain_ref(entry_key: &str) -> String {
    format!("{KEYCHAIN_REF_PREFIX}{entry_key}")
}

/// Build the entry key for a provider's API key
pub fn provider_entry_key(provider_id: &str, app_type: &str) -> String {
    format!("{provider_id}/{app_type}")
}

/// Build the entry key for a specific credential field of a provider.
///
/// Each provider may expose multiple credential fields (e.g. ANTHROPIC_AUTH_TOKEN
/// and ANTHROPIC_API_KEY). Storing them under distinct entry keys avoids
/// overwriting one with another when both are present.
pub fn provider_field_entry_key(provider_id: &str, app_type: &str, field: &str) -> String {
    format!("{provider_id}/{app_type}/{field}")
}

/// Store a secret in the OS keychain (or fallback)
pub fn store_secret(entry_key: &str, secret: &str) -> Result<(), AppError> {
    #[cfg(test)]
    {
        if !secret.is_empty() {
            if let Ok(mut store) = test_secret_store().lock() {
                store.insert(entry_key.to_string(), secret.to_string());
            }
        }
        return Ok(());
    }

    #[cfg(not(test))]
    {
        if secret.is_empty() {
            return Ok(());
        }

        if is_platform_keychain_available() {
            let entry = keyring::Entry::new(SERVICE_NAME, entry_key)
                .map_err(|e| AppError::Config(format!("Keychain entry creation failed: {e}")))?;
            entry
                .set_password(secret)
                .map_err(|e| AppError::Config(format!("Keychain store failed: {e}")))?;
            // Write-through: populate in-memory cache so subsequent reads are instant,
            // bridging the Windows DPAPI propagation delay.
            if let Ok(mut cache) = keychain_cache().lock() {
                cache.insert(entry_key.to_string(), secret.to_string());
            }
        } else {
            store_fallback(entry_key, secret)?;
        }
        Ok(())
    }
}

/// Retrieve a secret from the OS keychain (or fallback).
///
/// Checks the in-memory write-through cache first — if the secret was stored
/// during this session, it is returned instantly without an OS keychain call.
/// This completely eliminates the Windows Credential Manager DPAPI propagation
/// delay for same-session store→read sequences.
///
/// For keys stored in a previous session (not in cache), falls back to the
/// platform keychain with a short retry on `NoEntry` to absorb any residual
/// timing issues.
pub fn get_secret(entry_key: &str) -> Result<Option<String>, AppError> {
    #[cfg(test)]
    {
        return Ok(test_secret_store()
            .lock()
            .ok()
            .and_then(|store| store.get(entry_key).cloned()));
    }

    #[cfg(not(test))]
    {
        // 1. In-memory cache hit — instant return, no OS call at all
        if let Ok(cache) = keychain_cache().lock() {
            if let Some(secret) = cache.get(entry_key) {
                return Ok(Some(secret.clone()));
            }
        }

        if is_platform_keychain_available() {
            let entry = keyring::Entry::new(SERVICE_NAME, entry_key)
                .map_err(|e| AppError::Config(format!("Keychain entry creation failed: {e}")))?;
            let max_attempts = if cfg!(target_os = "windows") { 5 } else { 2 };
            let delay_ms: u64 = if cfg!(target_os = "windows") { 200 } else { 50 };
            for attempt in 0..max_attempts {
                match entry.get_password() {
                    Ok(secret) => {
                        // Populate cache on successful read for future calls
                        if let Ok(mut cache) = keychain_cache().lock() {
                            cache.insert(entry_key.to_string(), secret.clone());
                        }
                        return Ok(Some(secret));
                    }
                    Err(keyring::Error::NoEntry) if attempt + 1 < max_attempts => {
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    }
                    Err(keyring::Error::NoEntry) => return Ok(None),
                    Err(e) => return Err(AppError::Config(format!("Keychain read failed: {e}"))),
                }
            }
            Ok(None) // unreachable, but satisfies the borrow checker
        } else {
            get_fallback(entry_key)
        }
    }
}

/// Delete a secret from the OS keychain (or fallback)
pub fn delete_secret(entry_key: &str) -> Result<(), AppError> {
    #[cfg(test)]
    {
        if let Ok(mut store) = test_secret_store().lock() {
            store.remove(entry_key);
        }
        return Ok(());
    }

    #[cfg(not(test))]
    {
        // Invalidate cache first — even if OS delete fails, we don't want stale reads
        if let Ok(mut cache) = keychain_cache().lock() {
            cache.remove(entry_key);
        }
        if is_platform_keychain_available() {
            let entry = keyring::Entry::new(SERVICE_NAME, entry_key)
                .map_err(|e| AppError::Config(format!("Keychain entry creation failed: {e}")))?;
            match entry.delete_credential() {
                Ok(()) => Ok(()),
                Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(AppError::Config(format!("Keychain delete failed: {e}"))),
            }
        } else {
            delete_fallback(entry_key)
        }
    }
}

/// Resolve a value that might be a keychain reference back to the actual secret
pub fn resolve_value(value: &str) -> Result<String, AppError> {
    if let Some(entry_key) = extract_ref_key(value) {
        match get_secret(entry_key)? {
            Some(secret) => Ok(secret),
            None => Err(AppError::Config(format!(
                "Keychain entry not found: {entry_key}. The credential may have been deleted from the system keychain."
            ))),
        }
    } else {
        Ok(value.to_string())
    }
}

/// Store the sync master key in keychain
pub fn store_sync_master_key(key_bytes: &[u8]) -> Result<(), AppError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let encoded = STANDARD.encode(key_bytes);
    store_secret("sync/master_key", &encoded)
}

/// Retrieve the sync master key from keychain
pub fn get_sync_master_key() -> Result<Option<Vec<u8>>, AppError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    match get_secret("sync/master_key")? {
        Some(encoded) => {
            let bytes = STANDARD
                .decode(&encoded)
                .map_err(|e| AppError::Config(format!("Failed to decode sync master key: {e}")))?;
            Ok(Some(bytes))
        }
        None => Ok(None),
    }
}

/// Generate a new sync master key and store it
pub fn ensure_sync_master_key() -> Result<Vec<u8>, AppError> {
    if let Some(key) = get_sync_master_key()? {
        return Ok(key);
    }
    let mut key = vec![0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut key);
    store_sync_master_key(&key)?;
    Ok(key)
}

/// Derive an encryption key from the master key for a specific snapshot
pub fn derive_snapshot_key(master_key: &[u8], snapshot_id: &str) -> Result<[u8; 32], AppError> {
    let hk = Hkdf::<Sha256>::new(Some(snapshot_id.as_bytes()), master_key);
    let mut okm = [0u8; 32];
    hk.expand(b"opensunstar-sync-encryption-v1", &mut okm)
        .map_err(|e| AppError::Config(format!("Key derivation failed: {e}")))?;
    Ok(okm)
}

/// Encrypt data using AES-256-GCM
pub fn encrypt_data(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::Config(format!("Cipher init failed: {e}")))?;
    let mut nonce_bytes = [0u8; 12];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| AppError::Config(format!("Encryption failed: {e}")))?;
    // Output format: nonce (12 bytes) || ciphertext (includes 16-byte auth tag)
    let mut output = Vec::with_capacity(12 + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt data using AES-256-GCM
pub fn decrypt_data(key: &[u8; 32], encrypted: &[u8]) -> Result<Vec<u8>, AppError> {
    if encrypted.len() < 12 + 16 {
        return Err(AppError::Config(
            "Encrypted data too short (missing nonce or auth tag)".to_string(),
        ));
    }
    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::Config(format!("Cipher init failed: {e}")))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::Config(
            "Decryption failed: invalid key or corrupted data. If you changed devices, ensure the sync master key is available.".to_string(),
        ))
}

// ─── Platform detection ─────────────────────────────────────

fn is_platform_keychain_available() -> bool {
    *KEYCHAIN_AVAILABLE.get_or_init(|| {
        let test_key = "__opensunstar_keychain_probe__";
        let entry = match keyring::Entry::new(SERVICE_NAME, test_key) {
            Ok(e) => e,
            Err(_) => return false,
        };
        match entry.set_password("probe") {
            Ok(()) => {
                let _ = entry.delete_credential();
                true
            }
            Err(_) => false,
        }
    })
}

// ─── Fallback encrypted file store ──────────────────────────

fn get_fallback_path() -> std::path::PathBuf {
    get_app_config_dir().join("keystore.enc")
}

fn get_device_key() -> [u8; 32] {
    // 组合多个机器标识以提高密钥派生熵值：
    // 1. 持久化的随机机器盐（首次使用时生成，防止仅凭 hostname 推导密钥）
    // 2. 系统 hostname（传统标识，低熵但跨进程一致）
    let machine_id = read_or_create_machine_id();
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown-device".to_string());
    let combined = format!("{machine_id}:{hostname}");

    let hk = Hkdf::<Sha256>::new(Some(FALLBACK_SALT), combined.as_bytes());
    let mut key = [0u8; 32];
    hk.expand(b"opensunstar-fallback-device-key-v2", &mut key)
        .expect("HKDF expand should not fail with 32-byte output");
    key
}

/// 读取或创建持久化的机器标识盐
///
/// 文件路径: `~/.OpenSunstar/.machine-id`
/// 格式: 32 字节随机数的十六进制字符串（64 字符）
///
/// 安全作用: 即使攻击者知道 hostname，没有此文件也无法推导回退密钥库的加密密钥。
/// 迁移注意: 复制 keystore.enc 到新机器时，必须同时复制 .machine-id。
fn read_or_create_machine_id() -> String {
    let id_path = get_app_config_dir().join(".machine-id");

    // 尝试读取已有标识
    if let Ok(id) = fs::read_to_string(&id_path) {
        let trimmed = id.trim();
        if trimmed.len() >= 16 {
            return trimmed.to_string();
        }
    }

    // 首次使用 — 生成密码学安全的随机标识
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let id = hex_encode(&bytes);

    // 持久化（写入失败不阻断 — 退化为仅 hostname，与旧行为一致）
    if let Some(parent) = id_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(e) = fs::write(&id_path, &id) {
        log::warn!("[Keychain] 无法持久化机器标识: {e}，回退到仅 hostname");
    }

    id
}

/// 将字节数组编码为十六进制字符串（避免引入 hex crate 依赖）
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn init_fallback_store() -> &'static Mutex<FallbackStore> {
    FALLBACK_STORE.get_or_init(|| {
        let entries = load_fallback_file().unwrap_or_default();
        Mutex::new(FallbackStore {
            entries,
            dirty: false,
        })
    })
}

fn load_fallback_file() -> Option<HashMap<String, String>> {
    let path = get_fallback_path();
    let encrypted = fs::read(&path).ok()?;
    let key = get_device_key();
    let decrypted = decrypt_data(&key, &encrypted).ok()?;
    let json_str = String::from_utf8(decrypted).ok()?;
    serde_json::from_str(&json_str).ok()
}

fn save_fallback_file(entries: &HashMap<String, String>) -> Result<(), AppError> {
    let path = get_fallback_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let json = serde_json::to_string(entries)
        .map_err(|e| AppError::Config(format!("Serialize keystore failed: {e}")))?;
    let key = get_device_key();
    let encrypted = encrypt_data(&key, json.as_bytes())?;
    fs::write(&path, &encrypted).map_err(|e| AppError::io(&path, e))?;
    Ok(())
}

fn store_fallback(entry_key: &str, secret: &str) -> Result<(), AppError> {
    let store = init_fallback_store();
    let mut guard = store
        .lock()
        .map_err(|e| AppError::Config(format!("Fallback store lock failed: {e}")))?;
    guard
        .entries
        .insert(entry_key.to_string(), secret.to_string());
    guard.dirty = true;
    save_fallback_file(&guard.entries)?;
    guard.dirty = false;
    Ok(())
}

fn get_fallback(entry_key: &str) -> Result<Option<String>, AppError> {
    let store = init_fallback_store();
    let guard = store
        .lock()
        .map_err(|e| AppError::Config(format!("Fallback store lock failed: {e}")))?;
    Ok(guard.entries.get(entry_key).cloned())
}

fn delete_fallback(entry_key: &str) -> Result<(), AppError> {
    let store = init_fallback_store();
    let mut guard = store
        .lock()
        .map_err(|e| AppError::Config(format!("Fallback store lock failed: {e}")))?;
    if guard.entries.remove(entry_key).is_some() {
        guard.dirty = true;
        save_fallback_file(&guard.entries)?;
        guard.dirty = false;
    }
    Ok(())
}

// ─── Migration helpers ──────────────────────────────────────

/// Migrate a plaintext API key to the keychain, returning the reference placeholder.
/// If the value is already a keychain ref, returns it unchanged.
pub fn migrate_key_to_keychain(
    provider_id: &str,
    app_type: &str,
    plaintext_key: &str,
) -> Result<String, AppError> {
    if is_keychain_ref(plaintext_key) || plaintext_key.is_empty() {
        return Ok(plaintext_key.to_string());
    }
    let entry_key = provider_entry_key(provider_id, app_type);
    store_secret(&entry_key, plaintext_key)?;
    Ok(make_keychain_ref(&entry_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_ref_roundtrip() {
        let entry_key = "test-provider/claude";
        let reference = make_keychain_ref(entry_key);
        assert!(is_keychain_ref(&reference));
        assert_eq!(extract_ref_key(&reference), Some(entry_key));
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let plaintext = b"hello world secret";
        let encrypted = encrypt_data(&key, plaintext).unwrap();
        let decrypted = decrypt_data(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let plaintext = b"secret data";
        let encrypted = encrypt_data(&key1, plaintext).unwrap();
        assert!(decrypt_data(&key2, &encrypted).is_err());
    }

    #[test]
    fn test_derive_snapshot_key_deterministic() {
        let master = [99u8; 32];
        let k1 = derive_snapshot_key(&master, "snap-001").unwrap();
        let k2 = derive_snapshot_key(&master, "snap-001").unwrap();
        let k3 = derive_snapshot_key(&master, "snap-002").unwrap();
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_non_ref_value_not_detected() {
        assert!(!is_keychain_ref("sk-abc123"));
        assert!(!is_keychain_ref(""));
    }
}
