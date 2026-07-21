//! Ordered startup imports and one-time asset migrations.

use crate::store::AppState;
use crate::AppError;

pub(super) fn import_and_migrate(app_state: &AppState) {
    // ============================================================
    // 按表独立判断的导入逻辑（各类数据独立检查，互不影响）
    // ============================================================

    // 1. 初始化默认 Skills 仓库（已有内置检查：表非空则跳过）
    match app_state.db.init_default_skill_repos() {
        Ok(count) if count > 0 => {
            log::info!("✓ Initialized {count} default skill repositories");
        }
        Ok(_) => {} // 表非空，静默跳过
        Err(e) => log::warn!("✗ Failed to initialize default skill repos: {e}"),
    }

    // 1.1. Skills 统一管理迁移：当数据库迁移到 v3 结构后，自动从各应用目录导入到 SSOT
    // 触发条件由 schema 迁移设置 settings.skills_ssot_migration_pending = true 控制。
    match app_state.db.get_setting("skills_ssot_migration_pending") {
        Ok(Some(flag)) if flag == "true" || flag == "1" => {
            // 安全保护：如果用户已经有 v3 结构的 Skills 数据，就不要自动清空重建。
            let has_existing = app_state
                .db
                .get_all_installed_skills()
                .map(|skills| !skills.is_empty())
                .unwrap_or(false);

            if has_existing {
                log::info!(
                            "Detected skills_ssot_migration_pending but skills table not empty; skipping auto import."
                        );
                let _ = app_state
                    .db
                    .set_setting("skills_ssot_migration_pending", "false");
            } else {
                match crate::services::skill::migrate_skills_to_ssot(&app_state.db) {
                    Ok(count) => {
                        log::info!("✓ Auto imported {count} skill(s) into SSOT");
                        if count > 0 {
                            crate::init_status::set_skills_migration_result(count);
                        }
                        let _ = app_state
                            .db
                            .set_setting("skills_ssot_migration_pending", "false");
                    }
                    Err(e) => {
                        log::warn!("✗ Failed to auto import legacy skills to SSOT: {e}");
                        crate::init_status::set_skills_migration_error(e.to_string());
                        // 保留 pending 标志，方便下次启动重试
                    }
                }
            }
        }
        Ok(_) => {} // 未开启迁移标志，静默跳过
        Err(e) => log::warn!("✗ Failed to read skills migration flag: {e}"),
    }

    // 1.2. 启动时自动导入：如果 skills 表为空，从 SSOT 目录和旧版目录自动扫描导入
    // 这确保从旧版应用升级的用户，即使数据库已存在但 skills 目录之前未迁移，
    // 也能在下次启动时自动恢复技能数据。
    {
        let skills_empty = app_state
            .db
            .get_all_installed_skills()
            .map(|skills| skills.is_empty())
            .unwrap_or(true);

        if skills_empty {
            log::info!(
                "Skills table is empty, attempting auto-import from SSOT/legacy directories..."
            );
            match crate::services::skill::auto_import_ssot_skills(&app_state.db) {
                Ok(count) if count > 0 => {
                    log::info!("✓ Auto-imported {count} skills from SSOT/legacy directories");
                    crate::init_status::set_skills_migration_result(count);
                }
                Ok(_) => {
                    log::info!("No skills found to auto-import");
                }
                Err(e) => {
                    log::warn!("✗ Failed to auto-import skills: {e}");
                }
            }
        }
    }

    // 1.3. 提示词迁移：从旧版应用数据库导入提示词（如果旧库存在）
    // 使用 INSERT OR REPLACE，已有数据会被更新，不会丢失
    {
        // 旧版应用的数据库文件路径（固定值，用于数据迁移）
        let legacy_db = crate::config::get_home_dir()
            .join(".cc-switch")
            .join("cc-switch.db");

        // 先统计当前提示词数量（用于诊断）
        let current_prompt_count: usize = crate::app_config::AppType::all()
            .map(|app| {
                app_state
                    .db
                    .get_prompts(app.as_str())
                    .map(|p| p.len())
                    .unwrap_or(0)
            })
            .sum();

        log::info!("Current prompts in database: {current_prompt_count}");

        if legacy_db.exists() {
            log::info!("Attempting prompts migration from legacy database...");
            if let Ok(legacy_conn) = rusqlite::Connection::open_with_flags(
                &legacy_db,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            ) {
                let mut migrated = 0usize;

                // 直接从旧库读取所有提示词（不依赖 app_type 枚举值）
                if let Ok(mut stmt) = legacy_conn.prepare(
                            "SELECT id, app_type, name, content, description, enabled, created_at, updated_at \
                             FROM prompts",
                        ) {
                            if let Ok(rows) = stmt.query_map([], |row| {
                                Ok((
                                    row.get::<_, String>(0)?,   // id
                                    row.get::<_, String>(1)?,   // app_type
                                    row.get::<_, String>(2)?,   // name
                                    row.get::<_, String>(3)?,   // content
                                    row.get::<_, Option<String>>(4)?, // description
                                    row.get::<_, bool>(5)?,     // enabled
                                    row.get::<_, Option<i64>>(6)?,    // created_at
                                    row.get::<_, Option<i64>>(7)?,    // updated_at
                                ))
                            }) {
                                for (id, app_type, name, content, description, enabled, created_at, updated_at) in rows.flatten() {
                                    let prompt = crate::prompt::Prompt {
                                        id,
                                        name,
                                        content,
                                        description,
                                        enabled,
                                        created_at,
                                        updated_at,
                                        ..Default::default()
                                    };
                                    if app_state.db.save_prompt(&app_type, &prompt).is_ok() {
                                        migrated += 1;
                                    }
                                }
                            }
                        }

                if migrated > 0 {
                    log::info!("✓ Migrated {migrated} prompts from legacy database");
                } else {
                    log::info!("No additional prompts found in legacy database");
                }
            } else {
                log::warn!("✗ Failed to open legacy database for prompts migration");
            }
        } else {
            log::info!("Legacy database not found, skipping prompts migration");
        }
    }
    //
    // 先 import 后 seed 是有意为之：先把用户手动配置的 settings.json / auth.json / .env
    // 落成 "default" provider 设为 current，再追加官方预设（is_current=false）。
    // 这样用户切到官方预设时，回填机制会保护原 live 配置不丢失。
    //
    // 捕获首次运行快照：所有全新装用户都会看到欢迎弹窗介绍 OpenSunstar 的工作方式。
    // 读失败时默认不弹，宁可漏弹也不要因为故障打扰用户。
    let first_run_already_confirmed = crate::settings::get_settings()
        .first_run_notice_confirmed
        .unwrap_or(false);
    let fresh_install_at_startup = app_state.db.is_providers_empty().unwrap_or(false);

    for app_type in crate::app_config::AppType::all().filter(|t| !t.is_additive_mode()) {
        if !crate::services::provider::should_import_default_config_on_startup(app_state, &app_type)
            .unwrap_or(false)
        {
            log::debug!(
                "○ {} already has providers; live import skipped",
                app_type.as_str()
            );
            continue;
        }

        match crate::services::provider::import_default_config(app_state, app_type.clone()) {
            Ok(true) => log::info!(
                "✓ Imported live config for {} as default provider",
                app_type.as_str()
            ),
            Ok(false) => log::debug!(
                "○ {} already has providers; live import skipped",
                app_type.as_str()
            ),
            Err(e) => log::debug!("○ No live config to import for {}: {e}", app_type.as_str()),
        }
    }

    match app_state.db.init_default_official_providers() {
        Ok(count) if count > 0 => {
            log::info!("✓ Seeded {count} official provider(s)");
        }
        Ok(_) => {}
        Err(e) => log::warn!("✗ Failed to seed official providers: {e}"),
    }

    // One-time migration: move any plaintext API keys still stored in the
    // providers table into the OS keychain, replacing them with
    // keychain://ref/ placeholders. Idempotent — safe to run every launch.
    {
        let db_for_keychain_migration = app_state.db.clone();
        tauri::async_runtime::spawn_blocking(move || {
            match crate::provider_keychain::migrate_all_providers_if_needed(
                &db_for_keychain_migration,
            ) {
                Ok(count) if count > 0 => {
                    log::info!("✓ Migrated {count} provider key(s) to OS keychain");
                }
                Ok(_) => {
                    log::debug!("○ No plaintext provider keys needed keychain migration");
                }
                Err(e) => {
                    log::warn!("✗ Provider keychain migration failed: {e}");
                }
            }
        });
    }

    // Idempotent migration for legacy MCP rows: move secret-bearing
    // env/header/query/arg/extension values to the OS keychain.
    {
        let db_for_mcp_secret_migration = app_state.db.clone();
        tauri::async_runtime::spawn_blocking(move || {
            match crate::mcp_secret::migrate_all_mcp_servers_if_needed(&db_for_mcp_secret_migration)
            {
                Ok(count) if count > 0 => {
                    log::info!("✓ Migrated {count} MCP server secret(s) to OS keychain");
                }
                Ok(_) => {
                    log::debug!("○ No plaintext MCP secrets needed keychain migration");
                }
                Err(e) => {
                    log::warn!("✗ MCP SecretRef migration failed: {e}");
                }
            }
        });
    }

    {
        let db_for_codex_history_migration = app_state.db.clone();
        tauri::async_runtime::spawn_blocking(move || {
            match crate::codex_history_migration::maybe_migrate_codex_third_party_history_provider_bucket(
                        &db_for_codex_history_migration,
                    ) {
                        Ok(outcome) => {
                            if let Some(reason) = outcome.skipped_reason {
                                log::debug!("○ Codex history provider bucket migration skipped: {reason}");
                            } else {
                                log::info!(
                                    "✓ Codex history provider bucket migration completed: sources={}, jsonl_files={}, state_rows={}",
                                    outcome.source_provider_ids.len(),
                                    outcome.migrated_jsonl_files,
                                    outcome.migrated_state_rows
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("✗ Codex history provider bucket migration failed: {e}");
                        }
                    }

            match crate::codex_history_migration::maybe_migrate_codex_provider_template_bucket(
                &db_for_codex_history_migration,
            ) {
                Ok(outcome) => {
                    if let Some(reason) = outcome.skipped_reason {
                        log::debug!("○ Codex provider template bucket migration skipped: {reason}");
                    } else if !outcome.migrated_provider_ids.is_empty() {
                        log::info!(
                            "✓ Codex provider template bucket migration completed: providers={}",
                            outcome.migrated_provider_ids.len()
                        );
                    }
                }
                Err(e) => {
                    log::warn!("✗ Codex provider template bucket migration failed: {e}");
                }
            }
        });
    }

    // 老用户 / 已确认的路径由 `fresh_install_at_startup` 自行拦截，这里不做写入。
    // 字段只由前端在用户点击"我知道了"时 save_settings 回写，语义是"用户显式确认过"。
    if !first_run_already_confirmed && fresh_install_at_startup {
        log::info!("✓ First-run welcome notice pending");
    }

    // 1.6. 自动同步 OpenCode / OpenClaw 的 live providers 到数据库
    //
    // additive 模式（OpenCode / OpenClaw）的 import 函数本身按 id 幂等，
    // 已有的 provider 会被跳过，所以每次启动都跑是安全的——既保证新装
    // 用户开箱可见 live 中的供应商，也让外部修改的 live 文件能在重启
    // 后同步到数据库（与之前依赖前端"导入当前配置"按钮手动触发不同）。
    //
    // 底层 read_*_config 在文件不存在时返回默认空配置，因此新装且无
    // live 文件的用户走 Ok(0) 路径，不会产生错误日志噪音。
    match crate::services::provider::import_opencode_providers_from_live(app_state) {
        Ok(count) if count > 0 => {
            log::info!("✓ Imported {count} OpenCode provider(s) from live config");
        }
        Ok(_) => log::debug!("○ No new OpenCode providers to import"),
        Err(e) => log::warn!("✗ Failed to import OpenCode providers: {e}"),
    }
    match crate::services::provider::import_openclaw_providers_from_live(app_state) {
        Ok(count) if count > 0 => {
            log::info!("✓ Imported {count} OpenClaw provider(s) from live config");
        }
        Ok(_) => log::debug!("○ No new OpenClaw providers to import"),
        Err(e) => log::warn!("✗ Failed to import OpenClaw providers: {e}"),
    }
    match crate::services::provider::import_hermes_providers_from_live(app_state) {
        Ok(count) if count > 0 => {
            log::info!("✓ Imported {count} Hermes provider(s) from live config");
        }
        Ok(_) => log::debug!("○ No new Hermes providers to import"),
        Err(e) => log::warn!("✗ Failed to import Hermes providers: {e}"),
    }

    // 2. OMO 配置导入（当数据库中无 OMO provider 时，从本地文件导入）
    {
        let has_omo = app_state
            .db
            .get_all_providers("opencode")
            .map(|providers| {
                providers
                    .values()
                    .any(|p| p.category.as_deref() == Some("omo"))
            })
            .unwrap_or(false);
        if !has_omo {
            match crate::services::OmoService::import_from_local(
                app_state,
                &crate::services::omo::STANDARD,
            ) {
                Ok(provider) => {
                    log::info!(
                        "✓ Imported OMO config from local as provider '{}'",
                        provider.name
                    );
                }
                Err(AppError::OmoConfigNotFound) => {
                    log::debug!("○ No OMO config to import");
                }
                Err(e) => {
                    log::warn!("✗ Failed to import OMO config from local: {e}");
                }
            }
        }
    }

    // 2.3 OMO Slim config import (when no omo-slim provider in DB, import from local)
    {
        let has_omo_slim = app_state
            .db
            .get_all_providers("opencode")
            .map(|providers| {
                providers
                    .values()
                    .any(|p| p.category.as_deref() == Some("omo-slim"))
            })
            .unwrap_or(false);
        if !has_omo_slim {
            match crate::services::OmoService::import_from_local(
                app_state,
                &crate::services::omo::SLIM,
            ) {
                Ok(provider) => {
                    log::info!(
                        "✓ Imported OMO Slim config from local as provider '{}'",
                        provider.name
                    );
                }
                Err(AppError::OmoConfigNotFound) => {
                    log::debug!("○ No OMO Slim config to import");
                }
                Err(e) => {
                    log::warn!("✗ Failed to import OMO Slim config from local: {e}");
                }
            }
        }
    }

    // 3. 导入 MCP 服务器配置（表空时触发）
    if app_state.db.is_mcp_table_empty().unwrap_or(false) {
        log::info!("MCP table empty, importing from live configurations...");

        match crate::services::mcp::McpService::import_from_claude(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from Claude");
            }
            Ok(_) => log::debug!("○ No Claude MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import Claude MCP: {e}"),
        }

        match crate::services::mcp::McpService::import_from_codex(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from Codex");
            }
            Ok(_) => log::debug!("○ No Codex MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import Codex MCP: {e}"),
        }

        match crate::services::mcp::McpService::import_from_gemini(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from Gemini");
            }
            Ok(_) => log::debug!("○ No Gemini MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import Gemini MCP: {e}"),
        }

        match crate::services::mcp::McpService::import_from_opencode(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from OpenCode");
            }
            Ok(_) => log::debug!("○ No OpenCode MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import OpenCode MCP: {e}"),
        }

        match crate::services::mcp::McpService::import_from_hermes(app_state) {
            Ok(count) if count > 0 => {
                log::info!("✓ Imported {count} MCP server(s) from Hermes");
            }
            Ok(_) => log::debug!("○ No Hermes MCP servers found to import"),
            Err(e) => log::warn!("✗ Failed to import Hermes MCP: {e}"),
        }
    }

    // 4. 导入提示词文件（按应用独立检查，已有提示词的应用自动跳过）
    {
        log::info!("Checking for prompt files to import from live configurations...");

        for app in [
            crate::app_config::AppType::Claude,
            crate::app_config::AppType::Codex,
            crate::app_config::AppType::Gemini,
            crate::app_config::AppType::OpenCode,
            crate::app_config::AppType::OpenClaw,
            crate::app_config::AppType::Hermes,
        ] {
            match crate::services::prompt::PromptService::import_from_file_on_first_launch(
                app_state, &app,
            ) {
                Ok(count) if count > 0 => {
                    log::info!("✓ Imported {count} prompt(s) for {}", app.as_str());
                }
                Ok(_) => {} // 该应用已有提示词或无文件，静默跳过
                Err(e) => log::warn!("✗ Failed to import prompt for {}: {e}", app.as_str()),
            }
        }
    }
}
