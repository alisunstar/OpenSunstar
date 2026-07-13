//! AI Insights 缓存与成本日志 DAO

use crate::ai::types::CostByTypeEntry;
use crate::ai::types::{
    AIRiskResult, AIRoiReport, AIRoiTotals, CostByTypeDetail, ProjectRoiEntry, RoiTrendBucket,
};
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use serde::{Deserialize, Serialize};

/// AI 洞察缓存行（对应 ai_insights 表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIInsightRow {
    pub id: i64,
    pub project_id: String,
    pub insight_type: String,
    pub content: String,
    pub model_used: Option<String>,
    pub tokens_used: i64,
    pub cost_estimate: f64,
    pub created_at: i64,
    pub expires_at: i64,
    pub input_hash: String,
}

/// AI 成本日志行（对应 ai_cost_log 表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AICostLogRow {
    pub insight_type: String,
    pub project_id: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost: f64,
    pub created_at: i64,
}

/// NL 问答日志行（对应 ai_query_log 表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIQueryLogRow {
    pub query_text: String,
    pub answer_preview: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost: f64,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub created_at: i64,
}

/// AI 成本汇总结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AICostSummaryData {
    pub total_cost: f64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub insight_count: i64,
}

impl Database {
    /// 获取指定项目的指定类型 AI 洞察缓存
    pub fn get_ai_insight(
        &self,
        project_id: &str,
        insight_type: &str,
    ) -> Result<Option<AIInsightRow>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, insight_type, content, model_used,
                        tokens_used, cost_estimate, created_at, expires_at, input_hash
                 FROM ai_insights
                 WHERE project_id = ?1 AND insight_type = ?2",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let row = stmt
            .query_row(rusqlite::params![project_id, insight_type], |row| {
                Ok(AIInsightRow {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    insight_type: row.get(2)?,
                    content: row.get(3)?,
                    model_used: row.get(4)?,
                    tokens_used: row.get(5)?,
                    cost_estimate: row.get(6)?,
                    created_at: row.get(7)?,
                    expires_at: row.get(8)?,
                    input_hash: row.get(9)?,
                })
            })
            .ok();

        Ok(row)
    }

    /// 插入或更新 AI 洞察缓存（基于 project_id + insight_type UNIQUE 约束）
    pub fn upsert_ai_insight(&self, row: &AIInsightRow) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO ai_insights
             (project_id, insight_type, content, model_used, tokens_used,
              cost_estimate, created_at, expires_at, input_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                row.project_id,
                row.insight_type,
                row.content,
                row.model_used,
                row.tokens_used,
                row.cost_estimate,
                row.created_at,
                row.expires_at,
                row.input_hash,
            ],
        )
        .map_err(|e| AppError::Database(format!("upsert ai_insight 失败: {e}")))?;
        Ok(())
    }

    /// 删除已过期的 AI 洞察缓存，返回删除行数
    pub fn delete_expired_insights(&self) -> Result<u64, AppError> {
        let now = chrono::Utc::now().timestamp();
        let conn = lock_conn!(self.conn);
        let deleted = conn
            .execute("DELETE FROM ai_insights WHERE expires_at < ?1", [now])
            .map_err(|e| AppError::Database(e.to_string()))?;
        if deleted > 0 {
            log::info!("清理了 {deleted} 条过期 AI 洞察缓存");
        }
        Ok(deleted as u64)
    }

    /// 插入一条 AI 成本日志
    pub fn insert_ai_cost_log(&self, log: &AICostLogRow) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO ai_cost_log
             (insight_type, project_id, model, provider, prompt_tokens,
              completion_tokens, cost, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                log.insight_type,
                log.project_id,
                log.model,
                log.provider,
                log.prompt_tokens,
                log.completion_tokens,
                log.cost,
                log.created_at,
            ],
        )
        .map_err(|e| AppError::Database(format!("插入 ai_cost_log 失败: {e}")))?;
        Ok(())
    }

    /// 获取指定时间范围内的 AI 成本汇总
    pub fn get_ai_cost_summary(&self, since_timestamp: i64) -> Result<AICostSummaryData, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT COALESCE(SUM(cost), 0.0),
                        COALESCE(SUM(prompt_tokens), 0),
                        COALESCE(SUM(completion_tokens), 0),
                        COUNT(*)
                 FROM ai_cost_log
                 WHERE created_at >= ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let summary = stmt
            .query_row([since_timestamp], |row| {
                Ok(AICostSummaryData {
                    total_cost: row.get(0)?,
                    total_prompt_tokens: row.get(1)?,
                    total_completion_tokens: row.get(2)?,
                    insight_count: row.get(3)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(summary)
    }

    /// 按 insight_type 分组统计 AI 成本（Phase 3）
    pub fn get_ai_cost_by_type(
        &self,
        since_timestamp: i64,
    ) -> Result<Vec<CostByTypeEntry>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT insight_type, COUNT(*), COALESCE(SUM(cost), 0.0),
                        COALESCE(SUM(prompt_tokens + completion_tokens), 0)
                 FROM ai_cost_log
                 WHERE created_at >= ?1
                 GROUP BY insight_type
                 ORDER BY COUNT(*) DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([since_timestamp], |row| {
                Ok(CostByTypeEntry {
                    insight_type: row.get(0)?,
                    count: row.get(1)?,
                    total_cost: row.get(2)?,
                    total_tokens: row.get(3)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(entries)
    }

    /// 统计 NL 问答次数
    pub fn count_nl_queries(&self, since_timestamp: i64) -> Result<u32, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_query_log WHERE created_at >= ?1",
                [since_timestamp],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as u32)
    }

    /// 插入 NL 问答日志，返回新行 id
    pub fn insert_ai_query_log(&self, log: &AIQueryLogRow) -> Result<i64, AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO ai_query_log
             (query_text, answer_preview, prompt_tokens, completion_tokens,
              cost, model, provider, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                log.query_text,
                log.answer_preview,
                log.prompt_tokens,
                log.completion_tokens,
                log.cost,
                log.model,
                log.provider,
                log.created_at,
            ],
        )
        .map_err(|e| AppError::Database(format!("插入 ai_query_log 失败: {e}")))?;
        Ok(conn.last_insert_rowid())
    }

    /// 更新 NL 问答的用户反馈
    pub fn update_query_feedback(&self, query_id: i64, feedback: &str) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "UPDATE ai_query_log SET user_feedback = ?2 WHERE id = ?1",
                rusqlite::params![query_id, feedback],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }

    /// 统计指定反馈类型数量（洞察 + NL 问答）
    pub fn count_feedback_since(
        &self,
        since_timestamp: i64,
        feedback: &str,
    ) -> Result<u32, AppError> {
        let conn = lock_conn!(self.conn);
        let insight_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_insights WHERE user_feedback = ?1 AND created_at >= ?2",
                rusqlite::params![feedback, since_timestamp],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let query_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_query_log WHERE user_feedback = ?1 AND created_at >= ?2",
                rusqlite::params![feedback, since_timestamp],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok((insight_count + query_count) as u32)
    }

    /// 统计周期内 ai_insights 写入数（按 created_at）
    pub fn count_insights_since(&self, since_timestamp: i64) -> Result<u32, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_insights WHERE created_at >= ?1",
                [since_timestamp],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as u32)
    }

    /// 统计周期内风险项总数（解析 risk_analysis JSON）
    pub fn count_risks_since(&self, since_timestamp: i64) -> Result<u32, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT content FROM ai_insights
                 WHERE insight_type = 'risk_analysis' AND created_at >= ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([since_timestamp], |row| row.get::<_, String>(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut total = 0u32;
        for row in rows {
            if let Ok(content) = row {
                total += parse_risk_count(&content);
            }
        }
        Ok(total)
    }

    /// 按项目聚合成本
    pub fn get_cost_by_project(
        &self,
        since_timestamp: i64,
    ) -> Result<Vec<(String, String, f64, u64, u32)>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT c.project_id,
                        COALESCE(p.name, c.project_id),
                        COALESCE(SUM(c.cost), 0.0),
                        COALESCE(SUM(c.prompt_tokens + c.completion_tokens), 0),
                        COUNT(*)
                 FROM ai_cost_log c
                 LEFT JOIN projects p ON p.id = c.project_id
                 WHERE c.created_at >= ?1
                   AND c.project_id IS NOT NULL
                   AND c.project_id != '__portfolio__'
                 GROUP BY c.project_id
                 ORDER BY SUM(c.cost) DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([since_timestamp], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, i64>(3)? as u64,
                    row.get::<_, i64>(4)? as u32,
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(out)
    }

    /// 项目级 useful 反馈数
    pub fn count_useful_by_project(
        &self,
        project_id: &str,
        since_timestamp: i64,
    ) -> Result<u32, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_insights
                 WHERE project_id = ?1 AND user_feedback = 'useful' AND created_at >= ?2",
                rusqlite::params![project_id, since_timestamp],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as u32)
    }

    /// 项目级风险数与摘要（取最新 risk_analysis）
    pub fn project_risk_summary(
        &self,
        project_id: &str,
        since_timestamp: i64,
    ) -> Result<(u32, Vec<String>), AppError> {
        let conn = lock_conn!(self.conn);
        let content: Option<String> = conn
            .query_row(
                "SELECT content FROM ai_insights
                 WHERE project_id = ?1 AND insight_type = 'risk_analysis' AND created_at >= ?2
                 ORDER BY created_at DESC LIMIT 1",
                rusqlite::params![project_id, since_timestamp],
                |row| row.get(0),
            )
            .ok();
        Ok(parse_risk_summary(content.as_deref()))
    }

    /// 成本趋势（按天）
    pub fn get_cost_trends_daily(
        &self,
        since_timestamp: i64,
    ) -> Result<Vec<RoiTrendBucket>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT CAST(strftime('%s', date(created_at, 'unixepoch')) AS INTEGER),
                        COALESCE(SUM(cost), 0.0),
                        COALESCE(SUM(prompt_tokens + completion_tokens), 0),
                        COUNT(*)
                 FROM ai_cost_log
                 WHERE created_at >= ?1
                 GROUP BY date(created_at, 'unixepoch')
                 ORDER BY 1 ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([since_timestamp], |row| {
                Ok(RoiTrendBucket {
                    bucket_start: row.get(0)?,
                    cost: row.get(1)?,
                    tokens: row.get::<_, i64>(2)? as u64,
                    api_calls: row.get::<_, i64>(3)? as u32,
                    nl_answers: 0,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut buckets: Vec<RoiTrendBucket> = Vec::new();
        for row in rows {
            buckets.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }

        // 合并 NL 问答按天计数
        let mut nl_stmt = conn
            .prepare(
                "SELECT CAST(strftime('%s', date(created_at, 'unixepoch')) AS INTEGER),
                        COUNT(*)
                 FROM ai_query_log
                 WHERE created_at >= ?1
                 GROUP BY date(created_at, 'unixepoch')",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let nl_rows = nl_stmt
            .query_map([since_timestamp], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as u32))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        for row in nl_rows {
            if let Ok((day_start, nl_count)) = row {
                if let Some(bucket) = buckets.iter_mut().find(|b| b.bucket_start == day_start) {
                    bucket.nl_answers = nl_count;
                } else {
                    buckets.push(RoiTrendBucket {
                        bucket_start: day_start,
                        cost: 0.0,
                        tokens: 0,
                        api_calls: 0,
                        nl_answers: nl_count,
                    });
                }
            }
        }
        buckets.sort_by_key(|b| b.bucket_start);
        Ok(buckets)
    }

    /// 构建 AI ROI 报告
    pub fn get_ai_roi_report(
        &self,
        since_timestamp: i64,
        range_days: u32,
    ) -> Result<AIRoiReport, AppError> {
        let summary = self.get_ai_cost_summary(since_timestamp)?;
        let by_type_entries = self.get_ai_cost_by_type(since_timestamp)?;
        let nl_answers = self.count_nl_queries(since_timestamp)?;
        let risks_found = self.count_risks_since(since_timestamp)?;
        let insights_generated = self.count_insights_since(since_timestamp)?;
        let useful_feedback = self.count_feedback_since(since_timestamp, "useful")?;
        let not_useful_feedback = self.count_feedback_since(since_timestamp, "not_useful")?;

        let by_type: Vec<CostByTypeDetail> = by_type_entries
            .iter()
            .map(|e| CostByTypeDetail {
                insight_type: e.insight_type.clone(),
                count: e.count as u32,
                total_cost: e.total_cost,
                total_tokens: e.total_tokens as u64,
            })
            .collect();

        let project_rows = self.get_cost_by_project(since_timestamp)?;
        let mut by_project = Vec::new();
        for (project_id, project_name, cost, tokens, insight_count) in project_rows {
            let (risk_count, top_risks) =
                self.project_risk_summary(&project_id, since_timestamp)?;
            let useful_count = self.count_useful_by_project(&project_id, since_timestamp)?;
            by_project.push(ProjectRoiEntry {
                project_id,
                project_name,
                cost,
                tokens,
                insight_count,
                risk_count,
                useful_count,
                top_risks,
            });
        }

        let trends = self.get_cost_trends_daily(since_timestamp)?;

        let totals = AIRoiTotals {
            cost: summary.total_cost,
            tokens: (summary.total_prompt_tokens + summary.total_completion_tokens) as u64,
            api_calls: summary.insight_count as u32,
            insights_generated,
            risks_found,
            nl_answers,
            useful_feedback,
            not_useful_feedback,
        };

        let narrative = format!(
            "近 {} 天 AI 投入 ¥{:.2}（{} tokens），完成 {} 次 API 分析、{} 次 NL 问答，发现 {} 项风险，{} 条反馈标记为有用。",
            range_days,
            totals.cost,
            totals.tokens,
            totals.api_calls,
            totals.nl_answers,
            totals.risks_found,
            totals.useful_feedback,
        );

        Ok(AIRoiReport {
            period_days: range_days,
            totals,
            by_type,
            by_project,
            trends,
            narrative,
        })
    }

    /// 清理超过保留天数的 AI 成本日志
    pub fn prune_ai_cost_logs(&self, retain_days: i64) -> Result<u64, AppError> {
        let cutoff = chrono::Utc::now().timestamp() - retain_days * 86400;
        let conn = lock_conn!(self.conn);
        let deleted = conn
            .execute("DELETE FROM ai_cost_log WHERE created_at < ?1", [cutoff])
            .map_err(|e| AppError::Database(e.to_string()))?;
        if deleted > 0 {
            log::info!("清理了 {deleted} 条超过 {retain_days} 天的 AI 成本日志");
        }
        Ok(deleted as u64)
    }

    // ── Agent 配置就绪度查询（F-P2-1）────────────────────

    /// 查询项目已启用关联的 MCP 服务器数量
    pub fn count_enabled_project_mcp(&self, project_id: &str) -> Result<u32, AppError> {
        self.count_enabled_project_assets(
            project_id,
            crate::database::dao::project_assets::ASSET_MCP,
        )
    }

    /// 查询项目已启用关联的 Skills 数量
    pub fn count_enabled_project_skills(&self, project_id: &str) -> Result<u32, AppError> {
        self.count_enabled_project_assets(
            project_id,
            crate::database::dao::project_assets::ASSET_SKILL,
        )
    }

    /// 查询项目已启用关联的 Prompts 数量
    pub fn count_enabled_project_prompts(&self, project_id: &str) -> Result<u32, AppError> {
        self.count_enabled_project_assets(
            project_id,
            crate::database::dao::project_assets::ASSET_PROMPT,
        )
    }

    /// 查询项目关联配置的最新 updated_at（统一 `project_asset_links`）
    pub fn max_project_config_updated_at(&self, project_id: &str) -> Result<Option<i64>, AppError> {
        self.max_project_asset_links_updated_at(project_id)
    }

    /// 查询全局 ignore 规则总数
    pub fn count_global_ignore_rules(&self) -> Result<u32, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ignore_rules", [], |row| row.get(0))
            .unwrap_or(0);
        Ok(count as u32)
    }

    /// 查询全局 tool_permissions 总数
    pub fn count_global_permissions(&self) -> Result<u32, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_permissions", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);
        Ok(count as u32)
    }

    /// 通过路径查找 SQLite project（返回 id 和 path）
    pub fn get_project_id_by_path(&self, path: &str) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn
            .query_row("SELECT id FROM projects WHERE path = ?1", [path], |row| {
                row.get::<_, String>(0)
            })
            .ok();
        Ok(result)
    }

    /// 删除指定项目的指定类型 AI 洞察缓存
    pub fn delete_ai_insight(&self, project_id: &str, insight_type: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM ai_insights WHERE project_id = ?1 AND insight_type = ?2",
            rusqlite::params![project_id, insight_type],
        )
        .map_err(|e| AppError::Database(format!("delete ai_insight 失败: {e}")))?;
        Ok(())
    }

    /// 按洞察类型批量删除缓存（如全局配置变更后失效所有就绪分）
    pub fn delete_ai_insights_by_type(&self, insight_type: &str) -> Result<u32, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "DELETE FROM ai_insights WHERE insight_type = ?1",
                [insight_type],
            )
            .map_err(|e| AppError::Database(format!("delete ai_insights_by_type 失败: {e}")))?;
        Ok(affected as u32)
    }

    // ── 反馈闭环 ──────────────────────────────────

    /// 更新 AI 洞察的用户反馈（useful / not_useful）
    pub fn update_insight_feedback(
        &self,
        project_id: &str,
        insight_type: &str,
        feedback: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "UPDATE ai_insights SET user_feedback = ?3
                 WHERE project_id = ?1 AND insight_type = ?2",
                rusqlite::params![project_id, insight_type, feedback],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }
}

fn parse_risk_count(content: &str) -> u32 {
    parse_risk_summary(Some(content)).0
}

fn parse_risk_summary(content: Option<&str>) -> (u32, Vec<String>) {
    let Some(raw) = content else {
        return (0, Vec::new());
    };
    if let Ok(parsed) = serde_json::from_str::<AIRiskResult>(raw) {
        let top: Vec<String> = parsed
            .risks
            .iter()
            .take(3)
            .map(|r| r.evidence.clone())
            .collect();
        return (parsed.risks.len() as u32, top);
    }
    (0, Vec::new())
}
