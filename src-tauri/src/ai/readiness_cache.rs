//! Agent 就绪度缓存失效辅助

use crate::ai::project_id::resolve_canonical_project_id;
use crate::ai::types::InsightType;
use crate::database::Database;

const AGENT_READINESS: &str = "agent_readiness";

/// 删除单个项目的 Agent 就绪度缓存（兼容 legacy / canonical id）
pub fn invalidate_agent_readiness_for_project(
    db: &Database,
    project_id: &str,
    project_path: Option<&str>,
) {
    let insight_type = InsightType::AgentReadiness.as_str();
    let canonical = resolve_canonical_project_id(db, project_id, project_path);
    let _ = db.delete_ai_insight(&canonical, insight_type);
    if canonical != project_id {
        let _ = db.delete_ai_insight(project_id, insight_type);
    }
}

/// 全局 Ignore / Permissions 变更后，清空所有项目的就绪度缓存
pub fn invalidate_all_agent_readiness_caches(db: &Database) {
    let _ = db.delete_ai_insights_by_type(AGENT_READINESS);
}
