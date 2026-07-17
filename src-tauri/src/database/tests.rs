//! 数据库模块测试
//!
//! 包含 Schema 迁移和基本功能的测试。

use super::*;

#[test]
fn v32_to_current_encrypts_existing_proxy_live_backups() {
    let db = Database::memory().expect("database");
    {
        let conn = db.conn.lock().expect("lock");
        conn.execute(
            "INSERT OR REPLACE INTO proxy_live_backup (app_type, original_config, backed_up_at)
             VALUES ('codex', ?1, datetime('now'))",
            [r#"{"auth":{"OPENAI_API_KEY":"legacy-plaintext-secret","tokens":{"access_token":"legacy-oauth-secret"}},"config":"model_provider = \"custom\"\n\n[model_providers.custom]\nbase_url = \"https://example.test/v1\"\nwire_api = \"responses\"\n"}"#],
        )
        .expect("insert legacy backup");
        Database::set_user_version(&conn, 32).expect("set v32");
    }

    db.apply_schema_migrations().expect("migrate to current");

    let raw: String = db
        .conn
        .lock()
        .expect("lock")
        .query_row(
            "SELECT original_config FROM proxy_live_backup WHERE app_type = 'codex'",
            [],
            |row| row.get(0),
        )
        .expect("read backup");
    assert!(raw.starts_with("enc:v1:"));
    assert!(!raw.contains("legacy-plaintext-secret"));
    assert!(!raw.contains("legacy-oauth-secret"));

    let restored = futures::executor::block_on(db.get_live_backup("codex"))
        .expect("decrypt migrated backup")
        .expect("migrated backup exists");
    let restored: serde_json::Value =
        serde_json::from_str(&restored.original_config).expect("parse migrated backup");
    assert_eq!(restored.get("auth"), Some(&json!({})));
    assert!(!restored.to_string().contains("legacy-oauth-secret"));
}

#[test]
fn v36_to_current_scrubs_codex_credentials_from_all_persisted_snapshots() {
    let db = Database::memory().expect("database");
    let legacy_config =
        "model_provider = \"custom\"\n\n[model_providers.custom]\nbase_url = \"https://example.test/v1\"\nwire_api = \"responses\"\n";
    let legacy_settings = json!({
        "auth": {
            "OPENAI_API_KEY": "third-party-key",
            "tokens": {
                "access_token": "oauth-access",
                "refresh_token": "oauth-refresh"
            }
        },
        "config": legacy_config
    });
    let official_settings = json!({
        "auth": {
            "OPENAI_API_KEY": "official-api-key",
            "tokens": {"access_token": "official-oauth-access"}
        },
        "config": ""
    });
    let backup = crate::keychain::seal_local_secret(&legacy_settings.to_string())
        .expect("seal legacy backup");
    let quick_start_snapshot = crate::keychain::seal_local_secret(
        &json!({
            "Provider": {
                "Codex": {
                    "auth": {"tokens": {"access_token": "quick-start-oauth"}},
                    "config": legacy_config,
                    "model_catalog": null
                }
            }
        })
        .to_string(),
    )
    .expect("seal QuickStart snapshot");

    {
        let conn = db.conn.lock().expect("lock");
        conn.execute(
            "INSERT OR REPLACE INTO providers
             (id, app_type, name, settings_config, category, meta)
             VALUES ('legacy-custom', 'codex', 'Legacy Custom', ?1, 'custom', '{}')",
            [legacy_settings.to_string()],
        )
        .expect("insert custom provider");
        conn.execute(
            "INSERT OR REPLACE INTO providers
             (id, app_type, name, settings_config, category, meta)
             VALUES ('legacy-official', 'codex', 'Legacy Official', ?1, 'official', '{}')",
            [official_settings.to_string()],
        )
        .expect("insert official provider");
        conn.execute(
            "INSERT OR REPLACE INTO proxy_live_backup
             (app_type, original_config, backed_up_at)
             VALUES ('codex', ?1, datetime('now'))",
            [backup],
        )
        .expect("insert encrypted backup");
        conn.execute(
            "INSERT INTO quick_start_operations
             (id, idempotency_key, request_fingerprint, app_type, status, current_step,
              created_at, updated_at, live_snapshot)
             VALUES ('legacy-quick-start', 'legacy-quick-start-key', 'sha256:test', 'codex',
                     'succeeded', 'done', datetime('now'), datetime('now'), ?1)",
            [quick_start_snapshot],
        )
        .expect("insert QuickStart snapshot");
        Database::set_user_version(&conn, 36).expect("set v36");
    }

    db.apply_schema_migrations().expect("migrate to current");

    let custom = db
        .get_provider_by_id("legacy-custom", "codex")
        .expect("read custom provider")
        .expect("custom provider exists");
    assert_eq!(
        custom.settings_config.get("auth"),
        Some(&json!({"OPENAI_API_KEY": "third-party-key"}))
    );
    assert!(!custom.settings_config.to_string().contains("oauth-access"));
    assert!(!custom.settings_config.to_string().contains("oauth-refresh"));

    let official = db
        .get_provider_by_id("legacy-official", "codex")
        .expect("read official provider")
        .expect("official provider exists");
    assert_eq!(official.settings_config.get("auth"), Some(&json!({})));
    assert!(!official
        .settings_config
        .to_string()
        .contains("official-api-key"));
    assert!(!official
        .settings_config
        .to_string()
        .contains("official-oauth-access"));

    let backup = futures::executor::block_on(db.get_live_backup("codex"))
        .expect("read migrated backup")
        .expect("backup exists");
    let backup: serde_json::Value =
        serde_json::from_str(&backup.original_config).expect("parse migrated backup");
    assert_eq!(backup.get("auth"), Some(&json!({})));
    assert!(!backup.to_string().contains("oauth-access"));
    assert!(!backup.to_string().contains("oauth-refresh"));

    let sealed_snapshot: String = db
        .conn
        .lock()
        .expect("lock")
        .query_row(
            "SELECT live_snapshot FROM quick_start_operations WHERE id = 'legacy-quick-start'",
            [],
            |row| row.get(0),
        )
        .expect("read migrated QuickStart snapshot");
    let snapshot = crate::keychain::open_local_secret(&sealed_snapshot)
        .expect("open migrated QuickStart snapshot");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot).expect("parse migrated QuickStart snapshot");
    assert!(snapshot
        .pointer("/Provider/Codex/auth")
        .is_some_and(|v| v.is_null()));
    assert!(!snapshot.to_string().contains("quick-start-oauth"));
}
use crate::app_config::MultiAppConfig;
use crate::provider::{Provider, ProviderManager};
use indexmap::IndexMap;
use rusqlite::{params, Connection};
use serde_json::json;
use std::collections::HashMap;
use tempfile::NamedTempFile;

const LEGACY_SCHEMA_SQL: &str = r#"
    CREATE TABLE providers (
        id TEXT NOT NULL,
        app_type TEXT NOT NULL,
        name TEXT NOT NULL,
        settings_config TEXT NOT NULL,
        PRIMARY KEY (id, app_type)
    );
    CREATE TABLE provider_endpoints (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        provider_id TEXT NOT NULL,
        app_type TEXT NOT NULL,
        url TEXT NOT NULL
    );
    CREATE TABLE mcp_servers (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        server_config TEXT NOT NULL
    );
    CREATE TABLE prompts (
        id TEXT NOT NULL,
        app_type TEXT NOT NULL,
        name TEXT NOT NULL,
        content TEXT NOT NULL,
        PRIMARY KEY (id, app_type)
    );
    CREATE TABLE skills (
        key TEXT PRIMARY KEY,
        installed BOOLEAN NOT NULL DEFAULT 0
    );
    CREATE TABLE skill_repos (
        owner TEXT NOT NULL,
        name TEXT NOT NULL,
        PRIMARY KEY (owner, name)
    );
    CREATE TABLE settings (
        key TEXT PRIMARY KEY,
        value TEXT
    );
"#;

// v3.8.x（schema v1）的真实表结构快照：用于验证从 v3.8.* 升级到当前版本的迁移链路
// 参考：tag v3.8.3 的 src-tauri/src/database/schema.rs
const V3_8_SCHEMA_V1_SQL: &str = r#"
    CREATE TABLE providers (
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
        PRIMARY KEY (id, app_type)
    );
    CREATE TABLE provider_endpoints (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        provider_id TEXT NOT NULL,
        app_type TEXT NOT NULL,
        url TEXT NOT NULL,
        added_at INTEGER,
        FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
    );
    CREATE TABLE mcp_servers (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        server_config TEXT NOT NULL,
        description TEXT,
        homepage TEXT,
        docs TEXT,
        tags TEXT NOT NULL DEFAULT '[]',
        enabled_claude BOOLEAN NOT NULL DEFAULT 0,
        enabled_codex BOOLEAN NOT NULL DEFAULT 0,
        enabled_gemini BOOLEAN NOT NULL DEFAULT 0
    );
    CREATE TABLE prompts (
        id TEXT NOT NULL,
        app_type TEXT NOT NULL,
        name TEXT NOT NULL,
        content TEXT NOT NULL,
        description TEXT,
        enabled BOOLEAN NOT NULL DEFAULT 1,
        created_at INTEGER,
        updated_at INTEGER,
        PRIMARY KEY (id, app_type)
    );
    CREATE TABLE skills (
        key TEXT PRIMARY KEY,
        installed BOOLEAN NOT NULL DEFAULT 0,
        installed_at INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE skill_repos (
        owner TEXT NOT NULL,
        name TEXT NOT NULL,
        branch TEXT NOT NULL DEFAULT 'main',
        enabled BOOLEAN NOT NULL DEFAULT 1,
        PRIMARY KEY (owner, name)
    );
    CREATE TABLE settings (
        key TEXT PRIMARY KEY,
        value TEXT
    );
"#;

#[derive(Debug)]
struct ColumnInfo {
    r#type: String,
    notnull: i64,
    default: Option<String>,
}

fn get_column_info(conn: &Connection, table: &str, column: &str) -> ColumnInfo {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info(\"{table}\");"))
        .expect("prepare pragma");
    let mut rows = stmt.query([]).expect("query pragma");
    while let Some(row) = rows.next().expect("read row") {
        let column_name: String = row.get(1).expect("name");
        if column_name.eq_ignore_ascii_case(column) {
            return ColumnInfo {
                r#type: row.get::<_, String>(2).expect("type"),
                notnull: row.get::<_, i64>(3).expect("notnull"),
                default: row.get::<_, Option<String>>(4).ok().flatten(),
            };
        }
    }
    panic!("column {table}.{column} not found");
}

fn normalize_default(default: &Option<String>) -> Option<String> {
    default
        .as_ref()
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
}

#[test]
fn schema_migration_sets_user_version_when_missing() {
    let conn = Connection::open_in_memory().expect("open memory db");

    Database::create_tables_on_conn(&conn).expect("create tables");
    assert_eq!(
        Database::get_user_version(&conn).expect("read version before"),
        0
    );

    Database::apply_schema_migrations_on_conn(&conn).expect("apply migration");

    assert_eq!(
        Database::get_user_version(&conn).expect("read version after"),
        SCHEMA_VERSION
    );
}

#[test]
fn schema_migration_rejects_future_version() {
    let conn = Connection::open_in_memory().expect("open memory db");
    Database::create_tables_on_conn(&conn).expect("create tables");
    Database::set_user_version(&conn, SCHEMA_VERSION + 1).expect("set future version");

    let err =
        Database::apply_schema_migrations_on_conn(&conn).expect_err("should reject higher version");
    assert!(
        err.to_string().contains("数据库版本过新"),
        "unexpected error: {err}"
    );
}

#[test]
fn schema_migration_adds_missing_columns_for_providers() {
    let conn = Connection::open_in_memory().expect("open memory db");

    // 创建旧版 providers 表，缺少新增列
    conn.execute_batch(LEGACY_SCHEMA_SQL)
        .expect("seed old schema");

    Database::apply_schema_migrations_on_conn(&conn).expect("apply migrations");

    // 验证关键新增列已补齐
    for (table, column) in [
        ("providers", "meta"),
        ("providers", "is_current"),
        ("provider_endpoints", "added_at"),
        ("mcp_servers", "enabled_gemini"),
        ("prompts", "updated_at"),
        ("skills", "installed_at"),
        ("skill_repos", "enabled"),
    ] {
        assert!(
            Database::has_column(&conn, table, column).expect("check column"),
            "{table}.{column} should exist after migration"
        );
    }

    // 验证 meta 列约束保持一致
    let meta = get_column_info(&conn, "providers", "meta");
    assert_eq!(meta.notnull, 1, "meta should be NOT NULL");
    assert_eq!(
        normalize_default(&meta.default).as_deref(),
        Some("{}"),
        "meta default should be '{{}}'"
    );

    assert_eq!(
        Database::get_user_version(&conn).expect("version after migration"),
        SCHEMA_VERSION
    );
}

#[test]
fn schema_migration_aligns_column_defaults_and_types() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute_batch(LEGACY_SCHEMA_SQL)
        .expect("seed old schema");

    Database::apply_schema_migrations_on_conn(&conn).expect("apply migrations");

    let is_current = get_column_info(&conn, "providers", "is_current");
    assert_eq!(is_current.r#type, "BOOLEAN");
    assert_eq!(is_current.notnull, 1);
    assert_eq!(normalize_default(&is_current.default).as_deref(), Some("0"));

    let tags = get_column_info(&conn, "mcp_servers", "tags");
    assert_eq!(tags.r#type, "TEXT");
    assert_eq!(tags.notnull, 1);
    assert_eq!(normalize_default(&tags.default).as_deref(), Some("[]"));

    let enabled = get_column_info(&conn, "prompts", "enabled");
    assert_eq!(enabled.r#type, "BOOLEAN");
    assert_eq!(enabled.notnull, 1);
    assert_eq!(normalize_default(&enabled.default).as_deref(), Some("1"));

    let installed_at = get_column_info(&conn, "skills", "installed_at");
    assert_eq!(installed_at.r#type, "INTEGER");
    assert_eq!(installed_at.notnull, 1);
    assert_eq!(
        normalize_default(&installed_at.default).as_deref(),
        Some("0")
    );

    let branch = get_column_info(&conn, "skill_repos", "branch");
    assert_eq!(branch.r#type, "TEXT");
    assert_eq!(normalize_default(&branch.default).as_deref(), Some("main"));

    let skill_repo_enabled = get_column_info(&conn, "skill_repos", "enabled");
    assert_eq!(skill_repo_enabled.r#type, "BOOLEAN");
    assert_eq!(skill_repo_enabled.notnull, 1);
    assert_eq!(
        normalize_default(&skill_repo_enabled.default).as_deref(),
        Some("1")
    );
}

#[test]
fn schema_create_tables_include_pricing_model_columns() {
    let conn = Connection::open_in_memory().expect("open memory db");
    Database::create_tables_on_conn(&conn).expect("create tables");

    let multiplier = get_column_info(&conn, "proxy_config", "default_cost_multiplier");
    assert_eq!(multiplier.r#type, "TEXT");
    assert_eq!(multiplier.notnull, 1);
    assert_eq!(normalize_default(&multiplier.default).as_deref(), Some("1"));

    let pricing_source = get_column_info(&conn, "proxy_config", "pricing_model_source");
    assert_eq!(pricing_source.r#type, "TEXT");
    assert_eq!(pricing_source.notnull, 1);
    assert_eq!(
        normalize_default(&pricing_source.default).as_deref(),
        Some("response")
    );

    let request_model = get_column_info(&conn, "proxy_request_logs", "request_model");
    assert_eq!(request_model.r#type, "TEXT");
    assert_eq!(request_model.notnull, 0);
}

#[test]
fn schema_migration_v4_adds_pricing_model_columns() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute_batch(
        r#"
        CREATE TABLE providers (
            id TEXT NOT NULL,
            app_type TEXT NOT NULL,
            name TEXT NOT NULL,
            settings_config TEXT NOT NULL DEFAULT '{}',
            meta TEXT NOT NULL DEFAULT '{}',
            PRIMARY KEY (id, app_type)
        );
        CREATE TABLE proxy_config (app_type TEXT PRIMARY KEY);
        CREATE TABLE proxy_request_logs (request_id TEXT PRIMARY KEY, model TEXT NOT NULL);
        CREATE TABLE mcp_servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            server_config TEXT NOT NULL,
            enabled_claude INTEGER NOT NULL DEFAULT 0,
            enabled_codex INTEGER NOT NULL DEFAULT 0,
            enabled_gemini INTEGER NOT NULL DEFAULT 0,
            enabled_opencode INTEGER NOT NULL DEFAULT 0
        );
        "#,
    )
    .expect("seed v4 schema");

    Database::set_user_version(&conn, 4).expect("set user_version=4");
    Database::apply_schema_migrations_on_conn(&conn).expect("apply migrations");

    let multiplier = get_column_info(&conn, "proxy_config", "default_cost_multiplier");
    assert_eq!(multiplier.r#type, "TEXT");
    assert_eq!(multiplier.notnull, 1);
    assert_eq!(normalize_default(&multiplier.default).as_deref(), Some("1"));

    let pricing_source = get_column_info(&conn, "proxy_config", "pricing_model_source");
    assert_eq!(pricing_source.r#type, "TEXT");
    assert_eq!(pricing_source.notnull, 1);
    assert_eq!(
        normalize_default(&pricing_source.default).as_deref(),
        Some("response")
    );

    let request_model = get_column_info(&conn, "proxy_request_logs", "request_model");
    assert_eq!(request_model.r#type, "TEXT");
    assert_eq!(request_model.notnull, 0);

    assert_eq!(
        Database::get_user_version(&conn).expect("version after migration"),
        SCHEMA_VERSION
    );
}

#[test]
fn migration_v10_to_v11_rebuilds_rollups_with_request_model_dimension() {
    let conn = Connection::open_in_memory().expect("open memory db");

    // 模拟 v10 形状的 rollup 表（主键不含 request_model）+ 一行历史聚合数据，
    // 以及 v10 形状的明细表（无 pricing_model 列）
    conn.execute_batch(
        r#"
        CREATE TABLE proxy_request_logs (
            request_id TEXT PRIMARY KEY,
            model TEXT NOT NULL,
            request_model TEXT
        );
        CREATE TABLE usage_daily_rollups (
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
        );
        INSERT INTO usage_daily_rollups
            (date, app_type, provider_id, model, request_count, success_count,
             input_tokens, output_tokens, total_cost_usd, avg_latency_ms)
        VALUES ('2026-05-01', 'claude', 'p1', 'kimi-k2', 7, 7, 1000, 500, '0.07', 120);
        "#,
    )
    .expect("seed v10 rollup table");

    Database::set_user_version(&conn, 10).expect("set user_version=10");
    Database::apply_schema_migrations_on_conn(&conn).expect("apply migrations");

    // 新列存在且 NOT NULL DEFAULT ''
    let request_model = get_column_info(&conn, "usage_daily_rollups", "request_model");
    assert_eq!(request_model.r#type, "TEXT");
    assert_eq!(request_model.notnull, 1);
    let rollup_pricing_model = get_column_info(&conn, "usage_daily_rollups", "pricing_model");
    assert_eq!(rollup_pricing_model.r#type, "TEXT");
    assert_eq!(rollup_pricing_model.notnull, 1);

    // 明细表补上 pricing_model 列（可空，历史行 NULL）
    let pricing_model = get_column_info(&conn, "proxy_request_logs", "pricing_model");
    assert_eq!(pricing_model.r#type, "TEXT");
    assert_eq!(pricing_model.notnull, 0);

    // 历史行保留，request_model 填 ''（未知）
    let (rm, count, input, cost): (String, i64, i64, String) = conn
        .query_row(
            "SELECT request_model, request_count, input_tokens, total_cost_usd
             FROM usage_daily_rollups WHERE model = 'kimi-k2'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("migrated row");
    assert_eq!(rm, "");
    assert_eq!(count, 7);
    assert_eq!(input, 1000);
    assert_eq!(cost, "0.07");

    // 主键包含 request_model：同 model 不同别名可共存
    conn.execute(
        "INSERT INTO usage_daily_rollups
            (date, app_type, provider_id, model, request_model, request_count)
         VALUES ('2026-05-01', 'claude', 'p1', 'kimi-k2', 'claude-sonnet-4-6', 1)",
        [],
    )
    .expect("insert row with same model but different request_model");

    assert_eq!(
        Database::get_user_version(&conn).expect("version after migration"),
        SCHEMA_VERSION
    );
}

#[test]
fn schema_create_tables_repairs_legacy_proxy_config_singleton_to_per_app() {
    let conn = Connection::open_in_memory().expect("open memory db");

    // 模拟测试版 v2：user_version=2，但 proxy_config 仍是单例结构（无 app_type）
    Database::set_user_version(&conn, 2).expect("set user_version");
    conn.execute_batch(
        r#"
        CREATE TABLE proxy_config (
            id INTEGER PRIMARY KEY,
            enabled INTEGER NOT NULL DEFAULT 0,
            listen_address TEXT NOT NULL DEFAULT '127.0.0.1',
            listen_port INTEGER NOT NULL DEFAULT 5000,
            max_retries INTEGER NOT NULL DEFAULT 3,
            request_timeout INTEGER NOT NULL DEFAULT 300,
            enable_logging INTEGER NOT NULL DEFAULT 1,
            target_app TEXT NOT NULL DEFAULT 'claude',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO proxy_config (id, enabled) VALUES (1, 1);
        "#,
    )
    .expect("seed legacy proxy_config");

    Database::create_tables_on_conn(&conn).expect("create tables should repair proxy_config");

    assert!(
        Database::has_column(&conn, "proxy_config", "app_type").expect("check app_type"),
        "proxy_config should be migrated to per-app structure"
    );

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM proxy_config", [], |r| r.get(0))
        .expect("count rows");
    assert_eq!(count, 3, "per-app proxy_config should have 3 rows");

    // 新结构下应能按 app_type 查询
    let _: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM proxy_config WHERE app_type = 'claude'",
            [],
            |r| r.get(0),
        )
        .expect("query by app_type");
}

#[test]
fn migration_from_v3_8_schema_v1_to_current_schema_v3() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .expect("enable foreign keys");

    // 模拟 v3.8.* 用户的数据库（schema v1）
    conn.execute_batch(V3_8_SCHEMA_V1_SQL)
        .expect("seed v3.8 schema v1");
    Database::set_user_version(&conn, 1).expect("set user_version=1");

    // 插入一条旧版 Provider + Skill（用于验证迁移不会破坏既有数据）
    conn.execute(
        "INSERT INTO providers (
            id, app_type, name, settings_config, website_url, category,
            created_at, sort_index, notes, icon, icon_color, meta, is_current
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            "p1",
            "claude",
            "Test Provider",
            serde_json::to_string(&json!({ "anthropicApiKey": "sk-test" })).unwrap(),
            Option::<String>::None,
            Option::<String>::None,
            Option::<i64>::None,
            Option::<usize>::None,
            Option::<String>::None,
            Option::<String>::None,
            Option::<String>::None,
            "{}",
            1,
        ],
    )
    .expect("seed provider");

    conn.execute(
        "INSERT INTO skills (key, installed, installed_at) VALUES (?1, ?2, ?3)",
        params!["claude:demo-skill", 1, 1700000000i64],
    )
    .expect("seed legacy skill");

    // 按应用启动流程：先 create_tables（补齐新增表），再 apply_schema_migrations（按 user_version 迁移）
    Database::create_tables_on_conn(&conn).expect("create tables");
    Database::apply_schema_migrations_on_conn(&conn).expect("apply migrations");

    assert_eq!(
        Database::get_user_version(&conn).expect("user_version after migration"),
        SCHEMA_VERSION
    );

    // v1 -> v2：providers 新增字段必须补齐
    for column in [
        "cost_multiplier",
        "limit_daily_usd",
        "limit_monthly_usd",
        "provider_type",
        "in_failover_queue",
    ] {
        assert!(
            Database::has_column(&conn, "providers", column).expect("check column"),
            "providers.{column} should exist after migration"
        );
    }

    // 旧 provider 不应丢失，且新增字段应有默认值
    let provider_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM providers WHERE id = 'p1' AND app_type = 'claude'",
            [],
            |r| r.get(0),
        )
        .expect("count providers");
    assert_eq!(provider_count, 1);

    let cost_multiplier: String = conn
        .query_row(
            "SELECT cost_multiplier FROM providers WHERE id = 'p1' AND app_type = 'claude'",
            [],
            |r| r.get(0),
        )
        .expect("read cost_multiplier");
    assert_eq!(cost_multiplier, "1.0");

    // v2 -> v3：skills 表重建为统一结构，并设置 pending 标记（后续由启动时扫描文件系统重建数据）
    assert!(
        Database::has_column(&conn, "skills", "enabled_claude").expect("check skills v3 column"),
        "skills table should be migrated to v3 structure"
    );
    let skills_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM skills", [], |r| r.get(0))
        .expect("count skills");
    assert_eq!(skills_count, 0, "skills table should be rebuilt empty");

    let pending: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'skills_ssot_migration_pending'",
            [],
            |r| r.get(0),
        )
        .ok();
    assert!(
        matches!(pending.as_deref(), Some("true") | Some("1")),
        "skills_ssot_migration_pending should be set after v2->v3 migration"
    );
    let snapshot: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'skills_ssot_migration_snapshot'",
            [],
            |r| r.get(0),
        )
        .ok();
    let snapshot = snapshot.expect("skills migration snapshot should be recorded");
    let snapshot_rows: serde_json::Value =
        serde_json::from_str(&snapshot).expect("parse skills migration snapshot");
    assert!(
        snapshot_rows
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row.get("directory").and_then(|v| v.as_str()) == Some("demo-skill")
                    && row.get("app_type").and_then(|v| v.as_str()) == Some("claude")
            })),
        "skills migration snapshot should preserve legacy app mapping"
    );

    // v3.9+ 新增：proxy_config 三行 seed 必须存在（否则 UI 会查不到默认值）
    let proxy_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM proxy_config", [], |r| r.get(0))
        .expect("count proxy_config rows");
    assert_eq!(proxy_rows, 3);

    // model_pricing 应具备默认数据（迁移时会 seed）
    let pricing_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM model_pricing", [], |r| r.get(0))
        .expect("count model_pricing rows");
    assert!(pricing_rows > 0, "model_pricing should be seeded");
}

#[test]
fn schema_dry_run_does_not_write_to_disk() {
    // Create minimal valid config for migration
    let mut apps = HashMap::new();
    apps.insert("claude".to_string(), ProviderManager::default());

    let config = MultiAppConfig {
        version: 2,
        apps,
        mcp: Default::default(),
        prompts: Default::default(),
        skills: Default::default(),
        common_config_snippets: Default::default(),
        claude_common_config_snippet: None,
    };

    // Dry-run should succeed without any file I/O errors
    let result = Database::migrate_from_json_dry_run(&config);
    assert!(
        result.is_ok(),
        "Dry-run should succeed with valid config: {result:?}"
    );
}

#[test]
fn dry_run_validates_schema_compatibility() {
    // Create config with actual provider data
    let mut providers = IndexMap::new();
    providers.insert(
        "test-provider".to_string(),
        Provider {
            id: "test-provider".to_string(),
            name: "Test Provider".to_string(),
            settings_config: json!({
                "anthropicApiKey": "sk-test-123",
            }),
            website_url: None,
            category: None,
            created_at: Some(1234567890),
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        },
    );

    let manager = ProviderManager {
        providers,
        current: "test-provider".to_string(),
    };

    let mut apps = HashMap::new();
    apps.insert("claude".to_string(), manager);

    let config = MultiAppConfig {
        version: 2,
        apps,
        mcp: Default::default(),
        prompts: Default::default(),
        skills: Default::default(),
        common_config_snippets: Default::default(),
        claude_common_config_snippet: None,
    };

    // Dry-run should validate the full migration path
    let result = Database::migrate_from_json_dry_run(&config);
    assert!(
        result.is_ok(),
        "Dry-run should succeed with provider data: {result:?}"
    );
}

#[test]
fn schema_model_pricing_is_seeded_on_init() {
    let db = Database::memory().expect("create memory db");

    let conn = db.conn.lock().expect("lock conn");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM model_pricing", [], |row| row.get(0))
        .expect("count pricing");

    assert!(
        count > 0,
        "模型定价数据应该在初始化时自动填充，实际数量: {}",
        count
    );

    // 验证包含 Claude 模型
    let claude_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM model_pricing WHERE model_id LIKE 'claude-%'",
            [],
            |row| row.get(0),
        )
        .expect("check claude");
    assert!(
        claude_count > 0,
        "应该包含 Claude 模型定价，实际数量: {}",
        claude_count
    );

    // 验证包含 GPT 模型
    let gpt_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM model_pricing WHERE model_id LIKE 'gpt-%'",
            [],
            |row| row.get(0),
        )
        .expect("check gpt");
    assert!(
        gpt_count > 0,
        "应该包含 GPT 模型定价，实际数量: {}",
        gpt_count
    );

    // 验证包含 Gemini 模型
    let gemini_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM model_pricing WHERE model_id LIKE 'gemini-%'",
            [],
            |row| row.get(0),
        )
        .expect("check gemini");
    assert!(
        gemini_count > 0,
        "应该包含 Gemini 模型定价，实际数量: {}",
        gemini_count
    );
}

#[test]
fn gpt_5_6_pricing_seed_has_auditable_schedule_metadata() {
    let db = Database::memory().expect("create memory db");
    let conn = db.conn.lock().expect("lock conn");

    let sol: (String, String, String, String) = conn
        .query_row(
            "SELECT input_cost_per_million, output_cost_per_million,
                    cache_read_cost_per_million, cache_creation_cost_per_million
             FROM model_pricing WHERE model_id = 'gpt-5.6-sol'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("GPT-5.6 Sol pricing must be seeded");
    assert_eq!(sol, ("5".into(), "30".into(), "0.50".into(), "6.25".into()));

    for (model, expected) in [
        ("gpt-5.6-sol", ("5", "30", "0.50", "6.25")),
        ("gpt-5.6-terra", ("2.50", "15", "0.25", "3.125")),
        ("gpt-5.6-luna", ("1", "6", "0.10", "1.25")),
    ] {
        let actual: (String, String, String, String) = conn
            .query_row(
                "SELECT input_cost_per_million, output_cost_per_million,
                        cache_read_cost_per_million, cache_creation_cost_per_million
                 FROM model_pricing WHERE model_id = ?1",
                [model],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap_or_else(|error| panic!("{model} pricing must be seeded: {error}"));
        assert_eq!(
            actual,
            (
                expected.0.into(),
                expected.1.into(),
                expected.2.into(),
                expected.3.into(),
            ),
            "{model} pricing must include cache writes at 1.25x input price"
        );
    }

    let provenance: (String, String, String, String, i64, String, String) = conn
        .query_row(
            "SELECT source, source_version, effective_at, currency,
                    long_context_threshold_tokens,
                    long_context_input_multiplier, long_context_output_multiplier
             FROM model_pricing_provenance WHERE model_id = 'gpt-5.6-sol'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("GPT-5.6 provenance must be seeded");
    assert_eq!(
        provenance,
        (
            "openai_public_api".into(),
            "2026-07-09".into(),
            "2026-07-09".into(),
            "USD".into(),
            272_000,
            "2".into(),
            "1.5".into(),
        )
    );
}

#[test]
fn schema_repairs_existing_pricing_provenance_without_currency_column() {
    let db = Database::memory().expect("create memory db");

    {
        let conn = db.conn.lock().expect("lock database");
        conn.execute_batch(
            "DROP TABLE model_pricing_provenance;
             CREATE TABLE model_pricing_provenance (
                 model_id TEXT PRIMARY KEY NOT NULL,
                 source TEXT NOT NULL,
                 source_version TEXT NOT NULL,
                 effective_at TEXT NOT NULL,
                 long_context_threshold_tokens INTEGER,
                 long_context_input_multiplier TEXT NOT NULL DEFAULT '1',
                 long_context_output_multiplier TEXT NOT NULL DEFAULT '1'
             );",
        )
        .expect("create v36 provenance schema without currency");
    }

    db.create_tables()
        .expect("create tables must repair missing provenance column");

    let conn = db.conn.lock().expect("lock repaired database");
    assert!(
        Database::has_column(&conn, "model_pricing_provenance", "currency")
            .expect("inspect repaired provenance schema")
    );
}

#[test]
fn model_pricing_seed_repairs_known_outdated_builtin_prices() {
    let db = Database::memory().expect("create memory db");

    {
        let conn = db.conn.lock().expect("lock conn");
        conn.execute(
            "UPDATE model_pricing
             SET input_cost_per_million = '1.68',
                 output_cost_per_million = '3.36',
                 cache_read_cost_per_million = '0.14',
                 cache_creation_cost_per_million = '0'
             WHERE model_id = 'deepseek-v4-pro'",
            [],
        )
        .expect("restore old DeepSeek price");
        conn.execute(
            "UPDATE model_pricing
             SET input_cost_per_million = '9',
                 output_cost_per_million = '9',
                 cache_read_cost_per_million = '9',
                 cache_creation_cost_per_million = '0'
             WHERE model_id = 'glm-5.1'",
            [],
        )
        .expect("set custom GLM price");
    }

    db.ensure_model_pricing_seeded()
        .expect("ensure pricing seeded");

    let conn = db.conn.lock().expect("lock conn");
    let deepseek: (String, String, String) = conn
        .query_row(
            "SELECT input_cost_per_million, output_cost_per_million, cache_read_cost_per_million
             FROM model_pricing WHERE model_id = 'deepseek-v4-pro'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("query DeepSeek price");
    assert_eq!(
        deepseek,
        (
            "0.435".to_string(),
            "0.87".to_string(),
            "0.003625".to_string()
        )
    );

    let glm: (String, String, String) = conn
        .query_row(
            "SELECT input_cost_per_million, output_cost_per_million, cache_read_cost_per_million
             FROM model_pricing WHERE model_id = 'glm-5.1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("query GLM price");
    assert_eq!(glm, ("9".to_string(), "9".to_string(), "9".to_string()));
}

#[test]
fn ensure_incremental_auto_vacuum_rebuilds_existing_file_db() {
    let temp = NamedTempFile::new().expect("create temp db file");
    let path = temp.path().to_path_buf();

    let conn = Connection::open(&path).expect("open temp db");
    conn.execute("PRAGMA auto_vacuum = NONE;", [])
        .expect("set none auto_vacuum");
    Database::create_tables_on_conn(&conn).expect("create tables");

    assert_eq!(
        Database::get_auto_vacuum_mode(&conn).expect("auto_vacuum before rebuild"),
        0,
        "existing file db should start with NONE auto_vacuum"
    );

    let rebuilt =
        Database::ensure_incremental_auto_vacuum_on_conn(&conn).expect("enable incremental mode");
    assert!(rebuilt, "existing db should require rebuild via VACUUM");
    drop(conn);

    let reopened = Connection::open(&path).expect("reopen temp db");
    assert_eq!(
        Database::get_auto_vacuum_mode(&reopened).expect("auto_vacuum after rebuild"),
        2,
        "file db should persist INCREMENTAL auto_vacuum after VACUUM rebuild"
    );
}

#[test]
fn schema_v25_drops_legacy_project_link_tables() {
    let db = Database::memory().expect("memory db");
    let conn = db.conn.lock().expect("lock");
    let legacy_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='table' AND name IN ('project_mcp_servers','project_skills','project_prompts')",
            [],
            |row| row.get(0),
        )
        .expect("count legacy tables");
    assert_eq!(legacy_count, 0);
    assert_eq!(
        Database::get_user_version(&conn).expect("version"),
        SCHEMA_VERSION
    );
}

#[test]
fn schema_v25_migrates_legacy_rows_into_project_asset_links() {
    let conn = Connection::open_in_memory().expect("open memory db");
    Database::create_tables_on_conn(&conn).expect("create tables");

    conn.execute(
        "INSERT INTO projects (id, name, path, created_at, updated_at)
         VALUES ('p-legacy', 'Legacy', '/tmp/legacy', 100, 100)",
        [],
    )
    .expect("seed project");

    conn.execute(
        "CREATE TABLE project_mcp_servers (
            project_id TEXT NOT NULL,
            mcp_server_id TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (project_id, mcp_server_id)
        )",
        [],
    )
    .expect("create legacy mcp table");
    conn.execute(
        "INSERT INTO project_mcp_servers (project_id, mcp_server_id, enabled, created_at)
         VALUES ('p-legacy', 'srv-1', 1, 200)",
        [],
    )
    .expect("seed legacy mcp row");

    Database::set_user_version(&conn, 24).expect("set v24");
    Database::apply_schema_migrations_on_conn(&conn).expect("migrate to v25");

    let migrated: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM project_asset_links
             WHERE project_id='p-legacy' AND asset_type='mcp' AND asset_id='srv-1'",
            [],
            |row| row.get(0),
        )
        .expect("count migrated row");
    assert_eq!(migrated, 1);

    let legacy_left: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project_mcp_servers'",
            [],
            |row| row.get(0),
        )
        .expect("legacy table should be dropped");
    assert_eq!(legacy_left, 0);
}

/// v25 迁移：legacy 与 unified 主键冲突时，应以 legacy 的 enabled 覆盖
#[test]
fn schema_v25_migration_merges_conflicting_legacy_enabled() {
    let conn = Connection::open_in_memory().expect("open memory db");
    Database::create_tables_on_conn(&conn).expect("create tables");

    conn.execute(
        "INSERT INTO projects (id, name, path, created_at, updated_at)
         VALUES ('p-merge', 'Merge', '/tmp/merge', 100, 100)",
        [],
    )
    .expect("seed project");

    conn.execute(
        "CREATE TABLE project_mcp_servers (
            project_id TEXT NOT NULL,
            mcp_server_id TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (project_id, mcp_server_id)
        )",
        [],
    )
    .expect("legacy mcp");
    conn.execute(
        "INSERT INTO project_mcp_servers VALUES ('p-merge', 'srv-1', 1, 300)",
        [],
    )
    .expect("legacy row enabled=1");
    conn.execute(
        "INSERT INTO project_asset_links
         (project_id, asset_type, asset_id, asset_app_type, enabled, scope, source, created_at, updated_at)
         VALUES ('p-merge', 'mcp', 'srv-1', '', 0, 'project', 'manual', 100, 100)",
        [],
    )
    .expect("pre-existing unified row enabled=0");

    Database::set_user_version(&conn, 24).expect("v24");
    Database::apply_schema_migrations_on_conn(&conn).expect("migrate v25");

    let enabled: i64 = conn
        .query_row(
            "SELECT enabled FROM project_asset_links
             WHERE project_id='p-merge' AND asset_type='mcp' AND asset_id='srv-1'",
            [],
            |row| row.get(0),
        )
        .expect("merged row");
    assert_eq!(enabled, 1, "legacy enabled=1 should win on conflict");
}

/// 磁盘文件库 v24→v25 升级：旧三表 + 已有扩展关联一并保留，DAO 可读
#[test]
fn schema_v25_file_db_upgrade_roundtrip() {
    let temp = NamedTempFile::new().expect("temp db file");
    let path = temp.path().to_path_buf();

    {
        let conn = Connection::open(&path).expect("open file db");
        Database::create_tables_on_conn(&conn).expect("create tables");

        conn.execute(
            "INSERT INTO projects (id, name, path, created_at, updated_at)
             VALUES ('p-real', 'Real', 'E:/demo', 100, 100)",
            [],
        )
        .expect("seed project");

        for ddl in [
            "CREATE TABLE project_mcp_servers (
                project_id TEXT NOT NULL,
                mcp_server_id TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_id, mcp_server_id)
            )",
            "CREATE TABLE project_skills (
                project_id TEXT NOT NULL,
                skill_id TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_id, skill_id)
            )",
            "CREATE TABLE project_prompts (
                project_id TEXT NOT NULL,
                prompt_id TEXT NOT NULL,
                prompt_app_type TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_id, prompt_id, prompt_app_type)
            )",
        ] {
            conn.execute(ddl, []).expect("create legacy table");
        }

        conn.execute(
            "INSERT INTO project_mcp_servers VALUES ('p-real', 'mcp-a', 1, 200)",
            [],
        )
        .expect("seed mcp");
        conn.execute(
            "INSERT INTO project_skills VALUES ('p-real', 'skill-a', 1, 201)",
            [],
        )
        .expect("seed skill");
        conn.execute(
            "INSERT INTO project_prompts VALUES ('p-real', 'prompt-a', 'claude', 1, 202)",
            [],
        )
        .expect("seed prompt");
        conn.execute(
            "INSERT INTO project_asset_links
             (project_id, asset_type, asset_id, asset_app_type, enabled, scope, source, created_at, updated_at)
             VALUES ('p-real', 'hook', 'hook-a', '', 1, 'project', 'manual', 300, 300)",
            [],
        )
        .expect("seed extended link");

        Database::set_user_version(&conn, 24).expect("pin v24");
    }

    let db = {
        let conn = Connection::open(&path).expect("reopen file db");
        conn.execute("PRAGMA foreign_keys = ON;", []).ok();
        Database {
            conn: std::sync::Mutex::new(conn),
        }
    };
    db.apply_schema_migrations()
        .expect("migrate file db to v25");

    let conn = db.conn.lock().expect("lock");
    assert_eq!(
        Database::get_user_version(&conn).expect("version"),
        SCHEMA_VERSION
    );
    let legacy: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='table' AND name IN ('project_mcp_servers','project_skills','project_prompts')",
            [],
            |row| row.get(0),
        )
        .expect("legacy tables");
    assert_eq!(legacy, 0);
    drop(conn);

    let mcp = db.get_project_mcp_servers("p-real").expect("mcp dao");
    assert_eq!(mcp.len(), 1);
    assert_eq!(mcp[0].config_id, "mcp-a");

    let skills = db.get_project_skills("p-real").expect("skills dao");
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].config_id, "skill-a");

    let prompts = db.get_project_prompts("p-real").expect("prompts dao");
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].prompt_id, "prompt-a");
    assert_eq!(prompts[0].prompt_app_type, "claude");

    let hooks = db
        .get_project_asset_links("p-real", Some("hook"))
        .expect("hook links");
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].asset_id, "hook-a");

    let counts = db
        .get_project_all_asset_counts("p-real")
        .expect("asset counts");
    assert_eq!(counts.mcp, 1);
    assert_eq!(counts.skills, 1);
    assert_eq!(counts.prompts, 1);
    assert_eq!(counts.hooks, 1);
}

/// 只读检查本机 `~/.OpenSunstar/OpenSunstar.db`（若存在）是否已完成 v25 且无旧表
#[test]
fn verify_local_open_sunstar_db_post_v25() {
    let db_path = crate::config::get_app_config_dir().join("OpenSunstar.db");
    if !db_path.exists() {
        eprintln!("skip: no local OpenSunstar.db at {}", db_path.display());
        return;
    }

    let conn = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open local db read-only");

    let version = Database::get_user_version(&conn).expect("user_version");
    eprintln!("local OpenSunstar.db user_version={version}");

    if version >= 25 {
        let legacy: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='table' AND name IN ('project_mcp_servers','project_skills','project_prompts')",
                [],
                |row| row.get(0),
            )
            .expect("legacy count");
        assert_eq!(
            legacy, 0,
            "v25+ db must not contain legacy project link tables"
        );

        let link_types: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT asset_type) FROM project_asset_links",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        eprintln!("local project_asset_links distinct asset_type count={link_types}");
    } else if version == 24 {
        eprintln!("local db still v24 — next app start will migrate to v25 and drop legacy tables");
    }
}

/// 复制本机 OpenSunstar.db 到临时文件并执行 v24→v25 升级（不修改原库）
#[test]
fn upgrade_local_open_sunstar_db_copy_on_disk() {
    let source = crate::config::get_app_config_dir().join("OpenSunstar.db");
    if !source.exists() {
        eprintln!("skip: no local OpenSunstar.db at {}", source.display());
        return;
    }

    let temp = NamedTempFile::new().expect("temp db file");
    let dest = temp.path().to_path_buf();
    std::fs::copy(&source, &dest).expect("copy local db to temp");

    let pre_conn = Connection::open_with_flags(&dest, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open copy read-only pre-migration");
    let pre_version = Database::get_user_version(&pre_conn).expect("pre version");
    let pre_legacy_mcp: i64 = pre_conn
        .query_row("SELECT COUNT(*) FROM project_mcp_servers", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    let pre_links: i64 = pre_conn
        .query_row("SELECT COUNT(*) FROM project_asset_links", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    drop(pre_conn);

    eprintln!(
        "pre-migration copy: version={pre_version}, legacy_mcp_rows={pre_legacy_mcp}, project_asset_links_rows={pre_links}"
    );

    let db = {
        let conn = Connection::open(&dest).expect("open copy for migration");
        conn.execute("PRAGMA foreign_keys = ON;", []).ok();
        Database {
            conn: std::sync::Mutex::new(conn),
        }
    };
    db.apply_schema_migrations()
        .expect("migrate copied local db");

    let conn = db.conn.lock().expect("lock");
    assert_eq!(
        Database::get_user_version(&conn).expect("post version"),
        SCHEMA_VERSION
    );
    let legacy_tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='table' AND name IN ('project_mcp_servers','project_skills','project_prompts')",
            [],
            |row| row.get(0),
        )
        .expect("legacy tables");
    assert_eq!(legacy_tables, 0);

    let post_mcp_links: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM project_asset_links WHERE asset_type='mcp'",
            [],
            |row| row.get(0),
        )
        .expect("mcp links");
    let post_all_links: i64 = conn
        .query_row("SELECT COUNT(*) FROM project_asset_links", [], |row| {
            row.get(0)
        })
        .expect("all links");
    drop(conn);

    eprintln!(
        "post-migration copy: version={SCHEMA_VERSION}, legacy_tables=0, mcp_links={post_mcp_links}, total_links={post_all_links}"
    );

    if pre_legacy_mcp > 0 {
        assert!(
            post_mcp_links >= pre_legacy_mcp,
            "migrated mcp links should cover legacy rows"
        );
    }
    assert!(
        post_all_links >= pre_links + pre_legacy_mcp,
        "total links should not shrink after migration"
    );
}

#[test]
fn schema_v29_creates_asset_health_fact_tables() {
    let db = Database::memory().expect("create memory database");
    let conn = db.conn.lock().expect("lock database");

    for table in [
        "asset_revisions",
        "project_asset_expectations",
        "asset_deployment_receipts",
        "asset_runtime_evidence",
    ] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("query sqlite schema");
        assert_eq!(exists, 1, "{table} should exist after migration");
    }

    assert_eq!(
        Database::get_user_version(&conn).expect("schema version"),
        SCHEMA_VERSION
    );
}

#[test]
fn schema_v30_creates_asset_receipt_files_table() {
    let db = Database::memory().expect("create memory database");
    let conn = db.conn.lock().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'asset_receipt_files'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn schema_v31_binds_receipts_to_asset_revisions() {
    let db = Database::memory().expect("create memory database");
    let conn = db.conn.lock().expect("lock database");
    assert!(
        Database::has_column(&conn, "asset_deployment_receipts", "required_revision_id")
            .expect("inspect receipt schema")
    );
    assert_eq!(
        Database::get_user_version(&conn).expect("schema version"),
        SCHEMA_VERSION
    );
}
