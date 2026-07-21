//! 异步批量用量写入
//!
//! 代理热路径只做非阻塞入队；独立线程持有 `usage.db` 的第二 SQLite 连接（WAL），
//! 按批（最多 64 条或约 250ms）事务落库。
//!
//! 崩溃可能丢失最后一批未刷盘日志（可接受的用量统计折中）；`:memory:` 测试库
//! 无法共享第二连接，自动回退到同步 `UsageLogger`。

use super::logger::{insert_request_log, RequestLog, UsageLogger};
use crate::database::{apply_file_connection_pragmas, Database};
use crate::error::AppError;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const QUEUE_CAPACITY: usize = 4096;
const BATCH_MAX: usize = 64;
const FLUSH_INTERVAL: Duration = Duration::from_millis(250);

/// 代理用量异步写器（可 Clone：内部 Arc）。
#[derive(Clone)]
pub struct AsyncUsageWriter {
    inner: Arc<AsyncUsageWriterInner>,
}

struct AsyncUsageWriterInner {
    db: Arc<Database>,
    tx: Mutex<Option<SyncSender<RequestLog>>>,
    join: Mutex<Option<JoinHandle<()>>>,
    /// 已执行过 shutdown（幂等）。
    stopped: AtomicBool,
}

impl AsyncUsageWriter {
    /// 为文件库启动写线程；内存库则仅同步回退。
    pub fn new(db: Arc<Database>) -> Self {
        let inner = Arc::new(AsyncUsageWriterInner {
            db: db.clone(),
            tx: Mutex::new(None),
            join: Mutex::new(None),
            stopped: AtomicBool::new(false),
        });

        if let Some(path) = db.usage_path() {
            match spawn_writer(path.to_path_buf()) {
                Ok((tx, join)) => {
                    if let Ok(mut slot) = inner.tx.lock() {
                        *slot = Some(tx);
                    }
                    if let Ok(mut slot) = inner.join.lock() {
                        *slot = Some(join);
                    }
                    log::info!("异步用量写线程已启动");
                }
                Err(e) => {
                    log::warn!("异步用量写线程启动失败，热路径将同步写库: {e}");
                }
            }
        }

        Self { inner }
    }

    /// 非阻塞入队；队列满或写线程不可用时同步回退写主连接。
    pub fn enqueue(&self, log: RequestLog) {
        let send = {
            let guard = match self.inner.tx.lock() {
                Ok(g) => g,
                Err(_) => {
                    let _ = UsageLogger::new(&self.inner.db).log_request(&log);
                    return;
                }
            };
            match guard.as_ref() {
                Some(tx) => tx.try_send(log),
                None => {
                    drop(guard);
                    let _ = UsageLogger::new(&self.inner.db).log_request(&log);
                    return;
                }
            }
        };

        match send {
            Ok(()) => {}
            Err(TrySendError::Full(log)) => {
                log::warn!("用量写队列已满，同步回退写库");
                let _ = UsageLogger::new(&self.inner.db).log_request(&log);
            }
            Err(TrySendError::Disconnected(log)) => {
                let _ = UsageLogger::new(&self.inner.db).log_request(&log);
            }
        }
    }

    /// 关闭写线程并刷完队列（代理 stop / Drop 时调用，幂等）。
    pub fn flush_shutdown(&self) {
        if self
            .inner
            .stopped
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        if let Ok(mut guard) = self.inner.tx.lock() {
            *guard = None; // drop sender → 写线程 drain 后退出
        }

        if let Ok(mut guard) = self.inner.join.lock() {
            if let Some(handle) = guard.take() {
                if let Err(e) = handle.join() {
                    log::warn!("异步用量写线程 join 失败: {e:?}");
                } else {
                    log::info!("异步用量写线程已停止并刷盘");
                }
            }
        }
    }
}

impl Drop for AsyncUsageWriterInner {
    fn drop(&mut self) {
        // Arc 最后引用释放时刷盘；若已显式 shutdown 则 no-op。
        if self
            .stopped
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            if let Ok(mut guard) = self.tx.lock() {
                *guard = None;
            }
            if let Ok(mut guard) = self.join.lock() {
                if let Some(handle) = guard.take() {
                    let _ = handle.join();
                }
            }
        }
    }
}

fn spawn_writer(
    path: std::path::PathBuf,
) -> Result<(SyncSender<RequestLog>, JoinHandle<()>), AppError> {
    let conn = Connection::open(&path).map_err(|e| AppError::Database(e.to_string()))?;
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .map_err(|e| AppError::Database(e.to_string()))?;
    apply_file_connection_pragmas(&conn)?;
    // 与主连接一致：写库变更触发 WebDAV/S3 自动同步通知。
    conn.update_hook(Some(
        |action: rusqlite::hooks::Action, _database: &str, table: &str, _row_id: i64| match action {
            rusqlite::hooks::Action::SQLITE_INSERT
            | rusqlite::hooks::Action::SQLITE_UPDATE
            | rusqlite::hooks::Action::SQLITE_DELETE => {
                crate::services::webdav_auto_sync::notify_db_changed(table);
                crate::services::s3_auto_sync::notify_db_changed(table);
            }
            _ => {}
        },
    ));

    let (tx, rx) = mpsc::sync_channel::<RequestLog>(QUEUE_CAPACITY);
    let join = thread::Builder::new()
        .name("opensunstar-usage-writer".into())
        .spawn(move || writer_loop(conn, rx))
        .map_err(|e| AppError::Database(format!("启动用量写线程失败: {e}")))?;

    Ok((tx, join))
}

fn writer_loop(conn: Connection, rx: mpsc::Receiver<RequestLog>) {
    let mut batch: Vec<RequestLog> = Vec::with_capacity(BATCH_MAX);

    while let Ok(first) = rx.recv() {
        batch.push(first);
        let deadline = Instant::now() + FLUSH_INTERVAL;

        while batch.len() < BATCH_MAX {
            let wait = deadline.saturating_duration_since(Instant::now());
            if wait.is_zero() {
                break;
            }
            match rx.recv_timeout(wait) {
                Ok(log) => batch.push(log),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => {
                    while let Ok(log) = rx.try_recv() {
                        batch.push(log);
                    }
                    flush_batch(&conn, &batch);
                    return;
                }
            }
        }

        flush_batch(&conn, &batch);
        batch.clear();
    }

    while let Ok(log) = rx.try_recv() {
        batch.push(log);
    }
    if !batch.is_empty() {
        flush_batch(&conn, &batch);
    }
}

fn flush_batch(conn: &Connection, logs: &[RequestLog]) {
    if logs.is_empty() {
        return;
    }

    let result = (|| -> Result<(), AppError> {
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        for log in logs {
            insert_request_log(&tx, log)?;
        }
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            crate::usage_events::notify_log_recorded();
            crate::services::budget_alert::notify_after_log();
        }
        Err(e) => {
            log::error!("批量写入用量日志失败 ({} 条): {e}", logs.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::usage::parser::TokenUsage;
    use crate::services::usage_stats::LogFilters;
    use tempfile::tempdir;

    fn sample_log(id: &str) -> RequestLog {
        RequestLog {
            request_id: id.to_string(),
            provider_id: "p1".into(),
            app_type: "claude".into(),
            model: "m".into(),
            request_model: "m".into(),
            pricing_model: "m".into(),
            usage: TokenUsage::default(),
            cost: None,
            latency_ms: 10,
            first_token_ms: None,
            status_code: 200,
            error_message: None,
            session_id: None,
            provider_type: None,
            is_streaming: false,
            cost_multiplier: "1.0".into(),
        }
    }

    #[test]
    fn async_writer_flushes_to_file_db() -> Result<(), AppError> {
        let dir = tempdir().map_err(|e| AppError::Database(e.to_string()))?;
        let path = dir.path().join("OpenSunstar.db");
        let db = Arc::new(Database::open_file(&path)?);
        let writer = AsyncUsageWriter::new(db.clone());

        writer.enqueue(sample_log("req-1"));
        writer.enqueue(sample_log("req-2"));
        writer.flush_shutdown();

        let count: i64 = {
            let conn = crate::database::lock_conn!(db.usage_conn());
            conn.query_row("SELECT COUNT(*) FROM proxy_request_logs", [], |r| r.get(0))
                .map_err(|e| AppError::Database(e.to_string()))?
        };
        assert_eq!(count, 2);
        let core_count: i64 = {
            let conn = crate::database::lock_conn!(db.conn);
            conn.query_row("SELECT COUNT(*) FROM proxy_request_logs", [], |r| r.get(0))
                .map_err(|e| AppError::Database(e.to_string()))?
        };
        assert_eq!(
            core_count, 0,
            "proxy writes must not return to the core database"
        );
        let logs = db.get_request_logs(&LogFilters::default(), 0, 10)?;
        assert_eq!(logs.total, 2, "usage queries must read the sidecar");
        let full_export = db.export_sql_string()?;
        assert!(
            full_export.contains("req-1") && full_export.contains("req-2"),
            "full export must include sidecar telemetry"
        );
        Ok(())
    }

    #[test]
    fn memory_db_falls_back_to_sync_write() -> Result<(), AppError> {
        let db = Arc::new(Database::memory()?);
        let writer = AsyncUsageWriter::new(db.clone());
        writer.enqueue(sample_log("mem-1"));
        writer.flush_shutdown();

        let count: i64 = {
            let conn = crate::database::lock_conn!(db.conn);
            conn.query_row("SELECT COUNT(*) FROM proxy_request_logs", [], |r| r.get(0))
                .map_err(|e| AppError::Database(e.to_string()))?
        };
        assert_eq!(count, 1);
        Ok(())
    }
}
