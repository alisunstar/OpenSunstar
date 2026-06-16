use serde::Serialize;
use std::path::Path;
use tokei::{Config, Languages};

#[derive(Debug, Serialize)]
pub struct CodeLineResult {
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub files: usize,
    pub languages: Vec<LanguageStat>,
}

#[derive(Debug, Serialize)]
pub struct LanguageStat {
    pub language: String,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub files: usize,
}

fn common_excluded() -> Vec<&'static str> {
    vec![
        "node_modules",
        "target",
        ".git",
        "dist",
        "build",
        "out",
        ".next",
        ".nuxt",
        "vendor",
        "Pods",
        ".gradle",
        ".idea",
        ".vscode",
        ".cache",
        "__pycache__",
        ".venv",
        "venv",
        "env",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        "coverage",
        ".terraform",
        ".serverless",
        ".parcel-cache",
        ".DS_Store",
        "DerivedData",
    ]
}

pub fn count_code_lines(root: &Path) -> Result<CodeLineResult, String> {
    if !root.exists() {
        return Err("路径不存在".into());
    }
    if !root.is_dir() {
        return Err("路径不是文件夹".into());
    }

    let path_str = root.to_string_lossy().to_string();
    let paths = &[path_str.as_str()];
    let excluded = common_excluded();
    let config = Config::default();

    let mut languages = Languages::new();
    languages.get_statistics(paths, &excluded, &config);

    let mut total_code = 0usize;
    let mut total_comments = 0usize;
    let mut total_blanks = 0usize;
    let mut total_files = 0usize;
    let mut lang_stats: Vec<LanguageStat> = Vec::new();

    for (lang_type, language) in &languages {
        let code = language.code;
        let comments = language.comments;
        let blanks = language.blanks;
        let files = language.reports.len();

        if code == 0 && comments == 0 && blanks == 0 {
            continue;
        }

        total_code += code;
        total_comments += comments;
        total_blanks += blanks;
        total_files += files;

        lang_stats.push(LanguageStat {
            language: lang_type.name().to_string(),
            code_lines: code,
            comment_lines: comments,
            blank_lines: blanks,
            files,
        });
    }

    lang_stats.sort_by(|a, b| b.code_lines.cmp(&a.code_lines));

    Ok(CodeLineResult {
        total_lines: total_code + total_comments + total_blanks,
        code_lines: total_code,
        comment_lines: total_comments,
        blank_lines: total_blanks,
        files: total_files,
        languages: lang_stats,
    })
}
