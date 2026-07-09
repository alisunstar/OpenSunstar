//! CLI 输出格式化：human-readable 与 JSON 双模式
//!
//! 提供 console 着色、dialoguer 交互的统一封装。
//! 所有命令文件通过本模块输出，禁止直接 eprintln/println 裸写状态信息。

use serde::Serialize;

// ── 着色状态输出（console::style） ──────────────────────

/// 成功状态：绿色 ✓
pub fn success(msg: &str) {
    eprintln!("{}", console::style(format!("✓ {msg}")).green());
}

/// 错误状态：红色 ✗
pub fn error_msg(msg: &str) {
    eprintln!("{}", console::style(format!("✗ {msg}")).red());
}

/// 警告状态：黄色 ⚠
pub fn warning(msg: &str) {
    eprintln!("{}", console::style(format!("⚠ {msg}")).yellow());
}

/// 信息输出：青色
pub fn info(msg: &str) {
    eprintln!("{}", console::style(msg).cyan());
}

/// 表头/标题：青色加粗
pub fn header(msg: &str) {
    eprintln!("{}", console::style(msg).cyan().bold());
}

/// 次要标签：暗灰色
pub fn dim(msg: &str) {
    eprintln!("{}", console::style(msg).dim());
}

// ── 交互提示封装（dialoguer） ───────────────────────────

/// 交互式确认（y/N），--yes 或 --json 模式下跳过
///
/// 返回 `true` 表示用户确认执行。
/// 当 `skip` 为 true 时（--yes 或 --json），直接返回 `default_val`。
pub fn confirm(prompt: &str, skip: bool, default_val: bool) -> bool {
    if skip {
        return default_val;
    }
    dialoguer::Confirm::new()
        .with_prompt(prompt)
        .default(default_val)
        .interact()
        .unwrap_or(default_val)
}

/// 交互式单选列表（上下键导航），返回选中项的索引
///
/// 当 `skip` 为 true 或非 TTY 时返回 `None`（由调用方决定 fallback）。
pub fn select(prompt: &str, items: &[String], skip: bool) -> Option<usize> {
    if skip || items.is_empty() {
        return None;
    }
    dialoguer::Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact_opt()
        .ok()
        .flatten()
}

/// 交互式多选列表（空格勾选），返回选中项的索引列表
///
/// 当 `skip` 为 true 或非 TTY 时返回 `None`。
#[allow(dead_code)]
pub fn multi_select(prompt: &str, items: &[String], skip: bool) -> Option<Vec<usize>> {
    if skip || items.is_empty() {
        return None;
    }
    dialoguer::MultiSelect::new()
        .with_prompt(prompt)
        .items(items)
        .interact_opt()
        .ok()
        .flatten()
}

// ── 机器消费输出（JSON 双模式） ──────────────────────────

/// 打印错误信息
pub fn print_error(msg: &str, json: bool) {
    print_error_with_hint(msg, json, None)
}

/// 打印错误信息（附带可选 hint，Agent-Native 契约）
pub fn print_error_with_hint(msg: &str, json: bool, hint: Option<&str>) {
    if json {
        let mut err = serde_json::json!({
            "error": true,
            "code": "error",
            "message": msg,
        });
        if let Some(h) = hint {
            err["hint"] = serde_json::Value::String(h.to_string());
        }
        println!("{}", serde_json::to_string_pretty(&err).unwrap());
    } else {
        error_msg(msg);
        if let Some(h) = hint {
            eprintln!("{}", console::style(format!("  hint: {h}")).dim());
        }
    }
}

/// 打印结构化结果
pub fn print_result<T: Serialize>(data: &T, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(data).unwrap());
    } else {
        // 默认也输出 JSON（human-readable 格式在各自命令中实现）
        println!("{}", serde_json::to_string_pretty(data).unwrap());
    }
}
