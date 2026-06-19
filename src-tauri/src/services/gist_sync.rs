//! GitHub Gist sync backend.
//!
//! Uses GitHub REST API to store sync artifacts in a single private Gist.
//! Files: db.sql, skills.zip (base64-encoded), manifest.json

use crate::error::AppError;
use crate::keychain;
use crate::services::sync_protocol::{
    apply_snapshot_with_manifest, build_local_snapshot,
    validate_manifest_compat, verify_artifact, RemoteLayout, SyncManifest,
    REMOTE_DB_SQL, REMOTE_MANIFEST, REMOTE_SKILLS_ZIP,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

const GIST_DESCRIPTION_PREFIX: &str = "OpenSunstar Sync";
const KEYCHAIN_GIST_PAT: &str = "gist/github_pat";
const KEYCHAIN_GIST_ID: &str = "gist/gist_id";
const GITHUB_API: &str = "https://api.github.com";

pub fn sync_mutex() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[derive(Debug, Serialize)]
struct CreateGistRequest {
    description: String,
    public: bool,
    files: HashMap<String, GistFileContent>,
}

#[derive(Debug, Serialize)]
struct UpdateGistRequest {
    files: HashMap<String, GistFileContent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GistFileContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GistResponse {
    id: String,
    files: HashMap<String, GistFileInfo>,
}

#[derive(Debug, Deserialize)]
struct GistFileInfo {
    content: Option<String>,
    truncated: Option<bool>,
    raw_url: Option<String>,
}

fn get_pat() -> Result<String, AppError> {
    keychain::get_secret(KEYCHAIN_GIST_PAT)?
        .ok_or_else(|| AppError::Config("GitHub PAT not configured. Please set it in Sync settings.".to_string()))
}

fn get_gist_id() -> Result<Option<String>, AppError> {
    keychain::get_secret(KEYCHAIN_GIST_ID)
}

fn save_gist_id(id: &str) -> Result<(), AppError> {
    keychain::store_secret(KEYCHAIN_GIST_ID, id)
}

pub fn save_pat(pat: &str) -> Result<(), AppError> {
    keychain::store_secret(KEYCHAIN_GIST_PAT, pat)
}

pub fn clear_config() -> Result<(), AppError> {
    keychain::delete_secret(KEYCHAIN_GIST_PAT)?;
    keychain::delete_secret(KEYCHAIN_GIST_ID)?;
    Ok(())
}

pub fn is_configured() -> bool {
    keychain::get_secret(KEYCHAIN_GIST_PAT)
        .ok()
        .flatten()
        .is_some()
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

pub async fn upload(db: &crate::database::Database) -> Result<serde_json::Value, AppError> {
    let _lock = sync_mutex().lock().await;
    let pat = get_pat()?;
    let snapshot = build_local_snapshot(db)?;

    let db_sql_b64 = STANDARD.encode(&snapshot.db_sql);
    let skills_zip_b64 = STANDARD.encode(&snapshot.skills_zip);
    let manifest_str = String::from_utf8(snapshot.manifest_bytes.clone())
        .map_err(|e| AppError::Config(format!("Manifest not UTF-8: {e}")))?;

    let mut files = HashMap::new();
    files.insert(REMOTE_DB_SQL.to_string(), GistFileContent { content: db_sql_b64 });
    files.insert(REMOTE_SKILLS_ZIP.to_string(), GistFileContent { content: skills_zip_b64 });
    files.insert(REMOTE_MANIFEST.to_string(), GistFileContent { content: manifest_str });

    let gist_id = get_gist_id()?;

    let response = if let Some(id) = gist_id {
        let body = UpdateGistRequest { files };
        client()
            .patch(format!("{GITHUB_API}/gists/{id}"))
            .header(AUTHORIZATION, format!("Bearer {pat}"))
            .header(USER_AGENT, "OpenSunstar")
            .header(ACCEPT, "application/vnd.github+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?
    } else {
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        let body = CreateGistRequest {
            description: format!("{GIST_DESCRIPTION_PREFIX} - {device_name}"),
            public: false,
            files,
        };
        client()
            .post(format!("{GITHUB_API}/gists"))
            .header(AUTHORIZATION, format!("Bearer {pat}"))
            .header(USER_AGENT, "OpenSunstar")
            .header(ACCEPT, "application/vnd.github+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Network(format!("GitHub API error {status}: {body}")));
    }

    let gist: GistResponse = response.json().await
        .map_err(|e| AppError::Network(format!("Failed to parse Gist response: {e}")))?;
    save_gist_id(&gist.id)?;

    Ok(serde_json::json!({
        "status": "uploaded",
        "gist_id": gist.id,
        "snapshot_hash": snapshot.manifest_hash
    }))
}

pub async fn download(db: &crate::database::Database) -> Result<serde_json::Value, AppError> {
    let _lock = sync_mutex().lock().await;
    let pat = get_pat()?;
    let gist_id = get_gist_id()?
        .ok_or_else(|| AppError::Config("No Gist ID configured. Please upload first or set Gist ID.".to_string()))?;

    let response = client()
        .get(format!("{GITHUB_API}/gists/{gist_id}"))
        .header(AUTHORIZATION, format!("Bearer {pat}"))
        .header(USER_AGENT, "OpenSunstar")
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Network(format!("GitHub API error {status}: {body}")));
    }

    let gist: GistResponse = response.json().await
        .map_err(|e| AppError::Network(format!("Failed to parse Gist response: {e}")))?;

    let manifest_content = gist.files.get(REMOTE_MANIFEST)
        .and_then(|f| f.content.as_ref())
        .ok_or_else(|| AppError::Config("Gist missing manifest.json".to_string()))?;
    let manifest: SyncManifest = serde_json::from_str(manifest_content)
        .map_err(|e| AppError::Config(format!("Invalid manifest: {e}")))?;
    validate_manifest_compat(&manifest, RemoteLayout::Current)?;

    let db_sql_b64 = fetch_gist_file_content(&pat, &gist, REMOTE_DB_SQL).await?;
    let db_sql = STANDARD.decode(&db_sql_b64)
        .map_err(|e| AppError::Config(format!("Failed to decode db.sql: {e}")))?;

    let skills_zip_b64 = fetch_gist_file_content(&pat, &gist, REMOTE_SKILLS_ZIP).await?;
    let skills_zip = STANDARD.decode(&skills_zip_b64)
        .map_err(|e| AppError::Config(format!("Failed to decode skills.zip: {e}")))?;

    if let Some(meta) = manifest.artifacts.get(REMOTE_DB_SQL) {
        verify_artifact(&db_sql, REMOTE_DB_SQL, meta)?;
    }
    if let Some(meta) = manifest.artifacts.get(REMOTE_SKILLS_ZIP) {
        verify_artifact(&skills_zip, REMOTE_SKILLS_ZIP, meta)?;
    }

    apply_snapshot_with_manifest(db, &db_sql, &skills_zip, Some(&manifest))?;

    Ok(serde_json::json!({
        "status": "downloaded",
        "gist_id": gist_id,
        "device_name": manifest.device_name
    }))
}

/// Fetch a file's content from a Gist response, following raw_url if truncated.
async fn fetch_gist_file_content(
    pat: &str,
    gist: &GistResponse,
    file_name: &str,
) -> Result<String, AppError> {
    let file_info = gist.files.get(file_name)
        .ok_or_else(|| AppError::Config(format!("Gist missing {file_name}")))?;

    if file_info.truncated.unwrap_or(false) {
        let raw_url = file_info.raw_url.as_ref()
            .ok_or_else(|| AppError::Config(format!("Truncated {file_name} but no raw_url")))?;
        fetch_raw(pat, raw_url).await
    } else {
        file_info.content.clone()
            .ok_or_else(|| AppError::Config(format!("Gist file {file_name} has no content")))
    }
}

async fn fetch_raw(pat: &str, url: &str) -> Result<String, AppError> {
    let resp = client()
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {pat}"))
        .header(USER_AGENT, "OpenSunstar")
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;
    resp.text().await.map_err(|e| AppError::Network(e.to_string()))
}

pub async fn test_connection() -> Result<serde_json::Value, AppError> {
    let pat = get_pat()?;
    let resp = client()
        .get(format!("{GITHUB_API}/user"))
        .header(AUTHORIZATION, format!("Bearer {pat}"))
        .header(USER_AGENT, "OpenSunstar")
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(AppError::Network(format!("GitHub authentication failed: {}", resp.status())));
    }

    let user: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Network(format!("Parse user response failed: {e}")))?;
    let login = user.get("login").and_then(|v| v.as_str()).unwrap_or("unknown");

    Ok(serde_json::json!({
        "status": "connected",
        "username": login,
        "gist_id": get_gist_id()?.unwrap_or_default()
    }))
}
