//! DeepSeek Chat API — connectivity test + batched scenario classification.

use crate::scan::{attach_briefs, attach_scenarios, AgentInventory, AssetEntry};
use crate::storage;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tauri::AppHandle;

const SCENARIO_SLUGS: &[&str] = &[
    "dev", "office", "creative", "data", "network", "ops", "collab",
];

#[derive(Debug, Deserialize)]
struct ChatCompletionBody {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageBody,
}

#[derive(Debug, Deserialize)]
struct MessageBody {
    #[serde(default)]
    content: String,
}

fn normalize_slug(raw: &str) -> Option<String> {
    let s = raw.trim().to_lowercase();
    SCENARIO_SLUGS
        .iter()
        .find(|&&x| x == s.as_str())
        .map(|s| (*s).to_string())
}

fn extract_json_object(text: &str) -> Result<Value, String> {
    let t = text.trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        return Ok(v);
    }
    let start = t
        .find('{')
        .ok_or_else(|| "响应中未找到 JSON 对象".to_string())?;
    let end = t
        .rfind('}')
        .ok_or_else(|| "响应中未找到 JSON 对象结尾".to_string())?;
    let slice = &t[start..=end];
    serde_json::from_str(slice).map_err(|e| format!("解析模型 JSON 失败：{e}"))
}

fn provider_label(config: &storage::AiConfig) -> &str {
    if config.provider == storage::GLM_PROVIDER {
        "GLM"
    } else {
        "DeepSeek"
    }
}

async fn chat_completion(
    config: &storage::AiConfig,
    system: &str,
    user: &str,
    json_object_mode: bool,
    max_tokens: u32,
) -> Result<String, String> {
    let label = provider_label(config);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let mut body = json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": 0.2,
        "max_tokens": max_tokens,
    });
    if config.provider == storage::GLM_PROVIDER {
        if let Some(o) = body.as_object_mut() {
            o.insert("thinking".into(), json!({"type": "disabled"}));
        }
    }
    if json_object_mode {
        if let Some(o) = body.as_object_mut() {
            o.insert("response_format".into(), json!({"type": "json_object"}));
        }
    }

    let res = client
        .post(&config.api_url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("{label} 请求失败：{e}"))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!(
            "{label} 返回错误 HTTP {status}：{}",
            err_text.chars().take(400).collect::<String>()
        ));
    }

    let parsed: ChatCompletionBody = res.json().await.map_err(|e| e.to_string())?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| format!("{label} 响应缺少 choices"))
}

pub async fn test_ping(config: &storage::AiConfig) -> Result<String, String> {
    let reply = chat_completion(
        config,
        "You reply with exactly the word OK and nothing else.",
        "Ping.",
        false,
        16,
    )
    .await?;
    Ok(reply.trim().to_string())
}

fn classify_system_prompt() -> String {
    r#"你是 AIControls 的资产分类助手。输入是多条 Skill / MCP / Rule 的简要信息。
必须为每一条选出 **恰好一个** 英文类别 slug（小写），只能从下列集合中选：
dev — 各类编码、前后端、仓库与 API / MCP 集成等开发全流程
office — 办公自动化、日程、文档、邮件等
creative — 文案、音视频、设计、图像生成与多媒体处理等
data — 数据获取、分析、存储、向量库、数据库等
network — 浏览器自动化、网页抓取、联网搜索与 HTTP 抓取等
ops — 容器、云资源、监控、部署与故障排查等
collab — 团队沟通、项目管理、会议与任务协同等

只输出 **一个 JSON 对象**：键为每条资产的 id（字符串），值为 slug。
不要 Markdown，不要解释，不要多余字段。"#
        .to_string()
}

fn summarize_system_prompt(locale: &str) -> String {
    if locale == "zh" {
        r#"你是 AIControls 的资产缩略介绍助手。输入是多条 Skill / MCP / Rule 的条目信息。
请为每条生成中文缩略介绍，并严格遵守：
1) 每条最多 100 个中文字符；
2) 只基于给定 title/description/kind，禁止编造未给出的事实；
3) 风格中性、信息密度高，1-2 句；
4) 不使用 Markdown，不加序号，不输出额外解释。

只输出一个 JSON 对象：键为条目 id（字符串），值为缩略介绍（字符串）。"#
            .to_string()
    } else {
        r#"You are an AIControls asset brief assistant. Input includes Skill / MCP / Rule entries.
Generate concise English briefs and follow:
1) each brief <= 100 characters;
2) only use given title/description/kind, no fabrication;
3) neutral tone, high information density, 1-2 sentences;
4) no markdown, no numbering, no extra explanations.

Output exactly one JSON object: key is entry id (string), value is brief text (string)."#
            .to_string()
    }
}

async fn classify_batch(
    config: &storage::AiConfig,
    batch: &[AssetEntry],
) -> Result<HashMap<String, String>, String> {
    let mut lines = Vec::new();
    for e in batch {
        lines.push(format!(
            "- id={} kind={} title={} description={}",
            serde_json::to_string(&e.id).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.kind).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.title).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.description).map_err(|e| e.to_string())?,
        ));
    }
    let user = format!(
        "请为下列条目分类（输出 JSON 对象 id→slug）：\n{}",
        lines.join("\n")
    );

    let raw = chat_completion(config, &classify_system_prompt(), &user, true, 800).await?;
    let v = extract_json_object(&raw)?;
    let obj = v
        .as_object()
        .ok_or_else(|| "模型输出不是 JSON 对象".to_string())?;

    let mut out = HashMap::new();
    for (id, val) in obj {
        let slug = val
            .as_str()
            .ok_or_else(|| format!("字段 {id} 的值不是字符串"))?;
        let Some(norm) = normalize_slug(slug) else {
            return Err(format!("条目 {id} 的类别 {slug} 非法"));
        };
        out.insert(id.clone(), norm);
    }

    Ok(out)
}

async fn classify_batch_fill_missing(
    config: &storage::AiConfig,
    chunk: &[AssetEntry],
) -> Result<HashMap<String, String>, String> {
    let mut delta = classify_batch(config, chunk).await?;
    for e in chunk {
        if delta.contains_key(&e.id) {
            continue;
        }
        match classify_batch(config, &[e.clone()]).await {
            Ok(m) => delta.extend(m),
            Err(_) => { /* 单次失败则跳过该 id，下次扫描仍会尝试 */ }
        }
    }
    Ok(delta)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>()
    }
}

fn normalize_brief_text(raw: &str) -> Option<String> {
    let compact = raw
        .lines()
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let compact = compact.trim();
    if compact.is_empty() {
        return None;
    }
    let cut = truncate_chars(compact, 100);
    Some(cut)
}

fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

fn is_brief_compatible_locale(locale: &str, brief: &str) -> bool {
    if locale == "zh" {
        has_cjk(brief)
    } else {
        !has_cjk(brief)
    }
}

async fn summarize_batch(
    config: &storage::AiConfig,
    batch: &[AssetEntry],
    locale: &str,
) -> Result<HashMap<String, String>, String> {
    let mut lines = Vec::new();
    for e in batch {
        lines.push(format!(
            "- id={} kind={} title={} description={}",
            serde_json::to_string(&e.id).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.kind).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.title).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.description).map_err(|e| e.to_string())?,
        ));
    }
    let user = if locale == "zh" {
        format!(
            "请为下列条目生成中文缩略介绍（输出 JSON 对象 id→brief）：\n{}",
            lines.join("\n")
        )
    } else {
        format!(
            "Generate English briefs for entries below (output JSON object id->brief):\n{}",
            lines.join("\n")
        )
    };
    let raw = chat_completion(config, &summarize_system_prompt(locale), &user, true, 1400).await?;
    let v = extract_json_object(&raw)?;
    let obj = v
        .as_object()
        .ok_or_else(|| "模型输出不是 JSON 对象".to_string())?;

    let mut out = HashMap::new();
    for (id, val) in obj {
        let Some(txt) = val.as_str() else {
            continue;
        };
        if let Some(clean) = normalize_brief_text(txt) {
            out.insert(id.clone(), clean);
        }
    }
    Ok(out)
}

async fn summarize_batch_fill_missing(
    config: &storage::AiConfig,
    chunk: &[AssetEntry],
    locale: &str,
) -> Result<HashMap<String, String>, String> {
    let mut delta = summarize_batch(config, chunk, locale).await?;
    for e in chunk {
        if delta.contains_key(&e.id) {
            continue;
        }
        match summarize_batch(config, &[e.clone()], locale).await {
            Ok(m) => delta.extend(m),
            Err(_) => { /* 单次失败则跳过该 id，下次扫描仍会尝试 */ }
        }
    }
    Ok(delta)
}

/// 仅为尚未写入本地缓存 map 的条目调用 AI；结果持久化并写回 `inventory.scenario`。
pub async fn classify_inventory_missing(
    app: &AppHandle,
    mut inventory: AgentInventory,
) -> Result<AgentInventory, String> {
    let config = match storage::load_active_ai_config(app) {
        Ok(c) => c,
        Err(_) => return Ok(inventory),
    };

    let mut map = storage::load_scenario_map(app).unwrap_or_default();
    attach_scenarios(&mut inventory, &map);

    let mut seen = HashSet::<String>::new();
    let missing: Vec<AssetEntry> = inventory
        .skills
        .iter()
        .chain(inventory.mcp.iter())
        .chain(inventory.rules.iter())
        .filter(|e| !map.contains_key(&e.id))
        .filter(|e| seen.insert(e.id.clone()))
        .cloned()
        .collect();

    if missing.is_empty() {
        return Ok(inventory);
    }

    const BATCH: usize = 12;
    for chunk in missing.chunks(BATCH) {
        let delta = classify_batch_fill_missing(&config, chunk).await?;
        if !delta.is_empty() {
            storage::merge_scenario_map(app, &delta)?;
            map.extend(delta);
            attach_scenarios(&mut inventory, &map);
        }
    }

    Ok(inventory)
}

/// 仅为尚未写入本地 brief map 的条目调用 AI；结果持久化并写回 `inventory.brief_zh`。
pub async fn summarize_inventory_missing(
    app: &AppHandle,
    mut inventory: AgentInventory,
    locale: String,
) -> Result<AgentInventory, String> {
    let locale = if locale == "zh" { "zh" } else { "en" };
    let config = match storage::load_active_ai_config(app) {
        Ok(c) => c,
        Err(_) => return Ok(inventory),
    };

    let mut map = storage::load_brief_map(app, locale).unwrap_or_default();
    attach_briefs(&mut inventory, locale, &map);

    let mut seen = HashSet::<String>::new();
    let missing: Vec<AssetEntry> = inventory
        .skills
        .iter()
        .chain(inventory.mcp.iter())
        .chain(inventory.rules.iter())
        .filter(|e| match map.get(&e.id) {
            Some(v) => !is_brief_compatible_locale(locale, v),
            None => true,
        })
        .filter(|e| seen.insert(e.id.clone()))
        .cloned()
        .collect();

    if missing.is_empty() {
        return Ok(inventory);
    }

    const BATCH: usize = 8;
    for chunk in missing.chunks(BATCH) {
        let delta = summarize_batch_fill_missing(&config, chunk, locale).await?;
        if !delta.is_empty() {
            storage::merge_brief_map(app, locale, &delta)?;
            map.extend(delta);
            attach_briefs(&mut inventory, locale, &map);
        }
    }

    Ok(inventory)
}

/// 强制为单条资产重新生成摘要（覆盖缓存），并返回更新后的摘要文本。
pub async fn resummarize_single_asset(
    app: &AppHandle,
    asset: AssetEntry,
    locale: String,
) -> Result<String, String> {
    let locale = if locale == "zh" { "zh" } else { "en" };
    let config = storage::load_active_ai_config(app)?;

    let delta = summarize_batch(&config, &[asset.clone()], locale).await?;
    let brief = delta
        .get(&asset.id)
        .cloned()
        .ok_or_else(|| "模型未返回该条目的简介".to_string())?;

    let mut one = HashMap::new();
    one.insert(asset.id, brief.clone());
    storage::merge_brief_map(app, locale, &one)?;

    Ok(brief)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUrlEnrichment {
    pub title: String,
    pub tags: Vec<String>,
    pub note: String,
}

fn normalize_enrichment_tags(raw: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for t in raw {
        let t = t.trim().to_string();
        if t.is_empty() {
            continue;
        }
        let key = t.to_lowercase();
        if seen.insert(key) {
            out.push(t);
        }
        if out.len() >= 12 {
            break;
        }
    }
    out
}

fn resource_url_system_prompt() -> String {
    r#"你是资源库助手。根据用户给出的网页链接，推断该资源的简短中文标题、标签与用途备注。
规则：
1) title：准确概括站点或页面主题，尽量简短（通常不超过 20 字）；
2) tags：3 到 8 个标签，可用简短中文或英文小写词，去重、勿重复含义；
3) note：1 至 3 句中文，说明适用场景、何时使用或注意事项；不要复述完整 URL；
4) 若仅凭域名与路径难以确定具体内容，可依据常见站点类型合理推断，并在 note 末尾用括号标注「推测」。

只输出一个 JSON 对象，字段：title（字符串）、tags（字符串数组）、note（字符串）。不要 Markdown，不要其他字段。"#
        .to_string()
}

/// 根据链接文本调用 AI 生成标题、标签与备注（需已配置 API Key）。
pub async fn enrich_resource_from_url(
    app: &AppHandle,
    url: String,
) -> Result<ResourceUrlEnrichment, String> {
    let config = storage::load_active_ai_config(app)?;

    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("链接为空".into());
    }

    let user = format!(
        "链接：{}",
        serde_json::to_string(&url).map_err(|e| e.to_string())?
    );
    let raw = chat_completion(&config, &resource_url_system_prompt(), &user, true, 600).await?;
    let v = extract_json_object(&raw)?;

    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "模型未返回 title".to_string())?
        .to_string();

    let tags_arr = v
        .get("tags")
        .and_then(|x| x.as_array())
        .ok_or_else(|| "模型未返回 tags 数组".to_string())?;

    let mut tag_strings = Vec::new();
    for t in tags_arr {
        if let Some(s) = t.as_str() {
            tag_strings.push(s.to_string());
        } else if let Some(n) = t.as_f64() {
            tag_strings.push((n as i64).to_string());
        }
    }
    let tags = normalize_enrichment_tags(tag_strings);

    let note = v
        .get("note")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(ResourceUrlEnrichment { title, tags, note })
}

// ── 重新分类：生成新分类 + 按新分类重新归类 ──────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomCategory {
    pub slug: String,
    pub label_zh: String,
    #[serde(default)]
    pub label_en: Option<String>,
}

fn regenerate_categories_system_prompt(locale: &str) -> String {
    let preferred = if locale == "en" {
        "当前界面语言是英文；英文标签 labelEn 要自然、简短、适合 UI chip 展示（1-3 个英文单词，Title Case）。"
    } else {
        "当前界面语言是中文；中文标签 labelZh 要自然、简短、适合 UI chip 展示。"
    };
    format!(
        r#"你是 AIControls 的分类生成助手。输入是用户所有 Skill / MCP / Rule 的标题与描述信息。
请分析这些资产，总结出 5 到 8 个分类。严格要求：
1) 每个分类的中文名必须是 **恰好 2 个中文字**，例如"开发""设计""运维"；
2) 每个分类必须同时提供英文标签 labelEn（1-3 个英文单词，Title Case，例如 "Dev Tools"）；
3) 每个分类同时提供一个英文 slug（小写、用下划线连接，如 dev_tools）；
4) 分类之间互斥、覆盖尽可能全面；
5) 不用解释，直接输出 JSON。
{}

只输出一个 JSON 数组，每个元素是 {{ "slug": "xxx", "labelZh": "XX", "labelEn": "English" }}。
不要 Markdown，不要多余字段。"#,
        preferred
    )
}

fn reclassify_with_categories_system_prompt(categories: &[CustomCategory]) -> String {
    let cat_lines: Vec<String> = categories
        .iter()
        .map(|c| {
            let en = c.label_en.as_deref().unwrap_or("").trim();
            if en.is_empty() {
                format!("{} — {}", c.slug, c.label_zh)
            } else {
                format!("{} — {} / {}", c.slug, c.label_zh, en)
            }
        })
        .collect();
    format!(
        r#"你是 AIControls 的资产分类助手。输入是多条 Skill / MCP / Rule 的简要信息。
必须为每一条选出 **恰好一个** 分类 slug，只能从下列集合中选：
{}

只输出 **一个 JSON 对象**：键为每条资产的 id（字符串），值为 slug。
不要 Markdown，不要解释，不要多余字段。"#,
        cat_lines.join("\n")
    )
}

fn fallback_custom_category_slug(categories: &[CustomCategory]) -> Option<String> {
    categories
        .iter()
        .find(|c| c.label_zh.contains("其他") || c.slug.to_lowercase().contains("other"))
        .or_else(|| categories.first())
        .map(|c| c.slug.trim().to_lowercase())
        .filter(|s| !s.is_empty())
}

/// 让 AI 分析所有资产的 title/description，生成一组新的 2 字中文分类。
pub async fn regenerate_categories(
    app: &AppHandle,
    inventory: AgentInventory,
    locale: Option<String>,
) -> Result<Vec<CustomCategory>, String> {
    let config = storage::load_active_ai_config(app)?;

    let all_entries: Vec<&AssetEntry> = inventory
        .skills
        .iter()
        .chain(inventory.mcp.iter())
        .chain(inventory.rules.iter())
        .collect();

    if all_entries.is_empty() {
        return Ok(vec![]);
    }

    let mut lines = Vec::new();
    for e in &all_entries {
        lines.push(format!(
            "- title={} description={}",
            serde_json::to_string(&e.title).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.description).map_err(|e| e.to_string())?,
        ));
    }
    let user = format!(
        "请根据下列资产信息，生成新的分类方案：\n{}",
        lines.join("\n")
    );

    let raw = chat_completion(
        &config,
        &regenerate_categories_system_prompt(locale.as_deref().unwrap_or("zh")),
        &user,
        true,
        600,
    )
    .await?;

    let v = extract_json_object(&raw)?;
    let arr = v
        .as_array()
        .ok_or_else(|| "模型返回的不是 JSON 数组".to_string())?;

    let mut categories = Vec::new();
    for item in arr {
        let slug = item
            .get("slug")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let label_zh = item
            .get("labelZh")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let label_en = item
            .get("labelEn")
            .and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        if slug.is_empty() || label_zh.is_empty() {
            continue;
        }
        // 验证中文标签恰好 2 个字符
        let cjk_count = label_zh
            .chars()
            .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
            .count();
        if cjk_count == 0 {
            continue;
        }
        categories.push(CustomCategory { slug, label_zh, label_en });
    }

    if categories.is_empty() {
        return Err("AI 未返回有效的分类方案".to_string());
    }

    Ok(categories)
}

/// Fill translated labels for persisted custom categories when the UI switches language.
pub async fn translate_custom_categories(
    app: &AppHandle,
    categories: Vec<CustomCategory>,
    locale: Option<String>,
) -> Result<Vec<CustomCategory>, String> {
    let target = locale.unwrap_or_else(|| "en".to_string());
    if target != "en" {
        return Ok(categories);
    }
    if categories.iter().all(|c| c.label_en.as_deref().unwrap_or("").trim().len() > 0) {
        return Ok(categories);
    }
    let config = storage::load_active_ai_config(app)?;
    let input = serde_json::to_string(&categories).map_err(|e| e.to_string())?;
    let system = r#"You translate AIControls category labels for UI chips.
Input is a JSON array with slug and labelZh. Return the same array order with slug and labelEn.
labelEn must be natural English, 1-3 words, Title Case. Do not change slug. Output JSON only."#;
    let raw = chat_completion(&config, system, &input, true, 500).await?;
    let v = extract_json_object(&raw)?;
    let arr = v.as_array().ok_or_else(|| "模型返回的不是 JSON 数组".to_string())?;
    let mut en_by_slug: HashMap<String, String> = HashMap::new();
    for item in arr {
        let slug = item.get("slug").and_then(|x| x.as_str()).unwrap_or("").trim();
        let label_en = item.get("labelEn").and_then(|x| x.as_str()).unwrap_or("").trim();
        if !slug.is_empty() && !label_en.is_empty() {
            en_by_slug.insert(slug.to_string(), label_en.to_string());
        }
    }
    let mut out = categories;
    for c in &mut out {
        if c.label_en.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(en) = en_by_slug.get(&c.slug) {
                c.label_en = Some(en.clone());
            }
        }
    }
    storage::save_custom_categories(app, &out)?;
    Ok(out)
}

/// 使用自定义分类列表重新归类所有资产，返回 id → newSlug 映射；同时持久化到本地缓存。
pub async fn reclassify_with_new_categories(
    app: &AppHandle,
    inventory: AgentInventory,
    categories: Vec<CustomCategory>,
) -> Result<HashMap<String, String>, String> {
    let config = storage::load_active_ai_config(app)?;

    let all_entries: Vec<AssetEntry> = inventory
        .skills
        .iter()
        .chain(inventory.mcp.iter())
        .chain(inventory.rules.iter())
        .cloned()
        .collect();

    if all_entries.is_empty() {
        return Ok(HashMap::new());
    }

    let slug_set: HashSet<&str> = categories.iter().map(|c| c.slug.as_str()).collect();
    let system = reclassify_with_categories_system_prompt(&categories);

    const BATCH: usize = 12;
    let mut merged = HashMap::new();

    for chunk in all_entries.chunks(BATCH) {
        let mut lines = Vec::new();
        for e in chunk {
            lines.push(format!(
                "- id={} kind={} title={} description={}",
                serde_json::to_string(&e.id).map_err(|e| e.to_string())?,
                serde_json::to_string(&e.kind).map_err(|e| e.to_string())?,
                serde_json::to_string(&e.title).map_err(|e| e.to_string())?,
                serde_json::to_string(&e.description).map_err(|e| e.to_string())?,
            ));
        }
        let user = format!(
            "请为下列条目分类（输出 JSON 对象 id→slug）：\n{}",
            lines.join("\n")
        );

        match chat_completion(&config, &system, &user, true, 800).await {
            Ok(raw) => {
                if let Ok(v) = extract_json_object(&raw) {
                    if let Some(obj) = v.as_object() {
                        for (id, val) in obj {
                            if let Some(slug) = val.as_str() {
                                let s = slug.trim().to_lowercase();
                                if slug_set.contains(s.as_str()) {
                                    merged.insert(id.clone(), s);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // batch 失败则跳过
            }
        }
    }

    // 补全缺失条目（单条重试）
    for e in &all_entries {
        if merged.contains_key(&e.id) {
            continue;
        }
        let lines = vec![format!(
            "- id={} kind={} title={} description={}",
            serde_json::to_string(&e.id).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.kind).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.title).map_err(|e| e.to_string())?,
            serde_json::to_string(&e.description).map_err(|e| e.to_string())?,
        )];
        let user = format!(
            "请为下列条目分类（输出 JSON 对象 id→slug）：\n{}",
            lines.join("\n")
        );
        if let Ok(raw) = chat_completion(&config, &system, &user, true, 200).await {
            if let Ok(v) = extract_json_object(&raw) {
                if let Some(obj) = v.as_object() {
                    for (id, val) in obj {
                        if let Some(slug) = val.as_str() {
                            let s = slug.trim().to_lowercase();
                            if slug_set.contains(s.as_str()) {
                                merged.insert(id.clone(), s);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(fallback) = fallback_custom_category_slug(&categories) {
        for e in &all_entries {
            merged
                .entry(e.id.clone())
                .or_insert_with(|| fallback.clone());
        }
    }

    // 将新分类结果持久化到本地 scenario map
    if !merged.is_empty() {
        storage::merge_scenario_map(app, &merged)?;
        // 同时持久化自定义分类列表，后续加载时使用
        storage::save_custom_categories(app, &categories)?;
    }

    Ok(merged)
}

// ── 项目进度估算 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProjectProgressResult {
    pub progress: u32,
    pub summary: String,
}

fn progress_system_prompt() -> String {
    r#"你是一个资深的项目进度评估助手。根据给出的项目信息，评估该项目从 MVP 角度的完成进度。
评估标准：
1) 0-20%：仅有脚手架/配置文件，几乎没有业务代码；
2) 20-40%：有基本的项目结构和少量功能代码；
3) 40-60%：核心功能已初步实现，部分功能仍缺失；
4) 60-80%：主要功能完成，需要打磨和完善细节；
5) 80-95%：功能基本完整，进入测试和修复阶段；
6) 95-100%：项目已成熟，可上线或已上线。

只输出一个 JSON 对象，字段：
- progress：整数 0-100，表示完成百分比；
- summary：一句话中文评估（不超过 50 字）。
不要 Markdown，不要其他字段。"#
        .to_string()
}

pub async fn estimate_project_progress(
    app: &AppHandle,
    root: String,
) -> Result<ProjectProgressResult, String> {
    let config = storage::load_active_ai_config(app)?;

    let root_path = std::path::Path::new(root.trim());
    if !root_path.is_dir() {
        return Err("路径不是文件夹".into());
    }

    // Collect project context
    let mut context_parts: Vec<String> = Vec::new();

    // 1. package.json info
    let pkg_path = root_path.join("package.json");
    if pkg_path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&pkg_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let version = val
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.0.0");
                let desc = val
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let deps_count = val
                    .get("dependencies")
                    .and_then(|v| v.as_object())
                    .map(|o| o.len())
                    .unwrap_or(0);
                let dev_deps_count = val
                    .get("devDependencies")
                    .and_then(|v| v.as_object())
                    .map(|o| o.len())
                    .unwrap_or(0);
                let scripts_count = val
                    .get("scripts")
                    .and_then(|v| v.as_object())
                    .map(|o| o.len())
                    .unwrap_or(0);
                context_parts.push(format!(
                    "项目名称: {}, 版本: {}, 描述: {}, 依赖数: {}, 开发依赖数: {}, 脚本数: {}",
                    name, version, desc, deps_count, dev_deps_count, scripts_count
                ));
            }
        }
    }

    // 2. Top-level directory structure (first level only)
    let mut top_dirs: Vec<String> = Vec::new();
    let mut top_files: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                top_dirs.push(name);
            } else {
                top_files.push(name);
            }
        }
    }
    top_dirs.sort();
    top_files.sort();
    if !top_dirs.is_empty() {
        context_parts.push(format!("顶层目录: {}", top_dirs.join(", ")));
    }
    if !top_files.is_empty() {
        context_parts.push(format!("顶层文件: {}", top_files.join(", ")));
    }

    // 3. Key directory existence checks
    let key_dirs = [
        "src",
        "src-tauri",
        "app",
        "pages",
        "components",
        "lib",
        "test",
        "tests",
        "__tests__",
        "docs",
        "public",
        "dist",
        "build",
    ];
    let existing: Vec<&str> = key_dirs
        .iter()
        .filter(|d| root_path.join(d).is_dir())
        .copied()
        .collect();
    if !existing.is_empty() {
        context_parts.push(format!("关键目录存在: {}", existing.join(", ")));
    }

    // 4. README existence
    if root_path.join("README.md").is_file() || root_path.join("README").is_file() {
        context_parts.push("README: 存在".to_string());
    } else {
        context_parts.push("README: 不存在".to_string());
    }

    // 5. Code line counts (from tokei)
    if let Ok(stats) = crate::code_metrics::count_code_lines(root_path) {
        context_parts.push(format!(
            "代码统计: 总行数 {}, 代码行 {}, 注释行 {}, 空行 {}, 文件数 {}, 语言: {}",
            stats.total_lines,
            stats.code_lines,
            stats.comment_lines,
            stats.blank_lines,
            stats.files,
            stats
                .languages
                .iter()
                .take(5)
                .map(|l| format!("{}({})", l.language, l.code_lines))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // 6. Git info
    if root_path.join(".git").is_dir() {
        let commit_count = crate::git_command_output(root_path, &["rev-list", "--count", "HEAD"])
            .and_then(|s| s.parse::<u32>().ok())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "未知".to_string());
        let branch = crate::git_command_output(root_path, &["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string());
        context_parts.push(format!("Git: {} 次提交, 当前分支 {}", commit_count, branch));
    }

    if context_parts.is_empty() {
        return Ok(ProjectProgressResult {
            progress: 5,
            summary: "项目目录为空或无法读取".to_string(),
        });
    }

    let user = format!(
        "请评估以下项目的 MVP 完成进度：\n{}",
        context_parts.join("\n")
    );

    let raw = chat_completion(&config, &progress_system_prompt(), &user, true, 300).await?;
    let v = extract_json_object(&raw)?;

    let progress = v
        .get("progress")
        .and_then(|x| x.as_u64())
        .unwrap_or(50)
        .min(100) as u32;

    let summary = v
        .get("summary")
        .and_then(|x| x.as_str())
        .unwrap_or("无法评估")
        .trim()
        .to_string();

    Ok(ProjectProgressResult { progress, summary })
}
