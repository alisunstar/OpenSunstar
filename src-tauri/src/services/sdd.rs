//! SDD framework detection: read-only probes for 7 methodology frameworks.
//!
//! All probe operations are **read-only**: `fs::metadata`, `fs::read_to_string`.
//! No install commands, no file writes, no project directory modifications.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::error::AppError;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddDescriptorSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub phase_model: String,
    pub install_type: String,
    pub description_zh: Option<String>,
    pub description_en: Option<String>,
    pub repo_url: Option<String>,
    pub star_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalMatch {
    pub signal: String,
    pub matched_path: String,
    pub confidence: String, // "verified" | "inferred" | "absent"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddDetectionResult {
    pub descriptor_id: String,
    pub detected: bool,
    pub confidence: String,
    pub signal_matches: Vec<SignalMatch>,
}

// ─── Detector trait ──────────────────────────────────────────────────────────

trait FrameworkDetector {
    fn id(&self) -> &str;
    fn probe(&self, project_path: &Path) -> Vec<SignalMatch>;
}

// ─── 7 framework detectors ───────────────────────────────────────────────────

struct BmadDetector;
impl FrameworkDetector for BmadDetector {
    fn id(&self) -> &str { "bmad-method" }
    fn probe(&self, project_path: &Path) -> Vec<SignalMatch> {
        let mut matches = Vec::new();
        if project_path.join(".bmad").is_dir() {
            matches.push(SignalMatch {
                signal: ".bmad/ directory".into(),
                matched_path: project_path.join(".bmad").display().to_string(),
                confidence: "verified".into(),
            });
        }
        if let Some(m) = check_package_json_dep(project_path, "bmad-method") {
            matches.push(m);
        }
        matches
    }
}

struct TaskMasterDetector;
impl FrameworkDetector for TaskMasterDetector {
    fn id(&self) -> &str { "task-master" }
    fn probe(&self, project_path: &Path) -> Vec<SignalMatch> {
        let mut matches = Vec::new();
        if let Some(m) = check_package_json_dep(project_path, "task-master-ai") {
            matches.push(m);
        }
        if let Some(m) = check_package_json_dep(project_path, "task-master") {
            matches.push(m);
        }
        let task_dir = project_path.join(".task-master");
        if task_dir.is_dir() {
            matches.push(SignalMatch {
                signal: ".task-master/ directory".into(),
                matched_path: task_dir.display().to_string(),
                confidence: "inferred".into(),
            });
        }
        matches
    }
}

struct SuperpowersDetector;
impl FrameworkDetector for SuperpowersDetector {
    fn id(&self) -> &str { "superpowers" }
    fn probe(&self, project_path: &Path) -> Vec<SignalMatch> {
        let mut matches = Vec::new();
        let sp_dir = project_path.join(".superpowers");
        if sp_dir.is_dir() {
            matches.push(SignalMatch {
                signal: ".superpowers/ directory".into(),
                matched_path: sp_dir.display().to_string(),
                confidence: "verified".into(),
            });
        }
        if let Some(m) = check_package_json_dep(project_path, "superpowers") {
            matches.push(m);
        }
        // AGENTS.md reference check
        let agents_md = project_path.join("AGENTS.md");
        if agents_md.is_file() {
            if let Ok(content) = std::fs::read_to_string(&agents_md) {
                if content.to_lowercase().contains("superpowers") {
                    matches.push(SignalMatch {
                        signal: "AGENTS.md references superpowers".into(),
                        matched_path: agents_md.display().to_string(),
                        confidence: "inferred".into(),
                    });
                }
            }
        }
        matches
    }
}

struct GstackDetector;
impl FrameworkDetector for GstackDetector {
    fn id(&self) -> &str { "gstack" }
    fn probe(&self, project_path: &Path) -> Vec<SignalMatch> {
        let mut matches = Vec::new();
        let gstack_dir = project_path.join(".gstack");
        if gstack_dir.is_dir() {
            matches.push(SignalMatch {
                signal: ".gstack/ directory".into(),
                matched_path: gstack_dir.display().to_string(),
                confidence: "verified".into(),
            });
        }
        for fname in &["SKILL.md", "CLAUDE.md", "AGENTS.md"] {
            let fpath = project_path.join(fname);
            if fpath.is_file() {
                if let Ok(content) = std::fs::read_to_string(&fpath) {
                    if content.to_lowercase().contains("gstack") {
                        matches.push(SignalMatch {
                            signal: format!("{fname} references gstack"),
                            matched_path: fpath.display().to_string(),
                            confidence: "inferred".into(),
                        });
                    }
                }
            }
        }
        matches
    }
}

struct OpenSpecDetector;
impl FrameworkDetector for OpenSpecDetector {
    fn id(&self) -> &str { "openspec" }
    fn probe(&self, project_path: &Path) -> Vec<SignalMatch> {
        let mut matches = Vec::new();
        let os_dir = project_path.join(".openspec");
        if os_dir.is_dir() {
            matches.push(SignalMatch {
                signal: ".openspec/ directory".into(),
                matched_path: os_dir.display().to_string(),
                confidence: "verified".into(),
            });
        }
        matches
    }
}

struct SpecKitDetector;
impl FrameworkDetector for SpecKitDetector {
    fn id(&self) -> &str { "spec-kit" }
    fn probe(&self, project_path: &Path) -> Vec<SignalMatch> {
        let mut matches = Vec::new();
        if project_path.join(".spec-kit").is_dir() {
            matches.push(SignalMatch {
                signal: ".spec-kit/ directory".into(),
                matched_path: project_path.join(".spec-kit").display().to_string(),
                confidence: "verified".into(),
            });
        }
        if let Some(m) = check_package_json_dep(project_path, "spec-kit") {
            matches.push(m);
        }
        matches
    }
}

struct FlowKitDetector;
impl FrameworkDetector for FlowKitDetector {
    fn id(&self) -> &str { "flow-kit" }
    fn probe(&self, project_path: &Path) -> Vec<SignalMatch> {
        let mut matches = Vec::new();
        let fk_dir = project_path.join("flow-kit");
        let go_md = fk_dir.join("GO.md");
        if fk_dir.is_dir() && go_md.is_file() {
            matches.push(SignalMatch {
                signal: "flow-kit/ directory + GO.md".into(),
                matched_path: go_md.display().to_string(),
                confidence: "verified".into(),
            });
        } else if fk_dir.is_dir() {
            matches.push(SignalMatch {
                signal: "flow-kit/ directory (no GO.md)".into(),
                matched_path: fk_dir.display().to_string(),
                confidence: "inferred".into(),
            });
        }
        matches
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn check_package_json_dep(project_path: &Path, dep_name: &str) -> Option<SignalMatch> {
    let pkg_path = project_path.join("package.json");
    if !pkg_path.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(&pkg_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Check all standard dependency sections for exact key match
    let dep_sections = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];
    let found_in = dep_sections.iter().find(|section| {
        json.get(section)
            .and_then(|v| v.as_object())
            .map_or(false, |obj| obj.contains_key(dep_name))
    });

    found_in.map(|section| SignalMatch {
        signal: format!("{dep_name} in package.json {section}"),
        matched_path: pkg_path.display().to_string(),
        confidence: "inferred".into(),
    })
}

fn all_detectors() -> Vec<Box<dyn FrameworkDetector>> {
    vec![
        Box::new(BmadDetector),
        Box::new(TaskMasterDetector),
        Box::new(SuperpowersDetector),
        Box::new(GstackDetector),
        Box::new(OpenSpecDetector),
        Box::new(SpecKitDetector),
        Box::new(FlowKitDetector),
    ]
}

// ─── Public API ──────────────────────────────────────────────────────────────

pub fn list_descriptors(db: &Arc<Database>) -> Result<Vec<SddDescriptorSummary>, AppError> {
    db.sdd_list_descriptors()
}

/// Run all 7 detectors against a single project directory. Read-only.
pub fn detect_project(project_path: &str) -> Vec<SddDetectionResult> {
    let path = PathBuf::from(project_path);
    all_detectors()
        .iter()
        .map(|d| {
            let signals = d.probe(&path);
            let detected = !signals.is_empty();
            let confidence = if signals.iter().any(|s| s.confidence == "verified") {
                "verified"
            } else if detected {
                "inferred"
            } else {
                "absent"
            };
            SddDetectionResult {
                descriptor_id: d.id().to_string(),
                detected,
                confidence: confidence.to_string(),
                signal_matches: signals,
            }
        })
        .collect()
}

pub fn save_detection_results(
    db: &Arc<Database>,
    project_id: &str,
    results: &[SddDetectionResult],
) -> Result<(), AppError> {
    db.sdd_save_detection_results(project_id, results)
}

pub fn get_detection_results(
    db: &Arc<Database>,
    project_id: &str,
) -> Result<Vec<SddDetectionResult>, AppError> {
    db.sdd_get_detection_results(project_id)
}

pub fn detect_all_projects(
    db: &Arc<Database>,
) -> Result<HashMap<String, Vec<SddDetectionResult>>, AppError> {
    let projects = db.sdd_list_all_projects()?;
    let mut result_map = HashMap::new();
    for (project_id, project_path) in &projects {
        let results = detect_project(project_path);
        let _ = save_detection_results(db, project_id, &results);
        result_map.insert(project_id.clone(), results);
    }
    Ok(result_map)
}

/// Load persisted detection results for all projects that have been scanned before.
pub fn get_all_saved_detections(
    db: &Arc<Database>,
) -> Result<HashMap<String, Vec<SddDetectionResult>>, AppError> {
    let projects = db.sdd_list_all_projects()?;
    let mut result_map = HashMap::new();
    for (project_id, _) in projects {
        let results = get_detection_results(db, &project_id)?;
        if !results.is_empty() {
            result_map.insert(project_id, results);
        }
    }
    Ok(result_map)
}

/// Recommend a preset based on detection results (Track A → B linkage).
pub fn recommend_preset_from_detections(results: &[SddDetectionResult]) -> Option<String> {
    let detected_ids: Vec<&str> = results
        .iter()
        .filter(|r| r.detected)
        .map(|r| r.descriptor_id.as_str())
        .collect();

    if detected_ids.is_empty() {
        return None;
    }

    if detected_ids.contains(&"flow-kit") {
        Some("full".into())
    } else if detected_ids.contains(&"spec-kit") || detected_ids.contains(&"openspec") {
        Some("standard".into())
    } else if detected_ids.contains(&"bmad-method") || detected_ids.contains(&"gstack") {
        Some("standard".into())
    } else if detected_ids.contains(&"superpowers") || detected_ids.contains(&"task-master") {
        Some("mvp".into())
    } else {
        Some("review-only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use std::sync::atomic::{AtomicU64, Ordering};
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "sdd-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            n,
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn flow_kit_detected() {
        let root = temp_dir();
        let fk = root.join("flow-kit");
        fs::create_dir_all(&fk).unwrap();
        fs::write(fk.join("GO.md"), "# Flow Kit GO").unwrap();
        let results = detect_project(root.to_str().unwrap());
        let fk_result = results.iter().find(|r| r.descriptor_id == "flow-kit").unwrap();
        assert!(fk_result.detected);
        assert_eq!(fk_result.confidence, "verified");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn openspec_detected() {
        let root = temp_dir();
        fs::create_dir_all(root.join(".openspec")).unwrap();
        let results = detect_project(root.to_str().unwrap());
        let os_result = results.iter().find(|r| r.descriptor_id == "openspec").unwrap();
        assert!(os_result.detected);
        assert_eq!(os_result.confidence, "verified");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_detected_in_empty_dir() {
        let root = temp_dir();
        let results = detect_project(root.to_str().unwrap());
        assert_eq!(results.len(), 7);
        assert!(results.iter().all(|r| !r.detected));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn package_json_dep_detected() {
        let root = temp_dir();
        fs::write(
            root.join("package.json"),
            r#"{"dependencies": {"spec-kit": "^0.12.0"}}"#,
        )
        .unwrap();
        let results = detect_project(root.to_str().unwrap());
        let sk_result = results.iter().find(|r| r.descriptor_id == "spec-kit").unwrap();
        assert!(sk_result.detected);
        assert_eq!(sk_result.confidence, "inferred");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn recommend_preset_flow_kit() {
        let results = vec![SddDetectionResult {
            descriptor_id: "flow-kit".into(),
            detected: true,
            confidence: "verified".into(),
            signal_matches: vec![],
        }];
        assert_eq!(recommend_preset_from_detections(&results), Some("full".into()));
    }

    #[test]
    fn recommend_preset_none_detected() {
        let results: Vec<SddDetectionResult> = vec![];
        assert_eq!(recommend_preset_from_detections(&results), None);
    }
}
