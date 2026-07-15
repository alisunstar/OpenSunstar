use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuickStartOperationStatus {
    Pending,
    Applying,
    Verifying,
    Succeeded,
    Failed,
    RollingBack,
    RolledBack,
    RollbackFailed,
}

impl QuickStartOperationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Applying => "applying",
            Self::Verifying => "verifying",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::RollingBack => "rolling_back",
            Self::RolledBack => "rolled_back",
            Self::RollbackFailed => "rollback_failed",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::RolledBack)
    }

    fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Applying)
                | (Self::Pending, Self::Failed)
                | (Self::Pending, Self::RollingBack)
                | (Self::Applying, Self::Verifying)
                | (Self::Applying, Self::Failed)
                | (Self::Applying, Self::RollingBack)
                | (Self::Verifying, Self::Succeeded)
                | (Self::Verifying, Self::Failed)
                | (Self::Verifying, Self::RollingBack)
                | (Self::Failed, Self::Applying)
                | (Self::Failed, Self::RollingBack)
                | (Self::Succeeded, Self::RollingBack)
                | (Self::RollingBack, Self::RolledBack)
                | (Self::RollingBack, Self::RollbackFailed)
                | (Self::RollbackFailed, Self::RollingBack)
        )
    }
}

impl fmt::Display for QuickStartOperationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for QuickStartOperationStatus {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "applying" => Ok(Self::Applying),
            "verifying" => Ok(Self::Verifying),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "rolling_back" => Ok(Self::RollingBack),
            "rolled_back" => Ok(Self::RolledBack),
            "rollback_failed" => Ok(Self::RollbackFailed),
            other => Err(AppError::Database(format!(
                "Unknown QuickStart operation status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickStartOperation {
    pub id: String,
    pub idempotency_key: String,
    #[serde(skip_serializing)]
    pub request_fingerprint: String,
    pub app_type: String,
    pub provider_id: Option<String>,
    pub previous_provider_id: Option<String>,
    #[serde(skip_serializing)]
    pub live_snapshot: Option<String>,
    #[serde(skip_serializing)]
    pub applied_live_fingerprint: Option<String>,
    pub status: QuickStartOperationStatus,
    pub current_step: String,
    pub revision: i64,
    pub provider_created: bool,
    pub provider_switched: bool,
    pub takeover_enabled: bool,
    pub proxy_started: bool,
    pub post_verified: bool,
    pub takeover_was_enabled: bool,
    pub proxy_was_running: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickStartBeginResult {
    pub operation: QuickStartOperation,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickStartOperationEvent {
    pub sequence: i64,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub step: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub detail_json: Option<String>,
    pub created_at: String,
}

impl Database {
    pub fn begin_quick_start_operation(
        &self,
        idempotency_key: &str,
        request_fingerprint: &str,
        app_type: &str,
    ) -> Result<QuickStartBeginResult, AppError> {
        if idempotency_key.trim().is_empty() || idempotency_key.len() > 128 {
            return Err(AppError::InvalidInput(
                "QuickStart idempotency key must contain 1-128 characters".to_string(),
            ));
        }
        if !matches!(app_type, "claude" | "claude-desktop" | "codex" | "gemini") {
            return Err(AppError::InvalidInput(format!(
                "Unsupported QuickStart app type: {app_type}"
            )));
        }

        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let existing = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_IDEMPOTENCY,
                [idempotency_key],
                map_operation,
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(operation) = existing {
            if operation.request_fingerprint != request_fingerprint
                || operation.app_type != app_type
            {
                return Err(AppError::InvalidInput(
                    "QUICKSTART_IDEMPOTENCY_CONFLICT: key already belongs to another request"
                        .to_string(),
                ));
            }
            tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
            return Ok(QuickStartBeginResult {
                operation,
                created: false,
            });
        }

        let active_operation: Option<String> = tx
            .query_row(
                "SELECT id FROM quick_start_operations
                 WHERE app_type = ?1
                   AND status IN ('pending','applying','verifying','rolling_back','rollback_failed')
                 LIMIT 1",
                [app_type],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        if let Some(active_operation) = active_operation {
            return Err(AppError::Message(format!(
                "QUICKSTART_APP_BUSY: recover or finish operation {active_operation} first"
            )));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO quick_start_operations
             (id, idempotency_key, request_fingerprint, app_type, status, current_step,
              revision, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', 'accepted', 0, ?5, ?5)",
            params![id, idempotency_key, request_fingerprint, app_type, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        tx.execute(
            "INSERT INTO quick_start_operation_events
             (operation_id, sequence, event_type, to_status, step, created_at)
             VALUES (?1, 1, 'operation_started', 'pending', 'accepted', ?2)",
            params![id, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        let operation = tx
            .query_row(QUICK_START_OPERATION_SELECT_BY_ID, [&id], map_operation)
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(QuickStartBeginResult {
            operation,
            created: true,
        })
    }

    pub fn get_quick_start_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<QuickStartOperation>, AppError> {
        let conn = lock_conn!(self.conn);
        conn.query_row(
            QUICK_START_OPERATION_SELECT_BY_ID,
            [operation_id],
            map_operation,
        )
        .optional()
        .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn list_recoverable_quick_start_operations(
        &self,
    ) -> Result<Vec<QuickStartOperation>, AppError> {
        let conn = lock_conn!(self.conn);
        let sql = format!(
            "{} WHERE status IN ('pending','applying','verifying','rolling_back','rollback_failed')
             ORDER BY updated_at DESC",
            QUICK_START_OPERATION_SELECT_BASE
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;
        let operations = stmt
            .query_map([], map_operation)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(operations)
    }

    /// Return terminal operations so completed work remains auditable and
    /// roll-backable after a renderer or application restart.
    pub fn list_recent_quick_start_operations(
        &self,
        limit: usize,
    ) -> Result<Vec<QuickStartOperation>, AppError> {
        let limit = i64::try_from(limit.clamp(1, 100)).map_err(|_| {
            AppError::InvalidInput("QuickStart history limit is invalid".to_string())
        })?;
        let conn = lock_conn!(self.conn);
        let sql = format!(
            "{} WHERE status IN ('succeeded','failed','rolled_back','rollback_failed')
             ORDER BY updated_at DESC, id DESC LIMIT ?1",
            QUICK_START_OPERATION_SELECT_BASE
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;
        let operations = stmt
            .query_map([limit], map_operation)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(operations)
    }

    pub fn list_quick_start_operation_events(
        &self,
        operation_id: &str,
    ) -> Result<Vec<QuickStartOperationEvent>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT sequence, event_type, from_status, to_status, step,
                        error_code, error_message, detail_json, created_at
                 FROM quick_start_operation_events
                 WHERE operation_id = ?1 ORDER BY sequence",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let events = stmt
            .query_map([operation_id], |row| {
                Ok(QuickStartOperationEvent {
                    sequence: row.get(0)?,
                    event_type: row.get(1)?,
                    from_status: row.get(2)?,
                    to_status: row.get(3)?,
                    step: row.get(4)?,
                    error_code: row.get(5)?,
                    error_message: row.get(6)?,
                    detail_json: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(events)
    }

    pub fn transition_quick_start_operation(
        &self,
        operation_id: &str,
        expected_revision: i64,
        next_status: QuickStartOperationStatus,
        step: &str,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<QuickStartOperation, AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let current = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_ID,
                [operation_id],
                map_operation,
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                AppError::InvalidInput(format!("QuickStart operation not found: {operation_id}"))
            })?;
        if current.revision != expected_revision {
            return Err(AppError::Message(format!(
                "QUICKSTART_REVISION_CONFLICT: expected {expected_revision}, actual {}",
                current.revision
            )));
        }
        if !current.status.can_transition_to(next_status) {
            return Err(AppError::InvalidInput(format!(
                "QUICKSTART_INVALID_TRANSITION: {} -> {}",
                current.status, next_status
            )));
        }

        let now = chrono::Utc::now().to_rfc3339();
        let completed_at = next_status.is_terminal().then_some(now.as_str());
        let changed = tx
            .execute(
                "UPDATE quick_start_operations
                 SET status = ?1, current_step = ?2, revision = revision + 1,
                     error_code = ?3, error_message = ?4, updated_at = ?5,
                     completed_at = ?6
                 WHERE id = ?7 AND revision = ?8",
                params![
                    next_status.as_str(),
                    step,
                    error_code,
                    error_message,
                    now,
                    completed_at,
                    operation_id,
                    expected_revision
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if changed != 1 {
            return Err(AppError::Message(
                "QUICKSTART_REVISION_CONFLICT: operation changed concurrently".to_string(),
            ));
        }
        tx.execute(
            "INSERT INTO quick_start_operation_events
             (operation_id, sequence, event_type, from_status, to_status, step,
              error_code, error_message, created_at)
             SELECT ?1, COALESCE(MAX(sequence), 0) + 1, 'status_changed', ?2, ?3, ?4,
                    ?5, ?6, ?7
             FROM quick_start_operation_events WHERE operation_id = ?1",
            params![
                operation_id,
                current.status.as_str(),
                next_status.as_str(),
                step,
                error_code,
                error_message,
                now
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        let updated = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_ID,
                [operation_id],
                map_operation,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(updated)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_quick_start_progress(
        &self,
        operation_id: &str,
        expected_revision: i64,
        step: &str,
        provider_id: Option<&str>,
        previous_provider_id: Option<&str>,
        flag_column: Option<&str>,
        takeover_was_enabled: Option<bool>,
        proxy_was_running: Option<bool>,
    ) -> Result<QuickStartOperation, AppError> {
        let allowed_flag = match flag_column {
            None => None,
            Some(
                flag @ ("provider_created" | "provider_switched" | "takeover_enabled"
                | "proxy_started" | "post_verified"),
            ) => Some(flag),
            Some(other) => {
                return Err(AppError::InvalidInput(format!(
                    "Invalid QuickStart progress flag: {other}"
                )))
            }
        };
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let current = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_ID,
                [operation_id],
                map_operation,
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                AppError::InvalidInput(format!("QuickStart operation not found: {operation_id}"))
            })?;
        if current.revision != expected_revision {
            return Err(AppError::Message(format!(
                "QUICKSTART_REVISION_CONFLICT: expected {expected_revision}, actual {}",
                current.revision
            )));
        }

        tx.execute(
            "UPDATE quick_start_operations
             SET current_step = ?1, provider_id = COALESCE(?2, provider_id),
                 previous_provider_id = COALESCE(?3, previous_provider_id),
                 takeover_was_enabled = COALESCE(?4, takeover_was_enabled),
                 proxy_was_running = COALESCE(?5, proxy_was_running),
                 revision = revision + 1, updated_at = ?6
             WHERE id = ?7 AND revision = ?8",
            params![
                step,
                provider_id,
                previous_provider_id,
                takeover_was_enabled.map(i64::from),
                proxy_was_running.map(i64::from),
                chrono::Utc::now().to_rfc3339(),
                operation_id,
                expected_revision
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        if let Some(flag) = allowed_flag {
            let sql = format!(
                "UPDATE quick_start_operations SET {flag} = 1 WHERE id = ?1 AND revision = ?2"
            );
            tx.execute(&sql, params![operation_id, expected_revision + 1])
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO quick_start_operation_events
             (operation_id, sequence, event_type, from_status, to_status, step, detail_json, created_at)
             SELECT ?1, COALESCE(MAX(sequence), 0) + 1, 'step_completed', ?2, ?2, ?3, ?4, ?5
             FROM quick_start_operation_events WHERE operation_id = ?1",
            params![
                operation_id,
                current.status.as_str(),
                step,
                allowed_flag.map(|flag| format!(r#"{{"flag":"{flag}"}}"#)),
                now
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        let updated = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_ID,
                [operation_id],
                map_operation,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(updated)
    }

    pub fn record_quick_start_live_snapshot(
        &self,
        operation_id: &str,
        expected_revision: i64,
        sealed_snapshot: &str,
    ) -> Result<QuickStartOperation, AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let changed = tx
            .execute(
                "UPDATE quick_start_operations
                 SET live_snapshot = ?1, current_step = 'live_snapshot_captured',
                     revision = revision + 1, updated_at = ?2
                 WHERE id = ?3 AND revision = ?4 AND live_snapshot IS NULL",
                params![sealed_snapshot, now, operation_id, expected_revision],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if changed != 1 {
            return Err(AppError::Message(
                "QUICKSTART_REVISION_CONFLICT: live snapshot was already recorded or operation changed"
                    .to_string(),
            ));
        }
        tx.execute(
            "INSERT INTO quick_start_operation_events
             (operation_id, sequence, event_type, from_status, to_status, step, detail_json, created_at)
             SELECT ?1,
                    (SELECT COALESCE(MAX(sequence), 0) + 1
                     FROM quick_start_operation_events WHERE operation_id = ?1),
                    'snapshot_captured', status, status,
                    'live_snapshot_captured', '{\"encrypted\":true}', ?2
             FROM quick_start_operations WHERE id = ?1",
            params![operation_id, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        let updated = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_ID,
                [operation_id],
                map_operation,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(updated)
    }

    /// Append a safe receipt for the server-side upstream API Key verification.
    /// The receipt is immutable event evidence; credentials, full URLs and
    /// upstream bodies are intentionally not accepted by this API.
    pub fn record_quick_start_upstream_verification<T: Serialize>(
        &self,
        operation_id: &str,
        expected_revision: i64,
        receipt: &T,
    ) -> Result<QuickStartOperation, AppError> {
        let detail_json = serde_json::to_string(receipt).map_err(|error| {
            AppError::Serialization(format!("Serialize verification receipt failed: {error}"))
        })?;
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let current = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_ID,
                [operation_id],
                map_operation,
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                AppError::InvalidInput(format!("QuickStart operation not found: {operation_id}"))
            })?;
        if current.revision != expected_revision {
            return Err(AppError::Message(format!(
                "QUICKSTART_REVISION_CONFLICT: expected {expected_revision}, actual {}",
                current.revision
            )));
        }
        if current.status != QuickStartOperationStatus::Verifying {
            return Err(AppError::InvalidInput(
                "QuickStart upstream verification requires verifying status".to_string(),
            ));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let changed = tx
            .execute(
                "UPDATE quick_start_operations
                 SET current_step = 'upstream_verified', revision = revision + 1, updated_at = ?1
                 WHERE id = ?2 AND revision = ?3",
                params![now, operation_id, expected_revision],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if changed != 1 {
            return Err(AppError::Message(
                "QUICKSTART_REVISION_CONFLICT: upstream verification receipt was not recorded"
                    .to_string(),
            ));
        }
        tx.execute(
            "INSERT INTO quick_start_operation_events
             (operation_id, sequence, event_type, from_status, to_status, step, detail_json, created_at)
             SELECT ?1, COALESCE(MAX(sequence), 0) + 1, 'upstream_verification_succeeded',
                    ?2, ?2, 'upstream_verified', ?3, ?4
             FROM quick_start_operation_events WHERE operation_id = ?1",
            params![operation_id, current.status.as_str(), detail_json, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        let updated = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_ID,
                [operation_id],
                map_operation,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(updated)
    }

    pub fn record_quick_start_applied_live_fingerprint(
        &self,
        operation_id: &str,
        expected_revision: i64,
        fingerprint: &str,
    ) -> Result<QuickStartOperation, AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let changed = tx
            .execute(
                "UPDATE quick_start_operations
                 SET applied_live_fingerprint = ?1, current_step = 'rollback_guard_captured',
                     revision = revision + 1, updated_at = ?2
                 WHERE id = ?3 AND revision = ?4 AND applied_live_fingerprint IS NULL",
                params![fingerprint, now, operation_id, expected_revision],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if changed != 1 {
            return Err(AppError::Message(
                "QUICKSTART_REVISION_CONFLICT: rollback guard was already recorded or operation changed"
                    .to_string(),
            ));
        }
        tx.execute(
            "INSERT INTO quick_start_operation_events
             (operation_id, sequence, event_type, from_status, to_status, step, detail_json, created_at)
             SELECT ?1, (SELECT COALESCE(MAX(sequence), 0) + 1
                         FROM quick_start_operation_events WHERE operation_id = ?1),
                    'rollback_guard_captured', status, status,
                    'rollback_guard_captured', '{\"sha256\":true}', ?2
             FROM quick_start_operations WHERE id = ?1",
            params![operation_id, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        let updated = tx
            .query_row(
                QUICK_START_OPERATION_SELECT_BY_ID,
                [operation_id],
                map_operation,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(updated)
    }
}

const QUICK_START_OPERATION_SELECT_BASE: &str =
    "SELECT id, idempotency_key, request_fingerprint, app_type, provider_id,
            previous_provider_id, live_snapshot, status, current_step, revision, provider_created,
            provider_switched, takeover_enabled, proxy_started, post_verified,
            takeover_was_enabled, proxy_was_running, error_code, error_message,
            created_at, updated_at, completed_at, applied_live_fingerprint
     FROM quick_start_operations";
const QUICK_START_OPERATION_SELECT_BY_ID: &str =
    "SELECT id, idempotency_key, request_fingerprint, app_type, provider_id,
            previous_provider_id, live_snapshot, status, current_step, revision, provider_created,
            provider_switched, takeover_enabled, proxy_started, post_verified,
            takeover_was_enabled, proxy_was_running, error_code, error_message,
            created_at, updated_at, completed_at, applied_live_fingerprint
     FROM quick_start_operations WHERE id = ?1";
const QUICK_START_OPERATION_SELECT_BY_IDEMPOTENCY: &str =
    "SELECT id, idempotency_key, request_fingerprint, app_type, provider_id,
            previous_provider_id, live_snapshot, status, current_step, revision, provider_created,
            provider_switched, takeover_enabled, proxy_started, post_verified,
            takeover_was_enabled, proxy_was_running, error_code, error_message,
            created_at, updated_at, completed_at, applied_live_fingerprint
     FROM quick_start_operations WHERE idempotency_key = ?1";

fn map_operation(row: &Row<'_>) -> rusqlite::Result<QuickStartOperation> {
    let status: String = row.get(7)?;
    let status = QuickStartOperationStatus::try_from(status.as_str()).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(QuickStartOperation {
        id: row.get(0)?,
        idempotency_key: row.get(1)?,
        request_fingerprint: row.get(2)?,
        app_type: row.get(3)?,
        provider_id: row.get(4)?,
        previous_provider_id: row.get(5)?,
        live_snapshot: row.get(6)?,
        applied_live_fingerprint: row.get(22)?,
        status,
        current_step: row.get(8)?,
        revision: row.get(9)?,
        provider_created: row.get::<_, i64>(10)? != 0,
        provider_switched: row.get::<_, i64>(11)? != 0,
        takeover_enabled: row.get::<_, i64>(12)? != 0,
        proxy_started: row.get::<_, i64>(13)? != 0,
        post_verified: row.get::<_, i64>(14)? != 0,
        takeover_was_enabled: row.get::<_, i64>(15)? != 0,
        proxy_was_running: row.get::<_, i64>(16)? != 0,
        error_code: row.get(17)?,
        error_message: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
        completed_at: row.get(21)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotency_returns_same_operation_and_rejects_request_reuse() {
        let db = Database::memory().expect("database");
        let first = db
            .begin_quick_start_operation("request-1", "fingerprint-a", "codex")
            .expect("begin first");
        assert!(first.created);

        let repeated = db
            .begin_quick_start_operation("request-1", "fingerprint-a", "codex")
            .expect("repeat request");
        assert!(!repeated.created);
        assert_eq!(repeated.operation.id, first.operation.id);

        let conflict = db
            .begin_quick_start_operation("request-1", "fingerprint-b", "codex")
            .expect_err("idempotency reuse must fail");
        assert!(conflict.to_string().contains("IDEMPOTENCY_CONFLICT"));
    }

    #[test]
    fn transitions_are_revision_guarded_and_audited() {
        let db = Database::memory().expect("database");
        let started = db
            .begin_quick_start_operation("request-2", "fingerprint", "claude")
            .expect("begin")
            .operation;
        let applying = db
            .transition_quick_start_operation(
                &started.id,
                0,
                QuickStartOperationStatus::Applying,
                "provider_add",
                None,
                None,
            )
            .expect("transition");
        assert_eq!(applying.revision, 1);

        let stale = db
            .transition_quick_start_operation(
                &started.id,
                0,
                QuickStartOperationStatus::Failed,
                "provider_add",
                Some("TEST"),
                Some("stale writer"),
            )
            .expect_err("stale revision must fail");
        assert!(stale.to_string().contains("REVISION_CONFLICT"));

        let invalid = db
            .transition_quick_start_operation(
                &started.id,
                1,
                QuickStartOperationStatus::Succeeded,
                "done",
                None,
                None,
            )
            .expect_err("invalid transition must fail");
        assert!(invalid.to_string().contains("INVALID_TRANSITION"));

        let event_count: i64 = db
            .conn
            .lock()
            .expect("lock")
            .query_row(
                "SELECT COUNT(*) FROM quick_start_operation_events WHERE operation_id = ?1",
                [&started.id],
                |row| row.get(0),
            )
            .expect("count events");
        assert_eq!(event_count, 2);
    }

    #[test]
    fn only_one_active_operation_is_allowed_per_app() {
        let db = Database::memory().expect("database");
        db.begin_quick_start_operation("request-a", "fingerprint-a", "gemini")
            .expect("first operation");
        let busy = db
            .begin_quick_start_operation("request-b", "fingerprint-b", "gemini")
            .expect_err("second active operation must be rejected");
        assert!(busy.to_string().contains("QUICKSTART_APP_BUSY"));
    }

    #[test]
    fn pending_operation_is_recoverable_and_can_enter_rollback() {
        let db = Database::memory().expect("database");
        let pending = db
            .begin_quick_start_operation("request-pending", "fingerprint", "codex")
            .expect("begin")
            .operation;

        assert!(db
            .list_recoverable_quick_start_operations()
            .expect("recoverable list")
            .iter()
            .any(|operation| operation.id == pending.id));

        let rolling_back = db
            .transition_quick_start_operation(
                &pending.id,
                pending.revision,
                QuickStartOperationStatus::RollingBack,
                "crash_recovery_started",
                None,
                None,
            )
            .expect("pending must be recoverable");
        assert_eq!(rolling_back.status, QuickStartOperationStatus::RollingBack);
    }

    #[test]
    fn upstream_verification_receipt_is_append_only_and_revision_guarded() {
        let db = Database::memory().expect("database");
        let begun = db
            .begin_quick_start_operation("upstream-receipt", "fingerprint", "codex")
            .expect("begin")
            .operation;
        let verifying = db
            .transition_quick_start_operation(
                &begun.id,
                begun.revision,
                QuickStartOperationStatus::Applying,
                "preflight",
                None,
                None,
            )
            .expect("applying");
        let verifying = db
            .transition_quick_start_operation(
                &verifying.id,
                verifying.revision,
                QuickStartOperationStatus::Verifying,
                "post_apply_verification",
                None,
                None,
            )
            .expect("verifying");

        let updated = db
            .record_quick_start_upstream_verification(
                &verifying.id,
                verifying.revision,
                &serde_json::json!({
                    "providerFingerprint": "sha256:receipt",
                    "protocol": "openai",
                    "endpointHost": "api.example.test",
                    "modelCount": 2,
                }),
            )
            .expect("record receipt");
        assert_eq!(updated.current_step, "upstream_verified");

        let events = db
            .list_quick_start_operation_events(&updated.id)
            .expect("events");
        let receipt_event = events
            .iter()
            .find(|event| event.event_type == "upstream_verification_succeeded")
            .expect("receipt event");
        assert_eq!(receipt_event.step, "upstream_verified");
        assert!(receipt_event
            .detail_json
            .as_deref()
            .is_some_and(|detail| detail.contains("api.example.test")));

        let stale = db
            .record_quick_start_upstream_verification(
                &updated.id,
                verifying.revision,
                &serde_json::json!({"providerFingerprint": "sha256:stale"}),
            )
            .expect_err("stale receipt writer must fail");
        assert!(stale.to_string().contains("REVISION_CONFLICT"));
    }

    #[test]
    fn recent_operation_history_keeps_terminal_operations_after_restart() {
        let db = Database::memory().expect("database");
        let begun = db
            .begin_quick_start_operation("history-success", "fingerprint", "codex")
            .expect("begin")
            .operation;
        let applying = db
            .transition_quick_start_operation(
                &begun.id,
                begun.revision,
                QuickStartOperationStatus::Applying,
                "preflight",
                None,
                None,
            )
            .expect("applying");
        let verifying = db
            .transition_quick_start_operation(
                &applying.id,
                applying.revision,
                QuickStartOperationStatus::Verifying,
                "verify",
                None,
                None,
            )
            .expect("verifying");
        db.transition_quick_start_operation(
            &verifying.id,
            verifying.revision,
            QuickStartOperationStatus::Succeeded,
            "completed",
            None,
            None,
        )
        .expect("succeeded");

        let history = db
            .list_recent_quick_start_operations(10)
            .expect("history");
        assert!(history.iter().any(|operation| {
            operation.id == begun.id && operation.status == QuickStartOperationStatus::Succeeded
        }));
    }
}
