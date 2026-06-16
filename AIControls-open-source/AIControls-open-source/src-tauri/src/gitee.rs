//! Gitee OAuth2（授权码）与仓库文件备份。

use crate::storage::{self, GiteeBackupFingerprint, GiteeTokenFile};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

static OAUTH_IN_PROGRESS: Mutex<bool> = Mutex::new(false);

const PROMPT_FILE: &str = "prompt-library.json";
const RESOURCE_FILE: &str = "resource-library.json";
const ALL_SCENARIOS_FILE: &str = "asset_scenarios.json";
const ALL_BRIEFS_FILE: &str = "asset_briefs_zh.json";
const BACKUP_PREFIX: &str = "aicontrols-data";

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("AIControls/0.1 (Gitee backup)")
        .timeout(Duration::from_secs(60))
        .build()
        .expect("reqwest client")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// --- 首页展示的定时备份倒计时与上次结果 ---

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GiteeSyncStatusPublic {
    pub connected: bool,
    /// 预计触发自动检查的时间（Unix 毫秒）。倒计时用本地时钟与该值差值即可。
    pub next_auto_check_ms: i64,
    pub last_backup_ms: Option<i64>,
    pub last_message: Option<String>,
    /// 最近一次备份是否成功；`None` 表示尚无记录。
    pub last_ok: Option<bool>,
}

#[derive(Clone, Default)]
struct GiteeSyncUiInner {
    next_auto_check_ms: i64,
    last_backup_ms: Option<i64>,
    last_message: Option<String>,
    last_ok: Option<bool>,
}

fn sync_ui_cell() -> &'static Mutex<GiteeSyncUiInner> {
    static CELL: OnceLock<Mutex<GiteeSyncUiInner>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(GiteeSyncUiInner::default()))
}

/// 在休眠开始前调用：设置「下一次自动检查」的预计时间戳。
pub fn sync_ui_schedule_next_in_secs(secs: u64) {
    if let Ok(mut g) = sync_ui_cell().lock() {
        g.next_auto_check_ms = now_ms() + (secs as i64) * 1000;
    }
}

fn sync_ui_record_backup_result(result: &Result<String, String>, at_ms: i64) {
    if let Ok(mut g) = sync_ui_cell().lock() {
        g.last_backup_ms = Some(at_ms);
        match result {
            Ok(msg) => {
                g.last_ok = Some(true);
                g.last_message = Some(msg.clone());
            }
            Err(e) => {
                g.last_ok = Some(false);
                g.last_message = Some(e.clone());
            }
        }
    }
}

pub fn get_gitee_sync_status(app: &AppHandle) -> Result<GiteeSyncStatusPublic, String> {
    let connected = storage::load_gitee_token(app)?.is_some();
    let inner = sync_ui_cell()
        .lock()
        .map_err(|_| "同步状态锁异常".to_string())?;
    Ok(GiteeSyncStatusPublic {
        connected,
        next_auto_check_ms: inner.next_auto_check_ms,
        last_backup_ms: inner.last_backup_ms,
        last_message: inner.last_message.clone(),
        last_ok: inner.last_ok,
    })
}

fn sync_ui_on_disconnect() {
    if let Ok(mut g) = sync_ui_cell().lock() {
        g.last_message = Some("已解除 Gitee 授权".into());
        g.last_ok = Some(false);
        g.next_auto_check_ms = now_ms() + 300_000;
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let out = Sha256::digest(bytes);
    out.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_file_if_exists(path: &Path) -> Result<Option<String>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(|e| format!("读取文件失败：{e}"))?;
    Ok(Some(sha256_hex(&bytes)))
}

fn local_backup_fingerprint(app: &AppHandle) -> Result<GiteeBackupFingerprint, String> {
    let dir = storage::app_local_dir(app)?;
    Ok(GiteeBackupFingerprint {
        prompt_sha256: hash_file_if_exists(&dir.join(PROMPT_FILE))?,
        resource_sha256: hash_file_if_exists(&dir.join(RESOURCE_FILE))?,
        all_scenarios_sha256: hash_file_if_exists(&dir.join(ALL_SCENARIOS_FILE))?,
        all_briefs_sha256: hash_file_if_exists(&dir.join(ALL_BRIEFS_FILE))?,
    })
}

fn parse_query_string(q: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for pair in q.split('&') {
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("").to_string();
        let v = it.next().unwrap_or("").to_string();
        if k.is_empty() {
            continue;
        }
        let dk = urlencoding::decode(&k).map(|c| c.into_owned()).unwrap_or(k);
        let dv = urlencoding::decode(&v).map(|c| c.into_owned()).unwrap_or(v);
        m.insert(dk, dv);
    }
    m
}

fn parse_oauth_from_http(buf: &[u8]) -> Option<HashMap<String, String>> {
    let s = std::str::from_utf8(buf).ok()?;
    let line = s.lines().next()?;
    let rest = line.strip_prefix("GET ")?;
    let path = rest.split_whitespace().next()?;
    let q = path.find('?').map(|i| &path[i + 1..]).unwrap_or("");
    Some(parse_query_string(q))
}

fn write_http_ok_close(stream: &mut TcpStream, body: &str) {
    let len = body.as_bytes().len();
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        len, body
    );
    let _ = stream.flush();
}

fn write_http_plain_close(stream: &mut TcpStream, status: u16, msg: &str) {
    let body = format!("<html><body><p>{}</p></body></html>", msg);
    let len = body.as_bytes().len();
    let _ = write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        if status == 400 { "Bad Request" } else { "OK" },
        len,
        body
    );
    let _ = stream.flush();
}

fn contents_path_segments(repo_path: &str) -> String {
    repo_path
        .split('/')
        .map(|s| urlencoding::encode(s).into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// 在子线程中：绑定端口 → 通知就绪 → 循环 accept 直到拿到 code 或错误。
fn oauth_server_thread(
    state_expected: String,
    ready_tx: mpsc::Sender<Result<(), String>>,
    code_tx: mpsc::Sender<Result<String, String>>,
) {
    let listener = match TcpListener::bind("127.0.0.1:19876") {
        Ok(l) => l,
        Err(e) => {
            let _ = ready_tx.send(Err(format!(
                "无法绑定本机 19876 端口（请检查是否被占用）：{e}"
            )));
            return;
        }
    };
    if let Err(e) = listener.set_nonblocking(false) {
        let _ = ready_tx.send(Err(format!("设置监听失败：{e}")));
        return;
    }
    if ready_tx.send(Ok(())).is_err() {
        return;
    }

    for _ in 0..48u32 {
        let Ok((mut stream, _)) = listener.accept() else {
            continue;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
        let mut buf = [0u8; 8192];
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let Some(q) = parse_oauth_from_http(&buf[..n]) else {
            write_http_plain_close(&mut stream, 400, "无法解析请求");
            continue;
        };

        if let Some(err) = q.get("error").filter(|e| !e.is_empty()) {
            let desc = q.get("error_description").cloned().unwrap_or_default();
            write_http_ok_close(
                &mut stream,
                "<html><body><p>授权未完成，可关闭此页。</p></body></html>",
            );
            let _ = code_tx.send(Err(format!("Gitee 返回错误：{err} {desc}")));
            return;
        }

        let st = q.get("state").cloned().unwrap_or_default();
        if st != state_expected {
            write_http_plain_close(&mut stream, 400, "state 不匹配");
            continue;
        }

        let Some(code) = q.get("code").cloned().filter(|c| !c.is_empty()) else {
            write_http_plain_close(&mut stream, 400, "缺少 code");
            continue;
        };

        write_http_ok_close(
            &mut stream,
            "<html><body><p>授权成功，请返回 AIControls。</p></body></html>",
        );
        let _ = code_tx.send(Ok(code));
        return;
    }
    let _ = code_tx.send(Err("授权回调无效请求过多，已中止。".into()));
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

async fn exchange_code_for_token(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, String> {
    let client = http_client();
    let body = format!(
        "grant_type=authorization_code&code={}&client_id={}&redirect_uri={}&client_secret={}",
        urlencoding::encode(code),
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(client_secret),
    );
    let res = client
        .post("https://gitee.com/oauth/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("请求 token 失败：{e}"))?;

    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("读取 token 响应失败：{e}"))?;
    if !status.is_success() {
        return Err(format!("换取 token 失败（HTTP {status}）：{text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("解析 token JSON 失败：{e}，原文：{text}"))
}

async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<TokenResponse, String> {
    let client = http_client();
    let body = format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}&client_secret={}",
        urlencoding::encode(refresh_token),
        urlencoding::encode(client_id),
        urlencoding::encode(client_secret),
    );
    let res = client
        .post("https://gitee.com/oauth/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("刷新 token 请求失败：{e}"))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("读取刷新响应失败：{e}"))?;
    if !status.is_success() {
        return Err(format!("刷新 token 失败（HTTP {status}）：{text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("解析刷新 JSON 失败：{e}，原文：{text}"))
}

#[derive(Debug, Deserialize)]
struct GiteeUser {
    login: String,
}

async fn fetch_gitee_user(access_token: &str) -> Result<GiteeUser, String> {
    let client = http_client();
    let res = client
        .get("https://gitee.com/api/v5/user")
        .header("Authorization", format!("token {access_token}"))
        .send()
        .await
        .map_err(|e| format!("获取用户信息失败：{e}"))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("读取用户信息失败：{e}"))?;
    if !status.is_success() {
        return Err(format!("获取用户信息失败（HTTP {status}）：{text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("解析用户信息失败：{e}"))
}

#[derive(Debug, Deserialize)]
struct GiteeRepo {
    #[serde(default)]
    default_branch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GiteeContentFile {
    #[serde(default)]
    content: String,
    #[serde(default)]
    encoding: String,
}

async fn get_repo(access_token: &str, owner: &str, repo: &str) -> Result<GiteeRepo, String> {
    let client = http_client();
    let url = format!("https://gitee.com/api/v5/repos/{owner}/{repo}");
    let res = client
        .get(&url)
        .header("Authorization", format!("token {access_token}"))
        .send()
        .await
        .map_err(|e| format!("查询仓库失败：{e}"))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("读取仓库信息失败：{e}"))?;
    if !status.is_success() {
        return Err(format!("查询仓库失败（HTTP {status}）：{text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("解析仓库信息失败：{e}"))
}

async fn create_user_repo(
    access_token: &str,
    repo_name: &str,
    description: &str,
) -> Result<(), String> {
    let client = http_client();
    let body = json!({
        "name": repo_name,
        "description": description,
        "private": true,
        "auto_init": true,
    });
    let res = client
        .post("https://gitee.com/api/v5/user/repos")
        .header("Authorization", format!("token {access_token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("创建仓库请求失败：{e}"))?;
    let status = res.status();
    if status.is_success() {
        return Ok(());
    }
    let text = res.text().await.unwrap_or_default();
    if text.contains("已经存在")
        || text.contains("already exists")
        || (status.as_u16() == 400 && text.contains("name"))
        || status.as_u16() == 422
    {
        return Ok(());
    }
    Err(format!("创建仓库失败（HTTP {status}）：{text}"))
}

async fn ensure_repo_ready(
    access_token: &str,
    owner: &str,
    repo_name: &str,
) -> Result<String, String> {
    create_user_repo(access_token, repo_name, "由 AIControls 自动备份创建").await?;
    let r = get_repo(access_token, owner, repo_name).await?;
    Ok(r.default_branch.unwrap_or_else(|| "master".to_string()))
}

async fn get_file_sha(
    access_token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    branch: &str,
) -> Result<Option<String>, String> {
    let client = http_client();
    let enc = contents_path_segments(path);
    let url = format!("https://gitee.com/api/v5/repos/{owner}/{repo}/contents/{enc}");
    let res = client
        .get(&url)
        .query(&[("ref", branch)])
        .header("Authorization", format!("token {access_token}"))
        .send()
        .await
        .map_err(|e| format!("读取远端文件失败：{e}"))?;
    if res.status() == 404 {
        return Ok(None);
    }
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("读取文件元数据失败：{e}"))?;
    if !status.is_success() {
        return Err(format!("读取远端文件失败（HTTP {status}）：{text}"));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("解析文件元数据失败：{e}"))?;
    let sha = v.get("sha").and_then(|x| x.as_str()).map(|s| s.to_string());
    Ok(sha)
}

async fn get_repo_file_bytes(
    access_token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    branch: &str,
) -> Result<Option<Vec<u8>>, String> {
    let client = http_client();
    let enc = contents_path_segments(path);
    let url = format!("https://gitee.com/api/v5/repos/{owner}/{repo}/contents/{enc}");
    let res = client
        .get(&url)
        .query(&[("ref", branch)])
        .header("Authorization", format!("token {access_token}"))
        .send()
        .await
        .map_err(|e| format!("读取远端文件失败：{e}"))?;
    if res.status() == 404 {
        return Ok(None);
    }
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("读取远端文件响应失败：{e}"))?;
    if !status.is_success() {
        return Err(format!("读取远端文件失败（HTTP {status}）：{text}"));
    }
    let f: GiteeContentFile =
        serde_json::from_str(&text).map_err(|e| format!("解析远端文件响应失败：{e}"))?;
    let enc = f.encoding.trim().to_lowercase();
    if enc != "base64" {
        return Err(format!("远端文件编码不支持：{enc}"));
    }
    let clean = f.content.replace('\n', "").replace('\r', "");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(clean.as_bytes())
        .map_err(|e| format!("解码远端文件 base64 失败：{e}"))?;
    Ok(Some(bytes))
}

fn parse_repo_url(input: &str) -> Result<(String, String, Option<String>, Option<String>), String> {
    let mut s = input.trim().to_string();
    if s.is_empty() {
        return Err("仓库地址为空。".into());
    }
    if let Some(x) = s.strip_prefix("https://") {
        s = x.to_string();
    } else if let Some(x) = s.strip_prefix("http://") {
        s = x.to_string();
    }
    s = s.trim_end_matches('/').to_string();
    let Some(rest) = s.strip_prefix("gitee.com/") else {
        return Err("仅支持 gitee.com 仓库地址。".into());
    };
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 2 {
        return Err("仓库地址格式无效，应类似 https://gitee.com/<owner>/<repo>".into());
    }
    let owner = parts[0].trim().to_string();
    let repo = parts[1].trim().to_string();
    if owner.is_empty() || repo.is_empty() {
        return Err("仓库地址格式无效：owner/repo 不能为空。".into());
    }

    // 支持 tree URL: /<owner>/<repo>/tree/<branch>/<path...>
    let (branch, base_path) = if parts.len() >= 5 && parts[2] == "tree" {
        let b = parts[3].trim().to_string();
        let p = parts[4..].join("/");
        (
            if b.is_empty() { None } else { Some(b) },
            if p.trim().is_empty() { None } else { Some(p) },
        )
    } else {
        (None, None)
    };
    Ok((owner, repo, branch, base_path))
}

/// Gitee：新建文件用 **POST**，更新已有文件用 **PUT** 且必须带远端 `sha`。
/// 对「仅 PUT」且不带 `sha` 会返回 400：`sha is missing` / `sha is empty`。
async fn create_or_update_repo_file(
    access_token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    branch: &str,
    message: &str,
    bytes: &[u8],
    sha_opt: Option<&str>,
) -> Result<(), String> {
    let client = http_client();
    let enc = contents_path_segments(path);
    let url = format!("https://gitee.com/api/v5/repos/{owner}/{repo}/contents/{enc}");
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);

    let res = if let Some(sha) = sha_opt.map(str::trim).filter(|s| !s.is_empty()) {
        let body = json!({
            "message": message,
            "content": b64,
            "branch": branch,
            "sha": sha,
        });
        client
            .put(&url)
            .header("Authorization", format!("token {access_token}"))
            .json(&body)
            .send()
            .await
    } else {
        let body = json!({
            "message": message,
            "content": b64,
            "branch": branch,
        });
        client
            .post(&url)
            .header("Authorization", format!("token {access_token}"))
            .json(&body)
            .send()
            .await
    }
    .map_err(|e| format!("上传文件失败：{e}"))?;

    let status = res.status();
    if status.is_success() {
        return Ok(());
    }
    let text = res.text().await.unwrap_or_default();
    Err(format!("上传文件失败（HTTP {status}）：{text}"))
}

fn token_expired(token: &GiteeTokenFile) -> bool {
    match token.expires_at_ms {
        Some(t) => now_ms() > t - 120_000,
        None => false,
    }
}

async fn ensure_fresh_access_token(app: &AppHandle) -> Result<GiteeTokenFile, String> {
    let app_cfg = storage::load_gitee_app(app)?;
    if app_cfg.client_id.trim().is_empty() || app_cfg.client_secret.trim().is_empty() {
        return Err("请先保存 Gitee 应用的 Client ID 与 Secret。".into());
    }
    let mut t = storage::load_gitee_token(app)?
        .ok_or_else(|| "尚未完成 Gitee 授权，请先在下方点击「在 Gitee 授权」。".to_string())?;

    if !token_expired(&t) {
        return Ok(t);
    }
    let rt = t.refresh_token.trim();
    if rt.is_empty() {
        return Err("访问令牌已过期且无 refresh_token，请重新授权。".into());
    }
    let tr =
        refresh_access_token(app_cfg.client_id.trim(), app_cfg.client_secret.trim(), rt).await?;
    let expires_at_ms = tr.expires_in.map(|sec| now_ms() + (sec as i64) * 1000);
    t.access_token = tr.access_token;
    if let Some(nr) = tr.refresh_token {
        if !nr.is_empty() {
            t.refresh_token = nr;
        }
    }
    t.expires_at_ms = expires_at_ms;
    storage::save_gitee_token(app, &t)?;
    Ok(t)
}

pub async fn oauth_login(app: AppHandle) -> Result<String, String> {
    {
        let mut lock = OAUTH_IN_PROGRESS
            .lock()
            .map_err(|_| "内部锁错误".to_string())?;
        if *lock {
            return Err("已有授权流程在进行中，请稍候。".into());
        }
        *lock = true;
    }

    let result = async {
        let app_cfg = storage::load_gitee_app(&app)?;
        if app_cfg.client_id.trim().is_empty() || app_cfg.client_secret.trim().is_empty() {
            return Err("请先填写并保存 Client ID 与 Client Secret。".into());
        }
        let redirect_uri = storage::gitee_oauth_callback_url();
        let state = uuid::Uuid::new_v4().to_string();
        let auth_url = format!(
            "https://gitee.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            urlencoding::encode(app_cfg.client_id.trim()),
            urlencoding::encode(redirect_uri),
            urlencoding::encode("user_info projects"),
            urlencoding::encode(&state),
        );

        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let (code_tx, code_rx) = mpsc::channel::<Result<String, String>>();
        let state_thread = state.clone();
        let server = std::thread::spawn(move || oauth_server_thread(state_thread, ready_tx, code_tx));

        let bind_ok = ready_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "等待本机授权服务启动超时。".to_string())?;
        bind_ok?;

        open::that(&auth_url).map_err(|e| format!("无法打开浏览器：{e}"))?;

        let code = code_rx
            .recv_timeout(Duration::from_secs(120))
            .map_err(|_| "等待 Gitee 授权超时（120 秒内未完成）。".to_string())??;

        let _ = server.join();

        let tr = exchange_code_for_token(
            app_cfg.client_id.trim(),
            app_cfg.client_secret.trim(),
            &code,
            redirect_uri,
        )
        .await?;

        let user = fetch_gitee_user(&tr.access_token).await?;
        let repo_name = if app_cfg.repo_name.trim().is_empty() {
            "aicontrols-backup".to_string()
        } else {
            app_cfg.repo_name.trim().to_string()
        };

        let branch = ensure_repo_ready(&tr.access_token, &user.login, &repo_name).await?;

        let expires_at_ms = tr.expires_in.map(|sec| now_ms() + (sec as i64) * 1000);
        let token_file = GiteeTokenFile {
            access_token: tr.access_token,
            refresh_token: tr.refresh_token.unwrap_or_default(),
            expires_at_ms,
            owner_login: user.login.clone(),
            repo_name: repo_name.clone(),
        };
        storage::save_gitee_token(&app, &token_file)?;

        let base_msg = format!(
            "已授权为 Gitee 用户「{}」，备份仓库「{}」，默认分支「{}」。",
            user.login, repo_name, branch
        );
        match backup_now(app.clone(), true).await {
            Ok(b) => Ok(format!("{base_msg}\n{b}")),
            Err(e) => Ok(format!("{base_msg}\n首次云端备份失败：{e}")),
        }
    }
    .await;

    if let Ok(mut lock) = OAUTH_IN_PROGRESS.lock() {
        *lock = false;
    }
    result
}

/// `force == true`：忽略本地指纹，始终上传（用于授权后首次备份与用户手动备份）。
pub async fn backup_now(app: AppHandle, force: bool) -> Result<String, String> {
    let out = backup_now_inner(app, force).await;
    sync_ui_record_backup_result(&out, now_ms());
    out
}

pub async fn restore_from_repo_url(app: AppHandle, repo_url: String) -> Result<String, String> {
    let token = ensure_fresh_access_token(&app).await?;
    let (owner, repo, branch_from_url, base_path_from_url) = parse_repo_url(&repo_url)?;
    let branch = if let Some(b) = branch_from_url {
        b
    } else {
        get_repo(&token.access_token, &owner, &repo)
            .await?
            .default_branch
            .unwrap_or_else(|| "master".to_string())
    };
    let base_path = base_path_from_url.unwrap_or_else(|| BACKUP_PREFIX.to_string());
    let base_path = base_path.trim_matches('/').to_string();
    let local_dir = storage::app_local_dir(&app)?;

    let mut restored = 0usize;
    let mut missing = 0usize;
    for (fname, _label) in [
        (PROMPT_FILE, "提示词库"),
        (RESOURCE_FILE, "资源库"),
        (ALL_SCENARIOS_FILE, "全部-场景分类缓存"),
        (ALL_BRIEFS_FILE, "全部-简介缓存"),
    ] {
        let remote_path = if base_path.is_empty() {
            fname.to_string()
        } else {
            format!("{base_path}/{fname}")
        };
        let maybe =
            get_repo_file_bytes(&token.access_token, &owner, &repo, &remote_path, &branch).await?;
        if let Some(bytes) = maybe {
            let local = local_dir.join(fname);
            std::fs::write(local, bytes).map_err(|e| format!("写入本地文件失败：{e}"))?;
            restored += 1;
        } else {
            missing += 1;
        }
    }

    let fp = local_backup_fingerprint(&app)?;
    storage::save_gitee_backup_fingerprint(&app, &fp)?;

    Ok(format!(
        "已从 {owner}/{repo} 的 `{}` 载入 {} 个文件（缺失 {} 个）。",
        if base_path.is_empty() {
            "."
        } else {
            &base_path
        },
        restored,
        missing
    ))
}

async fn backup_now_inner(app: AppHandle, force: bool) -> Result<String, String> {
    let fp_now = local_backup_fingerprint(&app)?;
    if !force {
        if let Some(prev) = storage::load_gitee_backup_fingerprint(&app)? {
            if prev == fp_now {
                return Ok("本地数据无变化，未执行备份。".into());
            }
        }
    }

    let token = ensure_fresh_access_token(&app).await?;
    let owner = token.owner_login.trim();
    let repo = token.repo_name.trim();
    if owner.is_empty() || repo.is_empty() {
        return Err("授权信息不完整，请重新授权。".into());
    }

    let dir = storage::app_local_dir(&app)?;
    let branch = get_repo(&token.access_token, owner, repo)
        .await?
        .default_branch
        .unwrap_or_else(|| "master".to_string());

    let mut uploaded = 0usize;
    let mut skipped = 0usize;
    for (fname, label) in [
        (PROMPT_FILE, "提示词库"),
        (RESOURCE_FILE, "资源库"),
        (ALL_SCENARIOS_FILE, "全部-场景分类缓存"),
        (ALL_BRIEFS_FILE, "全部-简介缓存"),
    ] {
        let path = dir.join(fname);
        if !path.is_file() {
            skipped += 1;
            let _ = label;
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("读取{label}失败：{e}"))?;
        let remote_path = format!("{BACKUP_PREFIX}/{fname}");
        let sha = get_file_sha(&token.access_token, owner, repo, &remote_path, &branch).await?;
        let msg = format!("backup: {fname} @ {}", now_ms());
        create_or_update_repo_file(
            &token.access_token,
            owner,
            repo,
            &remote_path,
            &branch,
            &msg,
            &bytes,
            sha.as_deref(),
        )
        .await?;
        uploaded += 1;
    }

    let readme_path = format!("{BACKUP_PREFIX}/README.md");
    let tracked_list = format!(
        "- `{PROMPT_FILE}`\n- `{RESOURCE_FILE}`\n- `{ALL_SCENARIOS_FILE}`\n- `{ALL_BRIEFS_FILE}`"
    );
    let list = if skipped == 0 {
        tracked_list
    } else {
        format!(
            "{tracked_list}\n\n- 本次实际上传 {uploaded} 个文件（本地缺失已跳过 {skipped} 个）。"
        )
    };
    let readme = format!(
        "# AIControls 备份\n\n本目录由 AIControls 客户端同步。\n\n{list}\n\n最近同步（客户端本地时间戳 ms）：{}。\n",
        now_ms()
    );
    let sha = get_file_sha(&token.access_token, owner, repo, &readme_path, &branch).await?;
    create_or_update_repo_file(
        &token.access_token,
        owner,
        repo,
        &readme_path,
        &branch,
        "backup: README",
        readme.as_bytes(),
        sha.as_deref(),
    )
    .await?;

    storage::save_gitee_backup_fingerprint(&app, &fp_now)?;

    Ok(format!(
        "已上传到 {}/{}/{}（数据文件 {uploaded} 个，跳过缺失 {skipped} 个，含 README）。",
        owner, repo, BACKUP_PREFIX
    ))
}

pub fn disconnect(app: &AppHandle) -> Result<(), String> {
    storage::clear_gitee_token(app)?;
    sync_ui_on_disconnect();
    Ok(())
}

/// 定时任务：仅在有变更时上传；未授权或出错时安静忽略。
pub async fn backup_periodic_tick(app: AppHandle) {
    let _ = backup_now(app, false).await;
}
