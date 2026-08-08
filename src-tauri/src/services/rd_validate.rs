//! RD 过程资产层 · 对账确定性段（P1-4）
//!
//! `IMPLEMENTATION-CHECK.md` 五状态 schema 校验 + git diff 统计。
//! 纯确定性：无 LLM、无外部数据源、无 agent 循环（K1/K2/K3 合规）。
//! 语义段（五状态的真实判定）由外部 Agent 按 /rd:validate 协议执行，本模块只做 schema 与统计。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

/// 五状态枚举（与 rd-protocol/commands/validate.md 对齐）
pub const FIVE_STATES: &[&str] = &["done", "partial", "todo", "changed", "blocked"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffStat {
    pub file: String,
    pub added: u32,
    pub deleted: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RdValidateReport {
    pub change_id: String,
    pub schema_valid: bool,
    pub issues: Vec<String>,
    pub counts: BTreeMap<String, u32>,
    pub rows: u32,
    pub diff_stats: Vec<DiffStat>,
    pub diff_note: Option<String>,
}

/// 解析并校验 `.specs/<change-id>/IMPLEMENTATION-CHECK.md`。
///
/// 规则：
/// 1. 「对账结果」表每行必须 5 列（应用/契约项/状态/证据/缺口偏离说明）；
/// 2. 状态必须属于五状态枚举；
/// 3. 非 todo 行证据不得为空；
/// 4. 「汇总」段计数必须与表行统计一致。
pub fn validate_implementation_check(
    project_path: &str,
    change_id: &str,
) -> Result<RdValidateReport, String> {
    let path = Path::new(project_path)
        .join(".specs")
        .join(change_id)
        .join("IMPLEMENTATION-CHECK.md");
    if !path.is_file() {
        return Err(format!(
            "IMPLEMENTATION-CHECK.md 不存在: {}（先执行 /rd:validate）",
            path.display()
        ));
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {e}"))?;

    let mut issues: Vec<String> = Vec::new();
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for s in FIVE_STATES {
        counts.insert((*s).to_string(), 0);
    }
    let mut rows: u32 = 0;

    // 1. 对账结果表
    let table_rows = extract_section_table(&content, "对账结果");
    if table_rows.is_empty() {
        issues.push("缺少「对账结果」表或表为空".to_string());
    }
    for (idx, cols) in table_rows.iter().enumerate() {
        if cols.len() != 5 {
            issues.push(format!(
                "对账结果第 {} 行列数={}（应 5 列）",
                idx + 1,
                cols.len()
            ));
            continue;
        }
        let status = cols[2].trim().trim_matches('*').to_string();
        if !FIVE_STATES.contains(&status.as_str()) {
            issues.push(format!(
                "对账结果第 {} 行状态非法: {}（应 done/partial/todo/changed/blocked）",
                idx + 1,
                status
            ));
        } else {
            *counts.entry(status.clone()).or_insert(0) += 1;
            rows += 1;
        }
        let evidence = cols[3].trim();
        if evidence.is_empty() && status != "todo" {
            issues.push(format!(
                "对账结果第 {} 行证据为空（非 todo 状态必须给证据）",
                idx + 1
            ));
        }
    }

    // 2. 汇总段计数一致性
    let summary = extract_section_lines(&content, "汇总");
    for line in summary {
        let line = line.trim().trim_start_matches('-').trim();
        for s in FIVE_STATES {
            if let Some(rest) = line.strip_prefix(s) {
                let rest = rest.trim().trim_start_matches(':').trim();
                if let Ok(n) = rest.parse::<u32>() {
                    let actual = counts.get(*s).copied().unwrap_or(0);
                    if n != actual {
                        issues.push(format!("汇总 {s}={n} 与表行统计 {actual} 不一致"));
                    }
                }
            }
        }
    }

    // 3. git diff 统计（可选；失败仅记录 note）
    let (diff_stats, diff_note) = match git_numstat(project_path) {
        Ok(v) => (v, None),
        Err(e) => (Vec::new(), Some(e)),
    };

    Ok(RdValidateReport {
        change_id: change_id.to_string(),
        schema_valid: issues.is_empty(),
        issues,
        counts,
        rows,
        diff_stats,
        diff_note,
    })
}

/// 提取「## <name>」段内第一个 Markdown 表格的数据行（跳过表头与分隔行）。
fn extract_section_table(content: &str, name: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut in_section = false;
    let mut in_table = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            if in_section {
                break;
            }
            if t.trim_start_matches('#').trim().starts_with(name) {
                in_section = true;
            }
            continue;
        }
        if !in_section {
            continue;
        }
        if t.starts_with('|') {
            let cells: Vec<String> = t
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect();
            let is_sep = !cells.is_empty()
                && cells
                    .iter()
                    .all(|c| !c.is_empty() && c.replace('-', "").is_empty());
            let is_header = cells.first().map(|c| c.as_str()) == Some("应用");
            if is_sep || is_header {
                in_table = true;
                continue;
            }
            if in_table || !cells.is_empty() {
                out.push(cells);
                in_table = true;
            }
        } else if in_table && t.is_empty() {
            break;
        }
    }
    out
}

/// 提取「## <name>」段内的列表行。
fn extract_section_lines(content: &str, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            if in_section {
                break;
            }
            if t.trim_start_matches('#').trim().starts_with(name) {
                in_section = true;
            }
            continue;
        }
        if in_section && t.starts_with('-') {
            out.push(t.to_string());
        }
    }
    out
}

/// `git diff --numstat` 工作区统计（确定性、本地、可降级）。
fn git_numstat(project_path: &str) -> Result<Vec<DiffStat>, String> {
    let out = std::process::Command::new("git")
        .args(["-C", project_path, "diff", "--numstat"])
        .output()
        .map_err(|e| format!("git 不可用，跳过 diff 统计: {e}"))?;
    if !out.status.success() {
        return Err("git diff 执行失败，跳过 diff 统计".to_string());
    }
    let mut stats = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut it = line.split('\t');
        let (a, d, f) = match (it.next(), it.next(), it.next()) {
            (Some(a), Some(d), Some(f)) if !f.is_empty() => (a, d, f),
            _ => continue,
        };
        stats.push(DiffStat {
            file: f.to_string(),
            added: a.parse().unwrap_or(0),
            deleted: d.parse().unwrap_or(0),
        });
    }
    stats.sort_by_key(|stat| std::cmp::Reverse(stat.added + stat.deleted));
    stats.truncate(20);
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project() -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("opensunstar-rdvalidate-{n}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".specs/CHG-1")).unwrap();
        dir
    }

    fn write_check(dir: &Path, body: &str) {
        fs::write(dir.join(".specs/CHG-1/IMPLEMENTATION-CHECK.md"), body).unwrap();
    }

    const GOOD: &str = "# Implementation Check\n\n## 对账结果\n\n| 应用 | 契约项 | 状态 | 证据 | 缺口/偏离说明 |\n|------|--------|------|------|-------------|\n| app | R1 | done | cargo test 绿 | — |\n| app | R2 | todo |  | 未开始 |\n\n## 汇总\n\n- done: 1\n- partial: 0\n- todo: 1\n- changed: 0\n- blocked: 0\n";

    #[test]
    fn good_file_passes() {
        let dir = temp_project();
        write_check(&dir, GOOD);
        let r = validate_implementation_check(dir.to_str().unwrap(), "CHG-1").unwrap();
        assert!(r.schema_valid, "{:?}", r.issues);
        assert_eq!(r.rows, 2);
        assert_eq!(r.counts["done"], 1);
        assert_eq!(r.counts["todo"], 1);
    }

    #[test]
    fn bad_status_and_evidence_flagged() {
        let dir = temp_project();
        let bad = GOOD
            .replace("| done |", "| DONE-ISH |")
            .replace("| cargo test 绿 |", "|  |");
        write_check(&dir, &bad);
        let r = validate_implementation_check(dir.to_str().unwrap(), "CHG-1").unwrap();
        assert!(!r.schema_valid);
        assert!(r.issues.iter().any(|i| i.contains("状态非法")));
    }

    #[test]
    fn summary_mismatch_flagged() {
        let dir = temp_project();
        let bad = GOOD.replace("- done: 1", "- done: 9");
        write_check(&dir, &bad);
        let r = validate_implementation_check(dir.to_str().unwrap(), "CHG-1").unwrap();
        assert!(!r.schema_valid);
        assert!(r.issues.iter().any(|i| i.contains("不一致")));
    }

    #[test]
    fn missing_file_errors() {
        let dir = temp_project();
        assert!(validate_implementation_check(dir.to_str().unwrap(), "CHG-1").is_err());
    }
}
