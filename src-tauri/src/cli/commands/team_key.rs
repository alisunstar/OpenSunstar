//! `os team key` — 团队密钥管理 (D16)
//!
//! 本地命令（离线可用）：list, status
//! 网络命令（需控制面连接）：sync, slot create, slot rotate, rotation-status

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct TeamKeyArgs {
    #[command(subcommand)]
    pub action: TeamKeyAction,
}

#[derive(Subcommand)]
pub enum TeamKeyAction {
    /// 列出本机持有的团队密钥槽位
    List {
        /// 按组织 ID 过滤
        #[arg(long)]
        org_id: Option<String>,
    },
    /// 查看密钥状态（含 grant 过期检测）
    Status {
        /// 指定槽位 slug（不指定则显示全部）
        #[arg(long)]
        slot: Option<String>,
    },
    /// 从控制面同步团队密钥（下发 + 续期）
    Sync {
        /// 组织 ID
        #[arg(long)]
        org_id: String,
    },
    /// 密钥槽位管理（管理员操作）
    Slot {
        #[command(subcommand)]
        action: SlotAction,
    },
}

#[derive(Subcommand)]
pub enum SlotAction {
    /// 创建新密钥槽位并上传初始 Key
    Create {
        /// 组织 ID
        #[arg(long)]
        org_id: String,
        /// 槽位标识（如 openrouter-main）
        #[arg(long)]
        slug: String,
        /// 显示名称
        #[arg(long)]
        name: String,
        /// 供应商类型: openrouter|anthropic|openai|github|custom
        #[arg(long)]
        provider: String,
        /// 自定义端点 URL（provider=custom 时必填）
        #[arg(long)]
        endpoint: Option<String>,
        /// Key 值（直接传入）
        #[arg(long)]
        value: Option<String>,
        /// Key 值文件路径（与 --value 二选一）
        #[arg(long)]
        value_file: Option<String>,
    },
    /// 轮转密钥（上传新 Key，旧 Key 进入 24h 宽限期）
    Rotate {
        /// 组织 ID
        #[arg(long)]
        org_id: String,
        /// 槽位标识
        #[arg(long)]
        slug: String,
        /// 新 Key 值
        #[arg(long)]
        value: Option<String>,
        /// 新 Key 值文件路径
        #[arg(long)]
        value_file: Option<String>,
    },
    /// 查看轮转状态（哪些设备已确认切换）
    RotationStatus {
        /// 组织 ID
        #[arg(long)]
        org_id: String,
        /// 槽位标识
        #[arg(long)]
        slug: String,
    },
}

pub fn run(
    args: TeamKeyArgs,
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    match args.action {
        TeamKeyAction::List { org_id } => run_list(state, org_id, json),
        TeamKeyAction::Status { slot } => run_status(state, slot, json),
        TeamKeyAction::Sync { org_id } => run_sync(state, &org_id, json),
        TeamKeyAction::Slot { action } => match action {
            SlotAction::Create {
                org_id,
                slug,
                name,
                provider,
                endpoint,
                value,
                value_file,
            } => run_slot_create(
                state, &org_id, &slug, &name, &provider, endpoint, value, value_file, json,
            ),
            SlotAction::Rotate {
                org_id,
                slug,
                value,
                value_file,
            } => run_slot_rotate(state, &org_id, &slug, value, value_file, json),
            SlotAction::RotationStatus { org_id, slug } => {
                run_rotation_status(state, &org_id, &slug, json)
            }
        },
    }
}

// ─── Local commands (offline) ──────────────────────────────────────────────────

fn run_list(
    state: &open_sunstar_lib::AppState,
    org_id: Option<String>,
    json: bool,
) -> Result<(), String> {
    let keys = match org_id {
        Some(id) => state.db.list_team_keys(&id).map_err(|e| e.to_string())?,
        None => state.db.list_all_team_keys().map_err(|e| e.to_string())?,
    };

    if json {
        output::print_result(&keys, true);
    } else if keys.is_empty() {
        println!("本机暂无团队密钥。运行 `os team key sync --org-id <ID>` 从控制面同步。");
    } else {
        println!(
            "{:<20} {:<12} {:<8} {:<10} {}",
            "SLOT", "PROVIDER", "VERSION", "STATUS", "GRANT_EXPIRES"
        );
        for key in &keys {
            let expires = chrono::DateTime::from_timestamp_millis(key.grant_expires)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "{:<20} {:<12} {:<8} {:<10} {}",
                key.slot_slug, key.provider_kind, key.version_seq, key.status, expires
            );
        }
    }
    Ok(())
}

fn run_status(
    state: &open_sunstar_lib::AppState,
    slot: Option<String>,
    json: bool,
) -> Result<(), String> {
    // Check grant expiry first
    let expired =
        open_sunstar_lib::team_key::check_grant_expiry(&state.db).map_err(|e| e.to_string())?;
    if expired > 0 && !json {
        eprintln!("⚠ {expired} 个团队密钥 grant 已过期，请运行 `os team key sync` 续期");
    }

    match slot {
        Some(slug) => {
            let key = state.db.get_team_key(&slug).map_err(|e| e.to_string())?;
            match key {
                Some(k) => {
                    if json {
                        output::print_result(&k, true);
                    } else {
                        println!("槽位: {}", k.slot_slug);
                        println!("组织: {}", k.org_id);
                        println!("供应商: {}", k.provider_kind);
                        if let Some(url) = &k.endpoint_url {
                            println!("端点: {url}");
                        }
                        println!("版本: {}", k.version_seq);
                        println!("状态: {}", k.status);
                        println!("Grant ID: {}", k.grant_id);
                        let expires = chrono::DateTime::from_timestamp_millis(k.grant_expires)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        println!("Grant 到期: {expires}");
                        println!("SHA-256: {}", k.value_sha256);
                    }
                }
                None => {
                    return Err(format!(
                        "team_key_not_found: 本机未持有槽位 '{slug}' 的团队密钥"
                    ));
                }
            }
        }
        None => {
            let keys = state.db.list_all_team_keys().map_err(|e| e.to_string())?;
            if json {
                output::print_result(&keys, true);
            } else {
                let active = keys.iter().filter(|k| k.status == "active").count();
                let expired_count = keys.iter().filter(|k| k.status == "expired").count();
                let revoked = keys.iter().filter(|k| k.status == "revoked").count();
                println!(
                    "团队密钥状态: {} active, {} expired, {} revoked (共 {} 个)",
                    active,
                    expired_count,
                    revoked,
                    keys.len()
                );
            }
        }
    }
    Ok(())
}

// ─── Network commands (require control plane) ──────────────────────────────────

fn run_sync(state: &open_sunstar_lib::AppState, org_id: &str, json: bool) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio_runtime_failed: {e}"))?;
    let result = rt.block_on(async {
        let base_url = control_plane_url()?;
        let access_token = load_access_token()?;
        let device_id = load_device_id()?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|_| "team_key_client_failed".to_string())?;

        // Fetch grants
        let url = format!("{base_url}/v1/organizations/{org_id}/keys/grants?device_id={device_id}");
        let response = client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|_| "team_key_control_plane_unavailable".to_string())?;

        if !response.status().is_success() {
            return Err(format!(
                "team_key_sync_failed_{}",
                response.status().as_u16()
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "team_key_response_invalid".to_string())?;

        let grants = body
            .get("grants")
            .and_then(|g| g.as_array())
            .cloned()
            .unwrap_or_default();
        let mut synced = 0u32;
        let mut grant_ids: Vec<String> = Vec::new();

        for grant in &grants {
            let slot_slug = grant
                .get("slotSlug")
                .and_then(|s| s.as_str())
                .unwrap_or_default();
            let plaintext = grant
                .get("plaintext")
                .and_then(|p| p.as_str())
                .unwrap_or_default();
            let provider_kind = grant
                .get("providerKind")
                .and_then(|p| p.as_str())
                .unwrap_or("custom");
            let endpoint_url = grant.get("endpointUrl").and_then(|e| e.as_str());
            let version_seq = grant
                .get("versionSeq")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let value_sha256 = grant
                .get("valueSha256")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let grant_id = grant
                .get("grantId")
                .and_then(|g| g.as_str())
                .unwrap_or_default();
            let expires_at = grant
                .get("expiresAt")
                .and_then(|e| e.as_str())
                .unwrap_or_default();

            if slot_slug.is_empty() || plaintext.is_empty() {
                continue;
            }

            let grant_expires = chrono::DateTime::parse_from_rfc3339(expires_at)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);

            open_sunstar_lib::team_key::store_team_key(
                &state.db,
                org_id,
                slot_slug,
                provider_kind,
                endpoint_url,
                plaintext,
                version_seq,
                value_sha256,
                grant_id,
                grant_expires,
            )
            .map_err(|e| format!("team_key_store_failed: {e}"))?;

            grant_ids.push(grant_id.to_string());
            synced += 1;
        }

        // Ack
        if !grant_ids.is_empty() {
            let ack_url = format!("{base_url}/v1/organizations/{org_id}/keys/grants/ack");
            let _ = client
                .post(&ack_url)
                .bearer_auth(&access_token)
                .json(&serde_json::json!({ "grantIds": grant_ids }))
                .send()
                .await;
        }

        Ok::<u32, String>(synced)
    })?;

    if json {
        output::print_result(&serde_json::json!({ "synced": result }), true);
    } else {
        println!("✓ 已同步 {result} 个团队密钥到本机 Keychain");
    }
    Ok(())
}

fn run_slot_create(
    _state: &open_sunstar_lib::AppState,
    org_id: &str,
    slug: &str,
    name: &str,
    provider: &str,
    endpoint: Option<String>,
    value: Option<String>,
    value_file: Option<String>,
    json: bool,
) -> Result<(), String> {
    let key_value = resolve_key_value(value, value_file)?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio_runtime_failed: {e}"))?;
    let result = rt.block_on(async {
        let base_url = control_plane_url()?;
        let access_token = load_access_token()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|_| "team_key_client_failed".to_string())?;

        let url = format!("{base_url}/v1/organizations/{org_id}/keys/slots");
        let mut body = serde_json::json!({
            "slug": slug,
            "displayName": name,
            "providerKind": provider,
            "keyValue": key_value,
        });
        if let Some(ep) = endpoint {
            body["endpointUrl"] = serde_json::Value::String(ep);
        }

        let response = client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&body)
            .send()
            .await
            .map_err(|_| "team_key_control_plane_unavailable".to_string())?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let err_body = response.text().await.unwrap_or_default();
            return Err(format!("team_key_slot_create_failed_{status}: {err_body}"));
        }

        response
            .json::<serde_json::Value>()
            .await
            .map_err(|_| "team_key_response_invalid".to_string())
    })?;

    if json {
        output::print_result(&result, true);
    } else {
        println!("✓ 槽位 '{slug}' 创建成功 (provider={provider})");
    }
    Ok(())
}

fn run_slot_rotate(
    _state: &open_sunstar_lib::AppState,
    org_id: &str,
    slug: &str,
    value: Option<String>,
    value_file: Option<String>,
    json: bool,
) -> Result<(), String> {
    let key_value = resolve_key_value(value, value_file)?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio_runtime_failed: {e}"))?;
    let result = rt.block_on(async {
        let base_url = control_plane_url()?;
        let access_token = load_access_token()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|_| "team_key_client_failed".to_string())?;

        let url = format!("{base_url}/v1/organizations/{org_id}/keys/slots/{slug}/rotate");
        let response = client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&serde_json::json!({ "keyValue": key_value }))
            .send()
            .await
            .map_err(|_| "team_key_control_plane_unavailable".to_string())?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let err_body = response.text().await.unwrap_or_default();
            return Err(format!("team_key_rotate_failed_{status}: {err_body}"));
        }

        response
            .json::<serde_json::Value>()
            .await
            .map_err(|_| "team_key_response_invalid".to_string())
    })?;

    if json {
        output::print_result(&result, true);
    } else {
        let new_seq = result
            .get("newVersionSeq")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let devices_old = result
            .get("devicesHoldingOld")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        println!("✓ 槽位 '{slug}' 已轮转到版本 v{new_seq}");
        if devices_old > 0 {
            println!("  ⚠ {devices_old} 台设备仍持有旧版本 Key（24h 宽限期内自动切换）");
        }
    }
    Ok(())
}

fn run_rotation_status(
    _state: &open_sunstar_lib::AppState,
    org_id: &str,
    slug: &str,
    json: bool,
) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio_runtime_failed: {e}"))?;
    let result = rt.block_on(async {
        let base_url = control_plane_url()?;
        let access_token = load_access_token()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|_| "team_key_client_failed".to_string())?;

        let url = format!("{base_url}/v1/organizations/{org_id}/keys/slots/{slug}/rotation-status");
        let response = client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|_| "team_key_control_plane_unavailable".to_string())?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            return Err(format!("team_key_rotation_status_failed_{status}"));
        }

        response
            .json::<serde_json::Value>()
            .await
            .map_err(|_| "team_key_response_invalid".to_string())
    })?;

    if json {
        output::print_result(&result, true);
    } else {
        let total = result
            .get("totalDevices")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let acked = result
            .get("ackedDevices")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let version = result
            .get("activeVersionSeq")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        println!("槽位 '{slug}' 轮转状态 (当前版本 v{version}):");
        println!("  已确认: {acked}/{total} 台设备");
        if let Some(pending) = result.get("pendingDevices").and_then(|p| p.as_array()) {
            if !pending.is_empty() {
                println!("  待切换:");
                for device in pending {
                    let device_id = device
                        .get("deviceId")
                        .and_then(|d| d.as_str())
                        .unwrap_or("?");
                    let user_id = device.get("userId").and_then(|u| u.as_str()).unwrap_or("?");
                    println!("    - device={device_id} user={user_id}");
                }
            }
        }
    }
    Ok(())
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

fn resolve_key_value(value: Option<String>, value_file: Option<String>) -> Result<String, String> {
    match (value, value_file) {
        (Some(v), _) => Ok(v),
        (None, Some(path)) => std::fs::read_to_string(&path)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("team_key_file_read_failed: {e}")),
        (None, None) => {
            Err("team_key_value_required: 需要 --value 或 --value-file 参数".to_string())
        }
    }
}

fn control_plane_url() -> Result<String, String> {
    let raw = std::env::var("OPENSUNSTAR_CONTROL_PLANE_URL")
        .ok()
        .or_else(|| option_env!("OPENSUNSTAR_CONTROL_PLANE_URL_DEFAULT").map(str::to_string))
        .ok_or_else(|| {
            "team_key_control_plane_not_configured: 设置 OPENSUNSTAR_CONTROL_PLANE_URL 环境变量"
                .to_string()
        })?;
    Ok(raw.trim_end_matches('/').to_string())
}

fn load_access_token() -> Result<String, String> {
    let session_json = open_sunstar_lib::keychain::get_secret("product/auth/session-v1")
        .map_err(|e| format!("team_key_session_read_failed: {e}"))?
        .ok_or_else(|| "team_key_not_logged_in: 请先在 GUI 中登录产品账户".to_string())?;
    let session: serde_json::Value =
        serde_json::from_str(&session_json).map_err(|_| "team_key_session_invalid".to_string())?;
    session
        .get("access_token")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "team_key_session_no_token".to_string())
}

fn load_device_id() -> Result<String, String> {
    open_sunstar_lib::keychain::get_secret("product/auth/device-id-v1")
        .map_err(|e| format!("team_key_device_id_failed: {e}"))?
        .ok_or_else(|| "team_key_no_device_id: 请先在 GUI 中登录".to_string())
}
