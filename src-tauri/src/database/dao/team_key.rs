//! D16 Team Key Local — data access for team key references
//!
//! Red line: this table ONLY stores keychain://ref/ references and metadata.
//! Key plaintext NEVER touches SQLite.

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TeamKeyLocal {
    pub slot_slug: String,
    pub org_id: String,
    pub provider_kind: String,
    pub endpoint_url: Option<String>,
    pub keychain_ref: String,
    pub version_seq: i64,
    pub value_sha256: String,
    pub grant_id: String,
    pub grant_expires: i64,
    pub last_renewed: i64,
    pub status: String,
}

impl Database {
    pub fn upsert_team_key(&self, key: &TeamKeyLocal) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO team_key_local (
                slot_slug, org_id, provider_kind, endpoint_url,
                keychain_ref, version_seq, value_sha256,
                grant_id, grant_expires, last_renewed, status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                key.slot_slug,
                key.org_id,
                key.provider_kind,
                key.endpoint_url,
                key.keychain_ref,
                key.version_seq,
                key.value_sha256,
                key.grant_id,
                key.grant_expires,
                key.last_renewed,
                key.status,
            ],
        )
        .map_err(|e| AppError::Database(format!("upsert team_key_local 失败: {e}")))?;
        Ok(())
    }

    pub fn get_team_key(&self, slot_slug: &str) -> Result<Option<TeamKeyLocal>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT slot_slug, org_id, provider_kind, endpoint_url,
                        keychain_ref, version_seq, value_sha256,
                        grant_id, grant_expires, last_renewed, status
                 FROM team_key_local WHERE slot_slug = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let result = stmt
            .query_row(params![slot_slug], |row| {
                Ok(TeamKeyLocal {
                    slot_slug: row.get(0)?,
                    org_id: row.get(1)?,
                    provider_kind: row.get(2)?,
                    endpoint_url: row.get(3)?,
                    keychain_ref: row.get(4)?,
                    version_seq: row.get(5)?,
                    value_sha256: row.get(6)?,
                    grant_id: row.get(7)?,
                    grant_expires: row.get(8)?,
                    last_renewed: row.get(9)?,
                    status: row.get(10)?,
                })
            })
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(result)
    }

    pub fn list_team_keys(&self, org_id: &str) -> Result<Vec<TeamKeyLocal>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT slot_slug, org_id, provider_kind, endpoint_url,
                        keychain_ref, version_seq, value_sha256,
                        grant_id, grant_expires, last_renewed, status
                 FROM team_key_local WHERE org_id = ?1
                 ORDER BY slot_slug ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![org_id], |row| {
                Ok(TeamKeyLocal {
                    slot_slug: row.get(0)?,
                    org_id: row.get(1)?,
                    provider_kind: row.get(2)?,
                    endpoint_url: row.get(3)?,
                    keychain_ref: row.get(4)?,
                    version_seq: row.get(5)?,
                    value_sha256: row.get(6)?,
                    grant_id: row.get(7)?,
                    grant_expires: row.get(8)?,
                    last_renewed: row.get(9)?,
                    status: row.get(10)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn list_all_team_keys(&self) -> Result<Vec<TeamKeyLocal>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT slot_slug, org_id, provider_kind, endpoint_url,
                        keychain_ref, version_seq, value_sha256,
                        grant_id, grant_expires, last_renewed, status
                 FROM team_key_local ORDER BY org_id, slot_slug ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(TeamKeyLocal {
                    slot_slug: row.get(0)?,
                    org_id: row.get(1)?,
                    provider_kind: row.get(2)?,
                    endpoint_url: row.get(3)?,
                    keychain_ref: row.get(4)?,
                    version_seq: row.get(5)?,
                    value_sha256: row.get(6)?,
                    grant_id: row.get(7)?,
                    grant_expires: row.get(8)?,
                    last_renewed: row.get(9)?,
                    status: row.get(10)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn update_team_key_status(&self, slot_slug: &str, status: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE team_key_local SET status = ?2 WHERE slot_slug = ?1",
            params![slot_slug, status],
        )
        .map_err(|e| AppError::Database(format!("更新 team_key_local 状态失败: {e}")))?;
        Ok(())
    }

    pub fn delete_team_key(&self, slot_slug: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM team_key_local WHERE slot_slug = ?1",
            params![slot_slug],
        )
        .map_err(|e| AppError::Database(format!("删除 team_key_local 失败: {e}")))?;
        Ok(())
    }

    pub fn delete_team_keys_for_org(&self, org_id: &str) -> Result<u64, AppError> {
        let conn = lock_conn!(self.conn);
        let count = conn
            .execute(
                "DELETE FROM team_key_local WHERE org_id = ?1",
                params![org_id],
            )
            .map_err(|e| AppError::Database(format!("删除 org team_key_local 失败: {e}")))?;
        Ok(count as u64)
    }
}

use rusqlite::OptionalExtension;
