//! 静态正则分析器
//!
//! 对文件内容逐行执行正则匹配，记录命中规则、行号、摘要。

use super::super::engine::Finding;
use super::super::rules::RuleSet;

pub fn scan(file_path: &str, content: &str, rule_set: &RuleSet, findings: &mut Vec<Finding>) {
    let applicable_rules = rule_set.for_file(file_path);

    if applicable_rules.is_empty() {
        return;
    }

    for (line_num, line) in content.lines().enumerate() {
        for cr in &applicable_rules {
            if cr.regex.is_match(line) {
                let snippet = truncate_snippet(line, cr.rule.snippet_len);
                findings.push(Finding {
                    rule_id: cr.rule.id.to_string(),
                    severity: cr.rule.severity,
                    category: cr.rule.category.to_string(),
                    file: file_path.to_string(),
                    line: line_num + 1, // 1-based
                    snippet,
                    message: cr.rule.message.to_string(),
                });
            }
        }
    }
}

fn truncate_snippet(s: &str, max_len: usize) -> String {
    let trimmed = s.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{engine::Severity, rules::RuleSet};
    use super::*;

    #[test]
    fn detects_hardcoded_aws_key() {
        let rs = RuleSet::new();
        let content = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let mut findings = vec![];
        scan("test.env", content, &rs, &mut findings);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn no_false_positive_on_normal_content() {
        let rs = RuleSet::new();
        let content = "# This is a normal skill description\nprint('Hello World')\necho 'Setting up environment...'";
        let mut findings = vec![];
        scan("SKILL.md", content, &rs, &mut findings);
        // 正常内容不应有 CRITICAL/HIGH 命中
        let critical_or_high: Vec<_> = findings
            .iter()
            .filter(|f| f.severity >= Severity::High)
            .collect();
        assert!(
            critical_or_high.is_empty(),
            "正常内容不应有 HIGH+ 命中: {:?}",
            critical_or_high
        );
    }
}
