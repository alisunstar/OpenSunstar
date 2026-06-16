use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

const RESOURCE_LIBRARY_FILE: &str = "resource-library.json";
const RESOURCE_LIBRARY_TMP: &str = "resource-library.json.tmp";
const RESOURCE_LIBRARY_BAK: &str = "resource-library.json.bak";
const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceItem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub pinned: bool,
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLibraryFile {
    pub version: u32,
    pub items: Vec<ResourceItem>,
}

fn app_resource_library_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::storage::app_local_dir(app)?;
    Ok(dir.join(RESOURCE_LIBRARY_FILE))
}

fn default_library() -> ResourceLibraryFile {
    ResourceLibraryFile {
        version: CURRENT_VERSION,
        items: Vec::new(),
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for t in tags {
        let t = t.trim().to_string();
        if t.is_empty() {
            continue;
        }
        let key = t.to_lowercase();
        if seen.insert(key) {
            out.push(t);
        }
    }
    out
}

fn validate_and_normalize(mut lib: ResourceLibraryFile) -> Result<ResourceLibraryFile, String> {
    if lib.version == 0 {
        lib.version = CURRENT_VERSION;
    }
    if lib.version > CURRENT_VERSION {
        return Err(format!("资源库版本过高（{}），请升级应用。", lib.version));
    }
    lib.version = CURRENT_VERSION;

    for item in &mut lib.items {
        item.title = item.title.trim().to_string();
        item.url = item
            .url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        item.note = item.note.trim().to_string();
        item.tags = normalize_tags(std::mem::take(&mut item.tags));
        if item.id.trim().is_empty() {
            return Err("存在条目 id 为空".into());
        }
        if item.title.is_empty() {
            return Err(format!("条目 {} 缺少标题", item.id));
        }
        if item.updated_at <= 0 {
            item.updated_at = item.created_at;
        }
    }
    Ok(lib)
}

fn write_library_atomic(app: &AppHandle, lib: &ResourceLibraryFile) -> Result<(), String> {
    let path = app_resource_library_path(app)?;
    let parent = path
        .parent()
        .ok_or_else(|| "无法解析资源库目录".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let tmp = parent.join(RESOURCE_LIBRARY_TMP);
    let bak = parent.join(RESOURCE_LIBRARY_BAK);
    if path.exists() {
        let _ = fs::copy(&path, bak);
    }
    let json = serde_json::to_string_pretty(lib).map_err(|e| format!("序列化资源库失败：{e}"))?;
    fs::write(&tmp, json).map_err(|e| format!("写入临时文件失败：{e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("保存资源库失败：{e}"))?;
    Ok(())
}

pub fn load_resource_library(app: &AppHandle) -> Result<ResourceLibraryFile, String> {
    let path = app_resource_library_path(app)?;
    if !path.exists() {
        return Ok(default_library());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("读取资源库失败：{e}"))?;
    let lib: ResourceLibraryFile =
        serde_json::from_str(&text).map_err(|e| format!("解析资源库失败：{e}"))?;
    validate_and_normalize(lib)
}

pub fn save_resource_library(app: &AppHandle, lib: ResourceLibraryFile) -> Result<(), String> {
    let normalized = validate_and_normalize(lib)?;
    write_library_atomic(app, &normalized)
}
