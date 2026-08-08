use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::get_app_config_dir;

pub const LEADERBOARD_TOP_N: usize = 100;
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
pub(crate) struct ParsedLeaderboardMeta {
    total_skills: Option<u64>,
    all_time_total: Option<u64>,
}

pub fn parse_leaderboard_html(
    html: &str,
    period: SkillsShLeaderboardPeriod,
) -> Result<(Vec<SkillsShLeaderboardItem>, ParsedLeaderboardMeta)> {
    // Try new Next.js rendered format first (migrated ~2025)
    if let Ok(result) = parse_leaderboard_html_v2(html, period) {
        return Ok(result);
    }
    // Fall back to legacy embedded-JSON format
    parse_leaderboard_html_legacy(html, period)
}

/// Parse the newer Next.js client-rendered HTML structure.
///
/// Expected patterns (skills.sh ~2025+):
/// - Skill link: `<a ... href="/owner/repo/skillId" ...>`
/// - Name: `<h3 ...>skill-name</h3>` near the link
/// - Installs: `<span class="font-mono text-sm text-foreground">2.9M</span>` (K/M suffixes)
/// - Total skills: `All Time (1,145,344)` in a tab/link label
///
/// Uses positional correlation: extracts hrefs, names, and installs separately,
/// then correlates them by document order since they appear in repeating groups.
fn parse_leaderboard_html_v2(
    html: &str,
    _period: SkillsShLeaderboardPeriod,
) -> Result<(Vec<SkillsShLeaderboardItem>, ParsedLeaderboardMeta)> {
    // 1. Extract all skill entry hrefs: /owner/repo/skillId
    let href_re = Regex::new(r#"<a[^>]*class="group grid[^"]*"[^>]*href="(/([^"]+))""#)
        .expect("valid v2 href regex");

    // Quick sanity check – if no entries found, bail to legacy parser
    if !href_re.is_match(html) {
        return Err(anyhow!("HTML does not match v2 leaderboard format"));
    }

    // 2. Extract install counts with K/M/G/T suffixes
    let installs_re = Regex::new(r#"font-mono text-sm text-foreground">([0-9.]+)([KMGT]?)"#)
        .expect("valid installs regex");

    // 3. Extract skill names from <h3> tags
    let name_re = Regex::new(r#"<h3[^>]*class="[^"]*font-semibold[^"]*"[^>]*>([^<]+)</h3>"#)
        .expect("valid name regex");

    // 4. Extract total skills count from tab label
    let total_re = Regex::new(r#"All Time \(([0-9,]+)\)"#).expect("valid total regex");

    // Collect all positions in document order
    struct HrefMatch {
        path: String,
        pos: usize,
    }
    struct InstallsMatch {
        value: u64,
        pos: usize,
    }
    struct NameMatch {
        name: String,
        pos: usize,
    }

    let mut hrefs: Vec<HrefMatch> = Vec::new();
    for cap in href_re.captures_iter(html) {
        let path = cap
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or_default()
            .to_string();
        let pos = cap.get(0).map(|m| m.start()).unwrap_or(0);
        hrefs.push(HrefMatch { path, pos });
    }

    let mut installs_list: Vec<InstallsMatch> = Vec::new();
    for cap in installs_re.captures_iter(html) {
        let num_str = cap.get(1).map(|m| m.as_str()).unwrap_or("0");
        let suffix = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let base: f64 = num_str.parse().unwrap_or(0.0);
        let multiplier = match suffix {
            "K" => 1_000.0,
            "M" => 1_000_000.0,
            "G" => 1_000_000_000.0,
            "T" => 1_000_000_000_000.0,
            _ => 1.0,
        };
        let pos = cap.get(0).map(|m| m.start()).unwrap_or(0);
        installs_list.push(InstallsMatch {
            value: (base * multiplier).round() as u64,
            pos,
        });
    }

    let mut names: Vec<NameMatch> = Vec::new();
    for cap in name_re.captures_iter(html) {
        let name = cap
            .get(1)
            .map(|m| m.as_str().trim())
            .unwrap_or("")
            .to_string();
        let pos = cap.get(0).map(|m| m.start()).unwrap_or(0);
        names.push(NameMatch { name, pos });
    }

    // Correlate by document order: for each href, find the nearest following name and installs
    let mut skills = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (idx, href_entry) in hrefs.iter().enumerate() {
        let href_path = &href_entry.path;
        let href_pos = href_entry.pos;

        // Parse href path: "/owner/repo/skillId"
        let parts: Vec<&str> = href_path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            continue;
        }

        let (source, skill_id) = if parts.len() >= 3 && !parts[0].contains('.') {
            (format!("{}/{}", parts[0], parts[1]), parts[2].to_string())
        } else if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            let sk = parts.last().unwrap_or(&"").to_string();
            let src = parts[..parts.len().saturating_sub(1)].join("/");
            (src, sk)
        };

        if !is_github_source(&source) {
            continue;
        }

        let dedupe_key = format!("{source}/{skill_id}");
        if !seen.insert(dedupe_key.clone()) {
            continue;
        }

        // Find nearest name after this href position
        let name = names
            .iter()
            .find(|n| n.pos > href_pos)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| skill_id.clone());

        // Find nearest installs after this href position (use index as hint)
        let installs = installs_list
            .iter()
            .skip(idx.min(installs_list.len()))
            .find(|i| i.pos > href_pos)
            .or_else(|| installs_list.get(idx))
            .map(|i| i.value)
            .unwrap_or(0);

        let (repo_owner, repo_name) = split_source(&source);
        let rank = skills.len() as u32 + 1;
        skills.push(SkillsShLeaderboardItem {
            rank,
            key: dedupe_key,
            name,
            source: source.clone(),
            skill_id: skill_id.clone(),
            installs,
            repo_owner: repo_owner.clone(),
            repo_name: repo_name.clone(),
            directory: skill_id.clone(),
            readme_url: Some(format!("https://github.com/{repo_owner}/{repo_name}")),
        });

        if skills.len() >= LEADERBOARD_TOP_N {
            break;
        }
    }

    if skills.is_empty() {
        return Err(anyhow!(
            "no leaderboard skills parsed from skills.sh page (v2)"
        ));
    }

    let total_skills = total_re
        .captures(html)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().replace(',', "").parse::<u64>().ok());

    Ok((
        skills,
        ParsedLeaderboardMeta {
            total_skills,
            all_time_total: total_skills,
        },
    ))
}

/// Parse the legacy embedded-JSON format (pre-2025 skills.sh).
///
/// Expected patterns:
/// - View marker: `"view":"all-time"` or `"view":"trending"`
/// - Skill entries: `\"source\":\"owner/repo\",\"skillId\":\"name\",\"name\":\"...\",\"installs\":N`
/// - Meta: `\"totalSkills\":N`, `\"allTimeTotal\":N`
fn parse_leaderboard_html_legacy(
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

/// Raw skill entry from the skills.sh internal leaderboard API.
///
/// Endpoint (Next.js route handler used by the official site):
///   GET https://skills.sh/api/skills/{view}/{page}
/// where `{view}` is `all-time` or `trending` and `{page}` starts at 1.
///
/// Anti-bot guard: the handler returns an empty 200 body unless the request
/// carries a same-origin signal (`Sec-Fetch-Site: same-origin` or `Referer`).
/// The response is JSON of the form
///   { "skills": [{source, skillId, name, installs, weeklyInstalls?}], "total", "hasMore", "page" }
/// already sorted by the relevant metric (installs for all-time, weekly for trending).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillsShApiSkill {
    source: String,
    skill_id: String,
    name: String,
    installs: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillsShApiResponse {
    skills: Vec<SkillsShApiSkill>,
    total: u64,
}

fn api_view_slug(period: SkillsShLeaderboardPeriod) -> &'static str {
    match period {
        SkillsShLeaderboardPeriod::AllTime => "all-time",
        SkillsShLeaderboardPeriod::Trending24h => "trending",
    }
}

/// Parse the skills.sh leaderboard API JSON response into items.
/// Pure/sync so it can be unit-tested without network access.
fn parse_leaderboard_api_json(
    body: &str,
    _period: SkillsShLeaderboardPeriod,
) -> Result<(Vec<SkillsShLeaderboardItem>, ParsedLeaderboardMeta)> {
    if body.trim().is_empty() {
        return Err(anyhow!(
            "skills.sh leaderboard API returned empty body (bot guard)"
        ));
    }
    let api: SkillsShApiResponse =
        serde_json::from_str(body).with_context(|| "parse skills.sh leaderboard API JSON")?;

    let mut skills = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in api.skills.into_iter().take(LEADERBOARD_TOP_N) {
        if !is_github_source(&raw.source) {
            continue;
        }
        let dedupe_key = format!("{}/{}", raw.source, raw.skill_id);
        if !seen.insert(dedupe_key.clone()) {
            continue;
        }
        let (repo_owner, repo_name) = split_source(&raw.source);
        let rank = skills.len() as u32 + 1;
        skills.push(SkillsShLeaderboardItem {
            rank,
            key: dedupe_key,
            name: raw.name,
            source: raw.source,
            skill_id: raw.skill_id.clone(),
            installs: raw.installs,
            repo_owner: repo_owner.clone(),
            repo_name: repo_name.clone(),
            directory: raw.skill_id.clone(),
            readme_url: Some(format!("https://github.com/{repo_owner}/{repo_name}")),
        });
    }

    if skills.is_empty() {
        return Err(anyhow!("no leaderboard skills parsed from skills.sh API"));
    }

    Ok((
        skills,
        ParsedLeaderboardMeta {
            total_skills: Some(api.total),
            all_time_total: Some(api.total),
        },
    ))
}

/// Fetch the leaderboard as rendered by the official site frontend.
///
/// The server-rendered HTML is the ranking users actually see on skills.sh
/// (SSR includes ~100+ entries per view). The internal JSON API diverges from
/// it (all-time returns a windowed metric; trending is spam-polluted), so HTML
/// is the primary source and the API only a fallback.
async fn fetch_leaderboard_via_html(
    period: SkillsShLeaderboardPeriod,
    source_url: &str,
) -> Result<(Vec<SkillsShLeaderboardItem>, ParsedLeaderboardMeta)> {
    let client = crate::proxy::http_client::get();
    let html = client
        .get(source_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/126.0 Safari/537.36",
        )
        .header("Accept", "text/html,application/xhtml+xml")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .with_context(|| format!("fetch skills.sh leaderboard: {source_url}"))?
        .error_for_status()
        .with_context(|| format!("skills.sh leaderboard HTTP error: {source_url}"))?
        .text()
        .await
        .context("read skills.sh leaderboard response")?;

    parse_leaderboard_html(&html, period)
}

/// Fetch the leaderboard via the official site's internal JSON API (fallback).
async fn fetch_leaderboard_via_api(
    period: SkillsShLeaderboardPeriod,
) -> Result<(Vec<SkillsShLeaderboardItem>, ParsedLeaderboardMeta)> {
    let slug = api_view_slug(period);
    let source_url = format!("https://skills.sh/api/skills/{slug}/1");
    let client = crate::proxy::http_client::get();
    let body = client
        .get(&source_url)
        .header(
            "User-Agent",
            "OpenSunstar/1.0 (+https://github.com/alisunstar/OpenSunstar)",
        )
        .header("Accept", "application/json, */*")
        // Same-origin signal required to bypass the bot guard (returns empty body otherwise).
        .header("Sec-Fetch-Site", "same-origin")
        .header("Referer", "https://skills.sh/")
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .with_context(|| format!("fetch skills.sh leaderboard API: {source_url}"))?
        .error_for_status()
        .with_context(|| format!("skills.sh leaderboard API HTTP error: {source_url}"))?
        .text()
        .await
        .context("read skills.sh leaderboard API response")?;

    parse_leaderboard_api_json(&body, period)
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

    // 主路径：官网前端实际渲染的榜单（SSR 含 100+ 条目，与官网显示对齐）。
    // 兜底：内部 JSON API（排名口径与官网不一致，仅在 HTML 不可用时使用）。
    let html_url = format!("https://skills.sh{}", period.fetch_path());
    let (skills, meta, source_url) = match fetch_leaderboard_via_html(period, &html_url).await {
        Ok((skills, meta)) => (skills, meta, html_url),
        Err(html_err) => {
            log::warn!("skills.sh HTML 主路径失败，回退内部 API: {html_err}");
            match fetch_leaderboard_via_api(period).await {
                Ok((skills, meta)) => (
                    skills,
                    meta,
                    format!("https://skills.sh/api/skills/{}/1", api_view_slug(period)),
                ),
                Err(api_err) => {
                    return Err(anyhow!(
                        "skills.sh 榜单 HTML 与 API 均失败: HTML={html_err}; API={api_err}"
                    ))
                }
            }
        }
    };
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

    // ── Legacy format fixtures (pre-2025 embedded JSON) ──────────

    const FIXTURE_LEGACY_ALL_TIME: &str = r#"
    {\"source\":\"vercel-labs/skills\",\"skillId\":\"find-skills\",\"name\":\"find-skills\",\"installs\":2233252},
    {\"source\":\"anthropics/skills\",\"skillId\":\"frontend-design\",\"name\":\"frontend-design\",\"installs\":599111},
    {\"source\":\"vercel-labs/agent-skills\",\"skillId\":\"vercel-react-best-practices\",\"name\":\"vercel-react-best-practices\",\"installs\":508044},
    \"totalSkills\":9637,\"allTimeTotal\":811381,\"view\":\"all-time\"}
    "#;

    const FIXTURE_LEGACY_TRENDING: &str = r#"
    {\"source\":\"halt-catch-fire/skills\",\"skillId\":\"remotion-render\",\"name\":\"remotion-render\",\"installs\":21345},
    {\"source\":\"vercel-labs/skills\",\"skillId\":\"find-skills\",\"name\":\"find-skills\",\"installs\":12239},
    \"totalSkills\":9629,\"allTimeTotal\":811381,\"view\":\"trending\"}
    "#;

    // ── V2 format fixtures (Next.js rendered HTML, ~2025+) ───────

    const FIXTURE_V2_ALL_TIME: &str = r#"
    <h2 class="...">Skills Leaderboard</h2>
    <a class="pb-1 border-b-2 ... border-foreground text-foreground" href="/">All Time (1,145,344)</a>
    <a class="pb-1 border-b-2 ..." href="/trending">Trending (24h)</a>
    <div><div>
    <a class="group grid grid-cols-[auto_1fr_auto] lg:grid-cols-16 items-start lg:items-center gap-3 py-3 hover:bg-(--ds-gray-100)/30 border-b border-border h-full" href="/vercel-labs/skills/find-skills">
    <div class="min-w-7 lg:min-w-0 lg:col-span-1 text-left"><span class="text-sm lg:text-base text-(--ds-gray-600) font-mono">1</span></div>
    <div class="lg:col-span-11 min-w-1 flex flex-col lg:flex-row lg:items-baseline lg:gap-2">
    <h3 class="font-semibold text-foreground truncate whitespace-nowrap">find-skills</h3>
    <p class="text-xs lg:text-sm text-(--ds-gray-600) font-mono mt-0.5 lg:mt-0 truncate">vercel-labs/skills</p></div>
    <div class="lg:col-span-2 text-right flex items-center justify-end gap-2">
    <span class="font-mono text-sm text-foreground">2.9M</span></div></a>
    <a class="group grid grid-cols-[auto_1fr_auto] lg:grid-cols-16 items-start lg:items-center gap-3 py-3 hover:bg-(--ds-gray-100)/30 border-b border-border h-full" href="/mattpocock/skills/grill-me">
    <div class="min-w-7 lg:min-w-0 lg:col-span-1 text-left"><span class="text-sm lg:text-base text-(--ds-gray-600) font-mono">2</span></div>
    <div class="lg:col-span-11 min-w-1 flex flex-col lg:flex-row lg:items-baseline lg:gap-2">
    <h3 class="font-semibold text-foreground truncate whitespace-nowrap">grill-me</h3>
    <p class="text-xs lg:text-sm text-(--ds-gray-600) font-mono mt-0.5 lg:mt-0 truncate">mattpocock/skills</p></div>
    <div class="lg:col-span-2 text-right flex items-center justify-end gap-2">
    <span class="font-mono text-sm text-foreground">784.3K</span></div></a>
    <a class="group grid grid-cols-[auto_1fr_auto] lg:grid-cols-16 items-start lg:items-center gap-3 py-3 hover:bg-(--ds-gray-100)/30 border-b border-border h-full" href="/anthropics/skills/frontend-design">
    <div class="min-w-7 lg:min-w-0 lg:col-span-1 text-left"><span class="text-sm lg:text-base text-(--ds-gray-600) font-mono">3</span></div>
    <div class="lg:col-span-11 min-w-1 flex flex-col lg:flex-row lg:items-baseline lg:gap-2">
    <h3 class="font-semibold text-foreground truncate whitespace-nowrap">frontend-design</h3>
    <p class="text-xs lg:text-sm text-(--ds-gray-600) font-mono mt-0.5 lg:mt-0 truncate">anthropics/skills</p></div>
    <div class="lg:col-span-2 text-right flex items-center justify-end gap-2">
    <svg viewBox="0 0 16 16" height="16" width="16" data-slot="geist-icon" style="color:var(--ds-gray-600)" class="h-3.5 w-3.5 shrink-0"><path fill="currentColor" d="M8 0a1 1 0 0 1 .7.29l1.76 1.76h2.5a1 1 0 0 1 .99 1v2.49L15.7 7.3a1 1 0 0 1 0 1.4l-1.76 1.76v2.5a1 1 0 0 1-1 .99h-2.49L8.7 15.7a1 1 0 0 1-1.4 0l-1.76-1.76h-2.6v2.6L1.7 8l1.84 1.84v2.6h2.6l.45.45 1.4 1.4 1.4-1.4.44-.44h2.6v-2.6l.45-.45 1.4-1.4-1.84-1.84v-2.6h-2.6L8 1.7zm4.59 3.3-3.72 3.71c-.3.3-.77.3-1.06 0L4.8 8.53l1.07-1.06 1.06 1.06 3.18-3.18z"/></svg>
    <span class="font-mono text-sm text-foreground">750.3K</span></div></a>
    </div></div>
    "#;

    // ── Legacy format tests ─────────────────────────────────────

    #[test]
    fn parse_legacy_all_time_top3() {
        let (skills, meta) =
            parse_leaderboard_html(FIXTURE_LEGACY_ALL_TIME, SkillsShLeaderboardPeriod::AllTime)
                .unwrap();
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].rank, 1);
        assert_eq!(skills[0].key, "vercel-labs/skills/find-skills");
        assert_eq!(skills[0].installs, 2_233_252);
        assert_eq!(meta.all_time_total, Some(811_381));
    }

    #[test]
    fn parse_legacy_trending_preserves_order() {
        let (skills, _) = parse_leaderboard_html(
            FIXTURE_LEGACY_TRENDING,
            SkillsShLeaderboardPeriod::Trending24h,
        )
        .unwrap();
        assert_eq!(skills[0].key, "halt-catch-fire/skills/remotion-render");
        assert_eq!(skills[1].key, "vercel-labs/skills/find-skills");
    }

    #[test]
    fn rejects_wrong_view_marker() {
        let err =
            parse_leaderboard_html(FIXTURE_LEGACY_TRENDING, SkillsShLeaderboardPeriod::AllTime)
                .unwrap_err();
        assert!(err.to_string().contains("view marker"));
    }

    // ── V2 format tests ──────────────────────────────────────────

    #[test]
    fn parse_v2_all_time_top3() {
        let (skills, meta) =
            parse_leaderboard_html(FIXTURE_V2_ALL_TIME, SkillsShLeaderboardPeriod::AllTime)
                .unwrap();
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].rank, 1);
        assert_eq!(skills[0].key, "vercel-labs/skills/find-skills");
        assert_eq!(skills[0].name, "find-skills");
        assert_eq!(skills[0].source, "vercel-labs/skills");
        assert_eq!(skills[0].installs, 2_900_000); // 2.9M
        assert_eq!(skills[1].key, "mattpocock/skills/grill-me");
        assert_eq!(skills[1].installs, 784_300); // 784.3K
        assert_eq!(skills[2].key, "anthropics/skills/frontend-design");
        assert_eq!(skills[2].installs, 750_300); // 750.3K
        assert_eq!(meta.total_skills, Some(1_145_344));
    }

    #[test]
    fn parse_v2_falls_back_to_legacy_for_old_format() {
        // Legacy fixture should still work via fallback
        let (skills, _) =
            parse_leaderboard_html(FIXTURE_LEGACY_ALL_TIME, SkillsShLeaderboardPeriod::AllTime)
                .unwrap();
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].installs, 2_233_252); // exact number from legacy
    }

    // ── API (JSON) format tests ───────────────────────────────

    const FIXTURE_API_ALL_TIME: &str = r#"
    {
      "skills": [
        {"source":"heygen-com/hyperframes","skillId":"motion-graphics","name":"motion-graphics","installs":155567,"weeklyInstalls":[10,20,30]},
        {"source":"obra/superpowers","skillId":"finishing-a-development-branch","name":"finishing-a-development-branch","installs":154361},
        {"source":"leonxlnx/taste-skill","skillId":"design-taste-frontend-v1","name":"design-taste-frontend-v1","installs":151312}
      ],
      "total": 9299,
      "hasMore": true,
      "page": 1
    }
    "#;

    #[test]
    fn parse_api_all_time_top3() {
        let (skills, meta) =
            parse_leaderboard_api_json(FIXTURE_API_ALL_TIME, SkillsShLeaderboardPeriod::AllTime)
                .unwrap();
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].rank, 1);
        assert_eq!(skills[0].key, "heygen-com/hyperframes/motion-graphics");
        assert_eq!(skills[0].installs, 155_567);
        assert_eq!(
            skills[1].key,
            "obra/superpowers/finishing-a-development-branch"
        );
        assert_eq!(skills[2].installs, 151_312);
        assert_eq!(meta.total_skills, Some(9_299));
        assert_eq!(meta.all_time_total, Some(9_299));
    }

    #[test]
    fn parse_api_empty_body_errors() {
        let err =
            parse_leaderboard_api_json("   ", SkillsShLeaderboardPeriod::AllTime).unwrap_err();
        assert!(err.to_string().contains("empty body"));
    }
}
