use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 获取用户主目录，带回退和日志
///
/// ## Windows 注意事项
///
/// - `dirs::home_dir()` 在 Windows 上使用 `SHGetKnownFolderPath(FOLDERID_Profile)`，
///   返回的是真实用户目录（类似 `C:\\Users\\Alice`），与 v3.10.2 行为一致。
/// - 不要直接使用 `HOME` 环境变量：它可能由 Git/Cygwin/MSYS 等第三方工具注入，
///   且不一定等于用户目录，可能导致 `.OpenSunstar/OpenSunstar.db` 路径变化，从而"看起来像数据丢失"。
///
/// ## 测试隔离
///
/// 为了让 Windows CI/本地测试能稳定隔离真实用户数据，可通过 `OPEN_SUNSTAR_TEST_HOME`
/// 显式覆盖 home dir（仅用于测试/调试场景）。
pub fn get_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("OPEN_SUNSTAR_TEST_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    dirs::home_dir().unwrap_or_else(|| {
        log::warn!("无法获取用户主目录，回退到当前目录");
        PathBuf::from(".")
    })
}

/// 获取 Claude Code 配置目录路径
pub fn get_claude_config_dir() -> PathBuf {
    if let Some(custom) = crate::settings::get_claude_override_dir() {
        return custom;
    }

    get_home_dir().join(".claude")
}

/// 默认 Claude MCP 配置文件路径 (~/.claude.json)
pub fn get_default_claude_mcp_path() -> PathBuf {
    get_home_dir().join(".claude.json")
}

fn derive_mcp_path_from_override(dir: &Path) -> Option<PathBuf> {
    let file_name = dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())?
        .trim()
        .to_string();
    if file_name.is_empty() {
        return None;
    }
    let parent = dir.parent().unwrap_or_else(|| Path::new(""));
    Some(parent.join(format!("{file_name}.json")))
}

/// 获取 Claude MCP 配置文件路径，若设置了目录覆盖则与覆盖目录同级
pub fn get_claude_mcp_path() -> PathBuf {
    if let Some(custom_dir) = crate::settings::get_claude_override_dir() {
        if let Some(path) = derive_mcp_path_from_override(&custom_dir) {
            return path;
        }
    }
    get_default_claude_mcp_path()
}

/// 获取 Claude Code 主配置文件路径
pub fn get_claude_settings_path() -> PathBuf {
    let dir = get_claude_config_dir();
    let settings = dir.join("settings.json");
    if settings.exists() {
        return settings;
    }
    // 兼容旧版命名：若存在旧文件则继续使用
    let legacy = dir.join("claude.json");
    if legacy.exists() {
        return legacy;
    }
    // 默认新建：回落到标准文件名 settings.json（不再生成 claude.json）
    settings
}

/// 获取应用配置目录路径 (~/.OpenSunstar)
pub fn get_app_config_dir() -> PathBuf {
    if let Some(custom) = crate::app_store::get_app_config_dir_override() {
        return custom;
    }

    let default_dir = get_home_dir().join(".OpenSunstar");

    // 兼容旧版本：当用户环境存在 `HOME` 且与真实用户目录不同，
    // 旧版本可能在 `HOME/.OpenSunstar/` 下创建/使用了数据库。
    // 这里仅在"默认位置没有数据库"时回退到旧位置，避免再次出现"数据丢失"问题，
    // 同时也避免新安装因为 `HOME` 被设置而写入非预期路径。
    #[cfg(windows)]
    {
        let default_db = default_dir.join("OpenSunstar.db");
        if !default_db.exists() {
            if let Ok(home_env) = std::env::var("HOME") {
                let trimmed = home_env.trim();
                if !trimmed.is_empty() {
                    let legacy_dir = PathBuf::from(trimmed).join(".OpenSunstar");
                    if legacy_dir.join("OpenSunstar.db").exists() {
                        log::info!(
                            "Detected legacy database at {}, using it instead of {}",
                            legacy_dir.display(),
                            default_dir.display()
                        );
                        return legacy_dir;
                    }
                }
            }
        }
    }

    default_dir
}

/// 从旧版应用 `~/.cc-switch/` 目录迁移数据到 `~/.OpenSunstar/`。
///
/// 此函数在应用启动时、数据库初始化之前调用，确保用户从旧版应用升级到
/// OpenSunstar 后，所有数据（数据库、配置、技能等）都能被正确迁移到新目录。
///
/// 迁移策略：
/// - 仅当旧版目录存在且 `~/.OpenSunstar/` 中没有数据库时执行完整迁移
/// - 数据库已存在时，增量补充缺失的子目录（skills/ 等）
/// - 复制所有文件，旧数据库重命名为 `OpenSunstar.db`
/// - 不删除旧版目录（保留为备份）
/// - 迁移失败不会阻止应用启动，仅记录警告日志
pub fn migrate_from_legacy() {
    let new_dir = get_home_dir().join(".OpenSunstar");
    let legacy_dir = get_home_dir().join(".cc-switch");

    // 如果旧目录不存在，无需迁移
    if !legacy_dir.exists() {
        return;
    }

    // 确保新目录存在
    if let Err(e) = fs::create_dir_all(&new_dir) {
        log::warn!("Failed to create new config directory: {e}");
        return;
    }

    let new_db = new_dir.join("OpenSunstar.db");
    // 旧版数据库文件名（固定路径，用于识别和重命名）
    const LEGACY_DB_NAME: &str = "cc-switch.db";

    if !new_db.exists() {
        // 完整迁移：数据库还不存在，复制整个目录
        log::info!(
            "Detected legacy config directory at {}, migrating to {}",
            legacy_dir.display(),
            new_dir.display()
        );

        match copy_dir_recursive(&legacy_dir, &new_dir) {
            Ok(count) => {
                log::info!(
                    "Legacy migration complete: copied {count} entries from {} to {}",
                    legacy_dir.display(),
                    new_dir.display()
                );

                // 重命名数据库文件（如果存在）
                let legacy_db = new_dir.join(LEGACY_DB_NAME);
                if legacy_db.exists() && !new_db.exists() {
                    if let Err(e) = fs::rename(&legacy_db, &new_db) {
                        log::warn!("Failed to rename legacy database: {e}");
                    } else {
                        log::info!("Renamed legacy database → OpenSunstar.db");
                    }
                }
            }
            Err(e) => {
                log::warn!("Legacy migration failed: {e}");
            }
        }
    } else {
        log::info!(
            "Database already exists at {}, checking for missing subdirectories...",
            new_db.display()
        );

        // 增量迁移：数据库已存在，但 skills/ 和 skill-backups/ 等子目录可能缺失
        // 这些目录包含实际的技能文件，必须同步迁移
        let supplementary_dirs = ["skills", "skill-backups", "backups"];
        for subdir in &supplementary_dirs {
            let new_subdir = new_dir.join(subdir);
            let legacy_subdir = legacy_dir.join(subdir);

            if !new_subdir.exists() && legacy_subdir.exists() {
                log::info!(
                    "Migrating missing subdirectory: {} → {}",
                    legacy_subdir.display(),
                    new_subdir.display()
                );
                match copy_dir_recursive(&legacy_subdir, &new_subdir) {
                    Ok(count) => {
                        log::info!(
                            "✓ Migrated {subdir}/: copied {count} entries"
                        );
                    }
                    Err(e) => {
                        log::warn!("✗ Failed to migrate {subdir}/: {e}");
                    }
                }
            }
        }
    }
}

/// 递归复制目录内容（不删除源目录）
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<usize, AppError> {
    let mut count = 0;

    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| AppError::io(dst, e))?;
    }

    let entries = fs::read_dir(src).map_err(|e| AppError::io(src, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| AppError::io(src, e))?;
        let file_type = entry.file_type().map_err(|e| AppError::io(&entry.path(), e))?;
        let src_path = entry.path();
        let file_name = entry.file_name();

        // 跳过旧的数据库文件（将由调用者单独处理重命名）
        let dst_path = dst.join(&file_name);

        if file_type.is_dir() {
            count += copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            // 如果目标文件已存在则跳过（不覆盖）
            if dst_path.exists() {
                log::debug!("Skipping existing file: {}", dst_path.display());
                continue;
            }
            fs::copy(&src_path, &dst_path).map_err(|e| AppError::io(&src_path, e))?;
            count += 1;
        }
    }

    Ok(count)
}

/// 获取应用配置文件路径
pub fn get_app_config_path() -> PathBuf {
    get_app_config_dir().join("config.json")
}

/// 清理供应商名称，确保文件名安全
#[allow(dead_code)]
pub fn sanitize_provider_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// 获取供应商配置文件路径
#[allow(dead_code)]
pub fn get_provider_config_path(provider_id: &str, provider_name: Option<&str>) -> PathBuf {
    let base_name = provider_name
        .map(sanitize_provider_name)
        .unwrap_or_else(|| sanitize_provider_name(provider_id));

    get_claude_config_dir().join(format!("settings-{base_name}.json"))
}

/// 读取 JSON 配置文件
pub fn read_json_file<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, AppError> {
    if !path.exists() {
        return Err(AppError::Config(format!("文件不存在: {}", path.display())));
    }

    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;

    serde_json::from_str(&content).map_err(|e| AppError::json(path, e))
}

/// 递归排序 JSON 对象的键（按字母顺序），确保序列化输出是确定性的
fn sort_json_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted_map = Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted_map.insert(key.clone(), sort_json_keys(&map[key]));
            }
            Value::Object(sorted_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_json_keys).collect()),
        other => other.clone(),
    }
}

/// 写入 JSON 配置文件（键按字母排序，确保确定性输出）
pub fn write_json_file<T: Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let value = serde_json::to_value(data).map_err(|e| AppError::JsonSerialize { source: e })?;
    let sorted_value = sort_json_keys(&value);
    let json = serde_json::to_string_pretty(&sorted_value)
        .map_err(|e| AppError::JsonSerialize { source: e })?;

    atomic_write(path, json.as_bytes())
}

/// 原子写入文本文件（用于 TOML/纯文本）
pub fn write_text_file(path: &Path, data: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    atomic_write(path, data.as_bytes())
}

/// 原子写入：写入临时文件后 rename 替换，避免半写状态
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let parent = path
        .parent()
        .ok_or_else(|| AppError::Config("无效的路径".to_string()))?;
    let mut tmp = parent.to_path_buf();
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Config("无效的文件名".to_string()))?
        .to_string_lossy()
        .to_string();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    tmp.push(format!("{file_name}.tmp.{ts}"));

    {
        let mut f = fs::File::create(&tmp).map_err(|e| AppError::io(&tmp, e))?;
        f.write_all(data).map_err(|e| AppError::io(&tmp, e))?;
        f.flush().map_err(|e| AppError::io(&tmp, e))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let perm = meta.permissions().mode();
            let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(perm));
        }
    }

    #[cfg(windows)]
    {
        // Windows 上 rename 目标存在会失败，先移除再重命名（尽量接近原子性）
        if path.exists() {
            let _ = fs::remove_file(path);
        }
        fs::rename(&tmp, path).map_err(|e| AppError::IoContext {
            context: format!("原子替换失败: {} -> {}", tmp.display(), path.display()),
            source: e,
        })?;
    }

    #[cfg(not(windows))]
    {
        fs::rename(&tmp, path).map_err(|e| AppError::IoContext {
            context: format!("原子替换失败: {} -> {}", tmp.display(), path.display()),
            source: e,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_mcp_path_from_override_preserves_folder_name() {
        let override_dir = PathBuf::from("/tmp/profile/.claude");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for nested dir");
        assert_eq!(derived, PathBuf::from("/tmp/profile/.claude.json"));
    }

    #[test]
    fn derive_mcp_path_from_override_handles_non_hidden_folder() {
        let override_dir = PathBuf::from("/data/claude-config");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for standard dir");
        assert_eq!(derived, PathBuf::from("/data/claude-config.json"));
    }

    #[test]
    fn derive_mcp_path_from_override_supports_relative_rootless_dir() {
        let override_dir = PathBuf::from("claude");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for single segment");
        assert_eq!(derived, PathBuf::from("claude.json"));
    }

    #[test]
    fn derive_mcp_path_from_root_like_dir_returns_none() {
        let override_dir = PathBuf::from("/");
        assert!(derive_mcp_path_from_override(&override_dir).is_none());
    }

    #[test]
    fn sort_json_keys_sorts_top_level_object() {
        let input = serde_json::json!({
            "z": 1,
            "a": 2,
            "m": 3,
        });
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn sort_json_keys_recurses_into_nested_objects() {
        let input = serde_json::json!({
            "outer_b": {"z": 1, "a": 2},
            "outer_a": {"y": 3, "b": 4},
        });
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(
            serialized,
            r#"{"outer_a":{"b":4,"y":3},"outer_b":{"a":2,"z":1}}"#
        );
    }

    #[test]
    fn sort_json_keys_preserves_array_order() {
        let input = serde_json::json!([3, 1, 2]);
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, "[3,1,2]");
    }

    #[test]
    fn sort_json_keys_sorts_objects_inside_arrays_but_keeps_array_order() {
        let input = serde_json::json!([
            {"z": 1, "a": 2},
            {"y": 3, "b": 4},
        ]);
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, r#"[{"a":2,"z":1},{"b":4,"y":3}]"#);
    }

    #[test]
    fn sort_json_keys_passes_through_primitives() {
        let cases = vec![
            serde_json::json!("hello"),
            serde_json::json!(42),
            serde_json::json!(3.5),
            serde_json::json!(true),
            serde_json::json!(null),
        ];
        for value in cases {
            let sorted = sort_json_keys(&value);
            assert_eq!(sorted, value);
        }
    }

    #[test]
    fn sort_json_keys_handles_empty_collections() {
        let empty_obj = serde_json::json!({});
        assert_eq!(
            serde_json::to_string(&sort_json_keys(&empty_obj)).unwrap(),
            "{}"
        );

        let empty_arr = serde_json::json!([]);
        assert_eq!(
            serde_json::to_string(&sort_json_keys(&empty_arr)).unwrap(),
            "[]"
        );
    }

    #[test]
    fn sort_json_keys_produces_identical_output_for_different_insertion_orders() {
        // 核心保证：同一逻辑配置无论键的插入顺序如何，写出的字节序列必须一致。
        let mut a = Map::new();
        a.insert("env".to_string(), serde_json::json!({"PATH": "/usr/bin"}));
        a.insert("model".to_string(), serde_json::json!("claude-sonnet-4-5"));
        a.insert("permissions".to_string(), serde_json::json!({"allow": []}));

        let mut b = Map::new();
        b.insert("permissions".to_string(), serde_json::json!({"allow": []}));
        b.insert("model".to_string(), serde_json::json!("claude-sonnet-4-5"));
        b.insert("env".to_string(), serde_json::json!({"PATH": "/usr/bin"}));

        let sorted_a = sort_json_keys(&Value::Object(a));
        let sorted_b = sort_json_keys(&Value::Object(b));

        assert_eq!(
            serde_json::to_string(&sorted_a).unwrap(),
            serde_json::to_string(&sorted_b).unwrap(),
        );
    }

    #[test]
    fn copy_dir_recursive_copies_files_and_subdirs() {
        let tmp = std::env::temp_dir().join("test_copy_dir_recursive");
        let src = tmp.join("src");
        let dst = tmp.join("dst");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(src.join("subdir")).unwrap();
        fs::write(src.join("a.txt"), "hello").unwrap();
        fs::write(src.join("subdir").join("b.txt"), "world").unwrap();

        let count = super::copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(count, 2); // 2 files
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            fs::read_to_string(dst.join("subdir").join("b.txt")).unwrap(),
            "world"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn copy_dir_recursive_skips_existing_files() {
        let tmp = std::env::temp_dir().join("test_copy_dir_skip_existing");
        let src = tmp.join("src");
        let dst = tmp.join("dst");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join("a.txt"), "new content").unwrap();
        fs::write(dst.join("a.txt"), "old content").unwrap();

        let count = super::copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(count, 0); // already exists, skipped
        assert_eq!(
            fs::read_to_string(dst.join("a.txt")).unwrap(),
            "old content"
        );

        let _ = fs::remove_dir_all(&tmp);
    }
}

/// 复制文件
pub fn copy_file(from: &Path, to: &Path) -> Result<(), AppError> {
    fs::copy(from, to).map_err(|e| AppError::IoContext {
        context: format!("复制文件失败 ({} -> {})", from.display(), to.display()),
        source: e,
    })?;
    Ok(())
}

/// 删除文件
pub fn delete_file(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        fs::remove_file(path).map_err(|e| AppError::io(path, e))?;
    }
    Ok(())
}

/// 检查 Claude Code 配置状态
#[derive(Serialize, Deserialize)]
pub struct ConfigStatus {
    pub exists: bool,
    pub path: String,
}

/// 获取 Claude Code 配置状态
pub fn get_claude_config_status() -> ConfigStatus {
    let path = get_claude_settings_path();
    ConfigStatus {
        exists: path.exists(),
        path: path.to_string_lossy().to_string(),
    }
}
