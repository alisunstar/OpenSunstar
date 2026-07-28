//! 凭证槽位绑定（Git MVP M7）
//!
//! 将团队配置中声明的凭证槽位（CredentialSlot）绑定到实际密钥值。
//! MVP 约束（冻结文档 §二）：
//! - 仅 env 型（api_key / token），不实现 OAuth 流
//! - 仅 Claude Code + Codex 目标
//! - 复用 keychain.rs 存储（D5 决策）
//!
//! 绑定模型：slot_id → keychain entry `team/{workspace_id}/{slot_id}`
//! 凭证值与配置描述严格分离（第二版 §9）。

use serde::{Deserialize, Serialize};

use super::domain::CredentialSlot;

// ─── 绑定状态 ──────────────────────────────────────────────────────────────────

/// 单个槽位的绑定状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CredentialStatus {
    /// 已绑定且验证通过
    Bound,
    /// 未绑定（keychain 中无条目）
    Unbound,
    /// 已绑定但验证失败（值为空或不可读）
    Invalid,
}

impl CredentialStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bound => "bound",
            Self::Unbound => "unbound",
            Self::Invalid => "invalid",
        }
    }
}

/// 单个槽位的绑定信息
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialBindingInfo {
    pub slot_id: String,
    pub kind: String,
    pub provider: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    pub status: CredentialStatus,
}

/// 凭证绑定汇总
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSummary {
    pub total_slots: usize,
    pub bound_count: usize,
    pub unbound_count: usize,
    pub invalid_count: usize,
    /// 所有必需槽位是否已绑定
    pub all_required_bound: bool,
    /// 未绑定的必需槽位 ID 列表
    pub missing_required: Vec<String>,
}

// ─── Keychain 条目键 ───────────────────────────────────────────────────────────

/// 构建凭证在 keychain 中的条目键
///
/// 格式：`team/{workspace_id}/{slot_id}`
/// 与 provider 凭证（`{provider_id}/{app_type}`）命名空间隔离。
pub fn credential_entry_key(workspace_id: &str, slot_id: &str) -> String {
    format!("team/{workspace_id}/{slot_id}")
}

// ─── 绑定操作 ──────────────────────────────────────────────────────────────────

/// 绑定凭证值到槽位
///
/// 将密钥值存入 OS Keychain（或加密回退存储）。
/// 绑定后可通过 `verify_credential` 验证可访问性。
pub fn bind_credential(workspace_id: &str, slot_id: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("凭证值不能为空".to_string());
    }

    let entry_key = credential_entry_key(workspace_id, slot_id);
    crate::keychain::store_secret(&entry_key, value).map_err(|e| format!("存储凭证失败: {e}"))
}

/// 解绑凭证（从 keychain 真正移除）
///
/// C5 修复：使用 delete_secret 真正删除条目，而非空值覆盖。
/// 离职成员或凭证轮换场景下，旧值不再残留。
pub fn unbind_credential(workspace_id: &str, slot_id: &str) -> Result<(), String> {
    let entry_key = credential_entry_key(workspace_id, slot_id);
    crate::keychain::delete_secret(&entry_key).map_err(|e| format!("解绑凭证失败: {e}"))
}

/// 验证单个凭证是否可访问
pub fn verify_credential(workspace_id: &str, slot_id: &str) -> CredentialStatus {
    let entry_key = credential_entry_key(workspace_id, slot_id);
    match crate::keychain::get_secret(&entry_key) {
        Ok(Some(value)) if !value.is_empty() => CredentialStatus::Bound,
        Ok(_) => CredentialStatus::Unbound,
        Err(_) => CredentialStatus::Invalid,
    }
}

/// 批量检查所有槽位的绑定状态
pub fn check_credentials_status(
    workspace_id: &str,
    slots: &[CredentialSlot],
) -> Vec<CredentialBindingInfo> {
    slots
        .iter()
        .map(|slot| CredentialBindingInfo {
            slot_id: slot.slot_id.clone(),
            kind: slot.kind.clone(),
            provider: slot.provider.clone(),
            description: slot.description.clone(),
            required: slot.required,
            status: verify_credential(workspace_id, &slot.slot_id),
        })
        .collect()
}

/// 计算凭证绑定汇总
pub fn compute_credential_summary(bindings: &[CredentialBindingInfo]) -> CredentialSummary {
    let mut summary = CredentialSummary {
        total_slots: bindings.len(),
        bound_count: 0,
        unbound_count: 0,
        invalid_count: 0,
        all_required_bound: true,
        missing_required: Vec::new(),
    };

    for b in bindings {
        match b.status {
            CredentialStatus::Bound => summary.bound_count += 1,
            CredentialStatus::Unbound => {
                summary.unbound_count += 1;
                if b.required {
                    summary.all_required_bound = false;
                    summary.missing_required.push(b.slot_id.clone());
                }
            }
            CredentialStatus::Invalid => {
                summary.invalid_count += 1;
                if b.required {
                    summary.all_required_bound = false;
                    summary.missing_required.push(b.slot_id.clone());
                }
            }
        }
    }

    summary
}

// ─── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_slot(slot_id: &str, required: bool) -> CredentialSlot {
        CredentialSlot {
            slot_id: slot_id.to_string(),
            kind: "api_key".to_string(),
            provider: Some("openai".to_string()),
            description: Some("OpenAI API Key".to_string()),
            required,
        }
    }

    #[test]
    fn entry_key_format() {
        assert_eq!(
            credential_entry_key("ws_abc123", "openai_key"),
            "team/ws_abc123/openai_key"
        );
    }

    #[test]
    fn bind_and_verify() {
        let ws = "ws_test_bind";
        let slot = "test_key_1";

        // 初始状态：未绑定
        assert_eq!(verify_credential(ws, slot), CredentialStatus::Unbound);

        // 绑定
        bind_credential(ws, slot, "sk-test-12345").unwrap();
        assert_eq!(verify_credential(ws, slot), CredentialStatus::Bound);

        // 解绑（C5 修复：真正删除，旧值不残留）
        unbind_credential(ws, slot).unwrap();
        assert_eq!(verify_credential(ws, slot), CredentialStatus::Unbound);
    }

    #[test]
    fn bind_rejects_empty_value() {
        let result = bind_credential("ws_x", "slot_y", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不能为空"));
    }

    #[test]
    fn batch_status_check() {
        let ws = "ws_batch_test";
        let slots = vec![make_slot("key_a", true), make_slot("key_b", false)];

        // 绑定 key_a
        bind_credential(ws, "key_a", "value_a").unwrap();

        let bindings = check_credentials_status(ws, &slots);
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].status, CredentialStatus::Bound);
        assert_eq!(bindings[1].status, CredentialStatus::Unbound);
    }

    #[test]
    fn summary_all_required_bound() {
        let ws = "ws_summary_test";
        let slots = vec![make_slot("req_key", true), make_slot("opt_key", false)];

        bind_credential(ws, "req_key", "secret").unwrap();

        let bindings = check_credentials_status(ws, &slots);
        let summary = compute_credential_summary(&bindings);

        assert!(summary.all_required_bound);
        assert!(summary.missing_required.is_empty());
        assert_eq!(summary.bound_count, 1);
        assert_eq!(summary.unbound_count, 1);
    }

    #[test]
    fn summary_missing_required() {
        let ws = "ws_missing_req";
        let slots = vec![
            make_slot("must_have", true),
            make_slot("nice_to_have", false),
        ];

        // 不绑定任何
        let bindings = check_credentials_status(ws, &slots);
        let summary = compute_credential_summary(&bindings);

        assert!(!summary.all_required_bound);
        assert_eq!(summary.missing_required, vec!["must_have"]);
    }
}
