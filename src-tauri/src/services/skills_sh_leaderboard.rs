use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::get_app_config_dir;

pub const LEADERBOARD_TOP_N: usize = 50;
pub const LEADERBOARD_CACHE_TTL_SECS: u64 = 6 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillsShLeaderboardPeriod {
    AllTime,
    Trending24h,
}

impl SkillsShLeaderboardPeriod {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "all_time" | "allTime" => Ok(Self::AllTime),
            "trending_24h" | "trending24h" | "trending" => Ok(Self::Trending24h),
            other => Err(anyhow!("unsupported leaderboard period: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllTime => "all_time",
            Self::Trending24h => "trending_24h",
        }
    }

    fn fetch_path(self) -> &'static str {
        match self {
            Self::AllTime => "/",
            Self::Trending24h => "/trending",
        }
    }

    fn view_marker(self) -> &'static str {
        match self {
            Self::AllTime => "all-time",
            Self::Trending24h => "trending",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsShLeaderboardItem {
    pub rank: u32,
    pub key: String,
    pub name: String,
    pub source: String,
    pub skill_id: String,
    pub installs: u64,
    pub repo_owner: String,
    pub repo_name: String,
    pub directory: String,
    pub readme_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillsShLeaderboardCache {
    period: String,
    synced_at: i64,
    source_url: String,
    total_skills: Option<u64>,
    all_time_total: Option<u64>,
    skills: Vec<SkillsShLeaderboardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsShLeaderboardResult {
    pub period: String,
    pub synced_at: i64,
    pub from_cache: bool,
    pub source_url: String,
    pub total_skills: Option<u64>,
    pub all_time_total: Option<u64>,
    /// 本地缓存 TTL（秒），与 `LEADERBOARD_CACHE_TTL_SECS` 一致
    pub cache_ttl_secs: u64,
    pub skills: Vec<SkillsShLeaderboardItem>,
}

#[derive(Debug, Clone)]
struct ParsedLeaderboardMeta {
    total_skills: Option<u64>,
    all_time_total: Option<u64>,
}

pub fn parse_leaderboard_html(
    html: &str,
    period: SkillsShLeaderboardPeriod,
) -> Result<(Vec<SkillsShLeaderboardItem>, ParsedLeaderboardMeta)> {
    let view_marker = period.view_marker();
    let escaped_view = format!(r#"\"view\":\"{view_marker}\""#);
    if !html.contains(&escaped_view) {
        return Err(anyhow!(
            "skills.sh page missing expected view marker: {view_marker}"
        ));
    }

    let skill_re = Regex::new(
        r#"\\"source\\":\\"([^\\]+)\\",\\"skillId\\":\\"([^\\]+)\\",\\"name\\":\\"([^\\]+)\\",\\"installs\\":(\d+)"#,
    )
    .expect("valid leaderboard regex");

    let mut skills = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in skill_re.captures_iter(html) {
        let source = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let skill_id = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
        let name = cap.get(3).map(|m| m.as_str()).unwrap_or_default();
        let installs = cap
            .get(4)
            .and_then(|m| m.as_str().parse::<u64>().ok())
            .unwrap_or(0);

        if !is_github_source(source) {
            continue;
        }

        let dedupe_key = format!("{source}/{skill_id}");
        if !seen.insert(dedupe_key.clone()) {
            continue;
        }

        let (repo_owner, repo_name) = split_source(source);
        let rank = skills.len() as u32 + 1;
        skills.push(SkillsShLeaderboardItem {
            rank,
            key: dedupe_key,
            name: name.to_string(),
            source: source.to_string(),
            skill_id: skill_id.to_string(),
            installs,
            repo_owner: repo_owner.clone(),
            repo_name: repo_name.clone(),
            directory: skill_id.to_string(),
            readme_url: Some(format!("https://github.com/{repo_owner}/{repo_name}")),
        });

        if skills.len() >= LEADERBOARD_TOP_N {
            break;
        }
    }

    if skills.is_empty() {
        return Err(anyhow!("no leaderboard skills parsed from skills.sh page"));
    }

    let total_skills = Regex::new(r#"\\"totalSkills\\":(\d+)"#)
        .ok()
        .and_then(|re| re.captures(html))
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok());

    let all_time_total = Regex::new(r#"\\"allTimeTotal\\":(\d+)"#)
        .ok()
        .and_then(|re| re.captures(html))
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok());

    Ok((
        skills,
        ParsedLeaderboardMeta {
            total_skills,
            all_time_total,
        },
    ))
}

pub async fn get_skills_sh_leaderboard(
    period: SkillsShLeaderboardPeriod,
    force_refresh: bool,
) -> Result<SkillsShLeaderboardResult> {
    let cache_path = leaderboard_cache_path(period);
    if !force_refresh {
        if let Some(cached) = read_cache(&cache_path)? {
            if cache_is_fresh(cached.synced_at) {
                return Ok(SkillsShLeaderboardResult {
                    period: cached.period,
                    synced_at: cached.synced_at,
                    from_cache: true,
                    source_url: cached.source_url,
                    total_skills: cached.total_skills,
                    all_time_total: cached.all_time_total,
                    cache_ttl_secs: LEADERBOARD_CACHE_TTL_SECS,
                    skills: cached.skills,
                });
            }
        }
    }

    let source_url = format!("https://skills.sh{}", period.fetch_path());
    let client = crate::proxy::http_client::get();
    let html = client
        .get(&source_url)
        .header(
            "User-Agent",
            "OpenSunstar/1.0 (+https://github.com/alisunstar/OpenSunstar)",
        )
        .header("Accept", "text/html,application/xhtml+xml")
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .with_context(|| format!("fetch skills.sh leaderboard: {source_url}"))?
        .error_for_status()
        .with_context(|| format!("skills.sh leaderboard HTTP error: {source_url}"))?
        .text()
        .await
        .context("read skills.sh leaderboard response")?;

    let (skills, meta) = parse_leaderboard_html(&html, period)?;
    let synced_at = now_ms();
    let cached = SkillsShLeaderboardCache {
        period: period.as_str().to_string(),
        synced_at,
        source_url: source_url.clone(),
        total_skills: meta.total_skills,
        all_time_total: meta.all_time_total,
        skills,
    };
    write_cache(&cache_path, &cached)?;

    Ok(SkillsShLeaderboardResult {
        period: cached.period,
        synced_at,
        from_cache: false,
        source_url,
        total_skills: cached.total_skills,
        all_time_total: cached.all_time_total,
        cache_ttl_secs: LEADERBOARD_CACHE_TTL_SECS,
        skills: cached.skills,
    })
}

fn is_github_source(source: &str) -> bool {
    let parts: Vec<&str> = source.splitn(2, '/').collect();
    if parts.len() != 2 {
        return false;
    }
    let (owner, repo) = (parts[0], parts[1]);
    !owner.is_empty() && !repo.is_empty() && !owner.contains('.') && !repo.contains('.')
}

fn split_source(source: &str) -> (String, String) {
    let parts: Vec<&str> = source.splitn(2, '/').collect();
    if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (source.to_string(), String::new())
    }
}

fn leaderboard_cache_path(period: SkillsShLeaderboardPeriod) -> PathBuf {
    get_app_config_dir()
        .join("cache")
        .join(format!("skills-sh-leaderboard-{}.json", period.as_str()))
}

fn read_cache(path: &PathBuf) -> Result<Option<SkillsShLeaderboardCache>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).with_context(|| format!("read cache {}", path.display()))?;
    let parsed =
        serde_json::from_str(&raw).with_context(|| format!("parse cache {}", path.display()))?;
    Ok(Some(parsed))
}

fn write_cache(path: &PathBuf, cache: &SkillsShLeaderboardCache) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create cache dir {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(cache).context("serialize leaderboard cache")?;
    fs::write(path, raw).with_context(|| format!("write cache {}", path.display()))?;
    Ok(())
}

fn cache_is_fresh(synced_at_ms: i64) -> bool {
    let now = now_ms();
    let age_secs = ((now - synced_at_ms).max(0) as u64) / 1000;
    age_secs < LEADERBOARD_CACHE_TTL_SECS
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_ALL_TIME: &str = r#"
    {\"source\":\"vercel-labs/skills\",\"skillId\":\"find-skills\",\"name\":\"find-skills\",\"installs\":2233252},
    {\"source\":\"anthropics/skills\",\"skillId\":\"frontend-design\",\"name\":\"frontend-design\",\"installs\":599111},
    {\"source\":\"vercel-labs/agent-skills\",\"skillId\":\"vercel-react-best-practices\",\"name\":\"vercel-react-best-practices\",\"installs\":508044},
    \"totalSkills\":9637,\"allTimeTotal\":811381,\"view\":\"all-time\"}
    "#;

    const FIXTURE_TRENDING: &str = r#"
    {\"source\":\"halt-catch-fire/skills\",\"skillId\":\"remotion-render\",\"name\":\"remotion-render\",\"installs\":21345},
    {\"source\":\"vercel-labs/skills\",\"skillId\":\"find-skills\",\"name\":\"find-skills\",\"installs\":12239},
    \"totalSkills\":9629,\"allTimeTotal\":811381,\"view\":\"trending\"}
    "#;

    #[test]
    fn parse_all_time_top3() {
        let (skills, meta) =
            parse_leaderboard_html(FIXTURE_ALL_TIME, SkillsShLeaderboardPeriod::AllTime).unwrap();
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].rank, 1);
        assert_eq!(skills[0].key, "vercel-labs/skills/find-skills");
        assert_eq!(skills[0].installs, 2_233_252);
        assert_eq!(meta.all_time_total, Some(811_381));
    }

    #[test]
    fn parse_trending_preserves_order() {
        let (skills, _) =
            parse_leaderboard_html(FIXTURE_TRENDING, SkillsShLeaderboardPeriod::Trending24h)
                .unwrap();
        assert_eq!(skills[0].key, "halt-catch-fire/skills/remotion-render");
        assert_eq!(skills[1].key, "vercel-labs/skills/find-skills");
    }

    #[test]
    fn rejects_wrong_view_marker() {
        let err = parse_leaderboard_html(FIXTURE_TRENDING, SkillsShLeaderboardPeriod::AllTime)
            .unwrap_err();
        assert!(err.to_string().contains("view marker"));
    }
}
