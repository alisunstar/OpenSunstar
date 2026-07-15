//! Persistent facts for project AI asset health.
//!
//! This DAO deliberately stores only identifiers, hashes and structured status.
//! It never stores API keys, prompt bodies or full configuration files.

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::database::{lock_conn, Database};
use crate::error::AppError;

const KNOWN_APPS: &[&str] = &[
    "claude",
    "claude-desktop",
    "codex",
    "gemini",
    "opencode",
    "openclaw",
    "hermes",
];

const KNOWN_ASSET_TYPES: &[&str] = &[
    "mcp",
    "skill",
    "prompt",
    "command",
    "hook",
    "ignore",
    "permission",
    "subagent",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRevision {
    pub revision_id: String,
    pub asset_type: String,
    pub asset_id: String,
    pub version_label: Option<String>,
    pub content_sha256: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub source_revision: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAssetExpectation {
    pub expectation_id: String,
    pub project_id: String,
    pub asset_type: String,
    pub asset_id: String,
    pub target_app: String,
    pub desired_state: String,
    pub required_revision_id: Option<String>,
    pub verification_policy: Option<String>,
    pub scope: String,
    pub source: String,
    pub owner_mode: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDeploymentReceipt {
    pub receipt_id: String,
    pub expectation_id: String,
    pub operation_id: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub plan_sha256: String,
    pub required_revision_id: Option<String>,
    pub dry_run: bool,
    pub outcome: String,
    pub target_path: Option<String>,
    pub before_sha256: Option<String>,
    pub after_sha256: Option<String>,
    pub snapshot_ref: Option<String>,
    pub reason_code: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetReceiptFile {
    pub file_id: String,
    pub receipt_id: String,
    pub relative_path: String,
    pub action: String,
    pub before_sha256: Option<String>,
    pub after_sha256: Option<String>,
    pub snapshot_ref: Option<String>,
    pub reason_code: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRuntimeEvidence {
    pub evidence_id: String,
    pub expectation_id: String,
    pub evidence_kind: String,
    pub status: String,
    pub observed_revision_sha256: Option<String>,
    pub confidence: String,
    pub collector: String,
    pub collector_version: String,
    pub observed_at: i64,
    pub expires_at: Option<i64>,
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn legacy_revision_digest(asset_type: &str, asset_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"legacy-link:");
    hasher.update(asset_type.as_bytes());
    hasher.update(b":");
    hasher.update(asset_id.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn normalized_content_digest(content: &[u8]) -> String {
    let normalized = match std::str::from_utf8(content) {
        Ok(text) => text
            .strip_prefix('\u{feff}')
            .unwrap_or(text)
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .into_bytes(),
        Err(_) => content.to_vec(),
    };
    format!("{:x}", Sha256::digest(normalized))
}

fn metadata_looks_sensitive(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "authorization",
        "api_key",
        "apikey",
        "access_token",
        "bearer ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
        || normalized
            .split_once("://")
            .and_then(|(_, rest)| rest.split('/').next())
            .is_some_and(|authority| authority.contains('@'))
}

impl Database {
    fn capture_current_asset_revision(
        &self,
        asset_type: &str,
        asset_id: &str,
    ) -> Result<Option<AssetRevision>, AppError> {
        let sql = match asset_type {
            "mcp" => "SELECT server_config FROM mcp_servers WHERE id = ?1",
            "skill" => "SELECT COALESCE(content_hash, '') || char(0) || directory FROM skills WHERE id = ?1",
            "prompt" => "SELECT GROUP_CONCAT(app_type || char(0) || content, char(1)) FROM (SELECT app_type, content FROM prompts WHERE id = ?1 ORDER BY app_type)",
            "command" => "SELECT content || char(0) || arguments FROM commands WHERE id = ?1",
            "hook" => "SELECT event_type || char(0) || tool_pattern || char(0) || hook_command || char(0) || CAST(timeout_seconds AS TEXT) FROM hooks WHERE id = ?1",
            "ignore" => "SELECT pattern FROM ignore_rules WHERE id = ?1",
            "permission" => "SELECT permission_type || char(0) || tool_pattern FROM tool_permissions WHERE id = ?1",
            "subagent" => "SELECT content FROM agents WHERE id = ?1",
            _ => return Ok(None),
        };
        let content = {
            let conn = lock_conn!(self.conn);
            conn.query_row(sql, [asset_id], |row| row.get::<_, Option<String>>(0))
                .optional()
                .map_err(|error| AppError::Database(format!("捕获资产当前内容失败: {error}")))?
                .flatten()
        };
        content
            .map(|content| {
                self.register_asset_revision(
                    asset_type,
                    asset_id,
                    content.as_bytes(),
                    "local-db",
                    None,
                    None,
                    Some("captured"),
                )
            })
            .transpose()
    }

    /// Registers an immutable content revision without persisting the asset body.
    /// UTF-8 BOM and line-ending differences are normalized so the same logical
    /// content produces the same digest on Windows, macOS and Linux.
    pub fn register_asset_revision(
        &self,
        asset_type: &str,
        asset_id: &str,
        content: &[u8],
        source_kind: &str,
        source_ref: Option<&str>,
        source_revision: Option<&str>,
        version_label: Option<&str>,
    ) -> Result<AssetRevision, AppError> {
        if !KNOWN_ASSET_TYPES.contains(&asset_type) {
            return Err(AppError::InvalidInput(format!(
                "未知资产类型: {asset_type}"
            )));
        }
        if asset_id.trim().is_empty() || source_kind.trim().is_empty() {
            return Err(AppError::InvalidInput("资产 ID 和来源类型不能为空".into()));
        }
        if source_ref.is_some_and(metadata_looks_sensitive)
            || source_revision.is_some_and(metadata_looks_sensitive)
        {
            return Err(AppError::InvalidInput(
                "资产修订来源元数据疑似包含凭据，已拒绝持久化".into(),
            ));
        }
        let digest = normalized_content_digest(content);
        let revision_id = Uuid::new_v4().to_string();
        let created_at = now_ts();
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR IGNORE INTO asset_revisions
             (revision_id, asset_type, asset_id, version_label, content_sha256,
              source_kind, source_ref, source_revision, metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '{\"contentCaptured\":true}', ?9)",
            rusqlite::params![
                revision_id,
                asset_type,
                asset_id,
                version_label,
                digest,
                source_kind,
                source_ref,
                source_revision,
                created_at,
            ],
        )
        .map_err(|error| AppError::Database(format!("登记资产修订失败: {error}")))?;

        conn.query_row(
            "SELECT revision_id, asset_type, asset_id, version_label, content_sha256,
                    source_kind, source_ref, source_revision, created_at
             FROM asset_revisions
             WHERE asset_type = ?1 AND asset_id = ?2 AND content_sha256 = ?3",
            rusqlite::params![asset_type, asset_id, digest],
            |row| {
                Ok(AssetRevision {
                    revision_id: row.get(0)?,
                    asset_type: row.get(1)?,
                    asset_id: row.get(2)?,
                    version_label: row.get(3)?,
                    content_sha256: row.get(4)?,
                    source_kind: row.get(5)?,
                    source_ref: row.get(6)?,
                    source_revision: row.get(7)?,
                    created_at: row.get(8)?,
                })
            },
        )
        .map_err(|error| AppError::Database(format!("读取资产修订失败: {error}")))
    }

    /// Creates a non-authoritative revision for legacy links. The label makes it
    /// impossible for callers to mistake an ID hash for a captured asset body.
    pub fn ensure_legacy_asset_revision(
        &self,
        asset_type: &str,
        asset_id: &str,
    ) -> Result<AssetRevision, AppError> {
        let digest = legacy_revision_digest(asset_type, asset_id);
        let now = now_ts();
        let revision_id = format!("legacy-{}", &digest[..24]);
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR IGNORE INTO asset_revisions
             (revision_id, asset_type, asset_id, version_label, content_sha256, source_kind, metadata_json, created_at)
             VALUES (?1, ?2, ?3, 'legacy-unverified', ?4, 'legacy-import', '{\"contentCaptured\":false}', ?5)",
            rusqlite::params![revision_id, asset_type, asset_id, digest, now],
        )
        .map_err(|e| AppError::Database(format!("登记资产 legacy 修订失败: {e}")))?;

        Ok(AssetRevision {
            revision_id,
            asset_type: asset_type.to_string(),
            asset_id: asset_id.to_string(),
            version_label: Some("legacy-unverified".to_string()),
            content_sha256: digest,
            source_kind: "legacy-import".to_string(),
            source_ref: None,
            source_revision: None,
            created_at: now,
        })
    }

    pub fn upsert_project_asset_expectation(
        &self,
        project_id: &str,
        asset_type: &str,
        asset_id: &str,
        target_app: &str,
        required_revision_id: Option<&str>,
        scope: &str,
        source: &str,
        owner_mode: &str,
    ) -> Result<ProjectAssetExpectation, AppError> {
        if !KNOWN_APPS.contains(&target_app) {
            return Err(AppError::InvalidInput(format!(
                "未知目标应用: {target_app}"
            )));
        }
        let now = now_ts();
        let expectation_id = Uuid::new_v4().to_string();
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO project_asset_expectations
             (expectation_id, project_id, asset_type, asset_id, target_app, required_revision_id, scope, source, owner_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
             ON CONFLICT(project_id, asset_type, asset_id, target_app) DO UPDATE SET
                required_revision_id = excluded.required_revision_id,
                scope = excluded.scope,
                source = excluded.source,
                owner_mode = excluded.owner_mode,
                updated_at = excluded.updated_at",
            rusqlite::params![
                expectation_id,
                project_id,
                asset_type,
                asset_id,
                target_app,
                required_revision_id,
                scope,
                source,
                owner_mode,
                now,
            ],
        )
        .map_err(|e| AppError::Database(format!("保存项目资产期望失败: {e}")))?;

        conn.query_row(
            "SELECT expectation_id, project_id, asset_type, asset_id, target_app, desired_state,
                    required_revision_id, verification_policy, scope, source, owner_mode, created_at, updated_at
             FROM project_asset_expectations
             WHERE project_id = ?1 AND asset_type = ?2 AND asset_id = ?3 AND target_app = ?4",
            rusqlite::params![project_id, asset_type, asset_id, target_app],
            |row| Ok(ProjectAssetExpectation {
                expectation_id: row.get(0)?, project_id: row.get(1)?, asset_type: row.get(2)?,
                asset_id: row.get(3)?, target_app: row.get(4)?, desired_state: row.get(5)?,
                required_revision_id: row.get(6)?, verification_policy: row.get(7)?,
                scope: row.get(8)?, source: row.get(9)?, owner_mode: row.get(10)?,
                created_at: row.get(11)?, updated_at: row.get(12)?,
            }),
        )
        .map_err(|e| AppError::Database(format!("读取项目资产期望失败: {e}")))
    }

    /// Bridges old project links into observed expectations without changing the
    /// original link or claiming that its body was version-captured.
    pub fn ensure_project_asset_health_inventory(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAssetExpectation>, AppError> {
        let project = self
            .get_project(project_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("项目不存在: {project_id}")))?;
        let default_app = project
            .target_app
            .as_deref()
            .filter(|app| KNOWN_APPS.contains(app))
            .unwrap_or("claude");
        let links = self.get_project_asset_links(project_id, None)?;
        let mut expectations = Vec::new();
        for link in links.into_iter().filter(|link| link.enabled) {
            let target_app = if KNOWN_APPS.contains(&link.asset_app_type.as_str()) {
                link.asset_app_type.as_str()
            } else {
                default_app
            };
            let revision =
                match self.capture_current_asset_revision(&link.asset_type, &link.asset_id)? {
                    Some(revision) => revision,
                    None => self.ensure_legacy_asset_revision(&link.asset_type, &link.asset_id)?,
                };
            expectations.push(self.upsert_project_asset_expectation(
                project_id,
                &link.asset_type,
                &link.asset_id,
                target_app,
                Some(&revision.revision_id),
                &link.scope,
                &link.source,
                "observed",
            )?);
        }
        Ok(expectations)
    }

    pub fn get_project_asset_expectations(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAssetExpectation>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT expectation_id, project_id, asset_type, asset_id, target_app, desired_state,
                        required_revision_id, verification_policy, scope, source, owner_mode, created_at, updated_at
                 FROM project_asset_expectations WHERE project_id = ?1
                 ORDER BY asset_type, asset_id, target_app",
            )
            .map_err(|e| AppError::Database(format!("查询项目资产期望失败: {e}")))?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(ProjectAssetExpectation {
                    expectation_id: row.get(0)?,
                    project_id: row.get(1)?,
                    asset_type: row.get(2)?,
                    asset_id: row.get(3)?,
                    target_app: row.get(4)?,
                    desired_state: row.get(5)?,
                    required_revision_id: row.get(6)?,
                    verification_policy: row.get(7)?,
                    scope: row.get(8)?,
                    source: row.get(9)?,
                    owner_mode: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|e| AppError::Database(format!("读取项目资产期望失败: {e}")))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("解析项目资产期望失败: {e}")))
    }

    pub fn get_project_asset_expectation(
        &self,
        expectation_id: &str,
    ) -> Result<Option<ProjectAssetExpectation>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT expectation_id, project_id, asset_type, asset_id, target_app, desired_state,
                    required_revision_id, verification_policy, scope, source, owner_mode, created_at, updated_at
             FROM project_asset_expectations WHERE expectation_id = ?1",
            [expectation_id],
            |row| {
                Ok(ProjectAssetExpectation {
                    expectation_id: row.get(0)?,
                    project_id: row.get(1)?,
                    asset_type: row.get(2)?,
                    asset_id: row.get(3)?,
                    target_app: row.get(4)?,
                    desired_state: row.get(5)?,
                    required_revision_id: row.get(6)?,
                    verification_policy: row.get(7)?,
                    scope: row.get(8)?,
                    source: row.get(9)?,
                    owner_mode: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(AppError::Database(format!("读取项目资产期望失败: {error}"))),
        }
    }

    pub fn latest_asset_deployment_receipt(
        &self,
        expectation_id: &str,
    ) -> Result<Option<AssetDeploymentReceipt>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT receipt_id, expectation_id, operation_id, adapter_id, adapter_version, plan_sha256,
                    required_revision_id, dry_run, outcome, target_path, before_sha256, after_sha256, snapshot_ref, reason_code, created_at
             FROM asset_deployment_receipts WHERE expectation_id = ?1 ORDER BY created_at DESC, rowid DESC LIMIT 1",
            [expectation_id],
            |row| Ok(AssetDeploymentReceipt {
                receipt_id: row.get(0)?, expectation_id: row.get(1)?, operation_id: row.get(2)?,
                adapter_id: row.get(3)?, adapter_version: row.get(4)?, plan_sha256: row.get(5)?,
                required_revision_id: row.get(6)?, dry_run: row.get::<_, i64>(7)? != 0,
                outcome: row.get(8)?, target_path: row.get(9)?, before_sha256: row.get(10)?,
                after_sha256: row.get(11)?, snapshot_ref: row.get(12)?, reason_code: row.get(13)?,
                created_at: row.get(14)?,
            }),
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(format!("读取最近资产部署回执失败: {e}"))),
        }
    }

    pub fn get_asset_deployment_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<AssetDeploymentReceipt>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT receipt_id, expectation_id, operation_id, adapter_id, adapter_version, plan_sha256,
                    required_revision_id, dry_run, outcome, target_path, before_sha256, after_sha256, snapshot_ref, reason_code, created_at
             FROM asset_deployment_receipts WHERE receipt_id = ?1",
            [receipt_id],
            |row| Ok(AssetDeploymentReceipt {
                receipt_id: row.get(0)?, expectation_id: row.get(1)?, operation_id: row.get(2)?,
                adapter_id: row.get(3)?, adapter_version: row.get(4)?, plan_sha256: row.get(5)?,
                required_revision_id: row.get(6)?, dry_run: row.get::<_, i64>(7)? != 0,
                outcome: row.get(8)?, target_path: row.get(9)?, before_sha256: row.get(10)?,
                after_sha256: row.get(11)?, snapshot_ref: row.get(12)?, reason_code: row.get(13)?,
                created_at: row.get(14)?,
            }),
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(format!("读取资产部署回执失败: {e}"))),
        }
    }

    pub fn latest_asset_runtime_evidence(
        &self,
        expectation_id: &str,
    ) -> Result<Option<AssetRuntimeEvidence>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT evidence_id, expectation_id, evidence_kind, status, observed_revision_sha256,
                    confidence, collector, collector_version, observed_at, expires_at
             FROM asset_runtime_evidence WHERE expectation_id = ?1 ORDER BY observed_at DESC, rowid DESC LIMIT 1",
            [expectation_id],
            |row| Ok(AssetRuntimeEvidence {
                evidence_id: row.get(0)?, expectation_id: row.get(1)?, evidence_kind: row.get(2)?,
                status: row.get(3)?, observed_revision_sha256: row.get(4)?, confidence: row.get(5)?,
                collector: row.get(6)?, collector_version: row.get(7)?, observed_at: row.get(8)?, expires_at: row.get(9)?,
            }),
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(format!("读取最近资产验证证据失败: {e}"))),
        }
    }

    pub fn record_asset_deployment_receipt(
        &self,
        receipt: &AssetDeploymentReceipt,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO asset_deployment_receipts
             (receipt_id, expectation_id, operation_id, adapter_id, adapter_version, plan_sha256, required_revision_id, dry_run, outcome, target_path, before_sha256, after_sha256, snapshot_ref, reason_code, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![receipt.receipt_id, receipt.expectation_id, receipt.operation_id, receipt.adapter_id, receipt.adapter_version, receipt.plan_sha256, receipt.required_revision_id, receipt.dry_run as i64, receipt.outcome, receipt.target_path, receipt.before_sha256, receipt.after_sha256, receipt.snapshot_ref, receipt.reason_code, receipt.created_at],
        ).map_err(|e| AppError::Database(format!("保存资产部署回执失败: {e}")))?;
        Ok(())
    }

    pub fn update_asset_deployment_receipt(
        &self,
        receipt: &AssetDeploymentReceipt,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let changed = conn
            .execute(
                "UPDATE asset_deployment_receipts SET
                    adapter_id = ?2, adapter_version = ?3, plan_sha256 = ?4,
                    required_revision_id = ?5, dry_run = ?6, outcome = ?7,
                    target_path = ?8, before_sha256 = ?9, after_sha256 = ?10,
                    snapshot_ref = ?11, reason_code = ?12, created_at = ?13
                 WHERE receipt_id = ?1",
                rusqlite::params![
                    receipt.receipt_id,
                    receipt.adapter_id,
                    receipt.adapter_version,
                    receipt.plan_sha256,
                    receipt.required_revision_id,
                    receipt.dry_run as i64,
                    receipt.outcome,
                    receipt.target_path,
                    receipt.before_sha256,
                    receipt.after_sha256,
                    receipt.snapshot_ref,
                    receipt.reason_code,
                    receipt.created_at,
                ],
            )
            .map_err(|error| AppError::Database(format!("更新资产部署回执失败: {error}")))?;
        if changed != 1 {
            return Err(AppError::Database(format!(
                "资产部署回执不存在: {}",
                receipt.receipt_id
            )));
        }
        Ok(())
    }

    pub fn record_asset_receipt_file(&self, file: &AssetReceiptFile) -> Result<(), AppError> {
        if std::path::Path::new(&file.relative_path).is_absolute()
            || file
                .relative_path
                .split(['/', '\\'])
                .any(|part| part == "..")
        {
            return Err(AppError::InvalidInput(
                "逐文件回执只允许项目相对路径".into(),
            ));
        }
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO asset_receipt_files
             (file_id, receipt_id, relative_path, action, before_sha256, after_sha256, snapshot_ref, reason_code, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![file.file_id, file.receipt_id, file.relative_path, file.action, file.before_sha256, file.after_sha256, file.snapshot_ref, file.reason_code, file.created_at],
        )
        .map_err(|error| AppError::Database(format!("保存资产逐文件回执失败: {error}")))?;
        Ok(())
    }

    pub fn get_asset_receipt_files(
        &self,
        receipt_id: &str,
    ) -> Result<Vec<AssetReceiptFile>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut statement = conn
            .prepare(
                "SELECT file_id, receipt_id, relative_path, action, before_sha256, after_sha256,
                        snapshot_ref, reason_code, created_at
                 FROM asset_receipt_files WHERE receipt_id = ?1 ORDER BY created_at, relative_path",
            )
            .map_err(|error| AppError::Database(format!("查询资产逐文件回执失败: {error}")))?;
        let rows = statement
            .query_map([receipt_id], |row| {
                Ok(AssetReceiptFile {
                    file_id: row.get(0)?,
                    receipt_id: row.get(1)?,
                    relative_path: row.get(2)?,
                    action: row.get(3)?,
                    before_sha256: row.get(4)?,
                    after_sha256: row.get(5)?,
                    snapshot_ref: row.get(6)?,
                    reason_code: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|error| AppError::Database(format!("读取资产逐文件回执失败: {error}")))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::Database(format!("解析资产逐文件回执失败: {error}")))
    }

    pub fn record_asset_runtime_evidence(
        &self,
        evidence: &AssetRuntimeEvidence,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO asset_runtime_evidence
             (evidence_id, expectation_id, evidence_kind, status, observed_revision_sha256, confidence, collector, collector_version, observed_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![evidence.evidence_id, evidence.expectation_id, evidence.evidence_kind, evidence.status, evidence.observed_revision_sha256, evidence.confidence, evidence.collector, evidence.collector_version, evidence.observed_at, evidence.expires_at],
        ).map_err(|e| AppError::Database(format!("保存资产验证证据失败: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_eight_asset_types_get_stable_content_revisions() {
        let db = Database::memory().expect("create in-memory database");
        for asset_type in KNOWN_ASSET_TYPES {
            let first = db
                .register_asset_revision(
                    asset_type,
                    "asset-1",
                    b"\xef\xbb\xbfline one\r\nline two\r\n",
                    "local",
                    None,
                    None,
                    Some("v1"),
                )
                .expect("register first revision");
            let same = db
                .register_asset_revision(
                    asset_type,
                    "asset-1",
                    b"line one\nline two\n",
                    "local",
                    None,
                    None,
                    Some("same-content"),
                )
                .expect("deduplicate normalized revision");
            let edited = db
                .register_asset_revision(
                    asset_type,
                    "asset-1",
                    b"line one\nline two edited\n",
                    "local",
                    None,
                    None,
                    Some("v2"),
                )
                .expect("register edited revision");
            assert_eq!(first.revision_id, same.revision_id, "{asset_type}");
            assert_eq!(first.content_sha256, same.content_sha256, "{asset_type}");
            assert_ne!(first.revision_id, edited.revision_id, "{asset_type}");
            assert_ne!(first.content_sha256, edited.content_sha256, "{asset_type}");
        }
    }

    #[test]
    fn legacy_revision_is_explicitly_non_authoritative() {
        let db = Database::memory().expect("create in-memory database");
        let revision = db
            .ensure_legacy_asset_revision("mcp", "legacy-id")
            .expect("register legacy revision");
        assert_eq!(revision.source_kind, "legacy-import");
        assert_eq!(revision.version_label.as_deref(), Some("legacy-unverified"));
    }

    #[test]
    fn current_database_asset_edit_creates_a_new_revision() {
        let db = Database::memory().expect("create in-memory database");
        {
            let conn = db.conn.lock().expect("lock database");
            conn.execute(
                "INSERT INTO mcp_servers (id, name, server_config) VALUES ('mcp-1', 'MCP', '{\"command\":\"one\"}')",
                [],
            )
            .unwrap();
        }
        let first = db
            .capture_current_asset_revision("mcp", "mcp-1")
            .unwrap()
            .expect("capture first revision");
        {
            let conn = db.conn.lock().expect("lock database");
            conn.execute(
                "UPDATE mcp_servers SET server_config = '{\"command\":\"two\"}' WHERE id = 'mcp-1'",
                [],
            )
            .unwrap();
        }
        let edited = db
            .capture_current_asset_revision("mcp", "mcp-1")
            .unwrap()
            .expect("capture edited revision");
        assert_ne!(first.revision_id, edited.revision_id);
        assert_ne!(first.content_sha256, edited.content_sha256);
    }

    #[test]
    fn revision_metadata_rejects_embedded_credentials() {
        let db = Database::memory().expect("create in-memory database");
        let error = db
            .register_asset_revision(
                "mcp",
                "mcp-1",
                b"secret body is hashed, not stored",
                "git",
                Some("https://token@example.com/repository.git"),
                None,
                None,
            )
            .expect_err("credential-bearing source URL must be rejected");
        assert!(error.to_string().contains("疑似包含凭据"));
    }
}
