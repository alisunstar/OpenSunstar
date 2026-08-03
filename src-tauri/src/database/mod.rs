//! 数据库模块 - SQLite 数据持久化
//!
//! 此模块提供应用的核心数据存储功能，包括：
//! - 供应商配置管理
//! - MCP 服务器配置
//! - 提示词管理
//! - Skills 管理
//! - 通用设置存储
//!
//! ## 架构设计
//!
//! ```text
//! database/
//! ├── mod.rs        - Database 结构体 + 初始化
//! ├── schema.rs     - 表结构定义 + Schema 迁移
//! ├── backup.rs     - SQL 导入导出 + 快照备份
//! ├── migration.rs  - JSON → SQLite 数据迁移
//! └── dao/          - 数据访问对象
//!     ├── providers.rs
//!     ├── mcp.rs
//!     ├── prompts.rs
//!     ├── skills.rs
//!     └── settings.rs
//! ```

pub(crate) mod backup;
mod dao;
mod migration;
mod schema;

#[cfg(test)]
mod tests;

// DAO 类型导出供外部使用
pub(crate) use dao::ai_insight::{AICostLogRow, AIInsightRow, AIQueryLogRow};
#[allow(unused_imports)]
pub use dao::asset_health::{
    AssetDeploymentReceipt, AssetReceiptFile, AssetRevision, AssetRuntimeEvidence,
    ProjectAssetExpectation,
};
pub use dao::project_assets::{
    ProjectAllAssetCounts, ProjectAssetLink, ASSET_IGNORE, EXTENDED_ASSET_TYPES,
};
pub use dao::project_environment::ProjectEnvironmentSnapshot;
pub(crate) use dao::providers_seed::{is_official_seed_id, CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID};
pub(crate) use dao::proxy::{
    validate_cost_multiplier, validate_pricing_source, PRICING_SOURCE_REQUEST,
    PRICING_SOURCE_RESPONSE,
};
pub use dao::quick_start::{
    QuickStartOperation, QuickStartOperationEvent, QuickStartOperationStatus,
};
pub(crate) use dao::team_key::TeamKeyLocal;
pub(crate) use dao::team_workspace::{TeamAssignmentRow, TeamWorkspace};
pub use dao::FailoverQueueItem;
pub use dao::{Project, ProjectConfigLink, ProjectPromptLink};

use crate::config::get_app_config_dir;
use crate::error::AppError;
use crate::proxy::usage::calculator::ModelPricing;
use rusqlite::{hooks::Action, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

// DAO 方法通过 impl Database 提供，无需额外导出

/// 当前 Schema 版本号
/// 每次修改表结构时递增，并在 schema.rs 中添加相应的迁移逻辑
pub(crate) const SCHEMA_VERSION: i32 = 45;

/// 代理热路径计价查询的 TTL（全局默认倍率/来源 + model_pricing）。
const PRICING_CACHE_TTL: Duration = Duration::from_secs(30);

/// 安全地序列化 JSON，避免 unwrap panic
pub(crate) fn to_json_string<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|e| AppError::Config(format!("JSON serialization failed: {e}")))
}

/// 安全地获取 Mutex 锁，避免 unwrap panic
macro_rules! lock_conn {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|e| AppError::Database(format!("Mutex lock failed: {}", e)))?
    };
}

// 导出宏供子模块使用
pub(crate) use lock_conn;

/// 代理热路径计价查询缓存（短 TTL + 写路径显式失效）。
#[derive(Default)]
struct PricingLookupCache {
    /// app_type → (expires_at, default_multiplier, pricing_model_source)
    defaults: HashMap<String, (Instant, String, String)>,
    /// model_id → (expires_at, pricing)
    models: HashMap<String, (Instant, Option<ModelPricing>)>,
}

impl PricingLookupCache {
    fn clear(&mut self) {
        self.defaults.clear();
        self.models.clear();
    }
}

/// 数据库连接封装
///
/// 使用 Mutex 包装 Connection 以支持在多线程环境（如 Tauri State）中共享。
/// rusqlite::Connection 本身不是 Sync 的，因此需要这层包装。
///
/// 文件库启用 WAL：读路径与异步用量写线程的第二连接可并发；备份仍走
/// rusqlite `Backup` API（WAL 安全），勿直接复制 `.db` 文件。
pub struct Database {
    pub(crate) conn: Mutex<Connection>,
    /// 文件库路径；`:memory:` 测试库为 `None`（无法开第二连接共享同一内存库）。
    db_path: Option<PathBuf>,
    /// 高频、可增长的用量明细与 rollup 使用独立 SQLite 文件。内存数据库
    /// 不创建它，以保持现有测试夹具可直接使用 `conn`。
    usage_db_conn: OnceLock<Mutex<Connection>>,
    usage_db_path: OnceLock<PathBuf>,
    pricing_cache: Mutex<PricingLookupCache>,
}

fn register_db_change_hook(conn: &Connection) {
    conn.update_hook(Some(
        |action: Action, _database: &str, table: &str, _row_id: i64| match action {
            Action::SQLITE_INSERT | Action::SQLITE_UPDATE | Action::SQLITE_DELETE => {
                crate::services::webdav_auto_sync::notify_db_changed(table);
                crate::services::s3_auto_sync::notify_db_changed(table);
            }
            _ => {}
        },
    ));
}

/// 文件库运行时 pragma：WAL + busy_timeout + synchronous=NORMAL。
///
/// WAL 失败时告警并继续（例如只读介质）；busy_timeout 降低多连接锁等待立刻失败的概率。
pub(crate) fn apply_file_connection_pragmas(conn: &Connection) -> Result<(), AppError> {
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| AppError::Database(format!("设置 busy_timeout 失败: {e}")))?;

    match conn.query_row("PRAGMA journal_mode = WAL;", [], |row| {
        row.get::<_, String>(0)
    }) {
        Ok(mode) if mode.eq_ignore_ascii_case("wal") => {}
        Ok(mode) => {
            log::warn!("启用 WAL 未成功，当前 journal_mode={mode}；继续使用单连接语义");
        }
        Err(e) => {
            log::warn!("启用 WAL 失败: {e}；继续使用默认 journal 模式");
        }
    }

    conn.execute_batch("PRAGMA synchronous = NORMAL;")
        .map_err(|e| AppError::Database(format!("设置 synchronous 失败: {e}")))?;
    Ok(())
}

impl Database {
    fn wrap(conn: Connection, db_path: Option<PathBuf>) -> Self {
        Self {
            conn: Mutex::new(conn),
            db_path,
            usage_db_conn: OnceLock::new(),
            usage_db_path: OnceLock::new(),
            pricing_cache: Mutex::new(PricingLookupCache::default()),
        }
    }

    /// 文件库路径（异步用量写线程开第二连接用）。
    pub fn path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }

    /// 用量专用连接。内存数据库回退到主连接，保证测试夹具与开发期行为一致。
    pub(crate) fn usage_conn(&self) -> &Mutex<Connection> {
        self.usage_db_conn.get().unwrap_or(&self.conn)
    }

    /// 文件库的用量 sidecar 路径；内存数据库没有独立文件。
    pub(crate) fn usage_path(&self) -> Option<&Path> {
        self.usage_db_path.get().map(PathBuf::as_path)
    }

    fn usage_sidecar_path(core_path: &Path) -> Result<PathBuf, AppError> {
        let parent = core_path.parent().ok_or_else(|| {
            AppError::Database(format!("主数据库路径没有父目录: {}", core_path.display()))
        })?;
        Ok(parent.join("usage.db"))
    }

    /// 初始化独立 usage.db，并只迁移一次旧主库中的历史用量数据。
    ///
    /// sidecar 保留两个查询所需的 TEMP 只读视图（providers/model_pricing），
    /// 因而现有统计 SQL 可以在 usage 连接中继续关联主库配置，而不复制配置数据。
    fn initialize_usage_database(&self) -> Result<(), AppError> {
        let Some(core_path) = self.path() else {
            return Ok(());
        };
        let usage_path = Self::usage_sidecar_path(core_path)?;
        let new_usage_db = !usage_path.exists();
        let conn = Connection::open(&usage_path).map_err(|e| AppError::Database(e.to_string()))?;
        if new_usage_db {
            conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])
                .map_err(|e| AppError::Database(format!("设置 usage.db auto_vacuum 失败: {e}")))?;
        }
        apply_file_connection_pragmas(&conn)?;
        register_db_change_hook(&conn);
        Self::create_usage_tables_on_conn(&conn)?;

        let imported: Option<String> = conn
            .query_row(
                "SELECT value FROM usage_sidecar_meta WHERE key = 'legacy_core_import_v1'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| AppError::Database(format!("读取 usage.db 迁移标记失败: {e}")))?;

        if imported.is_none() {
            let core_path_string = core_path.to_string_lossy().into_owned();
            conn.execute("ATTACH DATABASE ?1 AS core", [core_path_string])
                .map_err(|e| AppError::Database(format!("关联旧主库失败: {e}")))?;

            let import_result = (|| -> Result<(), AppError> {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO proxy_request_logs (
                        request_id, provider_id, app_type, model, request_model, pricing_model,
                        input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                        input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd,
                        total_cost_usd, latency_ms, first_token_ms, duration_ms, status_code,
                        error_message, session_id, provider_type, is_streaming, cost_multiplier,
                        created_at, data_source
                    ) SELECT
                        request_id, provider_id, app_type, model, request_model, pricing_model,
                        input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                        input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd,
                        total_cost_usd, latency_ms, first_token_ms, duration_ms, status_code,
                        error_message, session_id, provider_type, is_streaming, cost_multiplier,
                        created_at, data_source
                    FROM core.proxy_request_logs;

                    INSERT OR IGNORE INTO usage_daily_rollups (
                        date, app_type, provider_id, model, request_model, pricing_model,
                        request_count, success_count, input_tokens, output_tokens,
                        cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                    ) SELECT
                        date, app_type, provider_id, model, request_model, pricing_model,
                        request_count, success_count, input_tokens, output_tokens,
                        cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
                    FROM core.usage_daily_rollups;",
                )
                .map_err(|e| AppError::Database(format!("迁移历史用量到 usage.db 失败: {e}")))?;
                conn.execute(
                    "INSERT INTO usage_sidecar_meta (key, value) VALUES ('legacy_core_import_v1', 'complete')",
                    [],
                )
                .map_err(|e| AppError::Database(format!("写入 usage.db 迁移标记失败: {e}")))?;
                Ok(())
            })();
            let detach_result = conn.execute_batch("DETACH DATABASE core;");
            import_result?;
            detach_result.map_err(|e| AppError::Database(format!("断开旧主库失败: {e}")))?;
        }

        let core_path_string = core_path.to_string_lossy().into_owned();
        conn.execute("ATTACH DATABASE ?1 AS core", [core_path_string])
            .map_err(|e| AppError::Database(format!("关联主库配置失败: {e}")))?;
        conn.execute_batch(
            "CREATE TEMP VIEW model_pricing AS SELECT * FROM core.model_pricing;
             CREATE TEMP VIEW providers AS SELECT * FROM core.providers;",
        )
        .map_err(|e| AppError::Database(format!("创建 usage.db 配置视图失败: {e}")))?;

        self.usage_db_path
            .set(usage_path)
            .map_err(|_| AppError::Database("usage.db 路径被重复初始化".to_string()))?;
        self.usage_db_conn
            .set(Mutex::new(conn))
            .map_err(|_| AppError::Database("usage.db 连接被重复初始化".to_string()))?;
        Ok(())
    }

    fn create_usage_tables_on_conn(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS proxy_request_logs (
                request_id TEXT PRIMARY KEY, provider_id TEXT NOT NULL, app_type TEXT NOT NULL, model TEXT NOT NULL,
                request_model TEXT, pricing_model TEXT,
                input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0, cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                input_cost_usd TEXT NOT NULL DEFAULT '0', output_cost_usd TEXT NOT NULL DEFAULT '0',
                cache_read_cost_usd TEXT NOT NULL DEFAULT '0', cache_creation_cost_usd TEXT NOT NULL DEFAULT '0',
                total_cost_usd TEXT NOT NULL DEFAULT '0', latency_ms INTEGER NOT NULL, first_token_ms INTEGER,
                duration_ms INTEGER, status_code INTEGER NOT NULL, error_message TEXT, session_id TEXT,
                provider_type TEXT, is_streaming INTEGER NOT NULL DEFAULT 0,
                cost_multiplier TEXT NOT NULL DEFAULT '1.0', created_at INTEGER NOT NULL,
                data_source TEXT NOT NULL DEFAULT 'proxy'
            );
            CREATE INDEX IF NOT EXISTS idx_request_logs_provider ON proxy_request_logs(provider_id, app_type);
            CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON proxy_request_logs(created_at);
            CREATE INDEX IF NOT EXISTS idx_request_logs_model ON proxy_request_logs(model);
            CREATE INDEX IF NOT EXISTS idx_request_logs_session ON proxy_request_logs(session_id);
            CREATE INDEX IF NOT EXISTS idx_request_logs_status ON proxy_request_logs(status_code);
            CREATE INDEX IF NOT EXISTS idx_request_logs_provider_app_created
                ON proxy_request_logs(provider_id, app_type, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_request_logs_app_created
                ON proxy_request_logs(app_type, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_request_logs_usage_group
                ON proxy_request_logs(app_type, data_source, input_tokens, output_tokens, created_at DESC);
            CREATE TABLE IF NOT EXISTS usage_daily_rollups (
                date TEXT NOT NULL, app_type TEXT NOT NULL, provider_id TEXT NOT NULL, model TEXT NOT NULL,
                request_model TEXT NOT NULL DEFAULT '', pricing_model TEXT NOT NULL DEFAULT '',
                request_count INTEGER NOT NULL DEFAULT 0, success_count INTEGER NOT NULL DEFAULT 0,
                input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0, cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost_usd TEXT NOT NULL DEFAULT '0', avg_latency_ms INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (date, app_type, provider_id, model, request_model, pricing_model)
            );
            CREATE TABLE IF NOT EXISTS usage_sidecar_meta (
                key TEXT PRIMARY KEY, value TEXT NOT NULL
            );",
        )
        .map_err(|e| AppError::Database(format!("创建 usage.db 表失败: {e}")))
    }

    /// 清空计价 TTL 缓存（全局默认倍率/来源或 model_pricing 变更后调用）。
    pub fn invalidate_pricing_cache(&self) {
        if let Ok(mut cache) = self.pricing_cache.lock() {
            cache.clear();
        }
    }

    /// 带 TTL 的全局代理计价默认值（一次查询拿齐倍率 + 来源）。
    pub fn get_proxy_pricing_defaults_cached(
        &self,
        app_type: &str,
    ) -> Result<(String, String), AppError> {
        let now = Instant::now();
        if let Ok(cache) = self.pricing_cache.lock() {
            if let Some((expires, multiplier, source)) = cache.defaults.get(app_type) {
                if *expires > now {
                    return Ok((multiplier.clone(), source.clone()));
                }
            }
        }

        let result = {
            let conn = lock_conn!(self.conn);
            conn.query_row(
                "SELECT default_cost_multiplier, pricing_model_source
                 FROM proxy_config WHERE app_type = ?1",
                [app_type],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
        };

        let (multiplier, source) = match result {
            Ok(pair) => pair,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                ("1".to_string(), PRICING_SOURCE_RESPONSE.to_string())
            }
            Err(e) => return Err(AppError::Database(e.to_string())),
        };

        if let Ok(mut cache) = self.pricing_cache.lock() {
            cache.defaults.insert(
                app_type.to_string(),
                (now + PRICING_CACHE_TTL, multiplier.clone(), source.clone()),
            );
        }

        Ok((multiplier, source))
    }

    /// 带 TTL 的模型定价查询（代理热路径避免每次锁库扫表）。
    pub fn lookup_model_pricing_cached(
        &self,
        model_id: &str,
    ) -> Result<Option<ModelPricing>, AppError> {
        let now = Instant::now();
        if let Ok(cache) = self.pricing_cache.lock() {
            if let Some((expires, pricing)) = cache.models.get(model_id) {
                if *expires > now {
                    return Ok(pricing.clone());
                }
            }
        }

        let pricing = {
            let conn = lock_conn!(self.conn);
            crate::services::usage_stats::find_model_pricing(&conn, model_id)
        };

        if let Ok(mut cache) = self.pricing_cache.lock() {
            cache.models.insert(
                model_id.to_string(),
                (now + PRICING_CACHE_TTL, pricing.clone()),
            );
        }

        Ok(pricing)
    }

    /// 初始化数据库连接并创建表
    ///
    /// 数据库文件位于 `~/.OpenSunstar/OpenSunstar.db`
    pub fn init() -> Result<Self, AppError> {
        let db_path = get_app_config_dir().join("OpenSunstar.db");
        let db_exists = db_path.exists();

        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let conn = Connection::open(&db_path).map_err(|e| AppError::Database(e.to_string()))?;

        // 启用外键约束
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        if !db_exists {
            // For a brand-new database, configure incremental auto-vacuum
            // before creating any tables so no rebuild is needed later.
            conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        // WAL / busy_timeout：允许异步用量写线程第二连接与主连接并发读；写仍串行化于 WAL。
        apply_file_connection_pragmas(&conn)?;
        register_db_change_hook(&conn);

        let db = Self::wrap(conn, Some(db_path));
        db.create_tables()?;

        // Pre-migration backup: only when upgrading from an existing database
        {
            let conn = lock_conn!(db.conn);
            let version = Self::get_user_version(&conn)?;
            drop(conn);
            if version > 0 && version < SCHEMA_VERSION {
                log::info!(
                    "Creating pre-migration database backup (v{version} → v{SCHEMA_VERSION})"
                );
                if let Err(e) = db.backup_database_file() {
                    // v25 会 DROP 旧三表，备份失败必须中止，避免不可逆数据丢失
                    if version <= 24 {
                        return Err(AppError::Database(format!(
                            "数据库升级前备份失败（v{version} → v{SCHEMA_VERSION}，含不可逆表删除），已中止: {e}"
                        )));
                    }
                    log::warn!("Pre-migration backup failed, continuing migration: {e}");
                }
            }
        }

        db.apply_schema_migrations()?;
        db.initialize_usage_database()?;
        if let Err(e) = db.ensure_incremental_auto_vacuum() {
            log::warn!("Failed to ensure incremental auto-vacuum: {e}");
        }
        db.ensure_model_pricing_seeded()?;

        // Startup cleanup: prune old logs and reclaim space
        if let Err(e) = db.cleanup_old_stream_check_logs(7) {
            log::warn!("Startup stream_check_logs cleanup failed: {e}");
        }
        if let Err(e) = db.rollup_and_prune(30) {
            log::warn!("Startup rollup_and_prune failed: {e}");
        }
        // Reclaim disk space after cleanup
        {
            let conn = lock_conn!(db.conn);
            if let Err(e) = conn.execute_batch("PRAGMA incremental_vacuum;") {
                log::warn!("Startup incremental vacuum failed: {e}");
            }
        }
        if db.usage_db_conn.get().is_some() {
            let conn = lock_conn!(db.usage_conn());
            if let Err(e) = conn.execute_batch("PRAGMA incremental_vacuum;") {
                log::warn!("Startup usage.db incremental vacuum failed: {e}");
            }
        }

        Ok(db)
    }

    /// 创建内存数据库（用于测试）
    pub fn memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))?;

        // 启用外键约束
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        register_db_change_hook(&conn);

        let db = Self::wrap(conn, None);
        db.create_tables()?;
        db.apply_schema_migrations()?;
        db.ensure_model_pricing_seeded()?;

        Ok(db)
    }

    /// 打开指定路径的文件库（测试 / 异步写线程对端）。会跑完整 schema 迁移。
    #[cfg(test)]
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }
        let conn = Connection::open(&path).map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        apply_file_connection_pragmas(&conn)?;
        register_db_change_hook(&conn);

        let db = Self::wrap(conn, Some(path));
        db.create_tables()?;
        db.apply_schema_migrations()?;
        db.initialize_usage_database()?;
        db.ensure_model_pricing_seeded()?;
        Ok(db)
    }

    /// 用已有连接包装（迁移测试夹具：已 pin 旧 version，仅跑 apply_schema_migrations）。
    #[cfg(test)]
    pub(crate) fn from_connection(conn: Connection, db_path: Option<PathBuf>) -> Self {
        Self::wrap(conn, db_path)
    }

    pub(crate) fn get_auto_vacuum_mode(conn: &Connection) -> Result<i32, AppError> {
        conn.query_row("PRAGMA auto_vacuum;", [], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("读取 auto_vacuum 失败: {e}")))
    }

    fn has_user_tables(conn: &Connection) -> Result<bool, AppError> {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(format!("读取表数量失败: {e}")))?;
        Ok(count > 0)
    }

    pub(crate) fn ensure_incremental_auto_vacuum_on_conn(
        conn: &Connection,
    ) -> Result<bool, AppError> {
        let mode = Self::get_auto_vacuum_mode(conn)?;
        if mode == 2 {
            return Ok(false);
        }

        let has_tables = Self::has_user_tables(conn)?;
        conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])
            .map_err(|e| AppError::Database(format!("设置 auto_vacuum 失败: {e}")))?;

        if !has_tables {
            return Ok(false);
        }

        conn.execute("VACUUM;", [])
            .map_err(|e| AppError::Database(format!("执行 VACUUM 失败: {e}")))?;
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(format!("恢复 foreign_keys 失败: {e}")))?;
        Ok(true)
    }

    pub(crate) fn ensure_incremental_auto_vacuum(&self) -> Result<bool, AppError> {
        let mode = {
            let conn = lock_conn!(self.conn);
            Self::get_auto_vacuum_mode(&conn)?
        };
        if mode == 2 {
            return Ok(false);
        }

        let has_tables = {
            let conn = lock_conn!(self.conn);
            Self::has_user_tables(&conn)?
        };
        if has_tables {
            log::info!(
                "Detected auto_vacuum={mode}, rebuilding database to enable incremental vacuum"
            );
            self.backup_database_file()?;
        }

        let rebuilt = {
            let conn = lock_conn!(self.conn);
            Self::ensure_incremental_auto_vacuum_on_conn(&conn)?
        };

        if rebuilt {
            log::info!("Incremental auto-vacuum enabled after database rebuild");
        } else {
            log::info!("Incremental auto-vacuum configured for new database");
        }

        Ok(rebuilt)
    }

    /// 检查 MCP 服务器表是否为空
    pub fn is_mcp_table_empty(&self) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mcp_servers", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count == 0)
    }

    /// 检查提示词表是否为空
    pub fn is_prompts_table_empty(&self) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count == 0)
    }
}
