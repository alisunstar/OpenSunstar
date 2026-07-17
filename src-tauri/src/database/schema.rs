//! Schema 定义和迁移
//!
//! 负责数据库表结构的创建和版本迁移。

use super::{lock_conn, Database, SCHEMA_VERSION};
use crate::error::AppError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct LegacySkillMigrationRow {
    directory: String,
    app_type: String,
}

impl Database {
    /// 创建所有数据库表
    pub(crate) fn create_tables(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::create_tables_on_conn(&conn)
    }

    /// 在指定连接上创建表（供迁移和测试使用）
    pub(crate) fn create_tables_on_conn(conn: &Connection) -> Result<(), AppError> {
        // 1. Providers 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS providers (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                website_url TEXT,
                category TEXT,
                created_at INTEGER,
                sort_index INTEGER,
                notes TEXT,
                icon TEXT,
                icon_color TEXT,
                meta TEXT NOT NULL DEFAULT '{}',
                is_current BOOLEAN NOT NULL DEFAULT 0,
                in_failover_queue BOOLEAN NOT NULL DEFAULT 0,
                PRIMARY KEY (id, app_type)
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 2. Provider Endpoints 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS provider_endpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                url TEXT NOT NULL,
                added_at INTEGER,
                FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 3. MCP Servers 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_servers (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, server_config TEXT NOT NULL,
            description TEXT, homepage TEXT, docs TEXT, tags TEXT NOT NULL DEFAULT '[]',
            enabled_claude BOOLEAN NOT NULL DEFAULT 0, enabled_codex BOOLEAN NOT NULL DEFAULT 0,
            enabled_gemini BOOLEAN NOT NULL DEFAULT 0, enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
            enabled_hermes BOOLEAN NOT NULL DEFAULT 0
        )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 4. Prompts 表
        conn.execute("CREATE TABLE IF NOT EXISTS prompts (
            id TEXT NOT NULL, app_type TEXT NOT NULL, name TEXT NOT NULL, content TEXT NOT NULL,
            description TEXT, enabled BOOLEAN NOT NULL DEFAULT 1, created_at INTEGER, updated_at INTEGER,
            targets TEXT NOT NULL DEFAULT '[\"*\"]',
            globs TEXT NOT NULL DEFAULT '[]',
            priority INTEGER NOT NULL DEFAULT 0,
            is_fragment BOOLEAN NOT NULL DEFAULT 0,
            parent_prompt_id TEXT,
            PRIMARY KEY (id, app_type)
        )", []).map_err(|e| AppError::Database(e.to_string()))?;

        // 5. Skills 表（v3.10.0+ 统一结构）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            directory TEXT NOT NULL,
            repo_owner TEXT,
            repo_name TEXT,
            repo_branch TEXT DEFAULT 'main',
            readme_url TEXT,
            enabled_claude BOOLEAN NOT NULL DEFAULT 0,
            enabled_codex BOOLEAN NOT NULL DEFAULT 0,
            enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
            enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
            enabled_hermes BOOLEAN NOT NULL DEFAULT 0,
            installed_at INTEGER NOT NULL DEFAULT 0,
            content_hash TEXT,
            updated_at INTEGER NOT NULL DEFAULT 0
        )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 6. Skill Repos 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS skill_repos (
            owner TEXT NOT NULL, name TEXT NOT NULL, branch TEXT NOT NULL DEFAULT 'main',
            enabled BOOLEAN NOT NULL DEFAULT 1, PRIMARY KEY (owner, name)
        )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 7. Settings 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 8. Proxy Config 表（三行结构，app_type 主键）
        conn.execute("CREATE TABLE IF NOT EXISTS proxy_config (
            app_type TEXT PRIMARY KEY CHECK (app_type IN ('claude','codex','gemini')),
            proxy_enabled INTEGER NOT NULL DEFAULT 0, listen_address TEXT NOT NULL DEFAULT '127.0.0.1',
            listen_port INTEGER NOT NULL DEFAULT 15721, enable_logging INTEGER NOT NULL DEFAULT 1,
            enabled INTEGER NOT NULL DEFAULT 0, auto_failover_enabled INTEGER NOT NULL DEFAULT 0,
            max_retries INTEGER NOT NULL DEFAULT 3, streaming_first_byte_timeout INTEGER NOT NULL DEFAULT 60,
            streaming_idle_timeout INTEGER NOT NULL DEFAULT 120, non_streaming_timeout INTEGER NOT NULL DEFAULT 600,
            circuit_failure_threshold INTEGER NOT NULL DEFAULT 4, circuit_success_threshold INTEGER NOT NULL DEFAULT 2,
            circuit_timeout_seconds INTEGER NOT NULL DEFAULT 60, circuit_error_rate_threshold REAL NOT NULL DEFAULT 0.6,
            circuit_min_requests INTEGER NOT NULL DEFAULT 10,
            default_cost_multiplier TEXT NOT NULL DEFAULT '1',
            pricing_model_source TEXT NOT NULL DEFAULT 'response',
            created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )", []).map_err(|e| AppError::Database(e.to_string()))?;

        // 初始化三行数据（每应用不同默认值）
        //
        // 兼容旧数据库：
        // - 老版本 proxy_config 是单例表（没有 app_type 列），此时不能执行三行 seed insert；
        // - 旧表会在 apply_schema_migrations() 中迁移为三行结构后再插入。
        if Self::has_column(conn, "proxy_config", "app_type")? {
            conn.execute(
                "INSERT OR IGNORE INTO proxy_config (app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests)
                VALUES ('claude', 6, 90, 180, 600, 8, 3, 90, 0.7, 15)",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT OR IGNORE INTO proxy_config (app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests)
                VALUES ('codex', 3, 60, 120, 600, 4, 2, 60, 0.6, 10)",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT OR IGNORE INTO proxy_config (app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests)
                VALUES ('gemini', 5, 60, 120, 600, 4, 2, 60, 0.6, 10)",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        // 9. Provider Health 表
        conn.execute("CREATE TABLE IF NOT EXISTS provider_health (
            provider_id TEXT NOT NULL, app_type TEXT NOT NULL, is_healthy INTEGER NOT NULL DEFAULT 1,
            consecutive_failures INTEGER NOT NULL DEFAULT 0, last_success_at TEXT, last_failure_at TEXT,
            last_error TEXT, updated_at TEXT NOT NULL,
            PRIMARY KEY (provider_id, app_type),
            FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
        )", []).map_err(|e| AppError::Database(e.to_string()))?;

        // 10. Proxy Request Logs 表
        // pricing_model = 写入时实际用于计价的模型名（pricing_model_source 解析结果），
        // 回填按它重算；NULL 表示 v11 之前的历史行，'' 表示未计价的错误行。
        conn.execute("CREATE TABLE IF NOT EXISTS proxy_request_logs (
            request_id TEXT PRIMARY KEY, provider_id TEXT NOT NULL, app_type TEXT NOT NULL, model TEXT NOT NULL,
            request_model TEXT,
            pricing_model TEXT,
            input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
            cache_read_tokens INTEGER NOT NULL DEFAULT 0, cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
            input_cost_usd TEXT NOT NULL DEFAULT '0', output_cost_usd TEXT NOT NULL DEFAULT '0',
            cache_read_cost_usd TEXT NOT NULL DEFAULT '0', cache_creation_cost_usd TEXT NOT NULL DEFAULT '0',
            total_cost_usd TEXT NOT NULL DEFAULT '0', latency_ms INTEGER NOT NULL, first_token_ms INTEGER,
            duration_ms INTEGER, status_code INTEGER NOT NULL, error_message TEXT, session_id TEXT,
            provider_type TEXT, is_streaming INTEGER NOT NULL DEFAULT 0,
            cost_multiplier TEXT NOT NULL DEFAULT '1.0', created_at INTEGER NOT NULL,
            data_source TEXT NOT NULL DEFAULT 'proxy'
        )", []).map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_request_logs_provider ON proxy_request_logs(provider_id, app_type)", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON proxy_request_logs(created_at)", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_model ON proxy_request_logs(model)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_session ON proxy_request_logs(session_id)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_status ON proxy_request_logs(status_code)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Self::create_request_logs_usage_indexes_if_supported(conn)?;

        // 11. Model Pricing 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS model_pricing (
            model_id TEXT PRIMARY KEY, display_name TEXT NOT NULL,
            input_cost_per_million TEXT NOT NULL, output_cost_per_million TEXT NOT NULL,
            cache_read_cost_per_million TEXT NOT NULL DEFAULT '0',
            cache_creation_cost_per_million TEXT NOT NULL DEFAULT '0'
        )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Self::create_model_pricing_provenance_table(conn)?;

        // 12. Stream Check Logs 表
        conn.execute("CREATE TABLE IF NOT EXISTS stream_check_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT, provider_id TEXT NOT NULL, provider_name TEXT NOT NULL,
            app_type TEXT NOT NULL, status TEXT NOT NULL, success INTEGER NOT NULL, message TEXT NOT NULL,
            response_time_ms INTEGER, http_status INTEGER, model_used TEXT,
            retry_count INTEGER DEFAULT 0, tested_at INTEGER NOT NULL
        )", []).map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_stream_check_logs_provider
             ON stream_check_logs(app_type, provider_id, tested_at DESC)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 注意：circuit_breaker_config 已合并到 proxy_config 表中

        // 16. Proxy Live Backup 表 (Live 配置备份)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS proxy_live_backup (
            app_type TEXT PRIMARY KEY, original_config TEXT NOT NULL, backed_up_at TEXT NOT NULL
        )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 17. Usage Daily Rollups 表 (日聚合统计)
        // request_model 保留路由接管的「客户端别名 → 真实模型」映射维度，
        // pricing_model 保留写入时的计价基准（request 计价模式下与 model 分叉），
        // 否则明细被 prune 后接管计费不可审计；历史行迁移时填 ''（未知）。
        conn.execute(
            "CREATE TABLE IF NOT EXISTS usage_daily_rollups (
                date TEXT NOT NULL,
                app_type TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                model TEXT NOT NULL,
                request_model TEXT NOT NULL DEFAULT '',
                pricing_model TEXT NOT NULL DEFAULT '',
                request_count INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost_usd TEXT NOT NULL DEFAULT '0',
                avg_latency_ms INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (date, app_type, provider_id, model, request_model, pricing_model)
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 18. Session Log Sync 表 (会话日志同步状态)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS session_log_sync (
                file_path TEXT PRIMARY KEY,
                last_modified INTEGER NOT NULL,
                last_line_offset INTEGER NOT NULL DEFAULT 0,
                last_synced_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 19. Projects 表（项目级配置隔离）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                git_remote_url TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 20. 项目 × 资产统一关联表（8 类 SSOT）
        Self::create_project_asset_links_table(conn)?;

        // 23. Commands 表（slash 命令管理）
        Self::create_commands_table(conn)?;

        // 24. Hooks 表（Claude Code 生命周期钩子）
        Self::create_hooks_table(conn)?;

        // 25. Ignore 规则表
        Self::create_ignore_rules_table(conn)?;

        // 26. Tool permissions 表
        Self::create_tool_permissions_table(conn)?;

        // 27. Agents 表（Subagent 管理）
        Self::create_agents_table(conn)?;

        // 28. AI Insights 缓存表（项目看板 AI 能力）
        Self::create_ai_insights_table(conn)?;

        // 29. AI 成本日志表（项目看板 AI 调用追踪）
        Self::create_ai_cost_log_table(conn)?;

        // 30. SDD 框架描述符目录 + 项目探测结果
        Self::create_sdd_descriptors_table(conn)?;
        Self::create_project_sdd_detections_table(conn)?;

        // 31. 项目环境快照（项目绑定的运行态配置快照）
        Self::create_project_environment_snapshots_table(conn)?;

        // 尝试添加 live_takeover_active 列到 proxy_config 表
        let _ = conn.execute(
            "ALTER TABLE proxy_config ADD COLUMN live_takeover_active INTEGER NOT NULL DEFAULT 0",
            [],
        );

        // 尝试添加基础配置列到 proxy_config 表（兼容 v3.9.0-2 升级）
        let _ = conn.execute(
            "ALTER TABLE proxy_config ADD COLUMN proxy_enabled INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE proxy_config ADD COLUMN listen_address TEXT NOT NULL DEFAULT '127.0.0.1'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE proxy_config ADD COLUMN listen_port INTEGER NOT NULL DEFAULT 15721",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE proxy_config ADD COLUMN enable_logging INTEGER NOT NULL DEFAULT 1",
            [],
        );

        // 尝试添加超时配置列到 proxy_config 表
        let _ = conn.execute(
            "ALTER TABLE proxy_config ADD COLUMN streaming_first_byte_timeout INTEGER NOT NULL DEFAULT 60",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE proxy_config ADD COLUMN streaming_idle_timeout INTEGER NOT NULL DEFAULT 120",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE proxy_config ADD COLUMN non_streaming_timeout INTEGER NOT NULL DEFAULT 600",
            [],
        );

        // 兼容：若旧版 proxy_config 仍为单例结构（无 app_type），则在启动时直接转换为三行结构
        // 说明：user_version=2 时不会再触发 v1->v2 迁移，但新代码查询依赖 app_type 列。
        if Self::table_exists(conn, "proxy_config")?
            && !Self::has_column(conn, "proxy_config", "app_type")?
        {
            Self::migrate_proxy_config_to_per_app(conn)?;
        }

        // 确保 in_failover_queue 列存在（对于已存在的 v2 数据库）
        Self::add_column_if_missing(
            conn,
            "providers",
            "in_failover_queue",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;

        // 删除旧的 failover_queue 表（如果存在）
        let _ = conn.execute("DROP INDEX IF EXISTS idx_failover_queue_order", []);
        let _ = conn.execute("DROP TABLE IF EXISTS failover_queue", []);

        // 为故障转移队列创建索引（基于 providers 表）
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_providers_failover
             ON providers(app_type, in_failover_queue, sort_index)",
            [],
        );

        Ok(())
    }

    /// 应用 Schema 迁移
    pub(crate) fn apply_schema_migrations(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::apply_schema_migrations_on_conn(&conn)
    }

    /// 在指定连接上应用 Schema 迁移
    pub(crate) fn apply_schema_migrations_on_conn(conn: &Connection) -> Result<(), AppError> {
        conn.execute("SAVEPOINT schema_migration;", [])
            .map_err(|e| AppError::Database(format!("开启迁移 savepoint 失败: {e}")))?;

        let mut version = Self::get_user_version(conn)?;

        if version > SCHEMA_VERSION {
            conn.execute("ROLLBACK TO schema_migration;", []).ok();
            conn.execute("RELEASE schema_migration;", []).ok();
            return Err(AppError::Database(format!(
                "数据库版本过新（{version}），当前应用仅支持 {SCHEMA_VERSION}，请升级应用后再尝试。"
            )));
        }

        let result = (|| {
            while version < SCHEMA_VERSION {
                match version {
                    0 => {
                        log::info!("检测到 user_version=0，迁移到 1（补齐缺失列并设置版本）");
                        Self::migrate_v0_to_v1(conn)?;
                        Self::set_user_version(conn, 1)?;
                    }
                    1 => {
                        log::info!(
                            "迁移数据库从 v1 到 v2（添加使用统计表和完整字段，重构 skills 表）"
                        );
                        Self::migrate_v1_to_v2(conn)?;
                        Self::set_user_version(conn, 2)?;
                    }
                    2 => {
                        log::info!("迁移数据库从 v2 到 v3（Skills 统一管理架构）");
                        Self::migrate_v2_to_v3(conn)?;
                        Self::set_user_version(conn, 3)?;
                    }
                    3 => {
                        log::info!("迁移数据库从 v3 到 v4（OpenCode 支持）");
                        Self::migrate_v3_to_v4(conn)?;
                        Self::set_user_version(conn, 4)?;
                    }
                    4 => {
                        log::info!("迁移数据库从 v4 到 v5（计费模式支持）");
                        Self::migrate_v4_to_v5(conn)?;
                        Self::set_user_version(conn, 5)?;
                    }
                    5 => {
                        log::info!("迁移数据库从 v5 到 v6（使用量聚合表 + Copilot 模板类型统一）");
                        Self::migrate_v5_to_v6(conn)?;
                        Self::set_user_version(conn, 6)?;
                    }
                    6 => {
                        log::info!("迁移数据库从 v6 到 v7（Skills 更新检测支持）");
                        Self::migrate_v6_to_v7(conn)?;
                        Self::set_user_version(conn, 7)?;
                    }
                    7 => {
                        log::info!("迁移数据库从 v7 到 v8（会话日志使用追踪 + 修正模型定价）");
                        Self::migrate_v7_to_v8(conn)?;
                        Self::set_user_version(conn, 8)?;
                    }
                    8 => {
                        log::info!("迁移数据库从 v8 到 v9（全面补充模型定价）");
                        Self::migrate_v8_to_v9(conn)?;
                        Self::set_user_version(conn, 9)?;
                    }
                    9 => {
                        log::info!("迁移数据库从 v9 到 v10（添加 Hermes Agent 支持）");
                        Self::migrate_v9_to_v10(conn)?;
                        Self::set_user_version(conn, 10)?;
                    }
                    10 => {
                        log::info!("迁移数据库从 v10 到 v11（usage_daily_rollups 保留 request_model 维度）");
                        Self::migrate_v10_to_v11(conn)?;
                        Self::set_user_version(conn, 11)?;
                    }
                    11 => {
                        log::info!("迁移数据库从 v11 到 v12（API Key 迁移至 OS Keychain）");
                        Self::migrate_v11_to_v12(conn)?;
                        Self::set_user_version(conn, 12)?;
                    }
                    12 => {
                        log::info!("迁移数据库从 v12 到 v13（Prompt 桥接支持）");
                        Self::migrate_v12_to_v13(conn)?;
                        Self::set_user_version(conn, 13)?;
                    }
                    13 => {
                        log::info!("迁移数据库从 v13 到 v14（项目级配置隔离 - 方案 E）");
                        Self::migrate_v13_to_v14(conn)?;
                        Self::set_user_version(conn, 14)?;
                    }
                    14 => {
                        log::info!("迁移数据库从 v14 到 v15（Commands + Hooks 管理）");
                        Self::migrate_v14_to_v15(conn)?;
                        Self::set_user_version(conn, 15)?;
                    }
                    15 => {
                        log::info!("迁移数据库从 v15 到 v16（Ignore + Permissions 管理）");
                        Self::migrate_v15_to_v16(conn)?;
                        Self::set_user_version(conn, 16)?;
                    }
                    16 => {
                        log::info!("迁移数据库从 v16 到 v17（Prompts fragments + dry run）");
                        Self::migrate_v16_to_v17(conn)?;
                        Self::set_user_version(conn, 17)?;
                    }
                    17 => {
                        log::info!("迁移数据库从 v17 到 v18（Agents / Subagents 管理）");
                        Self::migrate_v17_to_v18(conn)?;
                        Self::set_user_version(conn, 18)?;
                    }
                    18 => {
                        log::info!("迁移数据库从 v18 到 v19（AI Insights 缓存 + 成本日志）");
                        Self::migrate_v18_to_v19(conn)?;
                        Self::set_user_version(conn, 19)?;
                    }
                    19 => {
                        log::info!("迁移数据库从 v19 到 v20（AI Insights 用户反馈列）");
                        Self::migrate_v19_to_v20(conn)?;
                        Self::set_user_version(conn, 20)?;
                    }
                    20 => {
                        log::info!("迁移数据库从 v20 到 v21（NL 问答日志 + project_id 统一）");
                        Self::migrate_v20_to_v21(conn)?;
                        Self::set_user_version(conn, 21)?;
                    }
                    21 => {
                        log::info!(
                            "迁移数据库从 v21 到 v22（项目扩展资产关联表 project_asset_links）"
                        );
                        Self::migrate_v21_to_v22(conn)?;
                        Self::set_user_version(conn, 22)?;
                    }
                    22 => {
                        log::info!("迁移数据库从 v22 到 v23（项目 target_app / blueprint_id）");
                        Self::migrate_v22_to_v23(conn)?;
                        Self::set_user_version(conn, 23)?;
                    }
                    23 => {
                        log::info!("迁移数据库从 v23 到 v24（Hooks/Permissions 多 CLI enabled_*）");
                        Self::migrate_v23_to_v24(conn)?;
                        Self::set_user_version(conn, 24)?;
                    }
                    24 => {
                        log::info!(
                            "迁移数据库从 v24 到 v25（MCP/Skills/Prompts 并入 project_asset_links，废弃旧三表）"
                        );
                        Self::migrate_v24_to_v25(conn)?;
                        Self::set_user_version(conn, 25)?;
                    }
                    25 => {
                        log::info!(
                            "迁移数据库从 v25 到 v26（SDD 框架探测：sdd_descriptors + project_sdd_detections）"
                        );
                        Self::migrate_v25_to_v26(conn)?;
                        Self::set_user_version(conn, 26)?;
                    }
                    26 => {
                        log::info!(
                            "迁移数据库从 v26 到 v27（修正 install_type：BMAD→npm, Superpowers→plugin, Spec Kit→uvx）"
                        );
                        Self::migrate_v26_to_v27(conn)?;
                        Self::set_user_version(conn, 27)?;
                    }
                    27 => {
                        log::info!("迁移数据库从 v27 到 v28（projects 增加 stage / mvp_progress）");
                        Self::migrate_v27_to_v28(conn)?;
                        Self::set_user_version(conn, 28)?;
                    }
                    28 => {
                        log::info!("迁移数据库从 v28 到 v29（项目 AI 资产健康事实表）");
                        Self::migrate_v28_to_v29(conn)?;
                        Self::set_user_version(conn, 29)?;
                    }
                    29 => {
                        log::info!("迁移数据库从 v29 到 v30（资产部署逐文件回执）");
                        Self::migrate_v29_to_v30(conn)?;
                        Self::set_user_version(conn, 30)?;
                    }
                    30 => {
                        log::info!("迁移数据库从 v30 到 v31（部署回执绑定资产修订）");
                        Self::migrate_v30_to_v31(conn)?;
                        Self::set_user_version(conn, 31)?;
                    }
                    31 => {
                        log::info!("迁移数据库从 v31 到 v32（项目环境快照）");
                        Self::migrate_v31_to_v32(conn)?;
                        Self::set_user_version(conn, 32)?;
                    }
                    32 => {
                        log::info!("Migrating database from v32 to v33 (QuickStart operation state and protected backups)");
                        Self::migrate_v32_to_v33(conn)?;
                        Self::set_user_version(conn, 33)?;
                    }
                    33 => {
                        log::info!("Migrating database from v33 to v34 (encrypted QuickStart live snapshots)");
                        Self::migrate_v33_to_v34(conn)?;
                        Self::set_user_version(conn, 34)?;
                    }
                    34 => {
                        log::info!("Migrating database from v34 to v35 (QuickStart rollback ownership guard)");
                        Self::migrate_v34_to_v35(conn)?;
                        Self::set_user_version(conn, 35)?;
                    }
                    35 => {
                        log::info!("Migrating database from v35 to v36 (auditable model pricing provenance)");
                        Self::migrate_v35_to_v36(conn)?;
                        Self::set_user_version(conn, 36)?;
                    }
                    36 => {
                        log::info!(
                            "Migrating database from v36 to v37 (Codex credential isolation)"
                        );
                        Self::migrate_v36_to_v37(conn)?;
                        Self::set_user_version(conn, 37)?;
                    }
                    _ => {
                        return Err(AppError::Database(format!(
                            "未知的数据库版本 {version}，无法迁移到 {SCHEMA_VERSION}"
                        )));
                    }
                }
                version = Self::get_user_version(conn)?;
            }
            Ok(())
        })();

        match result {
            Ok(_) => {
                conn.execute("RELEASE schema_migration;", [])
                    .map_err(|e| AppError::Database(format!("提交迁移 savepoint 失败: {e}")))?;
                Ok(())
            }
            Err(e) => {
                conn.execute("ROLLBACK TO schema_migration;", []).ok();
                conn.execute("RELEASE schema_migration;", []).ok();
                Err(e)
            }
        }
    }

    /// v0 -> v1 迁移：补齐所有缺失列
    fn migrate_v0_to_v1(conn: &Connection) -> Result<(), AppError> {
        // providers 表
        Self::add_column_if_missing(conn, "providers", "category", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "created_at", "INTEGER")?;
        Self::add_column_if_missing(conn, "providers", "sort_index", "INTEGER")?;
        Self::add_column_if_missing(conn, "providers", "notes", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "icon", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "icon_color", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "meta", "TEXT NOT NULL DEFAULT '{}'")?;
        Self::add_column_if_missing(
            conn,
            "providers",
            "is_current",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;

        // provider_endpoints 表
        Self::add_column_if_missing(conn, "provider_endpoints", "added_at", "INTEGER")?;

        // mcp_servers 表
        Self::add_column_if_missing(conn, "mcp_servers", "description", "TEXT")?;
        Self::add_column_if_missing(conn, "mcp_servers", "homepage", "TEXT")?;
        Self::add_column_if_missing(conn, "mcp_servers", "docs", "TEXT")?;
        Self::add_column_if_missing(conn, "mcp_servers", "tags", "TEXT NOT NULL DEFAULT '[]'")?;
        Self::add_column_if_missing(
            conn,
            "mcp_servers",
            "enabled_codex",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;
        Self::add_column_if_missing(
            conn,
            "mcp_servers",
            "enabled_gemini",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;

        // prompts 表
        Self::add_column_if_missing(conn, "prompts", "description", "TEXT")?;
        Self::add_column_if_missing(conn, "prompts", "enabled", "BOOLEAN NOT NULL DEFAULT 1")?;
        Self::add_column_if_missing(conn, "prompts", "created_at", "INTEGER")?;
        Self::add_column_if_missing(conn, "prompts", "updated_at", "INTEGER")?;

        // skills 表
        Self::add_column_if_missing(conn, "skills", "installed_at", "INTEGER NOT NULL DEFAULT 0")?;

        // skill_repos 表
        Self::add_column_if_missing(
            conn,
            "skill_repos",
            "branch",
            "TEXT NOT NULL DEFAULT 'main'",
        )?;
        Self::add_column_if_missing(conn, "skill_repos", "enabled", "BOOLEAN NOT NULL DEFAULT 1")?;
        // 注意: skills_path 字段已被移除，因为现在支持全仓库递归扫描

        Ok(())
    }

    /// v1 -> v2 迁移：添加使用统计表和完整字段，重构 skills 表
    fn migrate_v1_to_v2(conn: &Connection) -> Result<(), AppError> {
        // providers 表字段
        Self::add_column_if_missing(
            conn,
            "providers",
            "cost_multiplier",
            "TEXT NOT NULL DEFAULT '1.0'",
        )?;
        Self::add_column_if_missing(conn, "providers", "limit_daily_usd", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "limit_monthly_usd", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "provider_type", "TEXT")?;
        Self::add_column_if_missing(
            conn,
            "providers",
            "in_failover_queue",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;

        // 添加代理超时配置字段
        if Self::table_exists(conn, "proxy_config")? {
            // 兼容旧版本缺失的基础字段
            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "proxy_enabled",
                "INTEGER NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "listen_address",
                "TEXT NOT NULL DEFAULT '127.0.0.1'",
            )?;
            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "listen_port",
                "INTEGER NOT NULL DEFAULT 15721",
            )?;
            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "enable_logging",
                "INTEGER NOT NULL DEFAULT 1",
            )?;

            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "streaming_first_byte_timeout",
                "INTEGER NOT NULL DEFAULT 60",
            )?;
            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "streaming_idle_timeout",
                "INTEGER NOT NULL DEFAULT 120",
            )?;
            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "non_streaming_timeout",
                "INTEGER NOT NULL DEFAULT 600",
            )?;
        }

        // 删除旧的 failover_queue 表（如果存在）
        conn.execute("DROP INDEX IF EXISTS idx_failover_queue_order", [])
            .map_err(|e| AppError::Database(format!("删除 failover_queue 索引失败: {e}")))?;
        conn.execute("DROP TABLE IF EXISTS failover_queue", [])
            .map_err(|e| AppError::Database(format!("删除 failover_queue 表失败: {e}")))?;

        // 创建 failover 索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_providers_failover
             ON providers(app_type, in_failover_queue, sort_index)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 failover 索引失败: {e}")))?;

        // proxy_request_logs 表
        conn.execute("CREATE TABLE IF NOT EXISTS proxy_request_logs (
            request_id TEXT PRIMARY KEY, provider_id TEXT NOT NULL, app_type TEXT NOT NULL, model TEXT NOT NULL,
            request_model TEXT,
            input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
            cache_read_tokens INTEGER NOT NULL DEFAULT 0, cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
            input_cost_usd TEXT NOT NULL DEFAULT '0', output_cost_usd TEXT NOT NULL DEFAULT '0',
            cache_read_cost_usd TEXT NOT NULL DEFAULT '0', cache_creation_cost_usd TEXT NOT NULL DEFAULT '0',
            total_cost_usd TEXT NOT NULL DEFAULT '0', latency_ms INTEGER NOT NULL, first_token_ms INTEGER,
            duration_ms INTEGER, status_code INTEGER NOT NULL, error_message TEXT, session_id TEXT,
            provider_type TEXT, is_streaming INTEGER NOT NULL DEFAULT 0,
            cost_multiplier TEXT NOT NULL DEFAULT '1.0', created_at INTEGER NOT NULL
        )", [])?;

        // 为已存在的表添加新字段
        Self::add_column_if_missing(conn, "proxy_request_logs", "provider_type", "TEXT")?;
        Self::add_column_if_missing(
            conn,
            "proxy_request_logs",
            "is_streaming",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        Self::add_column_if_missing(
            conn,
            "proxy_request_logs",
            "cost_multiplier",
            "TEXT NOT NULL DEFAULT '1.0'",
        )?;
        Self::add_column_if_missing(conn, "proxy_request_logs", "first_token_ms", "INTEGER")?;
        Self::add_column_if_missing(conn, "proxy_request_logs", "duration_ms", "INTEGER")?;

        // model_pricing 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS model_pricing (
            model_id TEXT PRIMARY KEY, display_name TEXT NOT NULL,
            input_cost_per_million TEXT NOT NULL, output_cost_per_million TEXT NOT NULL,
            cache_read_cost_per_million TEXT NOT NULL DEFAULT '0',
            cache_creation_cost_per_million TEXT NOT NULL DEFAULT '0'
        )",
            [],
        )?;

        // 清空并重新插入模型定价
        conn.execute("DELETE FROM model_pricing", [])
            .map_err(|e| AppError::Database(format!("清空模型定价失败: {e}")))?;
        Self::seed_model_pricing(conn)?;

        // 重构 skills 表（添加 app_type 字段）
        Self::migrate_skills_table(conn)?;

        // 重构 proxy_config 为三行结构（每应用独立配置）
        Self::migrate_proxy_config_to_per_app(conn)?;

        Ok(())
    }

    /// 将 proxy_config 迁移为三行结构（每应用独立配置）
    fn migrate_proxy_config_to_per_app(conn: &Connection) -> Result<(), AppError> {
        // 检查是否已经是新表结构（幂等性）
        if !Self::table_exists(conn, "proxy_config")? {
            // 表不存在，跳过迁移（新安装）
            return Ok(());
        }

        if Self::has_column(conn, "proxy_config", "app_type")? {
            // 已经是三行结构，跳过迁移
            log::info!("proxy_config 已经是三行结构，跳过迁移");
            return Ok(());
        }

        // 读取旧配置
        let old_config = conn
            .query_row(
                "SELECT listen_address, listen_port, max_retries, enable_logging,
                    streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout
             FROM proxy_config WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i32>(1)?,
                        row.get::<_, i32>(2)?,
                        row.get::<_, i32>(3)?,
                        row.get::<_, i32>(4).unwrap_or(30),
                        row.get::<_, i32>(5).unwrap_or(60),
                        row.get::<_, i32>(6).unwrap_or(300),
                    ))
                },
            )
            .unwrap_or_else(|_| ("127.0.0.1".to_string(), 5000, 3, 1, 30, 60, 300));

        let old_cb = conn.query_row(
            "SELECT failure_threshold, success_threshold, timeout_seconds, error_rate_threshold, min_requests
             FROM circuit_breaker_config WHERE id = 1", [],
            |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?, row.get::<_, i64>(2)?,
                      row.get::<_, f64>(3)?, row.get::<_, i32>(4)?))
        ).unwrap_or((5, 2, 60, 0.5, 10));

        let get_bool = |key: &str| -> bool {
            conn.query_row("SELECT value FROM settings WHERE key = ?", [key], |r| {
                r.get::<_, String>(0)
            })
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
        };

        let apps = [
            (
                "claude",
                get_bool("proxy_takeover_claude"),
                get_bool("auto_failover_enabled_claude"),
                6,
                45,
                90,
                8,
                3,
                90,
                0.6,
                15,
            ),
            (
                "codex",
                get_bool("proxy_takeover_codex"),
                get_bool("auto_failover_enabled_codex"),
                3,
                old_config.4,
                old_config.5,
                old_cb.0,
                old_cb.1,
                old_cb.2,
                old_cb.3,
                old_cb.4,
            ),
            (
                "gemini",
                get_bool("proxy_takeover_gemini"),
                get_bool("auto_failover_enabled_gemini"),
                5,
                old_config.4,
                old_config.5,
                old_cb.0,
                old_cb.1,
                old_cb.2,
                old_cb.3,
                old_cb.4,
            ),
        ];

        // 创建新表
        conn.execute("DROP TABLE IF EXISTS proxy_config_new", [])?;
        conn.execute("CREATE TABLE proxy_config_new (
            app_type TEXT PRIMARY KEY CHECK (app_type IN ('claude','codex','gemini')),
            proxy_enabled INTEGER NOT NULL DEFAULT 0, listen_address TEXT NOT NULL DEFAULT '127.0.0.1',
            listen_port INTEGER NOT NULL DEFAULT 15721, enable_logging INTEGER NOT NULL DEFAULT 1,
            enabled INTEGER NOT NULL DEFAULT 0, auto_failover_enabled INTEGER NOT NULL DEFAULT 0,
            max_retries INTEGER NOT NULL DEFAULT 3, streaming_first_byte_timeout INTEGER NOT NULL DEFAULT 60,
            streaming_idle_timeout INTEGER NOT NULL DEFAULT 120, non_streaming_timeout INTEGER NOT NULL DEFAULT 600,
            circuit_failure_threshold INTEGER NOT NULL DEFAULT 4, circuit_success_threshold INTEGER NOT NULL DEFAULT 2,
            circuit_timeout_seconds INTEGER NOT NULL DEFAULT 60, circuit_error_rate_threshold REAL NOT NULL DEFAULT 0.6,
            circuit_min_requests INTEGER NOT NULL DEFAULT 10,
            default_cost_multiplier TEXT NOT NULL DEFAULT '1',
            pricing_model_source TEXT NOT NULL DEFAULT 'response',
            created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )", [])?;

        // 插入三行配置
        for (app, takeover, failover, retries, fb, idle, cb_f, cb_s, cb_t, cb_r, cb_m) in apps {
            conn.execute(
                "INSERT INTO proxy_config_new (app_type, proxy_enabled, listen_address, listen_port, enable_logging,
                 enabled, auto_failover_enabled, max_retries, streaming_first_byte_timeout, streaming_idle_timeout,
                 non_streaming_timeout, circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                 circuit_error_rate_threshold, circuit_min_requests)
                 VALUES (?1, 0, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                rusqlite::params![app, old_config.0, old_config.1, old_config.3,
                    if takeover { 1 } else { 0 }, if failover { 1 } else { 0 },
                    retries, fb, idle, old_config.6, cb_f, cb_s, cb_t, cb_r, cb_m]
            ).map_err(|e| AppError::Database(format!("插入 {app} 配置失败: {e}")))?;
        }

        // 替换表并清理
        conn.execute("DROP TABLE IF EXISTS proxy_config", [])?;
        conn.execute("ALTER TABLE proxy_config_new RENAME TO proxy_config", [])?;
        conn.execute("DROP TABLE IF EXISTS circuit_breaker_config", [])?;
        conn.execute("DELETE FROM settings WHERE key LIKE 'proxy_takeover_%'", [])?;
        conn.execute(
            "DELETE FROM settings WHERE key LIKE 'auto_failover_enabled_%'",
            [],
        )?;

        log::info!("proxy_config 已迁移为三行结构");
        Ok(())
    }

    /// 迁移 skills 表：从单 key 主键改为 (directory, app_type) 复合主键
    fn migrate_skills_table(conn: &Connection) -> Result<(), AppError> {
        // v3 结构（统一管理架构）已经是更高版本的 skills 表：
        // - 主键为 id
        // - 包含 enabled_claude / enabled_codex / enabled_gemini 等列
        // 在这种情况下，不应再执行 v1 -> v2 的迁移逻辑，否则会因列不匹配而失败。
        if Self::has_column(conn, "skills", "enabled_claude")?
            || Self::has_column(conn, "skills", "id")?
        {
            log::info!("skills 表已经是 v3 结构，跳过 v1 -> v2 迁移");
            return Ok(());
        }

        // 检查是否已经是新表结构
        if Self::has_column(conn, "skills", "app_type")? {
            log::info!("skills 表已经包含 app_type 字段，跳过迁移");
            return Ok(());
        }

        log::info!("开始迁移 skills 表...");

        // 1. 重命名旧表
        conn.execute("ALTER TABLE skills RENAME TO skills_old", [])
            .map_err(|e| AppError::Database(format!("重命名旧 skills 表失败: {e}")))?;

        // 2. 创建新表
        conn.execute(
            "CREATE TABLE skills (
                directory TEXT NOT NULL,
                app_type TEXT NOT NULL,
                installed BOOLEAN NOT NULL DEFAULT 0,
                installed_at INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (directory, app_type)
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建新 skills 表失败: {e}")))?;

        // 3. 迁移数据：解析 key 格式（如 "claude:my-skill" 或 "codex:foo"）
        //    旧数据如果没有前缀，默认为 claude
        let mut stmt = conn
            .prepare("SELECT key, installed, installed_at FROM skills_old")
            .map_err(|e| AppError::Database(format!("查询旧 skills 数据失败: {e}")))?;

        let old_skills: Vec<(String, bool, i64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("读取旧 skills 数据失败: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("解析旧 skills 数据失败: {e}")))?;

        let count = old_skills.len();

        for (key, installed, installed_at) in old_skills {
            // 解析 key: "app:directory" 或 "directory"（默认 claude）
            let (app_type, directory) = if let Some(idx) = key.find(':') {
                let (app, dir) = key.split_at(idx);
                (app.to_string(), dir[1..].to_string()) // 跳过冒号
            } else {
                ("claude".to_string(), key.clone())
            };

            conn.execute(
                "INSERT INTO skills (directory, app_type, installed, installed_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![directory, app_type, installed, installed_at],
            )
            .map_err(|e| {
                AppError::Database(format!("迁移 skill {key} 到新表失败: {e}"))
            })?;
        }

        // 4. 删除旧表
        conn.execute("DROP TABLE skills_old", [])
            .map_err(|e| AppError::Database(format!("删除旧 skills 表失败: {e}")))?;

        log::info!("skills 表迁移完成，共迁移 {count} 条记录");
        Ok(())
    }

    /// v2 -> v3 迁移：Skills 统一管理架构
    ///
    /// 将 skills 表从 (directory, app_type) 复合主键结构迁移到统一的 id 主键结构，
    /// 支持三应用启用标志（enabled_claude, enabled_codex, enabled_gemini）。
    ///
    /// 迁移策略：
    /// 1. 旧数据库只存储安装记录，真正的 skill 文件在文件系统
    /// 2. 直接重建新表结构，后续由 SkillService 在首次启动时扫描文件系统重建数据
    fn migrate_v2_to_v3(conn: &Connection) -> Result<(), AppError> {
        // 检查是否已经是新结构（通过检查是否有 enabled_claude 列）
        if Self::has_column(conn, "skills", "enabled_claude")? {
            log::info!("skills 表已经是 v3 结构，跳过迁移");
            return Ok(());
        }

        log::info!("开始迁移 skills 表到 v3 结构（统一管理架构）...");

        // 1. 备份旧数据（用于日志和后续启动迁移）
        let old_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))
            .unwrap_or(0);
        log::info!("旧 skills 表有 {old_count} 条记录");

        let mut stmt = conn
            .prepare(
                "SELECT directory, app_type FROM skills
                 WHERE installed = 1",
            )
            .map_err(|e| AppError::Database(format!("查询旧 skills 快照失败: {e}")))?;
        let snapshot_rows: Vec<LegacySkillMigrationRow> = stmt
            .query_map([], |row| {
                Ok(LegacySkillMigrationRow {
                    directory: row.get(0)?,
                    app_type: row.get(1)?,
                })
            })
            .map_err(|e| AppError::Database(format!("读取旧 skills 快照失败: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("解析旧 skills 快照失败: {e}")))?;
        let snapshot_json = serde_json::to_string(&snapshot_rows)
            .map_err(|e| AppError::Database(format!("序列化旧 skills 快照失败: {e}")))?;

        // 标记：需要在启动后从文件系统扫描并重建 Skills 数据
        // 说明：v3 结构将 Skills 的 SSOT 迁移到 ~/.OpenSunstar/skills/，
        // 旧表只存“安装记录”，无法直接无损迁移到新结构，因此改为启动后扫描 app 目录导入。
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('skills_ssot_migration_pending', 'true')",
            [],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('skills_ssot_migration_snapshot', ?1)",
            [snapshot_json],
        );

        // 2. 删除旧表
        conn.execute("DROP TABLE IF EXISTS skills", [])
            .map_err(|e| AppError::Database(format!("删除旧 skills 表失败: {e}")))?;

        // 3. 创建新表
        conn.execute(
            "CREATE TABLE skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                directory TEXT NOT NULL,
                repo_owner TEXT,
                repo_name TEXT,
                repo_branch TEXT DEFAULT 'main',
                readme_url TEXT,
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                installed_at INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建新 skills 表失败: {e}")))?;

        log::info!(
            "skills 表已迁移到 v3 结构。\n\
             注意：旧的安装记录已清除，首次启动时将自动扫描文件系统重建数据。"
        );

        Ok(())
    }

    /// v3 -> v4 迁移：添加 OpenCode 支持
    ///
    /// 为 mcp_servers 和 skills 表添加 enabled_opencode 列。
    fn migrate_v3_to_v4(conn: &Connection) -> Result<(), AppError> {
        // 为 mcp_servers 表添加 enabled_opencode 列
        Self::add_column_if_missing(
            conn,
            "mcp_servers",
            "enabled_opencode",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;

        // 为 skills 表添加 enabled_opencode 列
        Self::add_column_if_missing(
            conn,
            "skills",
            "enabled_opencode",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;

        log::info!("v3 -> v4 迁移完成：已添加 OpenCode 支持");
        Ok(())
    }

    /// v4 -> v5 迁移：新增计费模式配置与请求模型字段
    fn migrate_v4_to_v5(conn: &Connection) -> Result<(), AppError> {
        if Self::table_exists(conn, "proxy_config")? {
            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "default_cost_multiplier",
                "TEXT NOT NULL DEFAULT '1'",
            )?;
            Self::add_column_if_missing(
                conn,
                "proxy_config",
                "pricing_model_source",
                "TEXT NOT NULL DEFAULT 'response'",
            )?;
        }
        if Self::table_exists(conn, "proxy_request_logs")? {
            Self::add_column_if_missing(conn, "proxy_request_logs", "request_model", "TEXT")?;
        }

        log::info!("v4 -> v5 迁移完成：已添加计费模式与请求模型字段");
        Ok(())
    }

    /// v5 -> v6 迁移：添加使用量日聚合表 + 统一 Copilot 模板类型
    fn migrate_v5_to_v6(conn: &Connection) -> Result<(), AppError> {
        // 1. 添加使用量日聚合表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS usage_daily_rollups (
                date TEXT NOT NULL,
                app_type TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                model TEXT NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost_usd TEXT NOT NULL DEFAULT '0',
                avg_latency_ms INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (date, app_type, provider_id, model)
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 usage_daily_rollups 表失败: {e}")))?;

        // 2. 统一 Copilot 模板类型为 github_copilot
        let mut stmt = conn
            .prepare("SELECT id, app_type, meta FROM providers")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut updates = Vec::new();
        for row in rows {
            let (id, app_type, meta_str) = row.map_err(|e| AppError::Database(e.to_string()))?;

            if let Ok(mut meta) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                let mut updated = false;

                if let Some(usage_script) = meta.get_mut("usage_script") {
                    if let Some(template_type) = usage_script.get_mut("template_type") {
                        if template_type == "copilot" {
                            *template_type =
                                serde_json::Value::String("github_copilot".to_string());
                            updated = true;
                        }
                    }
                }

                if updated {
                    let new_meta_str = serde_json::to_string(&meta)
                        .map_err(|e| AppError::Database(e.to_string()))?;
                    updates.push((id, app_type, new_meta_str));
                }
            }
        }

        for (id, app_type, new_meta) in updates {
            conn.execute(
                "UPDATE providers SET meta = ?1 WHERE id = ?2 AND app_type = ?3",
                params![new_meta, id, app_type],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        log::info!("v5 -> v6 迁移完成：已添加使用量日聚合表，统一 copilot 模板类型");
        Ok(())
    }

    /// v6 -> v7: Skills 更新检测支持（content_hash + updated_at）
    fn migrate_v6_to_v7(conn: &Connection) -> Result<(), AppError> {
        if Self::table_exists(conn, "skills")? {
            Self::add_column_if_missing(conn, "skills", "content_hash", "TEXT")?;
            Self::add_column_if_missing(
                conn,
                "skills",
                "updated_at",
                "INTEGER NOT NULL DEFAULT 0",
            )?;
        }
        log::info!("v6 -> v7 迁移完成：已添加 content_hash 和 updated_at 列");
        Ok(())
    }

    /// v7 -> v8: 会话日志使用追踪（无代理模式统计支持）
    fn migrate_v7_to_v8(conn: &Connection) -> Result<(), AppError> {
        // 1. 为 proxy_request_logs 添加 data_source 列，区分数据来源
        if Self::table_exists(conn, "proxy_request_logs")? {
            Self::add_column_if_missing(
                conn,
                "proxy_request_logs",
                "data_source",
                "TEXT NOT NULL DEFAULT 'proxy'",
            )?;
            Self::create_request_logs_usage_indexes_if_supported(conn)?;
        }

        // 2. 创建会话日志同步状态表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS session_log_sync (
                file_path TEXT PRIMARY KEY,
                last_modified INTEGER NOT NULL,
                last_line_offset INTEGER NOT NULL DEFAULT 0,
                last_synced_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 session_log_sync 表失败: {e}")))?;

        // 3. 修正国产模型定价：之前误将 CNY 值存为 USD 字段，统一转换为 USD
        if Self::table_exists(conn, "model_pricing")? {
            let pricing_fixes: &[(&str, &str, &str, &str, &str)] = &[
                ("deepseek-v3.2", "0.28", "0.42", "0.028", "0"),
                ("deepseek-v3.1", "0.55", "1.67", "0.055", "0"),
                ("deepseek-v3", "0.28", "1.11", "0.028", "0"),
                ("doubao-seed-code", "0.17", "1.11", "0.02", "0"),
                ("kimi-k2-thinking", "0.55", "2.20", "0.10", "0"),
                ("kimi-k2-0905", "0.55", "2.20", "0.10", "0"),
                ("kimi-k2-turbo", "1.11", "8.06", "0.14", "0"),
                ("minimax-m2.1", "0.27", "0.95", "0.03", "0"),
                ("minimax-m2.1-lightning", "0.27", "2.33", "0.03", "0"),
                ("minimax-m2", "0.27", "0.95", "0.03", "0"),
                ("glm-4.7", "0.39", "1.75", "0.04", "0"),
                ("glm-4.6", "0.28", "1.11", "0.03", "0"),
                ("mimo-v2-flash", "0.09", "0.29", "0.009", "0"),
            ];
            for (model_id, input, output, cache_read, cache_creation) in pricing_fixes {
                conn.execute(
                    "UPDATE model_pricing SET
                        input_cost_per_million = ?2,
                        output_cost_per_million = ?3,
                        cache_read_cost_per_million = ?4,
                        cache_creation_cost_per_million = ?5
                     WHERE model_id = ?1",
                    rusqlite::params![model_id, input, output, cache_read, cache_creation],
                )
                .map_err(|e| AppError::Database(format!("更新模型 {model_id} 定价失败: {e}")))?;
            }
        }

        log::info!("v7 -> v8 迁移完成：data_source 列、session_log_sync 表、修正 13 个模型定价");
        Ok(())
    }

    /// v8 → v9: 全面补充模型定价（清空 + 重新 seed）
    fn migrate_v8_to_v9(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS model_pricing (
                model_id TEXT PRIMARY KEY, display_name TEXT NOT NULL,
                input_cost_per_million TEXT NOT NULL, output_cost_per_million TEXT NOT NULL,
                cache_read_cost_per_million TEXT NOT NULL DEFAULT '0',
                cache_creation_cost_per_million TEXT NOT NULL DEFAULT '0'
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 model_pricing 表失败: {e}")))?;
        conn.execute("DELETE FROM model_pricing", [])
            .map_err(|e| AppError::Database(format!("清空模型定价失败: {e}")))?;
        Self::seed_model_pricing(conn)?;
        log::info!("v8 -> v9 迁移完成：已刷新全部模型定价数据");
        Ok(())
    }

    /// v9 -> v10 迁移：添加 Hermes Agent 支持
    fn migrate_v9_to_v10(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(
            conn,
            "mcp_servers",
            "enabled_hermes",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;

        // skills table may not exist in databases migrated from very old versions
        if Self::table_exists(conn, "skills")? {
            Self::add_column_if_missing(
                conn,
                "skills",
                "enabled_hermes",
                "BOOLEAN NOT NULL DEFAULT 0",
            )?;
        }

        log::info!("v9 -> v10 迁移完成：已添加 Hermes Agent 支持");
        Ok(())
    }

    /// v10 -> v11：usage_daily_rollups 增加 request_model 维度（进入主键），
    /// proxy_request_logs 增加 pricing_model 列（写入时的计价基准，回填依据）。
    ///
    /// 路由接管下 model（真实上游模型）≠ request_model（客户端别名），
    /// 旧 rollup 只按 model 聚合，明细 prune 后映射关系永久丢失、计费不可审计。
    /// SQLite 改主键必须重建表；历史行的 request_model 已不可知，填 ''。
    fn migrate_v10_to_v11(conn: &Connection) -> Result<(), AppError> {
        // proxy_request_logs.pricing_model：NULL = v11 前的历史行（回填走
        // model → 占位符回退 request_model 的旧逻辑），'' = 未计价的错误行
        if Self::table_exists(conn, "proxy_request_logs")? {
            Self::add_column_if_missing(conn, "proxy_request_logs", "pricing_model", "TEXT")?;
        }

        if !Self::table_exists(conn, "usage_daily_rollups")? {
            log::info!("v10 -> v11：usage_daily_rollups 不存在，跳过重建");
            return Ok(());
        }

        conn.execute_batch(
            "ALTER TABLE usage_daily_rollups RENAME TO usage_daily_rollups_v10;
             CREATE TABLE usage_daily_rollups (
                 date TEXT NOT NULL,
                 app_type TEXT NOT NULL,
                 provider_id TEXT NOT NULL,
                 model TEXT NOT NULL,
                 request_model TEXT NOT NULL DEFAULT '',
                 pricing_model TEXT NOT NULL DEFAULT '',
                 request_count INTEGER NOT NULL DEFAULT 0,
                 success_count INTEGER NOT NULL DEFAULT 0,
                 input_tokens INTEGER NOT NULL DEFAULT 0,
                 output_tokens INTEGER NOT NULL DEFAULT 0,
                 cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                 cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                 total_cost_usd TEXT NOT NULL DEFAULT '0',
                 avg_latency_ms INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (date, app_type, provider_id, model, request_model, pricing_model)
             );
             INSERT INTO usage_daily_rollups
                 (date, app_type, provider_id, model, request_model, pricing_model,
                  request_count, success_count, input_tokens, output_tokens,
                  cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms)
             SELECT date, app_type, provider_id, model, '', '',
                  request_count, success_count, input_tokens, output_tokens,
                  cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
             FROM usage_daily_rollups_v10;
             DROP TABLE usage_daily_rollups_v10;",
        )
        .map_err(|e| {
            AppError::Database(format!("v10 -> v11 重建 usage_daily_rollups 失败: {e}"))
        })?;

        log::info!(
            "v10 -> v11 迁移完成：usage_daily_rollups 已保留 request_model/pricing_model 维度"
        );
        Ok(())
    }

    /// v11 -> v12: API Key 迁移至 OS Keychain
    ///
    /// 扫描 providers 表中所有 settings_config JSON，将明文 API Key
    /// 写入 OS Keychain（或加密 fallback 文件），DB 中替换为 keychain://ref/... 占位符。
    fn migrate_v11_to_v12(conn: &Connection) -> Result<(), AppError> {
        use crate::keychain;

        if !Self::table_exists(conn, "providers")? {
            log::info!("v11 -> v12 迁移跳过：providers 表不存在");
            return Ok(());
        }

        let mut stmt = conn
            .prepare("SELECT id, app_type, settings_config FROM providers")
            .map_err(|e| AppError::Database(format!("v12 迁移：查询 providers 失败: {e}")))?;

        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("v12 迁移：读取 providers 失败: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("v12 迁移：解析 providers 失败: {e}")))?;

        let mut migrated_count = 0;

        for (id, app_type, config_str) in &rows {
            let mut config: serde_json::Value = match serde_json::from_str(config_str) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let mut updated = false;

            // Known API key field paths in settings_config JSON
            let key_fields = [
                "ANTHROPIC_API_KEY",
                "OPENAI_API_KEY",
                "GEMINI_API_KEY",
                "api_key",
                "apiKey",
            ];

            // Check top-level fields
            for field in &key_fields {
                if let Some(val) = config.get(*field).and_then(|v| v.as_str()) {
                    if !val.is_empty() && !keychain::is_keychain_ref(val) {
                        match keychain::migrate_key_to_keychain(id, app_type, val) {
                            Ok(ref_val) => {
                                config[*field] = serde_json::Value::String(ref_val);
                                updated = true;
                            }
                            Err(e) => {
                                log::warn!(
                                    "v12 迁移：provider {id}/{app_type} 的 {field} 迁移失败: {e}"
                                );
                            }
                        }
                    }
                }
            }

            // Check nested env.* / auth.* fields
            for container in ["env", "auth"] {
                if let Some(obj) = config.get(container).cloned() {
                    if let Some(map) = obj.as_object() {
                        let mut new_map = map.clone();
                        for (k, v) in map {
                            if let Some(val) = v.as_str() {
                                let is_key_like = k.contains("KEY")
                                    || k.contains("TOKEN")
                                    || k.contains("SECRET");
                                if is_key_like && !val.is_empty() && !keychain::is_keychain_ref(val)
                                {
                                    let entry_key = format!("{id}/{app_type}/{container}.{k}");
                                    match keychain::store_secret(&entry_key, val) {
                                        Ok(()) => {
                                            new_map.insert(
                                                k.clone(),
                                                serde_json::Value::String(
                                                    keychain::make_keychain_ref(&entry_key),
                                                ),
                                            );
                                            updated = true;
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "v12 迁移：provider {id}/{app_type} 的 {container}.{k} 迁移失败: {e}"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        if updated {
                            config[container] = serde_json::Value::Object(new_map);
                        }
                    }
                }
            }

            if updated {
                let new_config_str = serde_json::to_string(&config)
                    .map_err(|e| AppError::Database(format!("v12 迁移：序列化失败: {e}")))?;
                conn.execute(
                    "UPDATE providers SET settings_config = ?1 WHERE id = ?2 AND app_type = ?3",
                    params![new_config_str, id, app_type],
                )
                .map_err(|e| {
                    AppError::Database(format!("v12 迁移：更新 provider {id}/{app_type} 失败: {e}"))
                })?;
                migrated_count += 1;
            }
        }

        log::info!(
            "v11 -> v12 迁移完成：已将 {migrated_count} 个 provider 的 API Key 迁移至 OS Keychain"
        );
        Ok(())
    }

    /// v12 -> v13: Prompt 桥接支持（bridge_source 列）
    fn migrate_v12_to_v13(conn: &Connection) -> Result<(), AppError> {
        if Self::table_exists(conn, "prompts")? {
            Self::add_column_if_missing(conn, "prompts", "bridge_source", "TEXT")?;
        }
        log::info!("v12 -> v13 迁移完成：prompts 表已添加 bridge_source 列");
        Ok(())
    }

    /// v13 -> v14: 项目级配置隔离（方案 E）
    ///
    /// 创建 projects 表和三张中间表：
    /// - project_mcp_servers: 项目 × MCP 服务器映射
    /// - project_skills: 项目 × Skills 映射
    /// - project_prompts: 项目 × Prompts 映射
    fn migrate_v13_to_v14(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                git_remote_url TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 projects 表失败: {e}")))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_mcp_servers (
                project_id TEXT NOT NULL,
                mcp_server_id TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_id, mcp_server_id),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (mcp_server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 project_mcp_servers 表失败: {e}")))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_skills (
                project_id TEXT NOT NULL,
                skill_id TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_id, skill_id),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 project_skills 表失败: {e}")))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_prompts (
                project_id TEXT NOT NULL,
                prompt_id TEXT NOT NULL,
                prompt_app_type TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_id, prompt_id, prompt_app_type),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (prompt_id, prompt_app_type) REFERENCES prompts(id, app_type) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 project_prompts 表失败: {e}")))?;

        log::info!("v13 -> v14 迁移完成：已创建项目级配置隔离表（projects + 3 张中间表）");
        Ok(())
    }

    /// v14 -> v15: Commands + Hooks 管理（M1）
    fn migrate_v14_to_v15(conn: &Connection) -> Result<(), AppError> {
        Self::create_commands_table(conn)?;
        Self::create_hooks_table(conn)?;
        log::info!("v14 -> v15 迁移完成：已创建 commands 和 hooks 表");
        Ok(())
    }

    fn create_commands_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS commands (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                content TEXT NOT NULL,
                arguments TEXT NOT NULL DEFAULT '[]',
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
                enabled_hermes BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 commands 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_commands_name ON commands(name)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_commands_name 失败: {e}")))?;
        Ok(())
    }

    fn create_hooks_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS hooks (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                tool_pattern TEXT NOT NULL DEFAULT '*',
                hook_command TEXT NOT NULL,
                timeout_seconds INTEGER NOT NULL DEFAULT 30,
                enabled_claude BOOLEAN NOT NULL DEFAULT 1,
                description TEXT,
                sort_index INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 hooks 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hooks_event ON hooks(event_type)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_hooks_event 失败: {e}")))?;
        Ok(())
    }

    /// v16 -> v17: Prompts fragment 字段扩展（M4 / F6）
    fn migrate_v16_to_v17(conn: &Connection) -> Result<(), AppError> {
        if !Self::table_exists(conn, "prompts")? {
            log::info!("v16 -> v17 迁移跳过：prompts 表不存在");
            return Ok(());
        }
        Self::add_column_if_missing(
            conn,
            "prompts",
            "targets",
            "TEXT NOT NULL DEFAULT '[\"*\"]'",
        )?;
        Self::add_column_if_missing(conn, "prompts", "globs", "TEXT NOT NULL DEFAULT '[]'")?;
        Self::add_column_if_missing(conn, "prompts", "priority", "INTEGER NOT NULL DEFAULT 0")?;
        Self::add_column_if_missing(conn, "prompts", "is_fragment", "BOOLEAN NOT NULL DEFAULT 0")?;
        Self::add_column_if_missing(conn, "prompts", "parent_prompt_id", "TEXT")?;
        log::info!("v16 -> v17 迁移完成：已扩展 prompts 表 fragment 字段");
        Ok(())
    }

    /// v17 -> v18: Agents / Subagents 管理（M5 / F8）
    fn migrate_v17_to_v18(conn: &Connection) -> Result<(), AppError> {
        Self::create_agents_table(conn)?;
        log::info!("v17 -> v18 迁移完成：已创建 agents 表");
        Ok(())
    }

    fn create_agents_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                content TEXT NOT NULL,
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
                enabled_hermes BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 agents 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agents_name ON agents(name)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_agents_name 失败: {e}")))?;
        Ok(())
    }

    /// v18 -> v19: AI Insights 缓存 + 成本日志（项目看板 AI 能力 Phase 1）
    fn migrate_v18_to_v19(conn: &Connection) -> Result<(), AppError> {
        Self::create_ai_insights_table(conn)?;
        Self::create_ai_cost_log_table(conn)?;
        log::info!("v18 -> v19 迁移完成：已创建 ai_insights 和 ai_cost_log 表");
        Ok(())
    }

    /// v19 -> v20: AI Insights 用户反馈列（反馈闭环）
    fn migrate_v19_to_v20(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(conn, "ai_insights", "user_feedback", "TEXT DEFAULT NULL")?;
        log::info!("v19 -> v20 迁移完成：ai_insights 新增 user_feedback 列");
        Ok(())
    }

    /// v20 -> v21: NL 问答独立日志表 + 合并遗留 path_* project_id
    fn migrate_v20_to_v21(conn: &Connection) -> Result<(), AppError> {
        Self::create_ai_query_log_table(conn)?;
        Self::normalize_legacy_path_project_ids(conn)?;
        log::info!("v20 -> v21 迁移完成：已创建 ai_query_log 并规范化 project_id");
        Ok(())
    }

    /// v21 -> v22: 项目资产统一关联表（初版 5 类；v25 扩展为 8 类 SSOT）
    fn migrate_v21_to_v22(conn: &Connection) -> Result<(), AppError> {
        Self::create_project_asset_links_table(conn)?;
        log::info!("v21 -> v22 迁移完成：已创建 project_asset_links");
        Ok(())
    }

    fn create_project_asset_links_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_asset_links (
                project_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                asset_app_type TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                scope TEXT NOT NULL DEFAULT 'project',
                source TEXT NOT NULL DEFAULT 'manual',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (project_id, asset_type, asset_id, asset_app_type),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 project_asset_links 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_asset_links_project_type
             ON project_asset_links(project_id, asset_type)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_project_asset_links 失败: {e}")))?;
        Ok(())
    }

    /// v24 -> v25: MCP/Skills/Prompts 旧三表数据迁入 `project_asset_links` 并删除旧表
    fn migrate_v24_to_v25(conn: &Connection) -> Result<(), AppError> {
        Self::create_project_asset_links_table(conn)?;

        Self::migrate_legacy_project_link_table(
            conn,
            "project_mcp_servers",
            "mcp",
            "mcp_server_id",
            "''",
            "l.project_id = project_asset_links.project_id AND l.mcp_server_id = project_asset_links.asset_id AND project_asset_links.asset_app_type = ''",
        )?;
        Self::migrate_legacy_project_link_table(
            conn,
            "project_skills",
            "skill",
            "skill_id",
            "''",
            "l.project_id = project_asset_links.project_id AND l.skill_id = project_asset_links.asset_id AND project_asset_links.asset_app_type = ''",
        )?;
        Self::migrate_legacy_project_link_table(
            conn,
            "project_prompts",
            "prompt",
            "prompt_id",
            "prompt_app_type",
            "l.project_id = project_asset_links.project_id AND l.prompt_id = project_asset_links.asset_id AND l.prompt_app_type = project_asset_links.asset_app_type",
        )?;

        conn.execute("DROP TABLE IF EXISTS project_mcp_servers", [])
            .map_err(|e| AppError::Database(format!("删除 project_mcp_servers 失败: {e}")))?;
        conn.execute("DROP TABLE IF EXISTS project_skills", [])
            .map_err(|e| AppError::Database(format!("删除 project_skills 失败: {e}")))?;
        conn.execute("DROP TABLE IF EXISTS project_prompts", [])
            .map_err(|e| AppError::Database(format!("删除 project_prompts 失败: {e}")))?;

        log::info!("v24 -> v25 迁移完成：8 类资产统一至 project_asset_links");
        Ok(())
    }

    /// v25 -> v26: SDD 框架探测表（sdd_descriptors + project_sdd_detections）+ 7 框架种子数据
    fn migrate_v25_to_v26(conn: &Connection) -> Result<(), AppError> {
        Self::create_sdd_descriptors_table(conn)?;
        Self::create_project_sdd_detections_table(conn)?;

        // Seed 7 framework descriptors (idempotent via INSERT OR IGNORE)
        let seed_descriptors = [
            (
                "bmad-method",
                "BMAD-METHOD",
                "v6.10.0",
                "linear",
                "npm",
                "BMAD 全栈方法论：角色驱动 + 上下文分层",
                "BMAD full-stack methodology: role-driven + context-layered",
                "https://github.com/bmad-method/BMAD-METHOD",
            ),
            (
                "task-master",
                "Task Master AI",
                "0.43.1",
                "linear",
                "npm",
                "AI 驱动的任务拆解与执行管理",
                "AI-driven task decomposition and execution management",
                "https://github.com/eyecuelab/taskmaster",
            ),
            (
                "superpowers",
                "Superpowers",
                "v6.1.1",
                "linear",
                "plugin",
                "TDD + 角色工作流 + 技能路由",
                "TDD + role workflow + skill routing",
                "https://github.com/obra/superpowers",
            ),
            (
                "gstack",
                "gstack",
                "1.58.5.0",
                "linear",
                "file_copy",
                "GStack 编排指挥：Think→Plan→Build→Review→Test→Ship→Reflect",
                "GStack conductor: Think→Plan→Build→Review→Test→Ship→Reflect",
                "https://github.com/garrytan/gstack",
            ),
            (
                "openspec",
                "OpenSpec",
                "v1.5.0",
                "linear",
                "file_copy",
                "变更驱动的规格文档管理（ADR + CHANGE）",
                "Change-driven specification document management (ADR + CHANGE)",
                "https://github.com/Fission-AI/OpenSpec",
            ),
            (
                "spec-kit",
                "Spec Kit",
                "v0.12.4",
                "linear",
                "uvx",
                "规格工具链：AC 格式 + 级联文档",
                "Specification toolchain: AC format + cascading docs",
                "https://github.com/nicholasgriffintn/spec-kit",
            ),
            (
                "flow-kit",
                "flow-kit",
                "unversioned",
                "linear",
                "file_copy",
                "纯 Markdown AI 编程流程（39 文件模板体系）",
                "Pure markdown AI programming flow (39-file template system)",
                "https://github.com/rihebty/flow-kit",
            ),
        ];

        for (id, name, version, phase, install, desc_zh, desc_en, repo) in &seed_descriptors {
            let signals_json = match *id {
                "bmad-method" => r#"[".bmad/"]"#,
                "task-master" => r#"["task-master in package.json"]"#,
                "superpowers" => r#"[".superpowers/", "superpowers in package.json"]"#,
                "gstack" => r#"[".gstack/", "SKILL.md gstack routing"]"#,
                "openspec" => r#"[".openspec/"]"#,
                "spec-kit" => r#"[".spec-kit/", "spec-kit in package.json"]"#,
                "flow-kit" => r#"["flow-kit/", "flow-kit/GO.md"]"#,
                _ => "[]",
            };
            let descriptor_json = format!(
                r#"{{"id":"{}","name":"{}","version":"{}","phase_model":"{}"}}"#,
                id, name, version, phase
            );
            conn.execute(
                "INSERT OR IGNORE INTO sdd_descriptors
                    (id, name, version, phase_model, probe_signals, install_type, risk_tier,
                     description_zh, description_en, repo_url, descriptor_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'read_only', ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    id,
                    name,
                    version,
                    phase,
                    signals_json,
                    install,
                    desc_zh,
                    desc_en,
                    repo,
                    descriptor_json,
                ],
            )
            .map_err(|e| AppError::Database(format!("seed sdd_descriptors {id} 失败: {e}")))?;
        }

        log::info!("v25 -> v26 迁移完成：SDD 框架探测表 + 7 框架种子数据");
        Ok(())
    }

    /// v26 → v27: 修正 sdd_descriptors.install_type 字段值。
    ///
    /// 经人工验收查证：
    /// - BMAD-METHOD: file_copy → npm（官方安装方式为 `npx bmad-method install`）
    /// - Superpowers: file_copy → plugin（走 Claude Code 原生插件协议）
    /// - Spec Kit: npm → uvx（Python/uv 驱动，`uvx --from git+...specify init`）
    fn migrate_v26_to_v27(conn: &Connection) -> Result<(), AppError> {
        let updates = [
            ("npm", "bmad-method"),
            ("plugin", "superpowers"),
            ("uvx", "spec-kit"),
        ];
        for (new_type, id) in &updates {
            conn.execute(
                "UPDATE sdd_descriptors SET install_type = ?1 WHERE id = ?2",
                rusqlite::params![new_type, id],
            )
            .map_err(|e| AppError::Database(format!("修正 install_type for {id} 失败: {e}")))?;
        }
        log::info!("v26 -> v27 迁移完成：修正 BMAD→npm, Superpowers→plugin, Spec Kit→uvx");
        Ok(())
    }

    /// v27 → v28: 看板阶段与 MVP 进度迁入 SQLite projects 表
    fn migrate_v27_to_v28(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(conn, "projects", "stage", "TEXT NOT NULL DEFAULT 'mvp'")?;
        Self::add_column_if_missing(conn, "projects", "mvp_progress", "INTEGER")?;
        log::info!("v27 -> v28 迁移完成：projects 增加 stage、mvp_progress");
        Ok(())
    }

    /// v28 -> v29: 资产健康模型的不可变修订、项目期望、部署回执和验证证据。
    ///
    /// 现有 `project_asset_links` 保持为用户选择的兼容层；新表只追加健康事实，
    /// 不迁移或删除历史关联，避免把旧 `asset_app_type` 的多重语义当作目标应用。
    fn migrate_v28_to_v29(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS asset_revisions (
                revision_id TEXT PRIMARY KEY,
                asset_type TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                version_label TEXT,
                content_sha256 TEXT NOT NULL,
                source_kind TEXT NOT NULL,
                source_ref TEXT,
                source_revision TEXT,
                metadata_json TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                UNIQUE(asset_type, asset_id, content_sha256)
            );
            CREATE INDEX IF NOT EXISTS idx_asset_revisions_asset
                ON asset_revisions(asset_type, asset_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS project_asset_expectations (
                expectation_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                target_app TEXT NOT NULL,
                desired_state TEXT NOT NULL DEFAULT 'enabled',
                required_revision_id TEXT,
                verification_policy TEXT,
                scope TEXT NOT NULL DEFAULT 'project',
                source TEXT NOT NULL DEFAULT 'manual',
                owner_mode TEXT NOT NULL DEFAULT 'observed',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(project_id, asset_type, asset_id, target_app),
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY(required_revision_id) REFERENCES asset_revisions(revision_id)
            );
            CREATE INDEX IF NOT EXISTS idx_asset_expectations_project
                ON project_asset_expectations(project_id, target_app, desired_state);

            CREATE TABLE IF NOT EXISTS asset_deployment_receipts (
                receipt_id TEXT PRIMARY KEY,
                expectation_id TEXT NOT NULL,
                operation_id TEXT NOT NULL,
                adapter_id TEXT NOT NULL,
                adapter_version TEXT NOT NULL,
                plan_sha256 TEXT NOT NULL,
                dry_run INTEGER NOT NULL DEFAULT 0,
                outcome TEXT NOT NULL,
                target_path TEXT,
                before_sha256 TEXT,
                after_sha256 TEXT,
                snapshot_ref TEXT,
                reason_code TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(expectation_id) REFERENCES project_asset_expectations(expectation_id)
            );
            CREATE INDEX IF NOT EXISTS idx_asset_receipts_expectation
                ON asset_deployment_receipts(expectation_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS asset_runtime_evidence (
                evidence_id TEXT PRIMARY KEY,
                expectation_id TEXT NOT NULL,
                evidence_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                observed_revision_sha256 TEXT,
                confidence TEXT NOT NULL,
                collector TEXT NOT NULL,
                collector_version TEXT NOT NULL,
                observed_at INTEGER NOT NULL,
                expires_at INTEGER,
                details_json TEXT NOT NULL DEFAULT '{}',
                FOREIGN KEY(expectation_id) REFERENCES project_asset_expectations(expectation_id)
            );
            CREATE INDEX IF NOT EXISTS idx_asset_evidence_expectation
                ON asset_runtime_evidence(expectation_id, observed_at DESC);",
        )
        .map_err(|e| AppError::Database(format!("创建资产健康事实表失败: {e}")))?;
        log::info!("v28 -> v29 迁移完成：资产健康事实表");
        Ok(())
    }

    /// v29 -> v30: per-file deployment receipts. Only relative paths and
    /// content digests are persisted; file bodies and secrets are excluded.
    fn migrate_v29_to_v30(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS asset_receipt_files (
                file_id TEXT PRIMARY KEY NOT NULL,
                receipt_id TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                action TEXT NOT NULL,
                before_sha256 TEXT,
                after_sha256 TEXT,
                snapshot_ref TEXT,
                reason_code TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(receipt_id) REFERENCES asset_deployment_receipts(receipt_id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_asset_receipt_files_receipt
                ON asset_receipt_files(receipt_id, created_at);",
        )
        .map_err(|error| AppError::Database(format!("创建资产逐文件回执表失败: {error}")))?;
        Ok(())
    }

    /// v30 -> v31: bind every deployment receipt to the immutable asset
    /// revision that was required when its plan was confirmed.
    fn migrate_v30_to_v31(conn: &Connection) -> Result<(), AppError> {
        if !Self::has_column(conn, "asset_deployment_receipts", "required_revision_id")? {
            conn.execute(
                "ALTER TABLE asset_deployment_receipts ADD COLUMN required_revision_id TEXT",
                [],
            )
            .map_err(|error| {
                AppError::Database(format!("为资产部署回执增加修订引用失败: {error}"))
            })?;
        }
        Ok(())
    }

    fn create_project_environment_snapshots_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS project_environment_snapshots (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                name TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_applied_at INTEGER,
                last_apply_receipt TEXT,
                UNIQUE(project_id, name),
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_project_environment_snapshots_project
                ON project_environment_snapshots(project_id, updated_at DESC);",
        )
        .map_err(|error| AppError::Database(format!("创建项目环境快照表失败: {error}")))?;
        Ok(())
    }

    fn migrate_v31_to_v32(conn: &Connection) -> Result<(), AppError> {
        Self::create_project_environment_snapshots_table(conn)
    }

    fn migrate_v32_to_v33(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS quick_start_operations (
                id                   TEXT PRIMARY KEY,
                idempotency_key      TEXT NOT NULL UNIQUE,
                request_fingerprint  TEXT NOT NULL,
                app_type             TEXT NOT NULL CHECK(app_type IN ('claude','claude-desktop','codex','gemini')),
                provider_id          TEXT,
                previous_provider_id TEXT,
                status               TEXT NOT NULL CHECK(status IN ('pending','applying','verifying','succeeded','failed','rolling_back','rolled_back','rollback_failed')),
                current_step         TEXT NOT NULL,
                revision             INTEGER NOT NULL DEFAULT 0,
                provider_created     INTEGER NOT NULL DEFAULT 0,
                provider_switched    INTEGER NOT NULL DEFAULT 0,
                takeover_enabled     INTEGER NOT NULL DEFAULT 0,
                proxy_started        INTEGER NOT NULL DEFAULT 0,
                post_verified        INTEGER NOT NULL DEFAULT 0,
                takeover_was_enabled INTEGER NOT NULL DEFAULT 0,
                proxy_was_running    INTEGER NOT NULL DEFAULT 0,
                error_code           TEXT,
                error_message        TEXT,
                created_at           TEXT NOT NULL,
                updated_at           TEXT NOT NULL,
                completed_at         TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_quick_start_operations_status_updated
                ON quick_start_operations(status, updated_at DESC);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_quick_start_operations_active_app
                ON quick_start_operations(app_type)
                WHERE status IN ('pending','applying','verifying','rolling_back','rollback_failed');
            CREATE TABLE IF NOT EXISTS quick_start_operation_events (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                operation_id  TEXT NOT NULL REFERENCES quick_start_operations(id) ON DELETE CASCADE,
                sequence      INTEGER NOT NULL,
                event_type    TEXT NOT NULL,
                from_status   TEXT,
                to_status     TEXT,
                step          TEXT NOT NULL,
                error_code    TEXT,
                error_message TEXT,
                detail_json   TEXT,
                created_at    TEXT NOT NULL,
                UNIQUE(operation_id, sequence)
            );
            CREATE INDEX IF NOT EXISTS idx_quick_start_events_operation
                ON quick_start_operation_events(operation_id, sequence);",
        )
        .map_err(|e| AppError::Database(format!("Create QuickStart operation tables failed: {e}")))?;

        if Self::table_exists(conn, "proxy_live_backup")? {
            let mut stmt = conn
                .prepare(
                    "SELECT app_type, original_config FROM proxy_live_backup
                     WHERE original_config NOT LIKE 'enc:v1:%'",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let plaintext_rows: Vec<(String, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<_, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?;
            drop(stmt);
            for (app_type, plaintext) in plaintext_rows {
                let sealed = crate::keychain::seal_local_secret(&plaintext)?;
                conn.execute(
                    "UPDATE proxy_live_backup SET original_config = ?1 WHERE app_type = ?2",
                    rusqlite::params![sealed, app_type],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn migrate_v33_to_v34(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(conn, "quick_start_operations", "live_snapshot", "TEXT")?;
        Ok(())
    }

    fn migrate_v34_to_v35(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(
            conn,
            "quick_start_operations",
            "applied_live_fingerprint",
            "TEXT",
        )?;
        Ok(())
    }

    fn migrate_v35_to_v36(conn: &Connection) -> Result<(), AppError> {
        // 部分历史/测试数据库只保留了业务表，不能假设旧迁移一定创建过定价表。
        conn.execute(
            "CREATE TABLE IF NOT EXISTS model_pricing (
                model_id TEXT PRIMARY KEY, display_name TEXT NOT NULL,
                input_cost_per_million TEXT NOT NULL, output_cost_per_million TEXT NOT NULL,
                cache_read_cost_per_million TEXT NOT NULL DEFAULT '0',
                cache_creation_cost_per_million TEXT NOT NULL DEFAULT '0'
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 model_pricing 表失败: {e}")))?;
        Self::create_model_pricing_provenance_table(conn)?;
        // 旧数据库尚未包含 GPT-5.6 价格时，先用 INSERT OR IGNORE 补齐内置项；
        // 既有用户自定义价格不会被覆盖。
        Self::seed_model_pricing(conn)?;
        Self::seed_gpt_5_6_pricing_provenance(conn)
    }

    fn migrate_v36_to_v37(conn: &Connection) -> Result<(), AppError> {
        if Self::table_exists(conn, "providers")? {
            let mut stmt = conn
                .prepare(
                    "SELECT id, settings_config, category FROM providers WHERE app_type = 'codex'",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows: Vec<(String, String, Option<String>)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<_, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?;
            drop(stmt);

            for (id, settings_json, category) in rows {
                let settings: Value = serde_json::from_str(&settings_json).map_err(|e| {
                    AppError::Database(format!(
                        "解析 Codex Provider 凭据清理数据失败（provider={id}）: {e}"
                    ))
                })?;
                let sanitized =
                    crate::codex_config::sanitize_codex_settings_for_storage_with_category(
                        &settings,
                        category.as_deref(),
                    );
                let sanitized = serde_json::to_string(&sanitized)
                    .map_err(|e| AppError::Serialization(e.to_string()))?;
                conn.execute(
                    "UPDATE providers SET settings_config = ?1 WHERE id = ?2 AND app_type = 'codex'",
                    params![sanitized, id],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        if Self::table_exists(conn, "proxy_live_backup")? {
            let encrypted: Option<String> = conn
                .query_row(
                    "SELECT original_config FROM proxy_live_backup WHERE app_type = 'codex'",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| AppError::Database(e.to_string()))?;
            if let Some(encrypted) = encrypted {
                let sanitized = crate::keychain::open_local_secret(&encrypted)
                    .ok()
                    .and_then(|json| serde_json::from_str::<Value>(&json).ok())
                    .and_then(|settings| {
                        crate::codex_config::sanitize_codex_live_backup_for_storage(&settings).ok()
                    })
                    .and_then(|settings| serde_json::to_string(&settings).ok())
                    .and_then(|json| crate::keychain::seal_local_secret(&json).ok());
                if let Some(sanitized) = sanitized {
                    conn.execute(
                        "UPDATE proxy_live_backup SET original_config = ?1 WHERE app_type = 'codex'",
                        [sanitized],
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                } else {
                    log::warn!("无法安全清理旧 Codex Live 备份，已删除该凭据快照");
                    conn.execute("DELETE FROM proxy_live_backup WHERE app_type = 'codex'", [])
                        .map_err(|e| AppError::Database(e.to_string()))?;
                }
            }
        }

        if Self::table_exists(conn, "quick_start_operations")? {
            let mut stmt = conn
                .prepare(
                    "SELECT id, live_snapshot FROM quick_start_operations
                     WHERE app_type = 'codex' AND live_snapshot IS NOT NULL",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows: Vec<(String, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<_, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?;
            drop(stmt);

            for (id, encrypted) in rows {
                let sanitized = crate::keychain::open_local_secret(&encrypted)
                    .ok()
                    .and_then(|json| serde_json::from_str::<Value>(&json).ok())
                    .and_then(|mut snapshot| {
                        let codex = snapshot.pointer_mut("/Provider/Codex")?.as_object_mut()?;
                        codex.insert("auth".to_string(), Value::Null);
                        serde_json::to_string(&snapshot).ok()
                    })
                    .and_then(|json| crate::keychain::seal_local_secret(&json).ok());
                if let Some(sanitized) = sanitized {
                    conn.execute(
                        "UPDATE quick_start_operations SET live_snapshot = ?1 WHERE id = ?2",
                        params![sanitized, id],
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                } else {
                    log::warn!("无法安全清理旧 Codex QuickStart 快照，已删除该凭据快照");
                    conn.execute(
                        "UPDATE quick_start_operations SET live_snapshot = NULL WHERE id = ?1",
                        [id],
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                }
            }
        }

        Ok(())
    }

    fn create_model_pricing_provenance_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS model_pricing_provenance (
                model_id TEXT PRIMARY KEY NOT NULL,
                source TEXT NOT NULL,
                source_version TEXT NOT NULL,
                effective_at TEXT NOT NULL,
                currency TEXT NOT NULL DEFAULT 'USD',
                long_context_threshold_tokens INTEGER,
                long_context_input_multiplier TEXT NOT NULL DEFAULT '1',
                long_context_output_multiplier TEXT NOT NULL DEFAULT '1',
                FOREIGN KEY(model_id) REFERENCES model_pricing(model_id) ON DELETE CASCADE
            );",
        )
        .map_err(|e| AppError::Database(format!("创建 model_pricing_provenance 表失败: {e}")))?;

        // v36 开发期内曾创建过不含 currency 的同名表；不能仅依赖 user_version，
        // 启动时对既有表做幂等补列，避免种子写入阻断整个数据库初始化。
        Self::add_column_if_missing(
            conn,
            "model_pricing_provenance",
            "currency",
            "TEXT NOT NULL DEFAULT 'USD'",
        )?;
        Ok(())
    }

    /// 从旧三表之一拷贝行到 `project_asset_links`；冲突时以 legacy 行的 enabled/时间戳为准并打审计日志
    fn migrate_legacy_project_link_table(
        conn: &Connection,
        legacy_table: &str,
        asset_type: &str,
        asset_id_col: &str,
        asset_app_type_expr: &str,
        legacy_match_sql: &str,
    ) -> Result<(), AppError> {
        if !Self::table_exists(conn, legacy_table)? {
            return Ok(());
        }

        let legacy_count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {legacy_table}"), [], |row| {
                row.get(0)
            })
            .map_err(|e| AppError::Database(format!("统计 {legacy_table} 失败: {e}")))?;

        let unified_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_asset_links WHERE asset_type = ?1",
                [asset_type],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let sql = format!(
            "INSERT OR IGNORE INTO project_asset_links
             (project_id, asset_type, asset_id, asset_app_type, enabled, scope, source, created_at, updated_at)
             SELECT project_id, ?1, {asset_id_col}, {asset_app_type_expr}, enabled, 'project', 'migrated', created_at, created_at
             FROM {legacy_table}"
        );
        conn.execute(&sql, [asset_type])
            .map_err(|e| AppError::Database(format!("迁移 {legacy_table} 失败: {e}")))?;

        let merge_sql = format!(
            "UPDATE project_asset_links
             SET enabled = (
                 SELECT l.enabled FROM {legacy_table} l
                 WHERE {legacy_match_sql}
             ),
             updated_at = MAX(
                 project_asset_links.updated_at,
                 COALESCE(
                     (SELECT l.created_at FROM {legacy_table} l WHERE {legacy_match_sql}),
                     project_asset_links.updated_at
                 )
             )
             WHERE asset_type = ?1
               AND EXISTS (SELECT 1 FROM {legacy_table} l WHERE {legacy_match_sql})"
        );
        conn.execute(&merge_sql, [asset_type])
            .map_err(|e| AppError::Database(format!("合并 {legacy_table} 冲突行失败: {e}")))?;

        let unified_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_asset_links WHERE asset_type = ?1",
                [asset_type],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let net_new = unified_after.saturating_sub(unified_before);
        if legacy_count > 0 {
            log::info!(
                "v25 迁移 {legacy_table} → project_asset_links[{asset_type}]: legacy={legacy_count}, unified_before={unified_before}, unified_after={unified_after}, net_new={net_new}"
            );
            if net_new < legacy_count {
                log::warn!(
                    "v25 迁移 {legacy_table}: {conflicts} 行与既有 project_asset_links 主键冲突，已按 legacy 合并 enabled/updated_at",
                    conflicts = legacy_count - net_new
                );
            }
        }

        Ok(())
    }

    /// v22 -> v23: 项目级治理元数据（目标 CLI、已应用 Blueprint）
    fn migrate_v22_to_v23(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(conn, "projects", "target_app", "TEXT")?;
        Self::add_column_if_missing(conn, "projects", "blueprint_id", "TEXT")?;
        log::info!("v22 -> v23 迁移完成：projects 增加 target_app、blueprint_id");
        Ok(())
    }

    /// v23 -> v24: Hooks / Permissions 多 CLI 启用标志
    fn migrate_v23_to_v24(conn: &Connection) -> Result<(), AppError> {
        for table in ["hooks", "tool_permissions"] {
            Self::add_column_if_missing(
                conn,
                table,
                "enabled_codex",
                "BOOLEAN NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                table,
                "enabled_gemini",
                "BOOLEAN NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                table,
                "enabled_opencode",
                "BOOLEAN NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                table,
                "enabled_hermes",
                "BOOLEAN NOT NULL DEFAULT 0",
            )?;
        }
        Self::add_column_if_missing(
            conn,
            "tool_permissions",
            "enabled_openclaw",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;
        log::info!("v23 -> v24 迁移完成：hooks / tool_permissions 增加多 CLI enabled_* 列");
        Ok(())
    }

    fn create_ai_query_log_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_query_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query_text TEXT NOT NULL,
                answer_preview TEXT,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0.0,
                model TEXT,
                provider TEXT,
                user_feedback TEXT DEFAULT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 ai_query_log 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ai_query_created ON ai_query_log(created_at)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_ai_query_created 失败: {e}")))?;
        Ok(())
    }

    /// 将遗留 path_* id 合并为 projects 表中的 canonical proj_* id
    fn normalize_legacy_path_project_ids(conn: &Connection) -> Result<(), AppError> {
        let mut stmt = conn
            .prepare("SELECT id, path FROM projects")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| AppError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        for (canonical_id, path) in rows {
            let legacy_id = crate::ai::project_id::path_legacy_id(&path);
            if legacy_id == canonical_id {
                continue;
            }
            conn.execute(
                "UPDATE ai_insights SET project_id = ?1 WHERE project_id = ?2",
                rusqlite::params![canonical_id, legacy_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "UPDATE ai_cost_log SET project_id = ?1 WHERE project_id = ?2",
                rusqlite::params![canonical_id, legacy_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    fn create_ai_insights_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_insights (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id TEXT NOT NULL,
                insight_type TEXT NOT NULL,
                content TEXT NOT NULL,
                model_used TEXT,
                tokens_used INTEGER NOT NULL DEFAULT 0,
                cost_estimate REAL NOT NULL DEFAULT 0.0,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                input_hash TEXT NOT NULL,
                user_feedback TEXT DEFAULT NULL,
                UNIQUE(project_id, insight_type)
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 ai_insights 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ai_insights_project ON ai_insights(project_id)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_ai_insights_project 失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ai_insights_expires ON ai_insights(expires_at)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_ai_insights_expires 失败: {e}")))?;
        Ok(())
    }

    fn create_ai_cost_log_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_cost_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                insight_type TEXT NOT NULL,
                project_id TEXT,
                model TEXT,
                provider TEXT,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0.0,
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 ai_cost_log 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ai_cost_created ON ai_cost_log(created_at)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_ai_cost_created 失败: {e}")))?;
        Ok(())
    }

    /// v15 -> v16: Ignore + Permissions 管理（M3）
    fn migrate_v15_to_v16(conn: &Connection) -> Result<(), AppError> {
        Self::create_ignore_rules_table(conn)?;
        Self::create_tool_permissions_table(conn)?;
        log::info!("v15 -> v16 迁移完成：已创建 ignore_rules 和 tool_permissions 表");
        Ok(())
    }

    fn create_ignore_rules_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ignore_rules (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                description TEXT,
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
                enabled_hermes BOOLEAN NOT NULL DEFAULT 0,
                sort_index INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 ignore_rules 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ignore_rules_pattern ON ignore_rules(pattern)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_ignore_rules_pattern 失败: {e}")))?;
        Ok(())
    }

    fn create_tool_permissions_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tool_permissions (
                id TEXT PRIMARY KEY,
                permission_type TEXT NOT NULL CHECK (permission_type IN (
                    'allowedTools', 'deniedTools', 'autoApprove'
                )),
                tool_pattern TEXT NOT NULL,
                enabled_claude BOOLEAN NOT NULL DEFAULT 1,
                description TEXT,
                sort_index INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 tool_permissions 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tool_permissions_type ON tool_permissions(permission_type)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_tool_permissions_type 失败: {e}")))?;
        Ok(())
    }

    /// 插入默认模型定价数据
    /// 格式: (model_id, display_name, input, output, cache_read, cache_creation)
    /// 注意: model_id 使用短横线格式（如 claude-haiku-4-5），与 API 返回的模型名称标准化后一致
    fn seed_model_pricing(conn: &Connection) -> Result<(), AppError> {
        let pricing_data = [
            // Claude Fable 5（Opus 之上的新档）
            (
                "claude-fable-5",
                "Claude Fable 5",
                "10",
                "50",
                "1.00",
                "12.50",
            ),
            (
                "claude-mythos-5",
                "Claude Mythos 5",
                "10",
                "50",
                "1.00",
                "12.50",
            ),
            // Claude 4.8 系列
            (
                "claude-opus-4-8",
                "Claude Opus 4.8",
                "5",
                "25",
                "0.50",
                "6.25",
            ),
            // Claude 4.7 系列
            (
                "claude-opus-4-7",
                "Claude Opus 4.7",
                "5",
                "25",
                "0.50",
                "6.25",
            ),
            // Claude 4.6 系列
            (
                "claude-opus-4-6-20260206",
                "Claude Opus 4.6",
                "5",
                "25",
                "0.50",
                "6.25",
            ),
            (
                "claude-sonnet-4-6-20260217",
                "Claude Sonnet 4.6",
                "3",
                "15",
                "0.30",
                "3.75",
            ),
            // Claude 4.5 系列
            (
                "claude-opus-4-5-20251101",
                "Claude Opus 4.5",
                "5",
                "25",
                "0.50",
                "6.25",
            ),
            (
                "claude-sonnet-4-5-20250929",
                "Claude Sonnet 4.5",
                "3",
                "15",
                "0.30",
                "3.75",
            ),
            (
                "claude-haiku-4-5-20251001",
                "Claude Haiku 4.5",
                "1",
                "5",
                "0.10",
                "1.25",
            ),
            // Claude 4 系列 (Legacy Models)
            (
                "claude-opus-4-20250514",
                "Claude Opus 4",
                "15",
                "75",
                "1.50",
                "18.75",
            ),
            (
                "claude-opus-4-1-20250805",
                "Claude Opus 4.1",
                "15",
                "75",
                "1.50",
                "18.75",
            ),
            (
                "claude-sonnet-4-20250514",
                "Claude Sonnet 4",
                "3",
                "15",
                "0.30",
                "3.75",
            ),
            // Claude 3.5 系列
            (
                "claude-3-5-haiku-20241022",
                "Claude 3.5 Haiku",
                "0.80",
                "4",
                "0.08",
                "1",
            ),
            (
                "claude-3-5-sonnet-20241022",
                "Claude 3.5 Sonnet",
                "3",
                "15",
                "0.30",
                "3.75",
            ),
            // GPT-5.5 系列
            // GPT-5.6：公开 API 价格。`gpt-5.6` 是 Sol 的别名；reasoning 后缀是
            // OpenSunstar/Codex 配置归一化键，而非额外的公开 API 模型声明。
            ("gpt-5.6", "GPT-5.6 Sol", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-sol", "GPT-5.6 Sol", "5", "30", "0.50", "6.25"),
            (
                "gpt-5.6-terra",
                "GPT-5.6 Terra",
                "2.50",
                "15",
                "0.25",
                "3.125",
            ),
            ("gpt-5.6-luna", "GPT-5.6 Luna", "1", "6", "0.10", "1.25"),
            ("gpt-5.6-low", "GPT-5.6 Sol", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-medium", "GPT-5.6 Sol", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-high", "GPT-5.6 Sol", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-xhigh", "GPT-5.6 Sol", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-minimal", "GPT-5.6 Sol", "5", "30", "0.50", "6.25"),
            ("gpt-5.5", "GPT-5.5", "5", "30", "0.50", "0"),
            ("gpt-5.5-low", "GPT-5.5", "5", "30", "0.50", "0"),
            ("gpt-5.5-medium", "GPT-5.5", "5", "30", "0.50", "0"),
            ("gpt-5.5-high", "GPT-5.5", "5", "30", "0.50", "0"),
            ("gpt-5.5-xhigh", "GPT-5.5", "5", "30", "0.50", "0"),
            ("gpt-5.5-minimal", "GPT-5.5", "5", "30", "0.50", "0"),
            // GPT-5.4 系列
            ("gpt-5.4", "GPT-5.4", "2.50", "15", "0.25", "0"),
            ("gpt-5.4-mini", "GPT-5.4 Mini", "0.75", "4.50", "0.075", "0"),
            ("gpt-5.4-nano", "GPT-5.4 Nano", "0.20", "1.25", "0.02", "0"),
            // GPT-5.2 系列
            ("gpt-5.2", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-low", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-medium", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-high", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-xhigh", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-codex", "GPT-5.2 Codex", "1.75", "14", "0.175", "0"),
            (
                "gpt-5.2-codex-low",
                "GPT-5.2 Codex",
                "1.75",
                "14",
                "0.175",
                "0",
            ),
            (
                "gpt-5.2-codex-medium",
                "GPT-5.2 Codex",
                "1.75",
                "14",
                "0.175",
                "0",
            ),
            (
                "gpt-5.2-codex-high",
                "GPT-5.2 Codex",
                "1.75",
                "14",
                "0.175",
                "0",
            ),
            (
                "gpt-5.2-codex-xhigh",
                "GPT-5.2 Codex",
                "1.75",
                "14",
                "0.175",
                "0",
            ),
            // GPT-5.3 Codex 系列
            ("gpt-5.3-codex", "GPT-5.3 Codex", "1.75", "14", "0.175", "0"),
            (
                "gpt-5.3-codex-low",
                "GPT-5.3 Codex",
                "1.75",
                "14",
                "0.175",
                "0",
            ),
            (
                "gpt-5.3-codex-medium",
                "GPT-5.3 Codex",
                "1.75",
                "14",
                "0.175",
                "0",
            ),
            (
                "gpt-5.3-codex-high",
                "GPT-5.3 Codex",
                "1.75",
                "14",
                "0.175",
                "0",
            ),
            (
                "gpt-5.3-codex-xhigh",
                "GPT-5.3 Codex",
                "1.75",
                "14",
                "0.175",
                "0",
            ),
            // GPT-5.1 系列
            ("gpt-5.1", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-low", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-medium", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-high", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-minimal", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-codex", "GPT-5.1 Codex", "1.25", "10", "0.125", "0"),
            (
                "gpt-5.1-codex-mini",
                "GPT-5.1 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            (
                "gpt-5.1-codex-max",
                "GPT-5.1 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            (
                "gpt-5.1-codex-max-high",
                "GPT-5.1 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            (
                "gpt-5.1-codex-max-xhigh",
                "GPT-5.1 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            // GPT-5 系列
            ("gpt-5", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-low", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-medium", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-high", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-minimal", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex-low", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            (
                "gpt-5-codex-medium",
                "GPT-5 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            (
                "gpt-5-codex-high",
                "GPT-5 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            (
                "gpt-5-codex-mini",
                "GPT-5 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            (
                "gpt-5-codex-mini-medium",
                "GPT-5 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            (
                "gpt-5-codex-mini-high",
                "GPT-5 Codex",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            // OpenAI Reasoning 系列
            ("o3", "OpenAI o3", "2", "8", "0.50", "0"),
            ("o4-mini", "OpenAI o4-mini", "1.10", "4.40", "0.275", "0"),
            // GPT-4.1 系列
            ("gpt-4.1", "GPT-4.1", "2", "8", "0.50", "0"),
            ("gpt-4.1-mini", "GPT-4.1 Mini", "0.40", "1.60", "0.10", "0"),
            ("gpt-4.1-nano", "GPT-4.1 Nano", "0.10", "0.40", "0.025", "0"),
            // Gemini 3.5 系列
            (
                "gemini-3.5-flash",
                "Gemini 3.5 Flash",
                "1.50",
                "9.00",
                "0.15",
                "0",
            ),
            // Gemini 3.1 系列
            (
                "gemini-3.1-pro-preview",
                "Gemini 3.1 Pro Preview",
                "2",
                "12",
                "0.20",
                "0",
            ),
            (
                "gemini-3.1-flash-lite",
                "Gemini 3.1 Flash Lite",
                "0.25",
                "1.50",
                "0.025",
                "0",
            ),
            (
                "gemini-3.1-flash-lite-preview",
                "Gemini 3.1 Flash Lite Preview",
                "0.25",
                "1.50",
                "0.025",
                "0",
            ),
            // Gemini 3 系列
            (
                "gemini-3-pro-preview",
                "Gemini 3 Pro Preview",
                "2",
                "12",
                "0.2",
                "0",
            ),
            (
                "gemini-3-flash-preview",
                "Gemini 3 Flash Preview",
                "0.5",
                "3",
                "0.05",
                "0",
            ),
            // Gemini 2.5 系列
            (
                "gemini-2.5-pro",
                "Gemini 2.5 Pro",
                "1.25",
                "10",
                "0.125",
                "0",
            ),
            (
                "gemini-2.5-flash",
                "Gemini 2.5 Flash",
                "0.3",
                "2.5",
                "0.03",
                "0",
            ),
            (
                "gemini-2.5-flash-lite",
                "Gemini 2.5 Flash Lite",
                "0.10",
                "0.40",
                "0.01",
                "0",
            ),
            // Gemini 2.0 系列
            (
                "gemini-2.0-flash",
                "Gemini 2.0 Flash",
                "0.10",
                "0.40",
                "0.025",
                "0",
            ),
            // StepFun 系列
            (
                "step-3.7-flash",
                "Step 3.7 Flash",
                "0.19",
                "1.13",
                "0.04",
                "0",
            ),
            (
                "step-3.5-flash",
                "Step 3.5 Flash",
                "0.10",
                "0.30",
                "0.02",
                "0",
            ),
            (
                "step-3.5-flash-2603",
                "Step 3.5 Flash 2603",
                "0.10",
                "0.30",
                "0.02",
                "0",
            ),
            // ====== 国产模型 (USD/1M tokens) ======
            // Doubao (字节跳动)
            (
                "doubao-seed-code",
                "Doubao Seed Code",
                "0.17",
                "1.11",
                "0.02",
                "0",
            ),
            (
                "doubao-seed-2-0-pro",
                "Doubao Seed 2.0 Pro",
                "0.47",
                "2.37",
                "0.09",
                "0",
            ),
            (
                "doubao-seed-2-0-code",
                "Doubao Seed 2.0 Code",
                "0.47",
                "2.37",
                "0.09",
                "0",
            ),
            (
                "doubao-seed-2-0-code-preview-latest",
                "Doubao Seed 2.0 Code Preview",
                "0.47",
                "2.37",
                "0.09",
                "0",
            ),
            (
                "doubao-seed-2-0-lite",
                "Doubao Seed 2.0 Lite",
                "0.08",
                "0.50",
                "0.017",
                "0",
            ),
            (
                "doubao-seed-2-0-mini",
                "Doubao Seed 2.0 Mini",
                "0.03",
                "0.31",
                "0.0056",
                "0",
            ),
            // DeepSeek 系列
            (
                "deepseek-v3.2",
                "DeepSeek V3.2",
                "0.28",
                "0.42",
                "0.028",
                "0",
            ),
            (
                "deepseek-v3.1",
                "DeepSeek V3.1",
                "0.55",
                "1.67",
                "0.055",
                "0",
            ),
            ("deepseek-v3", "DeepSeek V3", "0.28", "1.11", "0.028", "0"),
            (
                "deepseek-chat",
                "DeepSeek Chat",
                "0.27",
                "1.10",
                "0.07",
                "0",
            ),
            (
                "deepseek-reasoner",
                "DeepSeek Reasoner",
                "0.55",
                "2.19",
                "0.14",
                "0",
            ),
            // DeepSeek V4 系列（官方 CNY 按 1 USD ≈ 7.14 折算）
            (
                "deepseek-v4-flash",
                "DeepSeek V4 Flash",
                "0.14",
                "0.28",
                "0.0028",
                "0",
            ),
            (
                "deepseek-v4-pro",
                "DeepSeek V4 Pro",
                "0.435",
                "0.87",
                "0.003625",
                "0",
            ),
            // Kimi (月之暗面)
            (
                "kimi-k2-thinking",
                "Kimi K2 Thinking",
                "0.55",
                "2.20",
                "0.10",
                "0",
            ),
            ("kimi-k2-0905", "Kimi K2", "0.55", "2.20", "0.10", "0"),
            (
                "kimi-k2-turbo",
                "Kimi K2 Turbo",
                "1.11",
                "8.06",
                "0.14",
                "0",
            ),
            ("kimi-k2.5", "Kimi K2.5", "0.60", "3.00", "0.10", "0"),
            ("kimi-k2.6", "Kimi K2.6", "0.95", "4.00", "0.16", "0"),
            // MiniMax 系列
            ("minimax-m2.1", "MiniMax M2.1", "0.27", "0.95", "0.03", "0"),
            (
                "minimax-m2.1-lightning",
                "MiniMax M2.1 Lightning",
                "0.27",
                "2.33",
                "0.03",
                "0",
            ),
            ("minimax-m2", "MiniMax M2", "0.27", "0.95", "0.03", "0"),
            ("minimax-m2.5", "MiniMax M2.5", "0.15", "0.95", "0.03", "0"),
            (
                "minimax-m2.5-lightning",
                "MiniMax M2.5 Lightning",
                "0.30",
                "2.40",
                "0.03",
                "0",
            ),
            (
                "minimax-m2.7",
                "MiniMax M2.7",
                "0.30",
                "1.20",
                "0.06",
                "0.375",
            ),
            (
                "minimax-m2.7-highspeed",
                "MiniMax M2.7 Highspeed",
                "0.60",
                "2.40",
                "0.06",
                "0.375",
            ),
            ("minimax-m3", "MiniMax M3", "0.60", "2.40", "0.12", "0"),
            // GLM (智谱)
            ("glm-4.7", "GLM-4.7", "0.6", "2.2", "0.11", "0"),
            ("glm-4.6", "GLM-4.6", "0.6", "2.2", "0.11", "0"),
            ("glm-5", "GLM-5", "1", "3.2", "0.2", "0"),
            ("glm-5.1", "GLM-5.1", "1.4", "4.4", "0.26", "0"),
            // MiMo (小米)
            (
                "mimo-v2-flash",
                "MiMo V2 Flash",
                "0.09",
                "0.29",
                "0.009",
                "0",
            ),
            ("mimo-v2-pro", "MiMo V2 Pro", "0.435", "0.87", "0.0036", "0"),
            ("mimo-v2.5", "MiMo V2.5", "0.14", "0.29", "0.0028", "0"),
            (
                "mimo-v2.5-pro",
                "MiMo V2.5 Pro",
                "0.435",
                "0.87",
                "0.0036",
                "0",
            ),
            // Qwen 系列 (阿里巴巴)
            ("qwen3.7-max", "Qwen3.7 Max", "2.50", "7.50", "0.25", "0"),
            ("qwen3.7-plus", "Qwen3.7 Plus", "0.40", "1.60", "0.08", "0"),
            (
                "qwen3.6-plus",
                "Qwen3.6 Plus",
                "0.325",
                "1.95",
                "0.065",
                "0",
            ),
            ("qwen3.5-plus", "Qwen3.5 Plus", "0.26", "1.56", "0.052", "0"),
            ("qwen3-max", "Qwen3 Max", "0.78", "3.90", "0", "0"),
            (
                "qwen3-235b-a22b",
                "Qwen3 235B-A22B",
                "0.70",
                "8.40",
                "0",
                "0",
            ),
            (
                "qwen3-coder-plus",
                "Qwen3 Coder Plus",
                "0.65",
                "3.25",
                "0.13",
                "0",
            ),
            (
                "qwen3-coder-480b",
                "Qwen3 Coder 480B",
                "0.65",
                "3.25",
                "0",
                "0",
            ),
            (
                "qwen3-coder-480b-a35b-instruct",
                "Qwen3 Coder 480B-A35B Instruct",
                "0.65",
                "3.25",
                "0",
                "0",
            ),
            (
                "qwen3-coder-flash",
                "Qwen3 Coder Flash",
                "0.195",
                "0.975",
                "0.039",
                "0",
            ),
            (
                "qwen3-coder-next",
                "Qwen3 Coder Next",
                "0.12",
                "0.75",
                "0",
                "0",
            ),
            ("qwq-plus", "QwQ Plus", "0.80", "2.40", "0", "0"),
            ("qwq-32b", "QwQ 32B", "0.20", "0.60", "0", "0"),
            ("qwen3-32b", "Qwen3 32B", "0.16", "0.64", "0", "0"),
            // Grok 系列 (xAI)
            ("grok-4.3", "Grok 4.3", "1.25", "2.50", "0.20", "0"),
            (
                "grok-4.20-0309-reasoning",
                "Grok 4.20 Reasoning",
                "1.25",
                "2.50",
                "0.20",
                "0",
            ),
            (
                "grok-4.20-0309-non-reasoning",
                "Grok 4.20",
                "1.25",
                "2.50",
                "0.20",
                "0",
            ),
            (
                "grok-4-1-fast-reasoning",
                "Grok 4.1 Fast Reasoning",
                "0.20",
                "0.50",
                "0.05",
                "0",
            ),
            (
                "grok-4-1-fast-non-reasoning",
                "Grok 4.1 Fast",
                "0.20",
                "0.50",
                "0.05",
                "0",
            ),
            ("grok-4", "Grok 4", "3", "15", "0.75", "0"),
            (
                "grok-code-fast-1",
                "Grok Build 0.1 (Code Fast Alias)",
                "1",
                "2",
                "0.20",
                "0",
            ),
            ("grok-build-0.1", "Grok Build 0.1", "1", "2", "0.20", "0"),
            ("grok-3", "Grok 3", "3", "15", "0.75", "0"),
            ("grok-3-mini", "Grok 3 Mini", "0.25", "0.50", "0.075", "0"),
            // Mistral 系列
            (
                "mistral-medium-3.5",
                "Mistral Medium 3.5",
                "1.50",
                "7.50",
                "0",
                "0",
            ),
            (
                "mistral-small-4",
                "Mistral Small 4",
                "0.10",
                "0.30",
                "0.01",
                "0",
            ),
            (
                "devstral-small-2-2512",
                "Devstral Small 2",
                "0.10",
                "0.30",
                "0.01",
                "0",
            ),
            (
                "magistral-small",
                "Magistral Small",
                "0.50",
                "1.50",
                "0",
                "0",
            ),
            ("codestral-2508", "Codestral", "0.30", "0.90", "0.03", "0"),
            (
                "devstral-small-1.1",
                "Devstral Small 1.1",
                "0.07",
                "0.28",
                "0.01",
                "0",
            ),
            ("devstral-2-2512", "Devstral 2", "0.40", "2", "0.04", "0"),
            (
                "devstral-medium",
                "Devstral Medium",
                "0.40",
                "2",
                "0.04",
                "0",
            ),
            (
                "mistral-large-3-2512",
                "Mistral Large 3",
                "0.50",
                "1.50",
                "0.05",
                "0",
            ),
            (
                "mistral-medium-3.1",
                "Mistral Medium 3.1",
                "0.40",
                "2",
                "0.04",
                "0",
            ),
            (
                "mistral-small-3.2-24b",
                "Mistral Small 3.2",
                "0.075",
                "0.20",
                "0.01",
                "0",
            ),
            ("magistral-medium", "Magistral Medium", "2", "5", "0", "0"),
            // Cohere 系列
            ("command-a", "Cohere Command A", "2.50", "10", "0", "0"),
            (
                "command-r-plus",
                "Cohere Command R+",
                "2.50",
                "10",
                "0",
                "0",
            ),
            ("command-r", "Cohere Command R", "0.15", "0.60", "0", "0"),
            // OpenAI 补充
            ("o3-pro", "OpenAI o3-pro", "20", "80", "0", "0"),
            ("o3-mini", "OpenAI o3-mini", "0.55", "2.20", "0.55", "0"),
            ("o1", "OpenAI o1", "15", "60", "7.50", "0"),
            ("o1-mini", "OpenAI o1-mini", "0.55", "2.20", "0.55", "0"),
            ("codex-mini", "Codex Mini", "0.75", "3", "0.025", "0"),
            ("gpt-5-mini", "GPT-5 Mini", "0.25", "2", "0.025", "0"),
            ("gpt-5-nano", "GPT-5 Nano", "0.05", "0.40", "0.005", "0"),
        ];

        let mut stmt = conn
            .prepare(
                "INSERT OR IGNORE INTO model_pricing (
                    model_id, display_name, input_cost_per_million, output_cost_per_million,
                    cache_read_cost_per_million, cache_creation_cost_per_million
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| AppError::Database(format!("准备模型定价语句失败: {e}")))?;
        for (model_id, display_name, input, output, cache_read, cache_creation) in pricing_data {
            stmt.execute(rusqlite::params![
                model_id,
                display_name,
                input,
                output,
                cache_read,
                cache_creation
            ])
            .map_err(|e| AppError::Database(format!("插入模型定价失败: {e}")))?;
        }

        log::info!("已插入 {} 条默认模型定价数据", pricing_data.len());
        // 早期迁移也会调用本函数，当时来源表尚未创建；仅在表已存在时写入来源。
        if Self::table_exists(conn, "model_pricing_provenance")? {
            Self::seed_gpt_5_6_pricing_provenance(conn)?;
        }
        Ok(())
    }

    fn seed_gpt_5_6_pricing_provenance(conn: &Connection) -> Result<(), AppError> {
        for (model_id, input, output, cache_read, cache_creation) in [
            ("gpt-5.6", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-sol", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-terra", "2.50", "15", "0.25", "3.125"),
            ("gpt-5.6-luna", "1", "6", "0.10", "1.25"),
            ("gpt-5.6-low", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-medium", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-high", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-xhigh", "5", "30", "0.50", "6.25"),
            ("gpt-5.6-minimal", "5", "30", "0.50", "6.25"),
        ] {
            conn.execute(
                "INSERT OR IGNORE INTO model_pricing_provenance (
                    model_id, source, source_version, effective_at,
                    currency,
                    long_context_threshold_tokens,
                    long_context_input_multiplier, long_context_output_multiplier
                ) SELECT ?1, 'openai_public_api', '2026-07-09', '2026-07-09', 'USD', 272000, '2', '1.5'
                  WHERE EXISTS (
                    SELECT 1 FROM model_pricing
                    WHERE model_id = ?1
                      AND input_cost_per_million = ?2
                      AND output_cost_per_million = ?3
                      AND cache_read_cost_per_million = ?4
                      AND cache_creation_cost_per_million = ?5
                  )",
                params![model_id, input, output, cache_read, cache_creation],
            )
            .map_err(|e| AppError::Database(format!("写入 GPT-5.6 定价来源失败: {e}")))?;
        }
        Ok(())
    }

    fn repair_current_model_pricing(conn: &Connection) -> Result<(), AppError> {
        let pricing_fixes = [
            // 2026-06-10 全量核价（厂商官方 list 价；CNY 按 ~7.14 折算）
            // GLM 4.6/4.7：旧值是中转/OpenRouter 折扣价，统一到 Z.ai 官方（与 glm-5/5.1 一致）
            (
                "glm-4.7", "GLM-4.7", "0.6", "2.2", "0.11", "0", "0.39", "1.75", "0.04", "0",
            ),
            (
                "glm-4.6", "GLM-4.6", "0.6", "2.2", "0.11", "0", "0.28", "1.11", "0.03", "0",
            ),
            // Grok 4.20：xAI 已降价 2/6 → 1.25/2.50
            (
                "grok-4.20-0309-reasoning",
                "Grok 4.20 Reasoning",
                "1.25",
                "2.50",
                "0.20",
                "0",
                "2",
                "6",
                "0.20",
                "0",
            ),
            (
                "grok-4.20-0309-non-reasoning",
                "Grok 4.20",
                "1.25",
                "2.50",
                "0.20",
                "0",
                "2",
                "6",
                "0.20",
                "0",
            ),
            // Kimi K2.5 官方 output 3.00
            (
                "kimi-k2.5",
                "Kimi K2.5",
                "0.60",
                "3.00",
                "0.10",
                "0",
                "0.60",
                "2.50",
                "0.10",
                "0",
            ),
            // MiniMax M2.5 input 0.15
            (
                "minimax-m2.5",
                "MiniMax M2.5",
                "0.15",
                "0.95",
                "0.03",
                "0",
                "0.12",
                "0.95",
                "0.03",
                "0",
            ),
            // Mistral Devstral 2 output 0.90 → 2（与同表 devstral-medium 一致）
            (
                "devstral-2-2512",
                "Devstral 2",
                "0.40",
                "2",
                "0.04",
                "0",
                "0.40",
                "0.90",
                "0.04",
                "0",
            ),
            // Doubao Seed 2.0：lite 旧价贵 3-4 倍 + 全系补 cache 命中价
            (
                "doubao-seed-2-0-lite",
                "Doubao Seed 2.0 Lite",
                "0.08",
                "0.50",
                "0.017",
                "0",
                "0.25",
                "2",
                "0",
                "0",
            ),
            (
                "doubao-seed-2-0-pro",
                "Doubao Seed 2.0 Pro",
                "0.47",
                "2.37",
                "0.09",
                "0",
                "0.47",
                "2.37",
                "0",
                "0",
            ),
            (
                "doubao-seed-2-0-code",
                "Doubao Seed 2.0 Code",
                "0.47",
                "2.37",
                "0.09",
                "0",
                "0.47",
                "2.37",
                "0",
                "0",
            ),
            (
                "doubao-seed-2-0-code-preview-latest",
                "Doubao Seed 2.0 Code Preview",
                "0.47",
                "2.37",
                "0.09",
                "0",
                "0.47",
                "2.37",
                "0",
                "0",
            ),
            (
                "doubao-seed-2-0-mini",
                "Doubao Seed 2.0 Mini",
                "0.03",
                "0.31",
                "0.0056",
                "0",
                "0.03",
                "0.31",
                "0",
                "0",
            ),
            // MiMo：5/27 永久降价，旧值是旧价
            (
                "mimo-v2-pro",
                "MiMo V2 Pro",
                "0.435",
                "0.87",
                "0.0036",
                "0",
                "1",
                "3",
                "0",
                "0",
            ),
            (
                "mimo-v2.5",
                "MiMo V2.5",
                "0.14",
                "0.29",
                "0.0028",
                "0",
                "0.09",
                "0.29",
                "0.009",
                "0",
            ),
            (
                "mimo-v2.5-pro",
                "MiMo V2.5 Pro",
                "0.435",
                "0.87",
                "0.0036",
                "0",
                "1",
                "3",
                "0",
                "0",
            ),
            // Qwen：官方"隐式缓存 = 输入 20%"补 cache 命中价
            (
                "qwen3.6-plus",
                "Qwen3.6 Plus",
                "0.325",
                "1.95",
                "0.065",
                "0",
                "0.325",
                "1.95",
                "0",
                "0",
            ),
            (
                "qwen3.5-plus",
                "Qwen3.5 Plus",
                "0.26",
                "1.56",
                "0.052",
                "0",
                "0.26",
                "1.56",
                "0",
                "0",
            ),
            (
                "qwen3-coder-plus",
                "Qwen3 Coder Plus",
                "0.65",
                "3.25",
                "0.13",
                "0",
                "0.65",
                "3.25",
                "0",
                "0",
            ),
            (
                "qwen3-coder-flash",
                "Qwen3 Coder Flash",
                "0.195",
                "0.975",
                "0.039",
                "0",
                "0.195",
                "0.975",
                "0",
                "0",
            ),
            (
                "deepseek-v4-flash",
                "DeepSeek V4 Flash",
                "0.14",
                "0.28",
                "0.0028",
                "0",
                "0.14",
                "0.28",
                "0.028",
                "0",
            ),
            (
                "deepseek-v4-pro",
                "DeepSeek V4 Pro",
                "0.435",
                "0.87",
                "0.003625",
                "0",
                "1.68",
                "3.36",
                "0.14",
                "0",
            ),
            (
                "glm-5", "GLM-5", "1", "3.2", "0.2", "0", "0.72", "2.30", "0", "0",
            ),
            (
                "glm-5.1", "GLM-5.1", "1.4", "4.4", "0.26", "0", "0.95", "3.15", "0", "0",
            ),
            (
                "grok-code-fast-1",
                "Grok Build 0.1 (Code Fast Alias)",
                "1",
                "2",
                "0.20",
                "0",
                "0.20",
                "1.50",
                "0.02",
                "0",
            ),
        ];

        for (
            model_id,
            display_name,
            input,
            output,
            cache_read,
            cache_creation,
            old_input,
            old_output,
            old_cache_read,
            old_cache_creation,
        ) in pricing_fixes
        {
            conn.execute(
                "UPDATE model_pricing SET
                    display_name = ?2,
                    input_cost_per_million = ?3,
                    output_cost_per_million = ?4,
                    cache_read_cost_per_million = ?5,
                    cache_creation_cost_per_million = ?6
                 WHERE model_id = ?1
                   AND input_cost_per_million = ?7
                   AND output_cost_per_million = ?8
                   AND cache_read_cost_per_million = ?9
                   AND cache_creation_cost_per_million = ?10",
                rusqlite::params![
                    model_id,
                    display_name,
                    input,
                    output,
                    cache_read,
                    cache_creation,
                    old_input,
                    old_output,
                    old_cache_read,
                    old_cache_creation
                ],
            )
            .map_err(|e| AppError::Database(format!("修复模型 {model_id} 定价失败: {e}")))?;
        }

        Ok(())
    }

    /// 确保模型定价表具备默认数据
    pub fn ensure_model_pricing_seeded(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::ensure_model_pricing_seeded_on_conn(&conn)
    }

    fn ensure_model_pricing_seeded_on_conn(conn: &Connection) -> Result<(), AppError> {
        // 每次启动都执行 INSERT OR IGNORE，增量追加新模型；仅修复仍等于旧内置值的定价。
        Self::seed_model_pricing(conn)?;
        Self::repair_current_model_pricing(conn)
    }

    // --- 辅助方法 ---

    pub(crate) fn get_user_version(conn: &Connection) -> Result<i32, AppError> {
        conn.query_row("PRAGMA user_version;", [], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("读取 user_version 失败: {e}")))
    }

    pub(crate) fn set_user_version(conn: &Connection, version: i32) -> Result<(), AppError> {
        if version < 0 {
            return Err(AppError::Database("user_version 不能为负数".to_string()));
        }
        let sql = format!("PRAGMA user_version = {version};");
        conn.execute(&sql, [])
            .map_err(|e| AppError::Database(format!("写入 user_version 失败: {e}")))?;
        Ok(())
    }

    fn create_request_logs_usage_indexes_if_supported(conn: &Connection) -> Result<(), AppError> {
        if !Self::table_exists(conn, "proxy_request_logs")? {
            return Ok(());
        }

        let has_app_type = Self::has_column(conn, "proxy_request_logs", "app_type")?;
        let has_created_at = Self::has_column(conn, "proxy_request_logs", "created_at")?;
        if has_app_type && has_created_at {
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_request_logs_app_created_at
                 ON proxy_request_logs(app_type, created_at DESC)",
                [],
            )
            .map_err(|e| AppError::Database(format!("创建使用量应用时间索引失败: {e}")))?;
        }

        let required_columns = [
            "app_type",
            "data_source",
            "input_tokens",
            "output_tokens",
            "cache_read_tokens",
            "created_at",
            "cache_creation_tokens",
        ];
        for column in required_columns {
            if !Self::has_column(conn, "proxy_request_logs", column)? {
                return Ok(());
            }
        }

        conn.execute("DROP INDEX IF EXISTS idx_request_logs_dedup_lookup", [])
            .map_err(|e| AppError::Database(format!("删除旧使用量去重索引失败: {e}")))?;

        // 查询层为了兼容历史 NULL data_source 行，会使用
        // COALESCE(data_source, 'proxy')。普通 data_source 索引无法匹配该表达式，
        // 会让跨源去重子查询退化成大量扫描；表达式索引让 SQLite 能按同一表达式查找。
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_dedup_lookup_expr
             ON proxy_request_logs(app_type, COALESCE(data_source, 'proxy'), input_tokens,
                                   output_tokens, cache_read_tokens, created_at,
                                   cache_creation_tokens)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建使用量去重表达式索引失败: {e}")))?;
        Ok(())
    }

    fn validate_identifier(s: &str, kind: &str) -> Result<(), AppError> {
        if s.is_empty() {
            return Err(AppError::Database(format!("{kind} 不能为空")));
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AppError::Database(format!(
                "非法{kind}: {s}，仅允许字母、数字和下划线"
            )));
        }
        Ok(())
    }

    pub(crate) fn table_exists(conn: &Connection, table: &str) -> Result<bool, AppError> {
        Self::validate_identifier(table, "表名")?;

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .map_err(|e| AppError::Database(format!("读取表名失败: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("查询表名失败: {e}")))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let name: String = row
                .get(0)
                .map_err(|e| AppError::Database(format!("解析表名失败: {e}")))?;
            if name.eq_ignore_ascii_case(table) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn has_column(
        conn: &Connection,
        table: &str,
        column: &str,
    ) -> Result<bool, AppError> {
        Self::validate_identifier(table, "表名")?;
        Self::validate_identifier(column, "列名")?;

        let sql = format!("PRAGMA table_info(\"{table}\");");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(format!("读取表结构失败: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("查询表结构失败: {e}")))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let name: String = row
                .get(1)
                .map_err(|e| AppError::Database(format!("读取列名失败: {e}")))?;
            if name.eq_ignore_ascii_case(column) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 30. SDD 框架描述符目录（7 个方法论框架的元数据）
    fn create_sdd_descriptors_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sdd_descriptors (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                version         TEXT NOT NULL,
                phase_model     TEXT NOT NULL DEFAULT 'linear',
                probe_signals   TEXT NOT NULL DEFAULT '[]',
                install_type    TEXT NOT NULL DEFAULT 'file_copy',
                risk_tier       TEXT NOT NULL DEFAULT 'read_only',
                description_zh  TEXT,
                description_en  TEXT,
                repo_url        TEXT,
                star_count      INTEGER,
                last_verified   TEXT,
                descriptor_json TEXT NOT NULL DEFAULT '{}'
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 sdd_descriptors 表失败: {e}")))?;
        Ok(())
    }

    /// 31. 项目 × SDD 框架探测结果（只读探测，每项目每框架一行）
    fn create_project_sdd_detections_table(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_sdd_detections (
                id              TEXT PRIMARY KEY,
                project_id      TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                descriptor_id   TEXT NOT NULL REFERENCES sdd_descriptors(id),
                detected        INTEGER NOT NULL DEFAULT 0,
                confidence      TEXT NOT NULL DEFAULT 'absent',
                signal_matches  TEXT,
                detected_at     TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(project_id, descriptor_id)
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 project_sdd_detections 表失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sdd_detections_project
             ON project_sdd_detections(project_id)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_sdd_detections_project 失败: {e}")))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sdd_detections_descriptor
             ON project_sdd_detections(descriptor_id)",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 idx_sdd_detections_descriptor 失败: {e}")))?;
        Ok(())
    }

    fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<bool, AppError> {
        Self::validate_identifier(table, "表名")?;
        Self::validate_identifier(column, "列名")?;

        if !Self::table_exists(conn, table)? {
            return Err(AppError::Database(format!(
                "表 {table} 不存在，无法添加列 {column}"
            )));
        }
        if Self::has_column(conn, table, column)? {
            return Ok(false);
        }

        let sql = format!("ALTER TABLE \"{table}\" ADD COLUMN \"{column}\" {definition};");
        conn.execute(&sql, [])
            .map_err(|e| AppError::Database(format!("为表 {table} 添加列 {column} 失败: {e}")))?;
        log::info!("已为表 {table} 添加缺失列 {column}");
        Ok(true)
    }
}
