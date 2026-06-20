//! Simple Connect API Key → OS Keychain（复用 `crate::keychain`）

use crate::error::AppError;
use crate::keychain;

const PRIMARY_KEY_ID: &str = "primary";

pub fn entry_key(supplier_id: &str, key_id: &str) -> String {
    format!("simple-connect/{supplier_id}/{key_id}")
}

pub fn store_api_key(supplier_id: &str, key_id: &str, secret: &str) -> Result<(), AppError> {
    if secret.trim().is_empty() {
        return Err(AppError::Message("API Key 不能为空".into()));
    }
    keychain::store_secret(&entry_key(supplier_id, key_id), secret.trim())
}

pub fn get_api_key(supplier_id: &str, key_id: &str) -> Result<Option<String>, AppError> {
    keychain::get_secret(&entry_key(supplier_id, key_id))
}

pub fn delete_api_key(supplier_id: &str, key_id: &str) -> Result<(), AppError> {
    keychain::delete_secret(&entry_key(supplier_id, key_id))
}

pub fn store_primary_key(supplier_id: &str, secret: &str) -> Result<(), AppError> {
    store_api_key(supplier_id, PRIMARY_KEY_ID, secret)
}

pub fn get_primary_key(supplier_id: &str) -> Result<Option<String>, AppError> {
    get_api_key(supplier_id, PRIMARY_KEY_ID)
}

pub fn key_hint(secret: &str) -> String {
    if secret.len() <= 8 {
        return "****".to_string();
    }
    format!("{}****{}", &secret[..4], &secret[secret.len() - 4..])
}
