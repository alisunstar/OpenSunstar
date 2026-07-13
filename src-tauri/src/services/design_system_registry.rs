#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_keeps_builtin_when_user_package_reuses_its_id() {
        let root = tempfile::tempdir().unwrap();
        let builtin = root.path().join("builtin");
        let user = root.path().join("user");
        write_test_package(&builtin, "web-product-core", "Web 产品核心", "MIT");
        write_test_package(&user, "web-product-core", "用户同名包", "MIT");

        let result = discover_from_dirs(&builtin, &user).unwrap();

        assert_eq!(result.packages.len(), 1);
        assert_eq!(result.packages[0].source, "builtin");
        assert_eq!(result.rejected.len(), 1);
    }

    #[test]
    fn discovery_rejects_package_without_required_license_or_assets() {
        let root = tempfile::tempdir().unwrap();
        let builtin = root.path().join("builtin");
        let user = root.path().join("user");
        std::fs::create_dir_all(user.join("broken")).unwrap();
        std::fs::write(
            user.join("broken/manifest.json"),
            r#"{"schemaVersion":1,"id":"broken","name":"Broken"}"#,
        )
        .unwrap();

        let result = discover_from_dirs(&builtin, &user).unwrap();

        assert!(result.packages.is_empty());
        assert_eq!(result.rejected.len(), 1);
    }

    #[test]
    fn bundled_offline_packages_are_all_discoverable_and_loadable() {
        let packages = discover_from_dirs(
            &bundled_design_systems_dir(),
            Path::new("__missing_user_packages__"),
        )
        .unwrap();
        assert_eq!(packages.packages.len(), 8);
        assert!(packages.rejected.is_empty());
        for package in packages.packages {
            let path = bundled_design_systems_dir()
                .join(&package.id)
                .join("tokens.json");
            let _: DesignContract =
                serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        }
    }

    #[test]
    fn package_detail_exposes_components_templates_and_accessibility_rules() {
        let detail = load_design_system_package_detail("enterprise-admin").unwrap();
        assert!(detail.components["pageTemplates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "列表筛选"));
        assert!(detail.responsive["modes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "dark"));
        assert!(detail.accessibility.contains("Accessibility"));
    }
}
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::services::design_contract::DesignContract;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSystemPackage {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub license_id: String,
    pub applicable_scenarios: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedDesignSystemPackage {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSystemDiscovery {
    pub packages: Vec<DesignSystemPackage>,
    pub rejected: Vec<RejectedDesignSystemPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSystemPackageDetail {
    pub package: DesignSystemPackage,
    pub components: serde_json::Value,
    pub responsive: serde_json::Value,
    pub accessibility: String,
}

const REQUIRED_FILES: &[&str] = &[
    "tokens.json",
    "components.json",
    "responsive.json",
    "a11y.md",
];
static BUNDLED_DIR_OVERRIDE: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();

pub fn set_bundled_design_systems_dir(path: PathBuf) {
    let cache = BUNDLED_DIR_OVERRIDE.get_or_init(|| RwLock::new(None));
    if let Ok(mut value) = cache.write() {
        *value = Some(path);
    }
}

pub fn bundled_design_systems_dir() -> PathBuf {
    if let Some(cache) = BUNDLED_DIR_OVERRIDE.get() {
        if let Ok(Some(path)) = cache.read().map(|value| value.clone()) {
            return path;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("design-systems")
}

pub fn user_design_systems_dir() -> PathBuf {
    crate::config::get_app_config_dir().join("design-systems")
}

pub fn discover_design_systems() -> Result<DesignSystemDiscovery, AppError> {
    discover_from_dirs(&bundled_design_systems_dir(), &user_design_systems_dir())
}

/// Load the contract payload of a discovered offline package. Package assets are
/// data only; no code is evaluated while loading.
pub fn load_design_system_contract(id: &str) -> Result<DesignContract, AppError> {
    let discovery = discover_design_systems()?;
    let package = discovery
        .packages
        .into_iter()
        .find(|package| package.id == id)
        .ok_or_else(|| AppError::Message(format!("设计系统包不存在: {id}")))?;
    let root = if package.source == "builtin" {
        bundled_design_systems_dir()
    } else {
        user_design_systems_dir()
    };
    let path = root.join(id).join("tokens.json");
    let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    serde_json::from_str(&content)
        .map_err(|e| AppError::Message(format!("设计系统包 tokens.json 无效: {e}")))
}

pub fn load_design_system_package_detail(id: &str) -> Result<DesignSystemPackageDetail, AppError> {
    let discovery = discover_design_systems()?;
    let package = discovery
        .packages
        .into_iter()
        .find(|package| package.id == id)
        .ok_or_else(|| AppError::Message(format!("设计系统包不存在: {id}")))?;
    let root = if package.source == "builtin" {
        bundled_design_systems_dir()
    } else {
        user_design_systems_dir()
    }
    .join(id);
    let read_json = |name: &str| -> Result<serde_json::Value, AppError> {
        let path = root.join(name);
        serde_json::from_str(&fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?)
            .map_err(|e| AppError::Message(format!("设计系统包 {name} 无效: {e}")))
    };
    let a11y_path = root.join("a11y.md");
    Ok(DesignSystemPackageDetail {
        package,
        components: read_json("components.json")?,
        responsive: read_json("responsive.json")?,
        accessibility: fs::read_to_string(&a11y_path).map_err(|e| AppError::io(&a11y_path, e))?,
    })
}

pub fn discover_from_dirs(
    builtin_dir: &Path,
    user_dir: &Path,
) -> Result<DesignSystemDiscovery, AppError> {
    let mut packages = Vec::new();
    let mut rejected = Vec::new();
    scan_dir(builtin_dir, "builtin", &mut packages, &mut rejected)?;
    scan_dir(user_dir, "user", &mut packages, &mut rejected)?;
    packages.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(DesignSystemDiscovery { packages, rejected })
}

fn scan_dir(
    dir: &Path,
    source: &str,
    packages: &mut Vec<DesignSystemPackage>,
    rejected: &mut Vec<RejectedDesignSystemPackage>,
) -> Result<(), AppError> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| AppError::io(dir, e))? {
        let path = entry.map_err(|e| AppError::io(dir, e))?.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        let result = (|| -> Result<DesignSystemPackage, String> {
            let content =
                fs::read_to_string(&manifest_path).map_err(|_| "缺少 manifest.json".to_string())?;
            let mut package: DesignSystemPackage =
                serde_json::from_str(&content).map_err(|_| "manifest.json 无效".to_string())?;
            if package.schema_version != 1
                || package.id.trim().is_empty()
                || package.license_id.trim().is_empty()
                || package.applicable_scenarios.is_empty()
            {
                return Err("manifest 缺少必需字段".into());
            }
            if REQUIRED_FILES.iter().any(|file| !path.join(file).is_file()) {
                return Err("缺少设计包必需资产".into());
            }
            if packages.iter().any(|existing| existing.id == package.id) {
                return Err("包 ID 与已发现的内置包冲突".into());
            }
            package.source = source.into();
            Ok(package)
        })();
        match result {
            Ok(package) => packages.push(package),
            Err(reason) => rejected.push(RejectedDesignSystemPackage {
                path: path.display().to_string(),
                reason,
            }),
        }
    }
    Ok(())
}

#[cfg(test)]
fn write_test_package(parent: &Path, id: &str, name: &str, license: &str) {
    let path = parent.join(id);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("manifest.json"), format!(r#"{{"schemaVersion":1,"id":"{id}","name":"{name}","version":"1.0.0","licenseId":"{license}","applicableScenarios":["web"],"source":""}}"#)).unwrap();
    for file in REQUIRED_FILES {
        fs::write(path.join(file), "{}").unwrap();
    }
}
