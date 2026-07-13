//! Budget alert engine — checks provider spending limits after each usage log.
//!
//! Alert levels:
//! - 80 % threshold → **warning** (emit event to frontend)
//! - 100 % threshold → **critical** (emit event + log)
//! - 120 % threshold → **emergency / auto-pause suggestion** (emit event + log)
//!
//! Integration: called from the usage logger after every write via
//! [`notify_after_log`], which debounces (2 s) then checks all providers
//! that have `limitDailyUsd` / `limitMonthlyUsd` set in their meta JSON.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::database::{lock_conn, Database};
use crate::error::AppError;

pub const EVENT_BUDGET_ALERT: &str = "budget-alert";

const CHECK_DEBOUNCE: Duration = Duration::from_secs(2);

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static DB_REF: OnceLock<Arc<Database>> = OnceLock::new();
static CHECK_SCHEDULED: AtomicBool = AtomicBool::new(false);

// ── public types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetAlert {
    pub provider_id: String,
    pub app_type: String,
    pub provider_name: String,
    pub alert_level: AlertLevel,
    /// `"daily"` or `"monthly"`
    pub period: String,
    pub usage_usd: f64,
    pub limit_usd: f64,
    /// Percentage of limit consumed (e.g. 85.3 means 85.3 %).
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Warning,   // ≥ 80 %
    Critical,  // ≥ 100 %
    Emergency, // ≥ 120 %
}

// ── init ────────────────────────────────────────────────────────

/// Inject the `AppHandle` (call once from `lib.rs` setup).
pub fn init(handle: AppHandle) {
    if APP_HANDLE.set(handle).is_err() {
        log::debug!("budget_alert::init called again, ignoring");
    }
}

/// Inject the shared `Database` handle (call once from `lib.rs` setup).
pub fn init_db(db: Arc<Database>) {
    if DB_REF.set(db).is_err() {
        log::debug!("budget_alert::init_db called again, ignoring");
    }
}

// ── trigger (called from usage logger) ──────────────────────────

/// Fire-and-forget: schedule a debounced budget check.
///
/// Safe to call from any thread; will silently no-op if init has not
/// been called yet (e.g. during unit tests).
pub fn notify_after_log() {
    if APP_HANDLE.get().is_none() || DB_REF.get().is_none() {
        return;
    }
    if CHECK_SCHEDULED.swap(true, Ordering::AcqRel) {
        return;
    }

    std::thread::spawn(move || {
        std::thread::sleep(CHECK_DEBOUNCE);
        CHECK_SCHEDULED.store(false, Ordering::Release);
        if let Err(e) = run_budget_checks() {
            log::warn!("Budget alert check failed: {e}");
        }
    });
}

// ── core logic ──────────────────────────────────────────────────

fn run_budget_checks() -> Result<(), AppError> {
    let handle = match APP_HANDLE.get() {
        Some(h) => h,
        None => return Ok(()),
    };
    let db = match DB_REF.get() {
        Some(d) => d,
        None => return Ok(()),
    };

    let conn = lock_conn!(db.conn);

    let mut stmt = conn
        .prepare(
            "SELECT id, app_type, name, meta FROM providers \
             WHERE meta LIKE '%limitDailyUsd%' OR meta LIKE '%limitMonthlyUsd%'",
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    let providers: Vec<(String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| AppError::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let month_start = &today[..7]; // "YYYY-MM"

    for (provider_id, app_type, provider_name, meta_str) in &providers {
        let meta: serde_json::Value = match serde_json::from_str(meta_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let limit_daily = parse_limit(&meta, "limitDailyUsd");
        let limit_monthly = parse_limit(&meta, "limitMonthlyUsd");

        if let Some(daily_limit) = limit_daily {
            if daily_limit > 0.0 {
                let daily_usage: f64 = conn
                    .query_row(
                        "SELECT COALESCE(SUM(CAST(total_cost_usd AS REAL)), 0) \
                         FROM proxy_request_logs \
                         WHERE provider_id = ?1 AND app_type = ?2 \
                           AND date(created_at, 'unixepoch') = ?3",
                        rusqlite::params![provider_id, app_type, today],
                        |row| row.get(0),
                    )
                    .unwrap_or(0.0);

                if let Some(level) = classify_alert(daily_usage / daily_limit * 100.0) {
                    let pct = daily_usage / daily_limit * 100.0;
                    let alert = BudgetAlert {
                        provider_id: provider_id.clone(),
                        app_type: app_type.clone(),
                        provider_name: provider_name.clone(),
                        alert_level: level,
                        period: "daily".to_string(),
                        usage_usd: daily_usage,
                        limit_usd: daily_limit,
                        percentage: pct,
                    };
                    emit_alert(handle, &alert);
                }
            }
        }

        if let Some(monthly_limit) = limit_monthly {
            if monthly_limit > 0.0 {
                let month_start_date = format!("{month_start}-01");
                let monthly_usage: f64 = conn
                    .query_row(
                        "SELECT COALESCE(SUM(CAST(total_cost_usd AS REAL)), 0) \
                         FROM proxy_request_logs \
                         WHERE provider_id = ?1 AND app_type = ?2 \
                           AND date(created_at, 'unixepoch') >= ?3",
                        rusqlite::params![provider_id, app_type, month_start_date],
                        |row| row.get(0),
                    )
                    .unwrap_or(0.0);

                if let Some(level) = classify_alert(monthly_usage / monthly_limit * 100.0) {
                    let pct = monthly_usage / monthly_limit * 100.0;
                    let alert = BudgetAlert {
                        provider_id: provider_id.clone(),
                        app_type: app_type.clone(),
                        provider_name: provider_name.clone(),
                        alert_level: level,
                        period: "monthly".to_string(),
                        usage_usd: monthly_usage,
                        limit_usd: monthly_limit,
                        percentage: pct,
                    };
                    emit_alert(handle, &alert);
                }
            }
        }
    }

    Ok(())
}

// ── helpers ─────────────────────────────────────────────────────

/// Parse a limit value from provider meta JSON.
///
/// The field may be stored as a JSON number *or* a JSON string; we
/// handle both.
fn parse_limit(meta: &serde_json::Value, key: &str) -> Option<f64> {
    let v = meta.get(key)?;
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn classify_alert(percentage: f64) -> Option<AlertLevel> {
    if percentage >= 120.0 {
        Some(AlertLevel::Emergency)
    } else if percentage >= 100.0 {
        Some(AlertLevel::Critical)
    } else if percentage >= 80.0 {
        Some(AlertLevel::Warning)
    } else {
        None
    }
}

fn emit_alert(handle: &AppHandle, alert: &BudgetAlert) {
    log::info!(
        "[budget-alert] {} {} {}: {:.1}% of {} ${:.4} limit (${:.4} used)",
        alert.provider_name,
        alert.period,
        format!("{:?}", alert.alert_level).to_uppercase(),
        alert.percentage,
        alert.period,
        alert.limit_usd,
        alert.usage_usd,
    );
    if let Err(e) = handle.emit(EVENT_BUDGET_ALERT, alert) {
        log::warn!("Failed to emit {EVENT_BUDGET_ALERT}: {e}");
    }
}
