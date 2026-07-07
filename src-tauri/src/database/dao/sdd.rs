//! DAO for SDD framework detection tables.

use rusqlite::params;

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::sdd::{SddDescriptorSummary, SddDetectionResult, SignalMatch};

impl Database {
    /// List all 7 framework descriptors from sdd_descriptors.
    pub fn sdd_list_descriptors(&self) -> Result<Vec<SddDescriptorSummary>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, name, version, phase_model, install_type,
                        description_zh, description_en, repo_url, star_count
                 FROM sdd_descriptors ORDER BY id",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(SddDescriptorSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    phase_model: row.get(3)?,
                    install_type: row.get(4)?,
                    description_zh: row.get(5)?,
                    description_en: row.get(6)?,
                    repo_url: row.get(7)?,
                    star_count: row.get(8)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(out)
    }

    /// Upsert detection results for a project.
    pub fn sdd_save_detection_results(
        &self,
        project_id: &str,
        results: &[SddDetectionResult],
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        for r in results {
            let id = format!("{}_{}", project_id, r.descriptor_id);
            let signals_json =
                serde_json::to_string(&r.signal_matches).unwrap_or_else(|_| "[]".into());
            conn.execute(
                "INSERT INTO project_sdd_detections
                    (id, project_id, descriptor_id, detected, confidence, signal_matches, detected_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
                 ON CONFLICT(project_id, descriptor_id) DO UPDATE SET
                    detected = excluded.detected,
                    confidence = excluded.confidence,
                    signal_matches = excluded.signal_matches,
                    detected_at = excluded.detected_at",
                params![
                    id,
                    project_id,
                    r.descriptor_id,
                    r.detected as i32,
                    r.confidence,
                    signals_json,
                ],
            )
            .map_err(|e| AppError::Database(format!("upsert sdd detection 失败: {e}")))?;
        }
        Ok(())
    }

    /// Get saved detection results for a project.
    pub fn sdd_get_detection_results(
        &self,
        project_id: &str,
    ) -> Result<Vec<SddDetectionResult>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT descriptor_id, detected, confidence, signal_matches
                 FROM project_sdd_detections
                 WHERE project_id = ?1 ORDER BY descriptor_id",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                let descriptor_id: String = row.get(0)?;
                let detected_int: i32 = row.get(1)?;
                let confidence: String = row.get(2)?;
                let signals_json: Option<String> = row.get(3)?;
                Ok((descriptor_id, detected_int, confidence, signals_json))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut out = Vec::new();
        for row in rows {
            let (descriptor_id, detected_int, confidence, signals_json) =
                row.map_err(|e| AppError::Database(e.to_string()))?;
            let signal_matches: Vec<SignalMatch> = signals_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            out.push(SddDetectionResult {
                descriptor_id,
                detected: detected_int != 0,
                confidence,
                signal_matches,
            });
        }
        Ok(out)
    }

    /// Get all project IDs and paths for batch detection.
    pub fn sdd_list_all_projects(&self) -> Result<Vec<(String, String)>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id, path FROM projects")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(out)
    }
}
