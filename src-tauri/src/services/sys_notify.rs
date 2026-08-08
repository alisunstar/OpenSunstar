//! OS 系统通知封装（工作区重构 2026-07-30）。
//!
//! 背景：预算告警此前只有应用内 toast（`useBudgetAlerts` → sonner），
//! 主窗口一关告警归零。这里把**真正烧得起的事**升级为系统级通知：
//!
//! - 预算 **critical / emergency**（warning 不发，不打扰）；
//! - 代理**故障转移**（key 熔断后自动切换——全天最有价值的一条通知）。
//!
//! 两级节流防轰炸：
//! 1. 预算按「provider+period+level」10 分钟窗只发一次（用量日志是高频
//!    写入，`budget_alert` 每次都会重算并重复 emit，不节流会把用户炸飞）；
//! 2. 故障转移按「app+to_provider」2 分钟窗只发一次（同一轮抖动可能
//!    连续触发多次切换）。
//!
//! 发送失败只记日志不抛出 —— 通知是增强，不是主链路。
//!
//! 实现：使用 `tauri-winrt-notification` 直接发送 Windows toast 通知，
//! 无需 `tauri-plugin-notification` 的 JS 桥接层，依赖更轻。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::budget_alert::{AlertLevel, BudgetAlert};

const BUDGET_THROTTLE: Duration = Duration::from_secs(10 * 60);
const FAILOVER_THROTTLE: Duration = Duration::from_secs(2 * 60);

static LAST_NOTIFY: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

fn throttled(key: &str, window: Duration) -> bool {
    let mut guard = LAST_NOTIFY.lock().unwrap_or_else(|p| p.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let now = Instant::now();
    match map.get(key) {
        Some(last) if now.duration_since(*last) < window => true,
        _ => {
            map.insert(key.to_string(), now);
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn notify(title: &str, body: &str) {
    use tauri_winrt_notification::Toast;
    let result = Toast::new("OpenSunstar").title(title).text1(body).show();
    if let Err(e) = result {
        log::warn!("[sys-notify] 发送系统通知失败: {e}");
    }
}

#[cfg(not(target_os = "windows"))]
fn notify(title: &str, body: &str) {
    // 非 Windows 平台：仅记日志，不发送系统通知。
    // macOS/Linux 的通知机制后续按需扩展。
    log::info!("[sys-notify] {title} — {body}");
}

/// 预算告警的系统通知。仅 critical / emergency 发系统级；warning 留在应用内。
pub fn notify_budget(_handle: &tauri::AppHandle, alert: &BudgetAlert) {
    if alert.alert_level == AlertLevel::Warning {
        return;
    }
    let settings = crate::settings::get_settings();
    if !settings.notification_preferences.budget_alert {
        return;
    }
    let key = format!(
        "budget:{}:{}:{:?}",
        alert.provider_id, alert.period, alert.alert_level
    );
    if throttled(&key, BUDGET_THROTTLE) {
        return;
    }
    let period_label = if alert.period == "daily" {
        "日"
    } else {
        "月"
    };
    let title = if alert.alert_level == AlertLevel::Emergency {
        format!("🔴 预算严重超限 · {}", alert.provider_name)
    } else {
        format!("🚨 预算超限 · {}", alert.provider_name)
    };
    let body = format!(
        "{}用量已达 {:.0}%（${:.2} / ${:.2}）",
        period_label, alert.percentage, alert.usage_usd, alert.limit_usd
    );
    notify(&title, &body);
}

/// 故障转移的系统通知：自动切换完成 = 用户的 key 挂了但任务保住了。
pub fn notify_failover(
    _handle: &tauri::AppHandle,
    app_type: &str,
    from_name: Option<&str>,
    to_name: &str,
) {
    let settings = crate::settings::get_settings();
    if !settings.notification_preferences.failover_alert {
        return;
    }
    let key = format!("failover:{app_type}:{to_name}");
    if throttled(&key, FAILOVER_THROTTLE) {
        return;
    }
    let from = from_name.unwrap_or("当前供应商");
    notify(
        &format!("🔴 {from} 已熔断"),
        &format!("{app_type} 已自动切换到 {to_name}，进行中的任务未中断"),
    );
}

/// 测试与调试用：清空节流状态。
#[cfg(test)]
pub fn reset_throttle() {
    if let Ok(mut guard) = LAST_NOTIFY.lock() {
        *guard = None;
    }
}
