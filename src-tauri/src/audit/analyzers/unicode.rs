//! Unicode 隐蔽字符检测器
//!
//! 检测：
//! - Unicode 标签字符 U+E0001–E007F
//! - 零宽字符 (U+200B-U+200F)
//! - Bidi 覆盖字符 (CVE-2021-42574 Trojan Source)

use super::super::engine::{Finding, Severity};

pub fn scan(file_path: &str, content: &str, findings: &mut Vec<Finding>) {
    for (line_num, line) in content.lines().enumerate() {
        detect_tag_chars(file_path, line_num, line, findings);
        detect_zero_width(file_path, line_num, line, findings);
        detect_bidi(file_path, line_num, line, findings);
    }
}

/// 检测 Unicode 标签字符 (U+E0001–E007F)
fn detect_tag_chars(file_path: &str, line_num: usize, line: &str, findings: &mut Vec<Finding>) {
    let mut start = 0;
    while let Some(pos) = line[start..].char_indices().find(|(_, c)| {
        let cp = *c as u32;
        (0xE0001..=0xE007F).contains(&cp)
    }) {
        let abs_pos = start + pos.0;
        findings.push(Finding {
            rule_id: "hidden-unicode-tag".into(),
            severity: Severity::Critical,
            category: "hidden-unicode".into(),
            file: file_path.into(),
            line: line_num + 1,
            snippet: format!(
                "...{}... (U+{:04X})",
                &line[abs_pos.saturating_sub(10)..(abs_pos + 10).min(line.len())],
                line[abs_pos..].chars().next().unwrap() as u32
            ),
            message: "检测到 Unicode 标签字符 — 可能用于隐藏恶意代码".into(),
        });
        start = abs_pos
            + line[abs_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
    }
}

/// 检测零宽字符
fn detect_zero_width(file_path: &str, line_num: usize, line: &str, findings: &mut Vec<Finding>) {
    let zero_width_ranges: [(u32, u32, &str); 5] = [
        (0x200B, 0x200F, "零宽空格/连接符"),
        (0x2060, 0x2064, "词连接符"),
        (0xFEFF, 0xFEFF, "BOM/零宽不中断空格"),
        (0x00AD, 0x00AD, "软连字符"),
        (0x180E, 0x180E, "蒙古文元音分隔符"),
    ];

    let mut found = false;
    let mut chars_found = Vec::new();

    for (_i, c) in line.char_indices() {
        let cp = c as u32;
        for &(lo, hi, desc) in &zero_width_ranges {
            if cp >= lo && cp <= hi {
                found = true;
                chars_found.push(format!("U+{cp:04X} ({desc})"));
                break;
            }
        }
    }

    if found {
        findings.push(Finding {
            rule_id: "hidden-unicode-zero-width".into(),
            severity: Severity::High,
            category: "hidden-unicode".into(),
            file: file_path.into(),
            line: line_num + 1,
            snippet: format!("发现零宽字符: {}", chars_found.join(", ")),
            message: "检测到零宽字符 — 可能用于隐藏恶意指令 (CVE-2021-42574)".into(),
        });
    }
}

/// 检测 Bidi 覆盖字符 (Trojan Source)
fn detect_bidi(file_path: &str, line_num: usize, line: &str, findings: &mut Vec<Finding>) {
    let bidi_chars = [
        (0x202A, "LEFT-TO-RIGHT EMBEDDING"),
        (0x202B, "RIGHT-TO-LEFT EMBEDDING"),
        (0x202D, "LEFT-TO-RIGHT OVERRIDE"),
        (0x202E, "RIGHT-TO-LEFT OVERRIDE"),
        (0x2066, "LEFT-TO-RIGHT ISOLATE"),
        (0x2067, "RIGHT-TO-LEFT ISOLATE"),
        (0x2068, "FIRST STRONG ISOLATE"),
        (0x2069, "POP DIRECTIONAL ISOLATE"),
    ];

    let mut found = Vec::new();
    for (_i, c) in line.char_indices() {
        let cp = c as u32;
        if let Some(&(_, name)) = bidi_chars.iter().find(|&&(code, _)| code == cp) {
            found.push(format!("U+{cp:04X} ({name})"));
        }
    }

    if !found.is_empty() {
        findings.push(Finding {
            rule_id: "hidden-unicode-bidi".into(),
            severity: Severity::High,
            category: "hidden-unicode".into(),
            file: file_path.into(),
            line: line_num + 1,
            snippet: format!("发现 Bidi 字符: {}", found.join(", ")),
            message: "检测到 Bidi 覆盖字符 — 可能用于伪装代码意图 (Trojan Source)".into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tag_chars() {
        let mut findings = vec![];
        // U+E0001 LANGUAGE TAG
        let tag_char = char::from_u32(0xE0001).unwrap();
        let content = format!("normal text{tag_char}hidden");
        scan("test.md", &content, &mut findings);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn detects_zero_width_space() {
        let mut findings = vec![];
        let zwsp = '\u{200B}';
        let content = format!("hello{zwsp}world");
        scan("test.sh", &content, &mut findings);
        assert!(findings
            .iter()
            .any(|f| f.rule_id == "hidden-unicode-zero-width"));
    }

    #[test]
    fn detects_bidi_override() {
        let mut findings = vec![];
        let rlo = '\u{202E}'; // RIGHT-TO-LEFT OVERRIDE
        let content = format!("safe{rlo}evomer");
        scan("test.sh", &content, &mut findings);
        assert!(findings.iter().any(|f| f.rule_id == "hidden-unicode-bidi"));
    }

    #[test]
    fn clean_content_no_findings() {
        let mut findings = vec![];
        scan(
            "normal.md",
            "# Hello World\nThis is normal text.",
            &mut findings,
        );
        assert!(findings.is_empty());
    }
}
