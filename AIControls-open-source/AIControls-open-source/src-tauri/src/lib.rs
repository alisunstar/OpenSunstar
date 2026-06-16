//! AIControls — scan installed agents and global skills / MCP / rules.

mod code_metrics;
mod deepseek;
mod gitee;
mod github_import;
mod my_skills_library;
mod prompt_library;
mod resource_library;
mod scan;
mod skill_copy;
mod storage;

use dirs::home_dir;
use scan::AgentInventory;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader},
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{
    menu::{Menu, MenuItem},
    path::BaseDirectory,
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, WindowEvent,
};

const MAIN_WINDOW_LABEL: &str = "main";
const FLOAT_BALL_WINDOW_LABEL: &str = "float-ball";
const FLOAT_BALL_HOVER_EVENT: &str = "float-ball-hover-state";
const CLAUDE_EXECUTION_STATE_EVENT: &str = "claude-execution-state";
const CLAUDE_COMPLETION_PENDING_EVENT: &str = "claude-completion-pending";
const CLAUDE_COMPLETION_PORT: u16 = 38971;
const CLAUDE_HOOK_SETTINGS_RELATIVE_PATH: &str = ".claude/settings.json";
const BRIDGE_SCRIPT_NAME: &str = "live-island-bridge.mjs";
const EMBEDDED_BRIDGE_SCRIPT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/live-island-bridge.mjs"));
const DEV_BRIDGE_SCRIPT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../scripts/live-island-bridge.mjs"
);
const REQUIRED_CLAUDE_HOOK_EVENTS: &[&str] = &[
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "PermissionRequest",
    "Stop",
    "StopFailure",
];
const FLOAT_BALL_HOVER_POLL_MS: u64 = 80;
const TRAY_SHOW_MAIN_ID: &str = "show-main";
const TRAY_QUIT_ID: &str = "quit";
const FLOAT_BALL_COLLAPSED_WIDTH: u32 = 46;
const FLOAT_BALL_COLLAPSED_HEIGHT: u32 = 56;
const FLOAT_BALL_EXPANDED_WIDTH: u32 = 224;
const FLOAT_BALL_EXPANDED_HEIGHT: u32 = 286;
const FLOAT_BALL_DEFAULT_RIGHT_OFFSET: i32 = 68;
const FLOAT_BALL_DEFAULT_BOTTOM_OFFSET: i32 = 220;
const FLOAT_BALL_SAFE_BOTTOM_MARGIN: i32 = 80;

#[derive(Clone, Copy, PartialEq, Serialize)]
struct FloatBallHoverPayload {
    inside: bool,
    x: f64,
    y: f64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeHookMessage {
    #[serde(rename = "type")]
    message_type: String,
    cwd: String,
    session_id: Option<String>,
    state: Option<String>,
    tool_name: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeExecutionStatePayload {
    cwd: String,
    session_id: Option<String>,
    state: String,
    tool_name: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeCompletionPendingPayload {
    cwd: String,
    session_id: Option<String>,
}

#[derive(Default)]
struct ClaudeCompletionState {
    pending: Mutex<Option<ClaudeCompletionPendingPayload>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeHookStatus {
    installed: bool,
    settings_path: String,
    bridge_script_path: String,
}

fn claude_settings_path() -> Result<PathBuf, String> {
    let home = home_dir().ok_or_else(|| "无法确定用户目录".to_string())?;
    Ok(home.join(CLAUDE_HOOK_SETTINGS_RELATIVE_PATH))
}

fn bridge_script_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from(DEV_BRIDGE_SCRIPT)];

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("scripts").join(BRIDGE_SCRIPT_NAME));
        candidates.push(current_dir.join("..").join("scripts").join(BRIDGE_SCRIPT_NAME));
    }
    if let Ok(path) = app
        .path()
        .resolve(BRIDGE_SCRIPT_NAME, BaseDirectory::Resource)
    {
        candidates.push(path);
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join(BRIDGE_SCRIPT_NAME));
        candidates.push(resource_dir.join("_up_").join("scripts").join(BRIDGE_SCRIPT_NAME));
    }

    candidates
}

fn find_bundled_bridge_script(app: &AppHandle) -> Option<PathBuf> {
    bridge_script_candidates(app)
        .into_iter()
        .find(|path| path.is_file())
}

fn read_bridge_script_content(app: &AppHandle) -> String {
    if let Some(path) = find_bundled_bridge_script(app) {
        if let Ok(text) = fs::read_to_string(&path) {
            return text;
        }
    }
    EMBEDDED_BRIDGE_SCRIPT.to_string()
}

fn bridge_script_for_hooks(app: &AppHandle) -> Result<PathBuf, String> {
    let bridge_content = read_bridge_script_content(app);
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法解析应用数据目录: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    let dest = dir.join(BRIDGE_SCRIPT_NAME);
    fs::write(&dest, bridge_content).map_err(|e| format!("写入 bridge 脚本失败: {e}"))?;
    Ok(dest)
}

fn hook_command_for_bridge(bridge_path: &Path) -> String {
    let normalized = bridge_path.to_string_lossy().replace('\\', "/");
    format!("node \"{normalized}\" hook")
}

fn command_to_bridge_path(command: &str) -> Option<PathBuf> {
    if !command.contains(BRIDGE_SCRIPT_NAME) {
        return None;
    }
    for token in command.split_whitespace() {
        let trimmed = token.trim_matches('"');
        if trimmed.contains(BRIDGE_SCRIPT_NAME) {
            let path = PathBuf::from(trimmed);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn bridge_path_from_settings(settings_path: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(settings_path).ok()?;
    let parsed = serde_json::from_str::<Value>(&text).ok()?;
    let hooks = parsed.get("hooks")?.as_object()?;
    for event in REQUIRED_CLAUDE_HOOK_EVENTS {
        let entries = hooks.get(*event)?.as_array()?;
        for entry in entries {
            let hooks_list = entry.get("hooks")?.as_array()?;
            for hook in hooks_list {
                if hook.get("type").and_then(Value::as_str) == Some("command") {
                    if let Some(command) = hook.get("command").and_then(Value::as_str) {
                        if let Some(path) = command_to_bridge_path(command) {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

fn app_data_bridge_script_path(app: &AppHandle) -> Option<PathBuf> {
    let dest = app.path().app_data_dir().ok()?.join(BRIDGE_SCRIPT_NAME);
    dest.is_file().then_some(dest)
}

fn resolved_bridge_script_path(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(settings_path) = claude_settings_path() {
        if let Some(path) = bridge_path_from_settings(&settings_path) {
            return Some(path);
        }
    }
    app_data_bridge_script_path(app).or_else(|| find_bundled_bridge_script(app))
}

fn event_has_managed_claude_hook(value: &Value) -> bool {
    value
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("type").and_then(Value::as_str) == Some("command")
                    && hook
                        .get("command")
                        .and_then(Value::as_str)
                        .is_some_and(|command| command.contains(BRIDGE_SCRIPT_NAME))
            })
        })
}

fn hooks_installed_in_settings(settings_path: &Path) -> Result<bool, String> {
    if !settings_path.is_file() {
        return Ok(false);
    }

    let raw = fs::read_to_string(settings_path)
        .map_err(|e| format!("读取 Claude 设置失败: {e}"))?;
    let parsed: Value =
        serde_json::from_str(&raw).map_err(|e| format!("解析 Claude 设置失败: {e}"))?;
    let Some(hooks) = parsed.get("hooks").and_then(Value::as_object) else {
        return Ok(false);
    };

    Ok(REQUIRED_CLAUDE_HOOK_EVENTS.iter().all(|event| {
        hooks
            .get(*event)
            .and_then(Value::as_array)
            .is_some_and(|entries| entries.iter().any(event_has_managed_claude_hook))
    }))
}

fn refresh_managed_claude_hook_commands(entries: &mut Vec<Value>, command: &str) {
    let mut found = false;
    for entry in entries.iter_mut() {
        let Some(hooks_list) = entry.get_mut("hooks").and_then(Value::as_array_mut) else {
            continue;
        };
        for hook in hooks_list.iter_mut() {
            let Some(hook_map) = hook.as_object_mut() else {
                continue;
            };
            if hook_map
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|c| c.contains(BRIDGE_SCRIPT_NAME))
            {
                hook_map.insert("type".to_string(), json!("command"));
                hook_map.insert("command".to_string(), json!(command));
                found = true;
            }
        }
    }
    if !found {
        entries.push(json!({
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": command,
            }],
        }));
    }
}

fn detect_claude_hook_status(app: &AppHandle) -> Result<ClaudeHookStatus, String> {
    let settings_path = claude_settings_path()?;
    let installed = hooks_installed_in_settings(&settings_path)?;
    let bridge_script_path = resolved_bridge_script_path(app)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(ClaudeHookStatus {
        installed,
        settings_path: settings_path.to_string_lossy().into_owned(),
        bridge_script_path,
    })
}

fn install_managed_claude_hooks(app: &AppHandle) -> Result<ClaudeHookStatus, String> {
    let settings_path = claude_settings_path()?;
    let bridge_path = bridge_script_for_hooks(app)?;
    let command = hook_command_for_bridge(&bridge_path);

    let mut root = if settings_path.is_file() {
        let text = fs::read_to_string(&settings_path)
            .map_err(|e| format!("读取 Claude 设置失败: {e}"))?;
        serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !root.is_object() {
        root = json!({});
    }

    let Some(root_map) = root.as_object_mut() else {
        return Err("无法写入 Claude hooks 配置".into());
    };
    let hooks_value = root_map
        .entry("hooks".to_string())
        .or_insert_with(|| json!({}));
    if !hooks_value.is_object() {
        *hooks_value = json!({});
    }
    let Some(hooks_map) = hooks_value.as_object_mut() else {
        return Err("无法写入 Claude hooks 配置".into());
    };

    for event in REQUIRED_CLAUDE_HOOK_EVENTS {
        let entry = hooks_map
            .entry((*event).to_string())
            .or_insert_with(|| json!([]));
        if !entry.is_array() {
            *entry = json!([]);
        }
        let Some(entries) = entry.as_array_mut() else {
            continue;
        };
        refresh_managed_claude_hook_commands(entries, &command);
    }

    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建 Claude 配置目录失败: {e}"))?;
    }
    fs::write(
        &settings_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&root)
                .map_err(|e| format!("写回 Claude 设置失败: {e}"))?
        ),
    )
    .map_err(|e| format!("写回 Claude 设置失败: {e}"))?;

    detect_claude_hook_status(app)
}

fn remove_managed_claude_hooks(settings_path: &Path) -> Result<(), String> {
    if !settings_path.exists() {
        return Ok(());
    }

    let raw = fs::read_to_string(settings_path)
        .map_err(|e| format!("读取 Claude 设置失败: {e}"))?;
    let mut root: Value =
        serde_json::from_str(&raw).map_err(|e| format!("解析 Claude 设置失败: {e}"))?;
    if let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) {
        for event in REQUIRED_CLAUDE_HOOK_EVENTS {
            if let Some(entries) = hooks.get_mut(*event).and_then(Value::as_array_mut) {
                entries.retain(|entry| !event_has_managed_claude_hook(entry));
            }
        }
    }

    fs::write(
        settings_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&root)
                .map_err(|e| format!("写回 Claude 设置失败: {e}"))?
        ),
    )
    .map_err(|e| format!("写回 Claude 设置失败: {e}"))?;
    Ok(())
}

fn emit_pending_claude_completion(app: &AppHandle, payload: ClaudeCompletionPendingPayload) {
    let state = app.state::<ClaudeCompletionState>();
    if let Ok(mut pending) = state.pending.lock() {
        *pending = Some(payload.clone());
    }
    if let Some(ball) = app.get_webview_window(FLOAT_BALL_WINDOW_LABEL) {
        let _ = ball.emit(CLAUDE_COMPLETION_PENDING_EVENT, payload);
    }
}

fn emit_claude_execution_state(app: &AppHandle, payload: ClaudeExecutionStatePayload) {
    if let Some(ball) = app.get_webview_window(FLOAT_BALL_WINDOW_LABEL) {
        let _ = ball.emit(CLAUDE_EXECUTION_STATE_EVENT, payload);
    }
}

fn handle_claude_hook_message(app: &AppHandle, message: ClaudeHookMessage) {
    if message.message_type != "claude-state" {
        return;
    }

    let Some(state) = message.state else {
        return;
    };

    match state.as_str() {
        "complete" => emit_pending_claude_completion(
            app,
            ClaudeCompletionPendingPayload {
                cwd: message.cwd,
                session_id: message.session_id,
            },
        ),
        "running" | "tool" | "waiting" | "error" => emit_claude_execution_state(
            app,
            ClaudeExecutionStatePayload {
                cwd: message.cwd,
                session_id: message.session_id,
                state,
                tool_name: message.tool_name,
            },
        ),
        _ => {}
    }
}

fn start_claude_hook_listener(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        let listener = match TcpListener::bind(("127.0.0.1", CLAUDE_COMPLETION_PORT)) {
            Ok(listener) => listener,
            Err(_) => return,
        };

        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            let reader = BufReader::new(stream);
            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(message) = serde_json::from_str::<ClaudeHookMessage>(trimmed) else {
                    continue;
                };
                handle_claude_hook_message(&app, message);
            }
        }
    });
}

fn latest_file_mtime_in_dir(root: &std::path::Path) -> Result<i64, String> {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    if !root.exists() {
        return Err("路径不存在".into());
    }
    if !root.is_dir() {
        return Err("路径不是文件夹".into());
    }

    let mut best: Option<SystemTime> = None;
    let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let md = fs::metadata(&dir).ok();
        if let Some(m) = md.and_then(|m| m.modified().ok()) {
            best = Some(best.map_or(m, |cur| cur.max(m)));
        }

        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for ent in rd.flatten() {
            let path = ent.path();
            // Avoid following symlink directories (can introduce cycles).
            let ft = ent.file_type().ok();
            if ft.as_ref().is_some_and(|t| t.is_symlink()) {
                let md = fs::symlink_metadata(&path).ok();
                if let Some(m) = md.and_then(|m| m.modified().ok()) {
                    best = Some(best.map_or(m, |cur| cur.max(m)));
                }
                continue;
            }

            let md = ent.metadata().ok();
            if let Some(m) = md.as_ref().and_then(|m| m.modified().ok()) {
                best = Some(best.map_or(m, |cur| cur.max(m)));
            }
            if md.as_ref().is_some_and(|m| m.is_dir()) {
                stack.push(path);
            }
        }
    }

    let Some(t) = best else {
        return Err("无法读取目录修改时间".into());
    };
    let ms = t
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "无法解析修改时间".to_string())?
        .as_millis();
    Ok(ms as i64)
}

fn configure_float_ball(app: &AppHandle) {
    let Some(ball) = app.get_webview_window(FLOAT_BALL_WINDOW_LABEL) else {
        return;
    };

    let _ = ball.set_always_on_top(true);
    let _ = ball.set_visible_on_all_workspaces(true);
    let _ = ball.set_skip_taskbar(true);
    let _ = ball.set_ignore_cursor_events(true);

    let scale_factor = ball.scale_factor().unwrap_or(1.0);
    let collapsed_size = tauri::PhysicalSize::new(
        (FLOAT_BALL_COLLAPSED_WIDTH as f64 * scale_factor)
            .round()
            .max(1.0) as u32,
        (FLOAT_BALL_COLLAPSED_HEIGHT as f64 * scale_factor)
            .round()
            .max(1.0) as u32,
    );
    let expanded_size = tauri::PhysicalSize::new(
        (FLOAT_BALL_EXPANDED_WIDTH as f64 * scale_factor)
            .round()
            .max(1.0) as u32,
        (FLOAT_BALL_EXPANDED_HEIGHT as f64 * scale_factor)
            .round()
            .max(1.0) as u32,
    );
    let _ = ball.set_size(expanded_size);
    let expanded_from_collapsed = |position: PhysicalPosition<i32>| {
        PhysicalPosition::new(
            position.x - (expanded_size.width as i32 - collapsed_size.width as i32) / 2,
            position.y - (expanded_size.height as i32 - collapsed_size.height as i32),
        )
    };

    if let Ok(Some(saved)) = storage::load_float_ball_position(app) {
        let saved = PhysicalPosition::new(saved.x, saved.y);
        let position = clamp_float_ball_position(app, saved, Some(collapsed_size));
        let _ = ball.set_position(expanded_from_collapsed(position));
        return;
    }

    let monitor = ball
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let position = monitor.position();
        let size = monitor.size();
        let x = position.x + size.width as i32 - FLOAT_BALL_DEFAULT_RIGHT_OFFSET;
        let y = position.y + size.height as i32 - FLOAT_BALL_DEFAULT_BOTTOM_OFFSET;
        let default_position = PhysicalPosition::new(x.max(position.x), y.max(position.y));
        let position = clamp_float_ball_position(app, default_position, Some(collapsed_size));
        let _ = ball.set_position(expanded_from_collapsed(position));
    }
}

fn start_float_ball_hover_watcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_inside: Option<bool> = None;
        let mut last_payload_key: Option<(bool, i32, i32)> = None;
        let mut menu_open = false;

        loop {
            let payload = app
                .get_webview_window(FLOAT_BALL_WINDOW_LABEL)
                .and_then(|ball| {
                    let cursor = app.cursor_position().ok()?;
                    let position = ball.outer_position().ok()?;
                    let size = ball.outer_size().ok()?;
                    let scale_factor = ball.scale_factor().unwrap_or(1.0);
                    let left = position.x as f64;
                    let top = position.y as f64;
                    let right = left + size.width as f64;
                    let bottom = top + size.height as f64;
                    let collapsed_width = FLOAT_BALL_COLLAPSED_WIDTH as f64 * scale_factor;
                    let collapsed_height = FLOAT_BALL_COLLAPSED_HEIGHT as f64 * scale_factor;
                    let ball_left = left + (size.width as f64 - collapsed_width) / 2.0;
                    let ball_top = bottom - collapsed_height;
                    let ball_right = ball_left + collapsed_width;
                    let ball_bottom = bottom;

                    let inside_window = cursor.x >= left
                        && cursor.x <= right
                        && cursor.y >= top
                        && cursor.y <= bottom;
                    let inside_collapsed_ball = cursor.x >= ball_left
                        && cursor.x <= ball_right
                        && cursor.y >= ball_top
                        && cursor.y <= ball_bottom;
                    let inside = if menu_open {
                        inside_window
                    } else {
                        inside_collapsed_ball
                    };

                    if Some(inside) != last_inside {
                        let _ = ball.set_ignore_cursor_events(!inside);
                    }

                    let payload = FloatBallHoverPayload {
                        inside,
                        x: (cursor.x - left) / scale_factor,
                        y: (cursor.y - top) / scale_factor,
                    };
                    let payload_key = (inside, payload.x.round() as i32, payload.y.round() as i32);
                    if Some(payload_key) != last_payload_key {
                        let _ = ball.emit(FLOAT_BALL_HOVER_EVENT, payload);
                        last_payload_key = Some(payload_key);
                    }

                    Some(payload)
                })
                .unwrap_or(FloatBallHoverPayload {
                    inside: false,
                    x: -1.0,
                    y: -1.0,
                });

            menu_open = payload.inside;
            last_inside = Some(payload.inside);
            tokio::time::sleep(std::time::Duration::from_millis(FLOAT_BALL_HOVER_POLL_MS)).await;
        }
    });
}

fn clamp_float_ball_position(
    app: &AppHandle,
    position: PhysicalPosition<i32>,
    window_size: Option<tauri::PhysicalSize<u32>>,
) -> PhysicalPosition<i32> {
    let Some(window_size) = window_size else {
        return position;
    };

    let monitors = app.available_monitors().unwrap_or_default();
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let monitor_position = monitor.position();
            let monitor_size = monitor.size();
            let center_x = position.x + window_size.width as i32 / 2;
            let center_y = position.y + window_size.height as i32 / 2;
            center_x >= monitor_position.x
                && center_x <= monitor_position.x + monitor_size.width as i32
                && center_y >= monitor_position.y
                && center_y <= monitor_position.y + monitor_size.height as i32
        })
        .or_else(|| monitors.first());

    let Some(monitor) = monitor else {
        return position;
    };

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let max_x = monitor_position.x + monitor_size.width as i32 - window_size.width as i32;
    let safe_bottom_margin = (FLOAT_BALL_SAFE_BOTTOM_MARGIN as f64 * window_size.height as f64
        / FLOAT_BALL_COLLAPSED_HEIGHT as f64)
        .round() as i32;
    let max_y = monitor_position.y + monitor_size.height as i32
        - window_size.height as i32
        - safe_bottom_margin.max(0);

    PhysicalPosition::new(
        position
            .x
            .clamp(monitor_position.x, max_x.max(monitor_position.x)),
        position
            .y
            .clamp(monitor_position.y, max_y.max(monitor_position.y)),
    )
}

fn save_float_ball_position(app: &AppHandle, position: PhysicalPosition<i32>) {
    let _ = storage::save_float_ball_position(
        app,
        storage::FloatBallPosition {
            x: position.x,
            y: position.y,
        },
    );
}

fn collapsed_float_ball_position(
    window: &tauri::Window,
    position: PhysicalPosition<i32>,
) -> PhysicalPosition<i32> {
    let Ok(size) = window.outer_size() else {
        return position;
    };
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let collapsed_width = (FLOAT_BALL_COLLAPSED_WIDTH as f64 * scale_factor).round() as i32;
    let collapsed_height = (FLOAT_BALL_COLLAPSED_HEIGHT as f64 * scale_factor).round() as i32;
    let width = size.width as i32;
    let height = size.height as i32;

    if width <= collapsed_width + 2 && height <= collapsed_height + 2 {
        return position;
    }

    PhysicalPosition::new(
        position.x + (width - collapsed_width).max(0) / 2,
        position.y + (height - collapsed_height).max(0),
    )
}

fn collapsed_float_ball_size(window: &tauri::Window) -> Option<tauri::PhysicalSize<u32>> {
    let scale_factor = window.scale_factor().ok()?;
    Some(tauri::PhysicalSize::new(
        (FLOAT_BALL_COLLAPSED_WIDTH as f64 * scale_factor)
            .round()
            .max(1.0) as u32,
        (FLOAT_BALL_COLLAPSED_HEIGHT as f64 * scale_factor)
            .round()
            .max(1.0) as u32,
    ))
}

fn setup_tray(app: &mut tauri::App, is_quitting: Arc<AtomicBool>) -> tauri::Result<()> {
    let show_main = MenuItem::with_id(app, TRAY_SHOW_MAIN_ID, "显示主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_QUIT_ID, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_main, &quit])?;

    let mut tray = TrayIconBuilder::with_id("aicontrols-tray")
        .menu(&menu)
        .tooltip("AIControls")
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            TRAY_SHOW_MAIN_ID => {
                if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            TRAY_QUIT_ID => {
                is_quitting.store(true, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

#[tauri::command]
fn focus_main_project(app: AppHandle, path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("路径为空".into());
    }

    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "未找到主窗口".to_string())?;
    window.show().map_err(|e| format!("显示主窗口失败: {e}"))?;
    window
        .set_focus()
        .map_err(|e| format!("聚焦主窗口失败: {e}"))?;
    window
        .emit("main-navigate", path)
        .map_err(|e| format!("跳转项目失败: {e}"))?;

    let state = app.state::<ClaudeCompletionState>();
    if let Ok(mut pending) = state.pending.lock() {
        *pending = None;
    }

    Ok(())
}

#[tauri::command]
fn detect_claude_hook_status_command(app: AppHandle) -> Result<ClaudeHookStatus, String> {
    detect_claude_hook_status(&app)
}

#[tauri::command]
fn install_claude_hooks_command(app: AppHandle) -> Result<ClaudeHookStatus, String> {
    install_managed_claude_hooks(&app)
}

#[tauri::command]
fn remove_claude_hooks_command(app: AppHandle) -> Result<ClaudeHookStatus, String> {
    let settings_path = claude_settings_path()?;
    remove_managed_claude_hooks(&settings_path)?;
    detect_claude_hook_status(&app)
}

#[tauri::command]
fn list_detected_agents(app: AppHandle) -> Vec<scan::AgentScanResult> {
    let hidden = storage::load_hidden_sidebar_agent_ids(&app).unwrap_or_default();
    let mut out: Vec<scan::AgentScanResult> = scan::detect_agents()
        .into_iter()
        .filter(|a| !hidden.contains(&a.id))
        .collect();
    if let Ok(user_agents) = storage::load_user_agents(&app) {
        let builtin_canon: std::collections::HashSet<String> = out
            .iter()
            .filter_map(|a| std::path::Path::new(&a.root_path).canonicalize().ok())
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        for ua in user_agents {
            if let Ok(can) = std::path::Path::new(&ua.path).canonicalize() {
                let s = can.to_string_lossy().into_owned();
                if builtin_canon.contains(&s) {
                    continue;
                }
            }
            out.push(scan::AgentScanResult {
                id: ua.id,
                label: ua.label,
                root_path: ua.path,
            });
        }
    }
    out
}

#[tauri::command]
fn add_user_agent_from_path(app: AppHandle, path: String) -> Result<scan::AgentScanResult, String> {
    let ua = storage::add_user_agent_from_path(&app, &path)?;
    Ok(scan::AgentScanResult {
        id: ua.id,
        label: ua.label,
        root_path: ua.path,
    })
}

#[tauri::command]
fn remove_agent_from_sidebar(app: AppHandle, agent_id: String) -> Result<(), String> {
    if agent_id.starts_with("useragent-") {
        storage::remove_user_agent(&app, &agent_id)
    } else {
        storage::hide_sidebar_builtin_agent(&app, &agent_id)
    }
}

#[tauri::command]
fn clear_hidden_sidebar_agents(app: AppHandle) -> Result<(), String> {
    storage::clear_hidden_sidebar_agents(&app)
}

#[tauri::command]
async fn get_agent_global_inventory(
    app: AppHandle,
    agent_id: String,
) -> Result<AgentInventory, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut inv = if let Ok(Some(root)) = storage::user_agent_root_for_id(&app, &agent_id) {
            scan::global_inventory_at_agent_root(&root)?
        } else {
            scan::global_inventory(&agent_id)?
        };
        // Scan additional custom skill paths if configured
        if let Ok(custom_map) = storage::load_agent_custom_skill_paths(&app) {
            if let Some(extra_paths) = custom_map.get(&agent_id) {
                let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
                let extra_roots: Vec<std::path::PathBuf> = extra_paths
                    .iter()
                    .filter_map(|p| {
                        let expanded = if p.starts_with("~/") {
                            home.join(&p[2..])
                        } else if p == "~" {
                            home.clone()
                        } else {
                            std::path::PathBuf::from(p)
                        };
                        if expanded.is_dir() { Some(expanded) } else { None }
                    })
                    .collect();
                if !extra_roots.is_empty() {
                    let mut extra_skills = Vec::new();
                    scan::push_skills_from_roots_public(&extra_roots, &mut extra_skills);
                    inv.skills.extend(extra_skills);
                }
            }
        }
        let scenario_map = storage::load_scenario_map(&app).unwrap_or_default();
        scan::attach_scenarios(&mut inv, &scenario_map);
        let brief_map_zh = storage::load_brief_map(&app, "zh").unwrap_or_default();
        scan::attach_briefs(&mut inv, "zh", &brief_map_zh);
        let brief_map_en = storage::load_brief_map(&app, "en").unwrap_or_default();
        scan::attach_briefs(&mut inv, "en", &brief_map_en);
        Ok(inv)
    })
    .await
    .map_err(|e| format!("扫描任务失败: {e}"))?
}

#[tauri::command]
async fn scan_project_directory(app: AppHandle, root: String) -> Result<AgentInventory, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut inv = scan::scan_project_directory(std::path::Path::new(&root))?;
        let scenario_map = storage::load_scenario_map(&app).unwrap_or_default();
        scan::attach_scenarios(&mut inv, &scenario_map);
        let brief_map_zh = storage::load_brief_map(&app, "zh").unwrap_or_default();
        scan::attach_briefs(&mut inv, "zh", &brief_map_zh);
        let brief_map_en = storage::load_brief_map(&app, "en").unwrap_or_default();
        scan::attach_briefs(&mut inv, "en", &brief_map_en);
        Ok(inv)
    })
    .await
    .map_err(|e| format!("扫描任务失败: {e}"))?
}

#[tauri::command]
fn read_skill_document(path: String) -> Result<(String, String), String> {
    scan::read_skill_document(std::path::Path::new(&path))
}

#[tauri::command]
fn get_deepseek_settings(app: AppHandle) -> Result<storage::DeepseekSettingsPublic, String> {
    storage::get_deepseek_settings_public(&app)
}

#[tauri::command]
fn save_deepseek_settings(app: AppHandle, api_key: String) -> Result<(), String> {
    storage::save_deepseek_api_key(&app, api_key)
}

#[tauri::command]
async fn test_deepseek_connection(app: AppHandle) -> Result<String, String> {
    let config = storage::load_active_ai_config(&app)?;
    deepseek::test_ping(&config).await
}

#[tauri::command]
fn get_ai_provider(app: AppHandle) -> Result<String, String> {
    storage::load_ai_provider(&app)
}

#[tauri::command]
fn save_ai_provider(app: AppHandle, provider: String) -> Result<(), String> {
    storage::save_ai_provider(&app, provider)
}

#[tauri::command]
fn get_glm_settings(app: AppHandle) -> Result<storage::GlmSettingsPublic, String> {
    storage::get_glm_settings_public(&app)
}

#[tauri::command]
fn save_glm_settings(app: AppHandle, api_key: String, api_url: String, model: String) -> Result<(), String> {
    storage::save_glm_settings(&app, api_key, api_url, model)
}

#[tauri::command]
async fn test_glm_connection(app: AppHandle) -> Result<String, String> {
    // Temporarily switch to GLM, run ping, then restore provider
    let prev_provider = storage::load_ai_provider(&app)?;
    storage::save_ai_provider(&app, storage::GLM_PROVIDER.to_string())?;
    let result = deepseek::test_ping(&storage::load_active_ai_config(&app)?).await;
    // Restore previous provider
    let _ = storage::save_ai_provider(&app, prev_provider);
    result
}

#[tauri::command]
async fn deepseek_classify_inventory(
    app: AppHandle,
    inventory: AgentInventory,
) -> Result<AgentInventory, String> {
    deepseek::classify_inventory_missing(&app, inventory).await
}

#[tauri::command]
async fn deepseek_summarize_inventory(
    app: AppHandle,
    inventory: AgentInventory,
    locale: Option<String>,
) -> Result<AgentInventory, String> {
    deepseek::summarize_inventory_missing(&app, inventory, locale.unwrap_or_else(|| "zh".into()))
        .await
}

#[tauri::command]
async fn deepseek_resummarize_asset(
    app: AppHandle,
    asset: scan::AssetEntry,
    locale: Option<String>,
) -> Result<String, String> {
    deepseek::resummarize_single_asset(&app, asset, locale.unwrap_or_else(|| "zh".into())).await
}

#[tauri::command]
async fn deepseek_enrich_resource_url(
    app: AppHandle,
    url: String,
) -> Result<deepseek::ResourceUrlEnrichment, String> {
    deepseek::enrich_resource_from_url(&app, url).await
}

#[tauri::command]
async fn deepseek_regenerate_categories(
    app: AppHandle,
    inventory: AgentInventory,
    locale: Option<String>,
) -> Result<Vec<deepseek::CustomCategory>, String> {
    deepseek::regenerate_categories(&app, inventory, locale).await
}

#[tauri::command]
async fn deepseek_translate_custom_categories(
    app: AppHandle,
    categories: Vec<deepseek::CustomCategory>,
    locale: Option<String>,
) -> Result<Vec<deepseek::CustomCategory>, String> {
    deepseek::translate_custom_categories(&app, categories, locale).await
}

#[tauri::command]
async fn deepseek_reclassify_with_new_categories(
    app: AppHandle,
    inventory: AgentInventory,
    categories: Vec<deepseek::CustomCategory>,
) -> Result<std::collections::HashMap<String, String>, String> {
    deepseek::reclassify_with_new_categories(&app, inventory, categories).await
}

#[tauri::command]
fn get_custom_categories(app: AppHandle) -> Result<Vec<deepseek::CustomCategory>, String> {
    storage::load_custom_categories(&app)
}

#[tauri::command]
fn clear_custom_categories(app: AppHandle) -> Result<(), String> {
    storage::clear_custom_categories(&app)
}

#[tauri::command]
fn reset_all_categories(app: AppHandle) -> Result<(), String> {
    storage::clear_custom_categories(&app)?;
    storage::clear_scenario_map(&app)?;
    Ok(())
}

/// 在系统文件管理器中打开路径：文件则打开其所在文件夹并选中；文件夹则打开该文件夹。
#[tauri::command]
fn reveal_path_in_folder(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("路径为空".into());
    }
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err("路径不存在".into());
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let st = if p.is_dir() {
            Command::new("open").arg(p).status()
        } else {
            Command::new("open").arg("-R").arg(p).status()
        };
        st.map_err(|e| format!("无法打开访达: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if p.is_dir() {
            Command::new("explorer")
                .arg(p)
                .status()
                .map_err(|e| format!("无法打开资源管理器: {e}"))?;
        } else {
            let arg = format!("/select,{}", p.to_string_lossy());
            Command::new("explorer")
                .arg(arg)
                .status()
                .map_err(|e| format!("无法打开资源管理器: {e}"))?;
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        use std::process::Command;
        let dir = if p.is_dir() {
            p.to_path_buf()
        } else {
            p.parent()
                .ok_or_else(|| "无法解析父目录".to_string())?
                .to_path_buf()
        };
        Command::new("xdg-open")
            .arg(&dir)
            .status()
            .map_err(|e| format!("无法打开文件管理器: {e}"))?;
    }

    Ok(())
}

/// 未指定应用时：依次尝试 Visual Studio Code、Cursor；均不可用时在文件管理器中打开该文件夹。
fn open_project_folder_default_chain(p: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let try_app = |name: &str| -> bool {
            Command::new("open")
                .arg("-a")
                .arg(name)
                .arg(p)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        if try_app("Visual Studio Code") {
            return Ok(());
        }
        if try_app("Cursor") {
            return Ok(());
        }
        let st = Command::new("open")
            .arg(p)
            .status()
            .map_err(|e| format!("无法打开项目: {e}"))?;
        if !st.success() {
            return Err("打开项目失败".into());
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let try_cli = |name: &str| -> bool {
            Command::new(name)
                .arg(p)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        if try_cli("code") {
            return Ok(());
        }
        if try_cli("cursor") {
            return Ok(());
        }
        let st = Command::new("explorer")
            .arg(p)
            .status()
            .map_err(|e| format!("无法打开项目: {e}"))?;
        if !st.success() {
            return Err("打开项目失败".into());
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        use std::process::Command;
        let try_cli = |name: &str| -> bool {
            Command::new(name)
                .arg(p)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        if try_cli("code") {
            return Ok(());
        }
        if try_cli("cursor") {
            return Ok(());
        }
        let st = Command::new("xdg-open")
            .arg(p)
            .status()
            .map_err(|e| format!("无法打开项目: {e}"))?;
        if !st.success() {
            return Err("打开项目失败".into());
        }
    }
    Ok(())
}

/// 打开项目根目录：未指定 `application_path` 时先试 VS Code，再试 Cursor，再打开所在文件夹；
/// 指定时为该路径（如 `/Applications/Cursor.app` 或 Windows 下 `.exe` 全路径）打开此文件夹。
#[tauri::command]
fn open_project_path(path: String, application_path: Option<String>) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("路径为空".into());
    }
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err("路径不存在".into());
    }
    if !p.is_dir() {
        return Err("路径不是文件夹".into());
    }

    let app = application_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if let Some(a) = app {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let st = Command::new("open").arg("-a").arg(a).arg(p).status();
            let code = st.map_err(|e| format!("无法打开项目: {e}"))?;
            if !code.success() {
                return Err("打开项目失败".into());
            }
        }
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let st = Command::new(a).arg(p).status();
            let code = st.map_err(|e| format!("无法打开项目: {e}"))?;
            if !code.success() {
                return Err("打开项目失败".into());
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            use std::process::Command;
            let st = Command::new(a).arg(p).status();
            let code = st.map_err(|e| format!("无法打开项目: {e}"))?;
            if !code.success() {
                return Err("打开项目失败".into());
            }
        }
        Ok(())
    } else {
        open_project_folder_default_chain(p)
    }
}

/// 递归扫描目录，返回目录下（含子文件/子目录）的最新修改时间（Unix 毫秒）。
#[tauri::command]
async fn get_project_latest_mtime_ms(root: String) -> Result<i64, String> {
    tauri::async_runtime::spawn_blocking(move || {
        latest_file_mtime_in_dir(std::path::Path::new(root.trim()))
    })
    .await
    .map_err(|e| format!("扫描任务失败: {e}"))?
}

/// 使用 tokei 统计项目目录的代码行数。
#[tauri::command]
async fn count_project_code_lines(root: String) -> Result<code_metrics::CodeLineResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        code_metrics::count_code_lines(std::path::Path::new(root.trim()))
    })
    .await
    .map_err(|e| format!("统计任务失败: {e}"))?
}

/// 读取项目目录下 package.json 中的 version 字段。
#[tauri::command]
async fn read_package_version(root: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let pkg_path = std::path::Path::new(root.trim()).join("package.json");
        if !pkg_path.is_file() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&pkg_path)
            .map_err(|e| format!("读取 package.json 失败: {e}"))?;
        let val: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| format!("解析 package.json 失败: {e}"))?;
        let version = val.get("version").and_then(|v| v.as_str()).map(|v| {
            if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{v}")
            }
        });
        Ok(version)
    })
    .await
    .map_err(|e| format!("读取任务失败: {e}"))?
}

/// AI 评估 MVP 项目的完成进度。
#[tauri::command]
async fn estimate_project_progress(
    app: AppHandle,
    root: String,
) -> Result<deepseek::ProjectProgressResult, String> {
    deepseek::estimate_project_progress(&app, root).await
}

/// 统计近 N 天内的 Git 提交数量（用于计算项目活跃度）。
#[tauri::command]
async fn git_commit_count_last_n_days(root: String, days: u32) -> Result<u32, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        if !dir.is_dir() {
            return Err("路径不是文件夹".into());
        }
        if !dir.join(".git").is_dir() {
            return Ok(0);
        }
        let since = format!("--since={}.days", days);
        let count = git_command_output(dir, &["log", "--oneline", &since])
            .map(|s| s.lines().count() as u32)
            .unwrap_or(0);
        Ok(count)
    })
    .await
    .map_err(|e| format!("统计任务失败: {e}"))?
}

/// 返回最近 12 周每周的提交数量（从最旧到最新）。
#[tauri::command]
async fn git_weekly_commit_counts(root: String) -> Result<Vec<u32>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        if !dir.is_dir() || !dir.join(".git").is_dir() {
            return Ok(vec![0u32; 12]);
        }

        let mut weeks = Vec::with_capacity(12);
        for w in (0..12).rev() {
            let since = format!("--since={}.weeks", w + 1);
            let until = format!("--until={}.weeks", w);
            let count = git_command_output(dir, &["log", "--oneline", &since, &until])
                .map(|s| {
                    let n = s.lines().filter(|l| !l.trim().is_empty()).count() as u32;
                    n
                })
                .unwrap_or(0);
            weeks.push(count);
        }
        Ok(weeks)
    })
    .await
    .map_err(|e| format!("统计任务失败: {e}"))?
}

/// 返回 Git 仓库的贡献者列表（按提交数降序）。
#[derive(serde::Serialize)]
struct Contributor {
    name: String,
    email: String,
    commits: u32,
}

#[tauri::command]
async fn git_contributors(root: String) -> Result<Vec<Contributor>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        if !dir.is_dir() || !dir.join(".git").is_dir() {
            return Ok(vec![]);
        }
        // shortlog -sne outputs lines like "  123\tName <email>"
        let output = git_command_output(dir, &["shortlog", "-sne", "HEAD"]);
        let raw = match output {
            Some(s) => s,
            None => return Ok(vec![]),
        };
        let mut list = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Parse: "  count\tName <email>"
            let Some(tab_pos) = line.find('\t') else {
                continue;
            };
            let count_str = line[..tab_pos].trim();
            let info = &line[tab_pos + 1..];
            let commits: u32 = count_str.parse().unwrap_or(0);
            // Extract name and email from "Name <email>"
            let (name, email) = if let Some(lt) = info.rfind('<') {
                let name_part = info[..lt].trim().to_string();
                let email_part = if let Some(gt) = info.rfind('>') {
                    info[lt + 1..gt].trim().to_string()
                } else {
                    info[lt + 1..].trim().to_string()
                };
                (name_part, email_part)
            } else {
                (info.to_string(), String::new())
            };
            list.push(Contributor {
                name,
                email,
                commits,
            });
        }
        Ok(list)
    })
    .await
    .map_err(|e| format!("统计任务失败: {e}"))?
}

/// 检查 Git 仓库是否有未提交的本地修改。
#[derive(serde::Serialize)]
struct LocalChangeStatus {
    has_changes: bool,
    details: String,
}

#[tauri::command]
async fn git_check_local_changes(root: String) -> Result<LocalChangeStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        if !dir.is_dir() || !dir.join(".git").is_dir() {
            return Ok(LocalChangeStatus {
                has_changes: false,
                details: "非 Git 仓库".to_string(),
            });
        }
        // staged + unstaged + untracked
        let staged = git_command_output(dir, &["diff", "--cached", "--stat"])
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let unstaged = git_command_output(dir, &["diff", "--stat"])
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let untracked = git_command_output(dir, &["ls-files", "--others", "--exclude-standard"])
            .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
            .unwrap_or(0);

        let has_staged = !staged.is_empty();
        let has_unstaged = !unstaged.is_empty();
        let has_untracked = untracked > 0;

        let mut parts = Vec::new();
        if has_staged {
            let n = staged.lines().count();
            parts.push(format!("{} 个文件已暂存", n));
        }
        if has_unstaged {
            let n = unstaged.lines().count();
            parts.push(format!("{} 个文件已修改", n));
        }
        if untracked > 0 {
            parts.push(format!("{} 个未跟踪文件", untracked));
        }

        let has_changes = has_staged || has_unstaged || has_untracked;
        let details = if parts.is_empty() {
            "无本地修改".to_string()
        } else {
            parts.join("，")
        };

        Ok(LocalChangeStatus {
            has_changes,
            details,
        })
    })
    .await
    .map_err(|e| format!("检查任务失败: {e}"))?
}

/// 执行 git pull 拉取最新代码。
#[tauri::command]
async fn git_pull(root: String) -> Result<String, String> {
    let handle = tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        if !dir.is_dir() || !dir.join(".git").is_dir() {
            return Err("不是 Git 仓库".to_string());
        }
        use std::process::Command;
        let output = Command::new("git")
            .args(["pull"])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("执行 git pull 失败: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            return Err(if stderr.is_empty() { stdout } else { stderr });
        }
        Ok(if stdout.is_empty() { stderr } else { stdout })
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .map_err(|_| "网络超时，拉取失败。请检查网络连接后重试".to_string())?
        .map_err(|e| format!("拉取任务失败: {e}"))?
}

#[derive(serde::Serialize)]
struct ProjectGitInfo {
    is_repo: bool,
    branch: Option<String>,
    branches: Vec<String>,
    remote_url: Option<String>,
    remote_name: Option<String>,
    last_commit_hash: Option<String>,
    last_commit_message: Option<String>,
    last_commit_author: Option<String>,
    last_commit_date: Option<String>,
}

fn git_command_output(dir: &std::path::Path, args: &[&str]) -> Option<String> {
    use std::process::Command;
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_branches(dir: &std::path::Path) -> Vec<String> {
    use std::process::Command;
    let output = match Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(dir)
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// 检测项目目录的 Git 仓库信息。
#[tauri::command]
async fn detect_project_git_info(root: String) -> Result<ProjectGitInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        if !dir.is_dir() {
            return Err("路径不是文件夹".into());
        }

        let is_repo = dir.join(".git").is_dir();
        if !is_repo {
            return Ok(ProjectGitInfo {
                is_repo: false,
                branch: None,
                branches: Vec::new(),
                remote_url: None,
                remote_name: None,
                last_commit_hash: None,
                last_commit_message: None,
                last_commit_author: None,
                last_commit_date: None,
            });
        }

        let branch = git_command_output(dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
        let branches = git_branches(dir);
        let remote_url = git_command_output(dir, &["config", "--get", "remote.origin.url"]);
        let remote_name = remote_url
            .as_ref()
            .and_then(|_| git_command_output(dir, &["config", "--get", "remote.origin.name"]));
        let last_commit_hash = git_command_output(dir, &["log", "-1", "--format=%h"]);
        let last_commit_message = git_command_output(dir, &["log", "-1", "--format=%s"]);
        let last_commit_author = git_command_output(dir, &["log", "-1", "--format=%an"]);
        let last_commit_date = git_command_output(dir, &["log", "-1", "--format=%ar"]);

        Ok(ProjectGitInfo {
            is_repo: true,
            branch,
            branches,
            remote_url,
            remote_name,
            last_commit_hash,
            last_commit_message,
            last_commit_author,
            last_commit_date,
        })
    })
    .await
    .map_err(|e| format!("检测任务失败: {e}"))?
}

#[derive(serde::Serialize)]
struct BranchCommitInfo {
    hash: Option<String>,
    message: Option<String>,
    author: Option<String>,
    date: Option<String>,
}

/// 获取指定分支的最近一次提交信息。
#[tauri::command]
async fn detect_branch_commit_info(
    root: String,
    branch: String,
) -> Result<BranchCommitInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        if !dir.is_dir() {
            return Err("路径不是文件夹".into());
        }
        if !dir.join(".git").is_dir() {
            return Err("不是 Git 仓库".into());
        }

        let b = branch.trim();
        if b.is_empty() {
            return Err("分支名为空".into());
        }

        Ok(BranchCommitInfo {
            hash: git_command_output(dir, &["log", "-1", "--format=%h", b]),
            message: git_command_output(dir, &["log", "-1", "--format=%s", b]),
            author: git_command_output(dir, &["log", "-1", "--format=%an", b]),
            date: git_command_output(dir, &["log", "-1", "--format=%ar", b]),
        })
    })
    .await
    .map_err(|e| format!("检测任务失败: {e}"))?
}

#[tauri::command]
fn list_visible_project_skill_buckets(
    project_root: String,
) -> Result<Vec<skill_copy::VisibleProjectSkillBucket>, String> {
    skill_copy::list_visible_project_skill_buckets(&project_root)
}

#[tauri::command]
fn get_agent_skill_paths(
    app: AppHandle,
    agent_id: String,
) -> Result<serde_json::Value, String> {
    let default_paths = skill_copy::resolve_global_skill_buckets(Some(&app), &agent_id)?;
    let default_strs: Vec<String> = default_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let custom = storage::load_agent_custom_skill_paths(&app)?;
    let custom_paths = custom.get(&agent_id).cloned().unwrap_or_default();

    Ok(serde_json::json!({
        "defaultPaths": default_strs,
        "customPaths": custom_paths,
    }))
}

#[tauri::command]
fn set_agent_custom_skill_paths(
    app: AppHandle,
    agent_id: String,
    paths: Vec<String>,
) -> Result<(), String> {
    let mut all = storage::load_agent_custom_skill_paths(&app)?;
    if paths.is_empty() {
        all.remove(&agent_id);
    } else {
        all.insert(agent_id, paths);
    }
    storage::save_agent_custom_skill_paths(&app, &all)
}

/// 参数与前端 `invoke` 顶层 camelCase 字段一一对应（勿再用单字段 struct，否则需包一层 `{ args: {...} }`）。
#[tauri::command]
fn copy_skill_package(
    app: AppHandle,
    source_path: String,
    dest_kind: String,
    agent_id: String,
    bucket_index: usize,
    project_root: Option<String>,
    on_conflict: Option<String>,
    folder_name_prefix: Option<String>,
) -> Result<String, String> {
    let suffix = match on_conflict.as_deref() {
        Some("error") => false,
        _ => true,
    };
    skill_copy::perform_copy_with_options(
        Some(&app),
        &source_path,
        &dest_kind,
        &agent_id,
        bucket_index,
        project_root.as_deref(),
        suffix,
        folder_name_prefix.as_deref(),
    )
}

#[tauri::command]
fn delete_skill_at_path(path: String) -> Result<(), String> {
    skill_copy::perform_delete_skill(&path)
}

#[tauri::command]
async fn detect_github_repo_skills(
    repo_url: String,
) -> Result<github_import::GithubSkillDetectionResult, String> {
    github_import::detect_github_repo_skills(&repo_url).await
}

#[tauri::command]
async fn import_github_skill_to_destination(
    app: AppHandle,
    repo_url: String,
    skill_path: String,
    dest_kind: String,
    agent_id: String,
    bucket_index: usize,
    project_root: Option<String>,
    on_conflict: Option<String>,
) -> Result<String, String> {
    let suffix = match on_conflict.as_deref() {
        Some("error") => false,
        _ => true,
    };
    github_import::import_github_skill_to_destination(
        &app,
        &repo_url,
        &skill_path,
        &dest_kind,
        &agent_id,
        bucket_index,
        project_root.as_deref(),
        suffix,
    )
    .await
}

#[tauri::command]
fn get_prompt_library(app: AppHandle) -> Result<prompt_library::PromptLibraryFile, String> {
    prompt_library::load_prompt_library(&app)
}

#[tauri::command]
fn save_prompt_library(
    app: AppHandle,
    library: prompt_library::PromptLibraryFile,
) -> Result<(), String> {
    prompt_library::save_prompt_library(&app, library)
}

#[tauri::command]
fn convert_prompt_to_my_skill(
    app: AppHandle,
    title: String,
    prompt: String,
    output_type: String,
    output_example: String,
    command_name: Option<String>,
) -> Result<my_skills_library::MySkillItem, String> {
    my_skills_library::convert_prompt_to_my_skill(
        &app,
        title,
        prompt,
        output_type,
        output_example,
        command_name,
    )
}

#[tauri::command]
fn apply_prompt_command_to_agent(
    app: AppHandle,
    agent_id: String,
    title: String,
    prompt: String,
    command_name: String,
) -> Result<String, String> {
    prompt_library::apply_prompt_command_to_agent(&app, &agent_id, &title, &prompt, &command_name)
}

#[tauri::command]
fn get_resource_library(app: AppHandle) -> Result<resource_library::ResourceLibraryFile, String> {
    resource_library::load_resource_library(&app)
}

#[tauri::command]
fn save_resource_library(
    app: AppHandle,
    library: resource_library::ResourceLibraryFile,
) -> Result<(), String> {
    resource_library::save_resource_library(&app, library)
}

#[tauri::command]
fn get_my_skills_library(app: AppHandle) -> Result<my_skills_library::MySkillsLibraryFile, String> {
    my_skills_library::load_my_skills_library(&app)
}

#[tauri::command]
fn add_skill_to_my_library(
    app: AppHandle,
    source_path: String,
) -> Result<my_skills_library::MySkillItem, String> {
    my_skills_library::add_skill_to_my_library(&app, source_path)
}

#[tauri::command]
fn remove_my_skill(app: AppHandle, id: String) -> Result<(), String> {
    my_skills_library::remove_my_skill(&app, id)
}

#[tauri::command]
fn get_gitee_settings(app: AppHandle) -> Result<storage::GiteeSettingsPublic, String> {
    storage::get_gitee_settings_public(&app)
}

#[tauri::command]
fn save_gitee_app(
    app: AppHandle,
    client_id: String,
    client_secret: String,
    repo_name: String,
) -> Result<(), String> {
    storage::save_gitee_app(&app, client_id, client_secret, repo_name)
}

#[tauri::command]
async fn gitee_oauth_login(app: AppHandle) -> Result<String, String> {
    gitee::oauth_login(app).await
}

/// `force` 默认 `true`：手动备份始终上传。定时任务使用 `force: false` 以在无变更时跳过。
#[tauri::command]
async fn gitee_backup_now(app: AppHandle, force: Option<bool>) -> Result<String, String> {
    gitee::backup_now(app, force.unwrap_or(true)).await
}

#[tauri::command]
fn gitee_disconnect(app: AppHandle) -> Result<(), String> {
    gitee::disconnect(&app)
}

#[tauri::command]
async fn gitee_restore_from_repo_url(app: AppHandle, repo_url: String) -> Result<String, String> {
    gitee::restore_from_repo_url(app, repo_url).await
}

#[tauri::command]
fn get_gitee_sync_status(app: AppHandle) -> Result<gitee::GiteeSyncStatusPublic, String> {
    gitee::get_gitee_sync_status(&app)
}

pub fn run() {
    let is_quitting = Arc::new(AtomicBool::new(false));
    let close_is_quitting = Arc::clone(&is_quitting);
    let tray_is_quitting = Arc::clone(&is_quitting);

    tauri::Builder::default()
        .manage(ClaudeCompletionState::default())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(move |window, event| {
            if window.label() == MAIN_WINDOW_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    if !close_is_quitting.load(Ordering::SeqCst) {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
            if window.label() == FLOAT_BALL_WINDOW_LABEL {
                if let WindowEvent::Moved(position) = event {
                    let position = collapsed_float_ball_position(window, *position);
                    let position = clamp_float_ball_position(
                        window.app_handle(),
                        position,
                        collapsed_float_ball_size(window),
                    );
                    save_float_ball_position(window.app_handle(), position);
                }
            }
        })
        .setup(move |app| {
            setup_tray(app, Arc::clone(&tray_is_quitting))?;
            configure_float_ball(app.handle());
            start_float_ball_hover_watcher(app.handle().clone());
            start_claude_hook_listener(app.handle().clone());
            gitee::sync_ui_schedule_next_in_secs(300);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    gitee::sync_ui_schedule_next_in_secs(300);
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                    gitee::backup_periodic_tick(handle.clone()).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_detected_agents,
            add_user_agent_from_path,
            remove_agent_from_sidebar,
            clear_hidden_sidebar_agents,
            get_agent_global_inventory,
            scan_project_directory,
            read_skill_document,
            get_ai_provider,
            save_ai_provider,
            get_glm_settings,
            save_glm_settings,
            test_glm_connection,
            get_deepseek_settings,
            save_deepseek_settings,
            test_deepseek_connection,
            deepseek_classify_inventory,
            deepseek_summarize_inventory,
            deepseek_resummarize_asset,
            deepseek_enrich_resource_url,
            deepseek_regenerate_categories,
            deepseek_translate_custom_categories,
            deepseek_reclassify_with_new_categories,
            get_custom_categories,
            clear_custom_categories,
            reset_all_categories,
            reveal_path_in_folder,
            open_project_path,
            focus_main_project,
            detect_claude_hook_status_command,
            install_claude_hooks_command,
            remove_claude_hooks_command,
            get_project_latest_mtime_ms,
            count_project_code_lines,
            read_package_version,
            estimate_project_progress,
            git_commit_count_last_n_days,
            git_weekly_commit_counts,
            git_contributors,
            git_check_local_changes,
            git_pull,
            detect_project_git_info,
            detect_branch_commit_info,
            copy_skill_package,
            delete_skill_at_path,
            detect_github_repo_skills,
            import_github_skill_to_destination,
            list_visible_project_skill_buckets,
            get_agent_skill_paths,
            set_agent_custom_skill_paths,
            get_prompt_library,
            save_prompt_library,
            convert_prompt_to_my_skill,
            apply_prompt_command_to_agent,
            get_resource_library,
            save_resource_library,
            get_my_skills_library,
            add_skill_to_my_library,
            remove_my_skill,
            get_gitee_settings,
            save_gitee_app,
            gitee_oauth_login,
            gitee_backup_now,
            gitee_disconnect,
            gitee_restore_from_repo_url,
            get_gitee_sync_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
