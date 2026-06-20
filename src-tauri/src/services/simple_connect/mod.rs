//! Simple Connect — 双模配置器共享内核（Phase 0 Spike + Phase 1 MVP）

pub mod apply;
pub mod backup;
pub mod backup_audit;
pub mod cli_claude;
pub mod deeplink;
pub mod key_store;
pub mod pool;
pub mod proxy_poc;
pub mod security;
pub mod state;
pub mod suppliers;
pub mod token_usage;
pub mod tools;
pub mod usage;
pub mod verify;

pub use apply::{
    apply_tool, clear_tool, get_state, is_managed_marker, list_tool_status, save_pool_state,
    ApplyResult, ToolConfigStatus,
};
pub use backup::backup_file;
pub use backup_audit::{run_backup_audit, BackupAuditItem, BackupAuditReport};
pub use cli_claude::{apply_claude_code, is_managed_settings, ClaudeApplyResult};
pub use deeplink::{
    import_keys, is_simple_connect_import_url, try_parse_url, SimpleConnectImportPayload,
    SimpleConnectImportResult,
};
pub use tools::MANAGED_MARKER;
pub use key_store::{
    delete_api_key, get_primary_key, key_hint, store_api_key, store_primary_key,
};
pub use security::{run_p0_security_audit, P0CheckItem, P0SecurityReport};
pub use pool::{build_runtime_pool, KeyPool, PoolKey, PoolKeyStat, PoolSimulationStep};
pub use proxy_poc::{
    fetch_models_via_proxy, pool_runtime_stats, spike_proxy_info, start_spike_proxy,
    stop_spike_proxy, SimpleConnectRuntimeStats, SpikeProxyInfo, SPIKE_PROXY_PORT,
};
pub use state::{load_state, save_state, set_supplier, PoolKeyMeta, SimpleConnectState};
pub use suppliers::{
    get_supplier, list_builtin_suppliers, resolve_supplier, SupplierProfile,
};
pub use verify::{verify_api_key, VerifyKeyResult};
pub use usage::{build_usage_summary, SimpleConnectUsageSummary};
pub use token_usage::{snapshot as token_usage_snapshot, ScTokenUsage};
pub use tools::{ALL_TOOLS, PHASE1_TOOLS};

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SpikeReport {
    pub s1_keychain: SpikeStepResult,
    pub s2_claude_apply: SpikeStepResult,
    pub s3_proxy: SpikeStepResult,
    pub s4_pool: SpikeStepResult,
    pub all_passed: bool,
}

#[derive(Debug, Serialize)]
pub struct SpikeStepResult {
    pub id: String,
    pub ok: bool,
    pub detail: String,
}

pub fn run_pool_demo() -> Vec<PoolSimulationStep> {
    KeyPool::simulate_failover_demo()
}

/// Phase 0 Spike 自检（S1–S4）。S3 需有效 Key + 网络；S2 会写入真实 Claude settings。
pub async fn run_spike_report(
    supplier_id: &str,
    test_api_key: Option<&str>,
) -> Result<SpikeReport, crate::error::AppError> {
    let mut s1 = SpikeStepResult {
        id: "S1".into(),
        ok: false,
        detail: String::new(),
    };
    let mut s2 = SpikeStepResult {
        id: "S2".into(),
        ok: false,
        detail: String::new(),
    };
    let mut s3 = SpikeStepResult {
        id: "S3".into(),
        ok: false,
        detail: String::new(),
    };
    let mut s4 = SpikeStepResult {
        id: "S4".into(),
        ok: false,
        detail: String::new(),
    };

    if let Some(key) = test_api_key.map(str::trim).filter(|k| !k.is_empty()) {
        match store_primary_key(supplier_id, key) {
            Ok(()) => match get_primary_key(supplier_id) {
                Ok(Some(roundtrip)) if roundtrip == key => {
                    s1.ok = true;
                    s1.detail = format!("Keychain 读写 OK，hint={}", key_hint(&roundtrip));
                }
                Ok(_) => s1.detail = "Keychain 读回不匹配".into(),
                Err(e) => s1.detail = format!("Keychain 读取失败: {e}"),
            },
            Err(e) => s1.detail = format!("Keychain 写入失败: {e}"),
        }
    } else if get_primary_key(supplier_id)?.is_some() {
        s1.ok = true;
        s1.detail = "使用已存在的 Keychain 条目".into();
    } else {
        s1.detail = "跳过：未提供 test_api_key 且 Keychain 无条目".into();
    }

    if let Some(key) = get_primary_key(supplier_id)? {
        if let Some(supplier) = get_supplier(supplier_id) {
            let base = supplier
                .anthropic_base
                .as_deref()
                .unwrap_or(supplier.openai_base.as_str());
            let model = if supplier.default_model.is_empty() {
                "deepseek-chat".to_string()
            } else {
                supplier.default_model.clone()
            };
            match apply_claude_code(&key, &model, base) {
                Ok(r) => {
                    s2.ok = true;
                    s2.detail = format!("已写入 {} (base={})", r.settings_path, r.base_url);
                }
                Err(e) => s2.detail = format!("Claude 写入失败: {e}"),
            }
        } else {
            s2.detail = "供应商预设未找到".into();
        }
    } else {
        s2.detail = "跳过：无 Keychain Key".into();
    }

    if s1.ok {
        if let Some(supplier) = get_supplier(supplier_id) {
            match fetch_models_via_proxy(supplier_id, &supplier.openai_base).await {
                Ok(models) => {
                    s3.ok = !models.is_empty();
                    s3.detail = format!(
                        "代理已启动 (:{SPIKE_PROXY_PORT})，模型数={}，示例={}",
                        models.len(),
                        models.first().cloned().unwrap_or_default()
                    );
                }
                Err(e) => s3.detail = format!("模型拉取失败（可能 Key/网络）: {e}"),
            }
        }
    } else {
        s3.detail = "跳过：S1 未通过".into();
    }

    let steps = run_pool_demo();
    s4.ok = steps.iter().any(|s| s.key_id == "k2" && s.action == "pick");
    s4.detail = format!("模拟 {} 步，含 429 failover", steps.len());

    let all_passed = s1.ok && s2.ok && s3.ok && s4.ok;

    Ok(SpikeReport {
        s1_keychain: s1,
        s2_claude_apply: s2,
        s3_proxy: s3,
        s4_pool: s4,
        all_passed,
    })
}

#[cfg(test)]
mod spike_tests {
    use super::*;
    use crate::services::simple_connect::key_store::{delete_api_key, entry_key, store_primary_key};
    use serial_test::serial;

    fn unique_supplier() -> String {
        format!("phase0spike{}", uuid::Uuid::new_v4().simple())
    }

    #[test]
    fn offline_kernel_suppliers_and_pool() {
        assert!(
            list_builtin_suppliers()
                .iter()
                .all(|s| s.id != "beeapi" && !s.openai_base.contains("beeapi.ai"))
        );
        let steps = run_pool_demo();
        assert!(steps.iter().any(|s| s.action == "fail" && s.key_id == "k1"));
        assert!(steps.iter().any(|s| s.action == "pick" && s.key_id == "k2"));
    }

    #[test]
    fn key_store_entry_shape_and_empty_rejected() {
        assert_eq!(
            entry_key("deepseek", "primary"),
            "simple-connect/deepseek/primary"
        );
        assert!(store_primary_key("deepseek", "   ").is_err());
        assert_eq!(key_hint("sk-abcd1234wxyz"), "sk-a****wxyz");
    }

    #[test]
    #[serial]
    #[ignore = "manual or local: OS keychain roundtrip"]
    fn offline_keychain_roundtrip() {
        let supplier = unique_supplier();
        let key = format!("sk-phase0-test-{}", uuid::Uuid::new_v4());
        store_primary_key(&supplier, &key).expect("store");
        let read = get_primary_key(&supplier)
            .expect("read")
            .unwrap_or_else(|| {
                panic!(
                    "Keychain roundtrip failed for entry {}",
                    entry_key(&supplier, "primary")
                )
            });
        assert_eq!(read, key);
        delete_api_key(&supplier, "primary").expect("cleanup");
    }

    #[test]
    fn backup_creates_timestamped_copy() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("settings.json");
        std::fs::write(&src, r#"{"env":{}}"#).unwrap();
        let backup = backup_file("claude-code", &src)
            .expect("backup")
            .expect("path");
        assert!(backup.exists());
        let content = std::fs::read_to_string(&backup).unwrap();
        assert!(content.contains("env"));
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires OS keychain to bootstrap proxy pool"]
    async fn spike_proxy_health_responds_ok() {
        let supplier = unique_supplier();
        store_primary_key(&supplier, "sk-test-health-only-proxy")
            .expect("store test key for proxy bind");
        let info = start_spike_proxy(&supplier, "https://api.deepseek.com")
            .await
            .expect("start proxy");
        let url = format!("{}/__simple_connect/health", info.local_base);
        let resp = reqwest::get(&url).await.expect("health request");
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.text().await.unwrap(), "ok");
        stop_spike_proxy().await;
        let _ = delete_api_key(&supplier, "primary");
    }
}
