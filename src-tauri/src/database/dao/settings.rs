//! 通用设置数据访问对象
//!
//! 提供键值对形式的通用设置存储。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;

impl Database {
    const LEGACY_COMMON_CONFIG_MIGRATED_KEY: &'static str = "common_config_legacy_migrated_v1";

    fn config_snippet_cleared_key(app_type: &str) -> String {
        format!("common_config_{app_type}_cleared")
    }

    /// 获取设置值
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT value FROM settings WHERE key = ?1")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut rows = stmt
            .query(params![key])
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(
                row.get(0).map_err(|e| AppError::Database(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    /// 以布尔语义读取 flag：`"true"` 或 `"1"` → true，其它全部 false。
    ///
    /// 用于一次性启动 flag（`official_providers_seeded` / `first_run_notice_shown` 等）。
    /// 与 `is_legacy_common_config_migrated` 等只认 `"true"` 的历史辅助函数**不同**——
    /// 这里同时接受 `"1"` 是为了兼容 `init_default_official_providers` 既有写法。
    pub fn get_bool_flag(&self, key: &str) -> Result<bool, AppError> {
        Ok(matches!(
            self.get_setting(key)?.as_deref(),
            Some("true") | Some("1")
        ))
    }

    /// 设置值
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 获取（或首次生成）代理接管占位令牌。
    ///
    /// 历史上接管态会向各 CLI 写入固定常量 `PROXY_MANAGED` 作为 Bearer 占位符，
    /// 而该常量随开源代码公开——任意本地进程都能凭它经回环向代理借用用户存储的
    /// 真实密钥配额（阶段 1 报告 §4.3 ①）。此处改为“每安装一次性随机派生 +
    /// 持久化”：真实写入 CLI 配置并由代理回环认证校验的是 `PROXY_MANAGED-<随机>`，
    /// 固定裸常量不再被接受为有效凭据。前缀保留是为了让既有的“是否为接管占位符”
    /// 判定继续以前缀匹配兼容历史裸值。
    pub fn get_or_create_proxy_takeover_token(&self) -> Result<String, AppError> {
        // 注意：键名**不能**落入 `proxy_takeover_%` 命名空间——该前缀被
        // `clear_all_proxy_takeover` 与三行 proxy_config 迁移用 LIKE 批量清理，
        // 会误删/改写本令牌。故使用独立的 `proxy_managed_auth_token`。
        const KEY: &str = "proxy_managed_auth_token";
        const PREFIX: &str = "PROXY_MANAGED-";

        if let Some(existing) = self.get_setting(KEY)? {
            let trimmed = existing.trim();
            if trimmed.starts_with(PREFIX) && trimmed.len() > PREFIX.len() {
                return Ok(trimmed.to_string());
            }
        }

        let token = format!("{PREFIX}{}", uuid::Uuid::new_v4());
        self.set_setting(KEY, &token)?;
        Ok(token)
    }

    // --- 用量脚本域名确认闸门（P0-2） ---

    /// custom 用量脚本已确认外发目标 host 的存储键。
    fn usage_script_confirmed_host_key(app_type: &str, provider_id: &str) -> String {
        format!("usage_script_confirmed_host_{app_type}_{provider_id}")
    }

    /// 读取某 provider 的 custom 用量脚本已确认的外发目标 host 标签。
    ///
    /// 返回 `None` 表示尚未确认；后端算出的目标 host 标签与此值不符（含 host 变更）时会
    /// 重新触发确认。custom 用量脚本首次外发到非回环主机前，`query_usage` /
    /// `test_usage_script` 会读取本值并传入执行函数做 fail-closed 闸门。
    pub fn get_usage_script_confirmed_host(
        &self,
        app_type: &str,
        provider_id: &str,
    ) -> Result<Option<String>, AppError> {
        self.get_setting(&Self::usage_script_confirmed_host_key(
            app_type,
            provider_id,
        ))
    }

    /// 持久化某 provider 的 custom 用量脚本已确认的外发目标 host 标签。
    pub fn set_usage_script_confirmed_host(
        &self,
        app_type: &str,
        provider_id: &str,
        host: &str,
    ) -> Result<(), AppError> {
        self.set_setting(
            &Self::usage_script_confirmed_host_key(app_type, provider_id),
            host,
        )
    }

    // --- 通用配置片段 (Common Config Snippet) ---

    /// 获取通用配置片段
    pub fn get_config_snippet(&self, app_type: &str) -> Result<Option<String>, AppError> {
        self.get_setting(&format!("common_config_{app_type}"))
    }

    /// 检查通用配置片段是否被用户显式清空
    pub fn is_config_snippet_cleared(&self, app_type: &str) -> Result<bool, AppError> {
        Ok(self
            .get_setting(&Self::config_snippet_cleared_key(app_type))?
            .as_deref()
            == Some("true"))
    }

    /// 设置通用配置片段是否被显式清空
    pub fn set_config_snippet_cleared(
        &self,
        app_type: &str,
        cleared: bool,
    ) -> Result<(), AppError> {
        let key = Self::config_snippet_cleared_key(app_type);
        if cleared {
            self.set_setting(&key, "true")
        } else {
            let conn = lock_conn!(self.conn);
            conn.execute("DELETE FROM settings WHERE key = ?1", params![key])
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        }
    }

    /// 当前是否允许从 live 配置自动抽取通用配置片段
    pub fn should_auto_extract_config_snippet(&self, app_type: &str) -> Result<bool, AppError> {
        Ok(self.get_config_snippet(app_type)?.is_none()
            && !self.is_config_snippet_cleared(app_type)?)
    }

    /// 检查历史通用配置迁移是否已经执行过
    pub fn is_legacy_common_config_migrated(&self) -> Result<bool, AppError> {
        Ok(self
            .get_setting(Self::LEGACY_COMMON_CONFIG_MIGRATED_KEY)?
            .as_deref()
            == Some("true"))
    }

    /// 标记历史通用配置迁移已经执行完成
    pub fn set_legacy_common_config_migrated(&self, migrated: bool) -> Result<(), AppError> {
        if migrated {
            self.set_setting(Self::LEGACY_COMMON_CONFIG_MIGRATED_KEY, "true")
        } else {
            let conn = lock_conn!(self.conn);
            conn.execute(
                "DELETE FROM settings WHERE key = ?1",
                params![Self::LEGACY_COMMON_CONFIG_MIGRATED_KEY],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        }
    }

    /// 设置通用配置片段
    pub fn set_config_snippet(
        &self,
        app_type: &str,
        snippet: Option<String>,
    ) -> Result<(), AppError> {
        let key = format!("common_config_{app_type}");
        if let Some(value) = snippet {
            self.set_setting(&key, &value)
        } else {
            // 如果为 None 则删除
            let conn = lock_conn!(self.conn);
            conn.execute("DELETE FROM settings WHERE key = ?1", params![key])
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        }
    }

    // --- 全局出站代理 ---

    /// 全局代理 URL 的存储键名
    const GLOBAL_PROXY_URL_KEY: &'static str = "global_proxy_url";

    /// 获取全局出站代理 URL
    ///
    /// 返回 None 表示未配置或已清除代理（直连）
    /// 返回 Some(url) 表示已配置代理
    pub fn get_global_proxy_url(&self) -> Result<Option<String>, AppError> {
        self.get_setting(Self::GLOBAL_PROXY_URL_KEY)
    }

    /// 设置全局出站代理 URL
    ///
    /// - 传入非空字符串：启用代理
    /// - 传入空字符串或 None：清除代理设置（直连）
    pub fn set_global_proxy_url(&self, url: Option<&str>) -> Result<(), AppError> {
        match url {
            Some(u) if !u.trim().is_empty() => {
                self.set_setting(Self::GLOBAL_PROXY_URL_KEY, u.trim())
            }
            _ => {
                // 清除代理设置
                let conn = lock_conn!(self.conn);
                conn.execute(
                    "DELETE FROM settings WHERE key = ?1",
                    params![Self::GLOBAL_PROXY_URL_KEY],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            }
        }
    }

    // --- 代理接管状态管理（已废弃，使用 proxy_config.enabled 替代）---

    /// 获取指定应用的代理接管状态
    ///
    /// **已废弃**: 请使用 `proxy_config.enabled` 字段替代
    /// 此方法仅用于数据库迁移时读取旧数据
    #[deprecated(since = "3.9.0", note = "使用 get_proxy_config_for_app().enabled 替代")]
    pub fn get_proxy_takeover_enabled(&self, app_type: &str) -> Result<bool, AppError> {
        let key = format!("proxy_takeover_{app_type}");
        match self.get_setting(&key)? {
            Some(value) => Ok(value == "true"),
            None => Ok(false),
        }
    }

    /// 设置指定应用的代理接管状态
    ///
    /// **已废弃**: 请使用 `proxy_config.enabled` 字段替代
    #[deprecated(
        since = "3.9.0",
        note = "使用 update_proxy_config_for_app() 修改 enabled 字段"
    )]
    pub fn set_proxy_takeover_enabled(
        &self,
        app_type: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        let key = format!("proxy_takeover_{app_type}");
        let value = if enabled { "true" } else { "false" };
        self.set_setting(&key, value)
    }

    /// 检查是否有任一应用开启了代理接管
    ///
    /// **已废弃**: 请使用 `is_live_takeover_active()` 替代
    #[deprecated(since = "3.9.0", note = "使用 is_live_takeover_active() 替代")]
    pub fn has_any_proxy_takeover(&self) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key LIKE 'proxy_takeover_%' AND value = 'true'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    /// 清除所有代理接管状态（将所有 proxy_takeover_* 设置为 false）
    ///
    /// **已废弃**: settings 表不再用于存储代理状态
    #[deprecated(
        since = "3.9.0",
        note = "使用 update_proxy_config_for_app() 清除各应用的 enabled 字段"
    )]
    pub fn clear_all_proxy_takeover(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE settings SET value = 'false' WHERE key LIKE 'proxy_takeover_%'",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        log::info!("已清除所有代理接管状态");
        Ok(())
    }

    // --- 整流器配置 ---

    /// 获取整流器配置
    ///
    /// 返回整流器配置，如果不存在则返回默认值（全部开启）
    pub fn get_rectifier_config(&self) -> Result<crate::proxy::types::RectifierConfig, AppError> {
        match self.get_setting("rectifier_config")? {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Database(format!("解析整流器配置失败: {e}"))),
            None => Ok(crate::proxy::types::RectifierConfig::default()),
        }
    }

    /// 更新整流器配置
    pub fn set_rectifier_config(
        &self,
        config: &crate::proxy::types::RectifierConfig,
    ) -> Result<(), AppError> {
        let json = serde_json::to_string(config)
            .map_err(|e| AppError::Database(format!("序列化整流器配置失败: {e}")))?;
        self.set_setting("rectifier_config", &json)
    }

    // --- 优化器配置 ---

    /// 获取优化器配置
    ///
    /// 返回优化器配置，如果不存在则返回默认值（默认关闭）
    pub fn get_optimizer_config(&self) -> Result<crate::proxy::types::OptimizerConfig, AppError> {
        match self.get_setting("optimizer_config")? {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Database(format!("解析优化器配置失败: {e}"))),
            None => Ok(crate::proxy::types::OptimizerConfig::default()),
        }
    }

    /// 更新优化器配置
    pub fn set_optimizer_config(
        &self,
        config: &crate::proxy::types::OptimizerConfig,
    ) -> Result<(), AppError> {
        let json = serde_json::to_string(config)
            .map_err(|e| AppError::Database(format!("序列化优化器配置失败: {e}")))?;
        self.set_setting("optimizer_config", &json)
    }

    // --- Copilot 优化器配置 ---

    /// 获取 Copilot 优化器配置
    ///
    /// 返回配置，如果不存在则返回默认值（默认开启）
    pub fn get_copilot_optimizer_config(
        &self,
    ) -> Result<crate::proxy::types::CopilotOptimizerConfig, AppError> {
        match self.get_setting("copilot_optimizer_config")? {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Database(format!("解析 Copilot 优化器配置失败: {e}"))),
            None => Ok(crate::proxy::types::CopilotOptimizerConfig::default()),
        }
    }

    /// 更新 Copilot 优化器配置
    pub fn set_copilot_optimizer_config(
        &self,
        config: &crate::proxy::types::CopilotOptimizerConfig,
    ) -> Result<(), AppError> {
        let json = serde_json::to_string(config)
            .map_err(|e| AppError::Database(format!("序列化 Copilot 优化器配置失败: {e}")))?;
        self.set_setting("copilot_optimizer_config", &json)
    }

    // --- 日志配置 ---

    /// 获取日志配置
    pub fn get_log_config(&self) -> Result<crate::proxy::types::LogConfig, AppError> {
        match self.get_setting("log_config")? {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Database(format!("解析日志配置失败: {e}"))),
            None => Ok(crate::proxy::types::LogConfig::default()),
        }
    }

    /// 更新日志配置
    pub fn set_log_config(&self, config: &crate::proxy::types::LogConfig) -> Result<(), AppError> {
        let json = serde_json::to_string(config)
            .map_err(|e| AppError::Database(format!("序列化日志配置失败: {e}")))?;
        self.set_setting("log_config", &json)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::Database;

    #[test]
    fn proxy_takeover_token_is_random_prefixed_and_stable() {
        let db = Database::memory().expect("memory db");

        let first = db
            .get_or_create_proxy_takeover_token()
            .expect("first token");
        assert!(
            first.starts_with("PROXY_MANAGED-"),
            "token must keep the PROXY_MANAGED- prefix for placeholder detection: {first}"
        );
        assert!(
            first.len() > "PROXY_MANAGED-".len(),
            "token must carry a random suffix, not the bare constant"
        );
        assert_ne!(
            first, "PROXY_MANAGED",
            "must never be the guessable bare constant"
        );

        // 稳定性：同一安装重复读取返回同一令牌（否则接管配置与代理认证会失配）。
        let second = db
            .get_or_create_proxy_takeover_token()
            .expect("second token");
        assert_eq!(first, second, "token must persist across reads");
    }

    #[test]
    fn proxy_takeover_token_heals_from_legacy_bare_constant() {
        let db = Database::memory().expect("memory db");

        // 模拟历史/被篡改的裸常量落库：必须被重新派生为带随机后缀的安全令牌。
        db.set_setting("proxy_managed_auth_token", "PROXY_MANAGED")
            .expect("seed legacy value");

        let token = db
            .get_or_create_proxy_takeover_token()
            .expect("token after heal");
        assert_ne!(token, "PROXY_MANAGED");
        assert!(token.starts_with("PROXY_MANAGED-") && token.len() > "PROXY_MANAGED-".len());
    }

    #[test]
    fn usage_script_confirmed_host_round_trip_and_isolation() {
        let db = Database::memory().expect("memory db");

        // 未确认时返回 None
        assert_eq!(
            db.get_usage_script_confirmed_host("claude", "p1")
                .expect("read"),
            None
        );

        db.set_usage_script_confirmed_host("claude", "p1", "quota.example.net")
            .expect("persist confirmed host");
        assert_eq!(
            db.get_usage_script_confirmed_host("claude", "p1")
                .expect("read"),
            Some("quota.example.net".to_string())
        );

        // 不同 provider / 不同 app 互不影响（key 按 app_type + provider_id 隔离）
        assert_eq!(
            db.get_usage_script_confirmed_host("claude", "p2")
                .expect("read"),
            None
        );
        assert_eq!(
            db.get_usage_script_confirmed_host("codex", "p1")
                .expect("read"),
            None
        );

        // host 变更（覆盖写入）后返回新值
        db.set_usage_script_confirmed_host("claude", "p1", "other.example.com")
            .expect("update confirmed host");
        assert_eq!(
            db.get_usage_script_confirmed_host("claude", "p1")
                .expect("read"),
            Some("other.example.com".to_string())
        );
    }

    #[test]
    fn proxy_takeover_token_key_is_outside_takeover_namespace() {
        // 回归：令牌键不能落入 `proxy_takeover_%`，否则会被 clear_all_proxy_takeover 与
        // 三行 proxy_config 迁移的 `LIKE 'proxy_takeover_%'` 批量清理误伤。
        assert!(
            !"proxy_managed_auth_token".starts_with("proxy_takeover_"),
            "key must stay outside the proxy_takeover_ LIKE namespace"
        );

        let db = Database::memory().expect("memory db");
        let token = db.get_or_create_proxy_takeover_token().expect("token");
        assert_eq!(
            db.get_setting("proxy_managed_auth_token").expect("read"),
            Some(token),
            "token must be stored under the dedicated non-takeover key"
        );
    }
}
