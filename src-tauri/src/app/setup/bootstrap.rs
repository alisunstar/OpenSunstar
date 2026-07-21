//! Paths, logging, legacy migration, and database initialization.

use super::super::dialogs;
#[cfg(target_os = "windows")]
use super::super::set_windows_app_user_model_id;
use crate::store::AppState;
use crate::{app_store, panic_hook, usage_events};
use std::sync::Arc;
use tauri::Manager;

pub(super) fn initialize(app: &mut tauri::App) -> Result<AppState, Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    match app.path().resource_dir() {
        Ok(dir) => crate::services::design_system_registry::set_bundled_design_systems_dir(
            dir.join("resources").join("design-systems"),
        ),
        Err(error) => log::warn!("无法解析内置设计系统资源目录: {error}"),
    }

    // 预先刷新 Store 覆盖配置，确保后续路径读取正确（日志/数据库等）
    app_store::refresh_app_config_dir_override(app.handle());
    panic_hook::init_app_config_dir(crate::config::get_app_config_dir());
    #[cfg(target_os = "windows")]
    set_windows_app_user_model_id(app.handle());

    // 注册 Updater 插件（桌面端）
    #[cfg(desktop)]
    {
        if let Err(e) = app
            .handle()
            .plugin(tauri_plugin_updater::Builder::new().build())
        {
            // 若配置不完整（如缺少 pubkey），跳过 Updater 而不中断应用
            log::warn!("初始化 Updater 插件失败，已跳过：{e}");
        }
    }
    // 初始化日志（单文件输出到 <app_config_dir>/logs/OpenSunstar.log）
    {
        use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

        let log_dir = panic_hook::get_log_dir();

        // 确保日志目录存在
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            eprintln!("创建日志目录失败: {e}");
        }

        // 启动时删除旧日志文件，实现单文件覆盖效果
        let log_file_path = log_dir.join("OpenSunstar.log");
        let _ = std::fs::remove_file(&log_file_path);

        app.handle().plugin(
            tauri_plugin_log::Builder::default()
                // 初始化为 Trace，允许后续通过 log::set_max_level() 动态调整级别
                .level(log::LevelFilter::Trace)
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Folder {
                        path: log_dir,
                        file_name: Some("OpenSunstar".into()),
                    }),
                ])
                // 单文件模式：启动时删除旧文件，达到大小时轮转
                // 注意：KeepSome(n) 内部会做 n-2 运算，n=1 会导致 usize 下溢
                // KeepSome(2) 是最小安全值，表示不保留轮转文件
                .rotation_strategy(RotationStrategy::KeepSome(2))
                // 单文件大小限制 1GB
                .max_file_size(1024 * 1024 * 1024)
                .timezone_strategy(TimezoneStrategy::UseLocal)
                .build(),
        )?;
    }

    // 注入 AppHandle 给 usage_events，让无 AppHandle 持有的写日志路径
    // 也能向前端推送 `usage-log-recorded`。
    // 放在日志系统初始化之后，确保 init 的日志能正常输出。
    usage_events::init(app.handle().clone());
    crate::services::budget_alert::init(app.handle().clone());

    // 从旧版应用目录迁移数据到 OpenSunstar (~/.OpenSunstar/)
    // 必须在数据库初始化之前执行，确保用户数据完整迁移
    crate::config::migrate_from_legacy();

    // 初始化数据库
    let app_config_dir = crate::config::get_app_config_dir();
    let db_path = app_config_dir.join("OpenSunstar.db");
    let json_path = app_config_dir.join("config.json");

    // 检查是否需要从 config.json 迁移到 SQLite
    let has_json = json_path.exists();
    let has_db = db_path.exists();

    // 如果需要迁移，先验证 config.json 是否可以加载（在创建数据库之前）
    // 这样如果加载失败用户选择退出，数据库文件还没被创建，下次可以正常重试
    let migration_config = if !has_db && has_json {
        log::info!("检测到旧版配置文件，验证配置文件...");

        // 循环：支持用户重试加载配置文件
        loop {
            match crate::app_config::MultiAppConfig::load() {
                Ok(config) => {
                    log::info!("✓ 配置文件加载成功");
                    break Some(config);
                }
                Err(e) => {
                    log::error!("加载旧配置文件失败: {e}");
                    // 弹出系统对话框让用户选择
                    if !dialogs::show_migration_error_dialog(app.handle(), &e.to_string()) {
                        // 用户选择退出（此时数据库还没创建，下次启动可以重试）
                        log::info!("用户选择退出程序");
                        std::process::exit(1);
                    }
                    // 用户选择重试，继续循环
                    log::info!("用户选择重试加载配置文件");
                }
            }
        }
    } else {
        None
    };

    // 现在创建数据库（包含 Schema 迁移）
    //
    // 说明：从 v3.8.* 升级的用户通常会走到这里的 SQLite schema 迁移，
    // 若迁移失败（数据库损坏/权限不足/user_version 过新等），需要给用户明确提示，
    // 否则表现可能只是“应用打不开/闪退”。
    let db = loop {
        match crate::database::Database::init() {
            Ok(db) => break Arc::new(db),
            Err(e) => {
                log::error!("Failed to init database: {e}");

                if !dialogs::show_database_init_error_dialog(app.handle(), &db_path, &e.to_string())
                {
                    log::info!("用户选择退出程序");
                    std::process::exit(1);
                }

                log::info!("用户选择重试初始化数据库");
            }
        }
    };

    // 如果有预加载的配置，执行迁移
    if let Some(config) = migration_config {
        log::info!("开始执行数据迁移...");

        match db.migrate_from_json(&config) {
            Ok(_) => {
                log::info!("✓ 配置迁移成功");
                // 标记迁移成功，供前端显示 Toast
                crate::init_status::set_migration_success();
                // 归档旧配置文件（重命名而非删除，便于用户恢复）
                let archive_path = json_path.with_extension("json.migrated");
                if let Err(e) = std::fs::rename(&json_path, &archive_path) {
                    log::warn!("归档旧配置文件失败: {e}");
                } else {
                    log::info!("✓ 旧配置已归档为 config.json.migrated");
                }
            }
            Err(e) => {
                // 配置加载成功但迁移失败的情况极少（磁盘满等），仅记录日志
                log::error!("配置迁移失败: {e}，将从现有配置导入");
            }
        }
    }

    crate::services::budget_alert::init_db(db.clone());

    let app_state = AppState::new(db);

    // 设置 AppHandle 用于代理故障转移时的 UI 更新
    app_state.proxy_service.set_app_handle(app.handle().clone());
    Ok(app_state)
}
