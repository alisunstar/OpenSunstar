//! 知识资产层 · 索引合并器（P1-2）
//!
//! 将 knowledge/ 正式区（main/、applications/）frontmatter 中**已声明**的 anchors/tags
//! 确定性合并进 knowledge/ROUTING.md 的受管表区。
//!
//! 边界（K1/K3）：只索引声明元数据，不读正文语义、不读 candidate/personal 未审区；
//! 无 LLM；受管标记外的人编内容永不覆写；二次运行幂等。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub const ROUTING_AUTO_BEGIN: &str = "<!-- opensunstar:routing-auto -->";
pub const ROUTING_AUTO_END: &str = "<!-- /opensunstar:routing-auto -->";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingMergeReport {
    pub pages_scanned: u32,
    pub anchors: u32,
    pub routing_written: bool,
}

/// 合并 knowledge/ 正式区锚点到 ROUTING.md 受管表区。
pub fn merge_routing_index(project_path: &str) -> Result<RoutingMergeReport, String> {
    let knowledge = PathBuf::from(project_path).join("knowledge");
    if !knowledge.is_dir() {
        return Err("knowledge/ 目录不存在（先安装 knowledge-baseline recipe）".into());
    }
    let routing_path = knowledge.join("ROUTING.md");
    if !routing_path.is_file() {
        return Err("knowledge/ROUTING.md 不存在".into());
    }

    let mut entries: Vec<(String, String, String)> = Vec::new();
    let mut pages = 0u32;
    for area in ["main", "applications"] {
        let root = knowledge.join(area);
        if root.is_dir() {
            collect(&root, &knowledge, &mut pages, &mut entries)?;
        }
    }
    entries.sort();
    entries.dedup();

    let table = build_table(&entries);
    let old = fs::read_to_string(&routing_path).map_err(|e| format!("读取 ROUTING 失败: {e}"))?;
    let new = replace_managed_section(&old, &table);
    let written = new != old;
    if written {
        fs::write(&routing_path, &new).map_err(|e| format!("写入 ROUTING 失败: {e}"))?;
    }
    Ok(RoutingMergeReport {
        pages_scanned: pages,
        anchors: entries.len() as u32,
        routing_written: written,
    })
}

fn collect(
    dir: &Path,
    base: &Path,
    pages: &mut u32,
    out: &mut Vec<(String, String, String)>,
) -> Result<(), String> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = fs::read_dir(&d).map_err(|e| format!("扫描失败: {e}"))?;
        for e in rd {
            let p = e.map_err(|e| format!("扫描失败: {e}"))?.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let content = fs::read_to_string(&p).unwrap_or_default();
            let fm = match frontmatter(&content) {
                Some(v) => v,
                None => continue,
            };
            *pages += 1;
            let rel = p
                .strip_prefix(base)
                .map(|x| x.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            for key in ["anchors", "tags"] {
                if let Some(seq) = fm.get(key).and_then(|v| v.as_sequence()) {
                    for item in seq {
                        if let Some(s) = item.as_str() {
                            let kind = s.split(':').next().unwrap_or("TAG").to_string();
                            out.push((kind, s.to_string(), rel.clone()));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn frontmatter(content: &str) -> Option<serde_yaml::Value> {
    let rest = content.trim_start().strip_prefix("---")?;
    let rest = rest.trim_start_matches(['\r', '\n']);
    let end = rest.find("\n---")?;
    serde_yaml::from_str(&rest[..end]).ok()
}

fn build_table(entries: &[(String, String, String)]) -> String {
    let mut s = String::from("| 锚点类型 | 锚点 | 知识入口 |\n|---------|------|---------|\n");
    for (k, a, p) in entries {
        s.push_str(&format!("| {k} | `{a}` | {p} |\n"));
    }
    s
}

/// 受管表区替换：标记外内容原样保留；无标记时追加到文末。幂等。
fn replace_managed_section(old: &str, table: &str) -> String {
    let body = format!(
        "{ROUTING_AUTO_BEGIN}\n## 自动路由表（os wiki routing 合并）\n\n{table}{ROUTING_AUTO_END}"
    );
    match (old.find(ROUTING_AUTO_BEGIN), old.find(ROUTING_AUTO_END)) {
        (Some(b), Some(e)) if b < e => {
            let e_end = e + ROUTING_AUTO_END.len();
            let tail = old[e_end..].trim_start_matches('\n');
            format!("{}{}\n{}", &old[..b], body, tail)
        }
        _ => format!("{old}\n\n{body}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("opensunstar-routing-{n}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("knowledge/main")).unwrap();
        fs::create_dir_all(dir.join("knowledge/applications/app-a")).unwrap();
        fs::create_dir_all(dir.join("knowledge/candidate")).unwrap();
        dir
    }

    #[test]
    fn merges_declared_anchors_idempotent_and_preserves_human_text() {
        let dir = temp_project();
        let k = dir.join("knowledge");
        fs::write(
            k.join("main/domain.md"),
            "---\ntitle: 域\nanchors:\n  - BIZ_IDENTITY:foo\ntags:\n  - logistics\n---\n# 域\n",
        )
        .unwrap();
        fs::write(
            k.join("applications/app-a/INDEX.md"),
            "---\ntitle: app-a\nanchors:\n  - APPLICATION:app-a\n---\n# app-a\n",
        )
        .unwrap();
        // 未审区锚点不得被合并
        fs::write(
            k.join("candidate/draft.md"),
            "---\nanchors:\n  - TOPIC:secret\n---\n# draft\n",
        )
        .unwrap();
        fs::write(k.join("ROUTING.md"), "# ROUTING\n\n人工编辑区：渐进式加载层级说明。\n").unwrap();

        let r1 = merge_routing_index(dir.to_str().unwrap()).unwrap();
        assert_eq!(r1.pages_scanned, 2);
        assert_eq!(r1.anchors, 3);
        assert!(r1.routing_written);
        let content = fs::read_to_string(k.join("ROUTING.md")).unwrap();
        assert!(content.contains("人工编辑区"));
        assert!(content.contains("BIZ_IDENTITY:foo"));
        assert!(content.contains("APPLICATION:app-a"));
        assert!(!content.contains("TOPIC:secret"));

        // 幂等：二次运行零写入
        let r2 = merge_routing_index(dir.to_str().unwrap()).unwrap();
        assert!(!r2.routing_written);
        let content2 = fs::read_to_string(k.join("ROUTING.md")).unwrap();
        assert_eq!(content, content2);
    }

    #[test]
    fn missing_knowledge_dir_errors() {
        let dir = std::env::temp_dir().join("opensunstar-routing-none");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(merge_routing_index(dir.to_str().unwrap()).is_err());
    }
}
