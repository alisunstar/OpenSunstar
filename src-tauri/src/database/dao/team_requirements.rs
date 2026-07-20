//! Persistence for team/project requirement inputs. Compiled expectations stay
//! in the existing asset-health tables; this DAO deliberately stores only the
//! independent sources used by the team configuration resolver.

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::team_config::{PolicyAction, RequirementKey, RequirementSource};

impl Database {
    pub fn upsert_project_asset_requirement_source(
        &self,
        source: &RequirementSource,
    ) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp();
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO project_asset_requirement_sources (
                source_id, project_id, asset_type, asset_id, target_app,
                scope_kind, scope_id, source_revision, policy_action,
                required_revision_id, constraint_json, priority_class, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
             ON CONFLICT(source_id) DO UPDATE SET
                project_id=excluded.project_id, asset_type=excluded.asset_type,
                asset_id=excluded.asset_id, target_app=excluded.target_app,
                scope_kind=excluded.scope_kind, scope_id=excluded.scope_id,
                source_revision=excluded.source_revision, policy_action=excluded.policy_action,
                required_revision_id=excluded.required_revision_id,
                constraint_json=excluded.constraint_json, priority_class=excluded.priority_class,
                updated_at=excluded.updated_at",
            rusqlite::params![
                source.source_id,
                source.key.project_id,
                source.key.asset_type,
                source.key.asset_id,
                source.key.target_app,
                source.scope_kind,
                source.scope_id,
                source.source_revision,
                source.policy_action.as_str(),
                source.required_revision_id,
                source.constraint_json,
                source.priority_class,
                now,
            ],
        )
        .map_err(|e| AppError::Database(format!("保存项目资产要求来源失败: {e}")))?;
        Ok(())
    }

    pub fn get_project_asset_requirement_sources(
        &self,
        project_id: &str,
    ) -> Result<Vec<RequirementSource>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT source_id, project_id, asset_type, asset_id, target_app,
                        scope_kind, scope_id, source_revision, policy_action,
                        required_revision_id, constraint_json, priority_class
                 FROM project_asset_requirement_sources
                 WHERE project_id = ?1
                 ORDER BY asset_type, asset_id, target_app, priority_class, source_id",
            )
            .map_err(|e| AppError::Database(format!("查询项目资产要求来源失败: {e}")))?;
        let rows = stmt
            .query_map([project_id], |row| {
                let policy_action = match row.get::<_, String>(8)?.as_str() {
                    "required" => PolicyAction::Required,
                    "recommended" => PolicyAction::Recommended,
                    "denied" => PolicyAction::Denied,
                    unknown => {
                        return Err(rusqlite::Error::FromSqlConversionFailure(
                            8,
                            rusqlite::types::Type::Text,
                            format!("未知团队策略动作: {unknown}").into(),
                        ))
                    }
                };
                Ok(RequirementSource {
                    source_id: row.get(0)?,
                    key: RequirementKey {
                        project_id: row.get(1)?,
                        asset_type: row.get(2)?,
                        asset_id: row.get(3)?,
                        target_app: row.get(4)?,
                    },
                    scope_kind: row.get(5)?,
                    scope_id: row.get(6)?,
                    source_revision: row.get(7)?,
                    policy_action,
                    required_revision_id: row.get(9)?,
                    constraint_json: row.get(10)?,
                    priority_class: row.get(11)?,
                })
            })
            .map_err(|e| AppError::Database(format!("读取项目资产要求来源失败: {e}")))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("解析项目资产要求来源失败: {e}")))
    }
}
