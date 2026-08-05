use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Router;
use tempfile::{tempdir, TempDir};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

#[derive(Clone, Default)]
pub(crate) struct MockObjectStore {
    objects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MockObjectStore {
    pub(crate) fn put(&self, key: impl Into<String>, bytes: Vec<u8>) {
        self.objects
            .lock()
            .expect("lock mock object store")
            .insert(key.into(), bytes);
    }

    pub(crate) fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.objects
            .lock()
            .expect("lock mock object store")
            .get(key)
            .cloned()
    }
}

pub(crate) struct MockObjectServer {
    pub(crate) base_url: String,
    pub(crate) store: MockObjectStore,
    handle: JoinHandle<()>,
}

impl Drop for MockObjectServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub(crate) async fn start_object_store_server() -> MockObjectServer {
    let store = MockObjectStore::default();
    let app = Router::new()
        .fallback(mock_object_store_handler)
        .with_state(store.clone());
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock object server");
    let addr = listener.local_addr().expect("mock object server addr");
    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("mock object server failed: {err}");
        }
    });

    MockObjectServer {
        base_url: format!("http://{addr}"),
        store,
        handle,
    }
}

async fn mock_object_store_handler(
    State(store): State<MockObjectStore>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    let key = uri.path().trim_start_matches('/').trim_end_matches('/');

    match method {
        Method::PUT => {
            store.put(key.to_string(), body.to_vec());
            StatusCode::CREATED.into_response()
        }
        Method::GET => match store.get(key) {
            Some(bytes) => response_with_etag(StatusCode::OK, bytes),
            None => StatusCode::NOT_FOUND.into_response(),
        },
        Method::HEAD => match store.get(key) {
            Some(_) => response_with_etag(StatusCode::OK, Vec::new()),
            None => StatusCode::NOT_FOUND.into_response(),
        },
        _ if method.as_str() == "MKCOL" => StatusCode::CREATED.into_response(),
        _ if method.as_str() == "PROPFIND" => StatusCode::MULTI_STATUS.into_response(),
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

fn response_with_etag(status: StatusCode, bytes: Vec<u8>) -> Response {
    let mut response = bytes.into_response();
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert("etag", HeaderValue::from_static("\"mock-etag\""));
    response
}

pub(crate) fn sync_e2e_async_mutex() -> &'static tokio::sync::Mutex<()> {
    static MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// 进程级测试 env 全局锁：所有改 home/proxy 环境变量的测试必须先持有它，
/// 并在 Drop 时还原。此前各测试族使用互不相通的锁（模块锁 / tokio 异步锁 /
/// #[serial]），并行运行时对同一进程 env 竞态，是 sync/webdav/s3 一族 flake 的根因。
pub(crate) fn sync_env_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

fn restore_env(key: &str, prev: Option<std::ffi::OsString>) {
    match prev {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}

/// home 隔离守卫：持有全局 env 锁 + 临时目录；Drop 时还原全部被改 env。
pub(crate) struct SyncHomeGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    _dir: TempDir,
    original_home: Option<std::ffi::OsString>,
    #[cfg(windows)]
    original_userprofile: Option<std::ffi::OsString>,
    original_test_home: Option<std::ffi::OsString>,
    original_no_proxy: Option<std::ffi::OsString>,
    original_no_proxy_lc: Option<std::ffi::OsString>,
    removed_proxy_vars: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl Drop for SyncHomeGuard {
    fn drop(&mut self) {
        restore_env("OPEN_SUNSTAR_TEST_HOME", self.original_test_home.take());
        #[cfg(windows)]
        restore_env("USERPROFILE", self.original_userprofile.take());
        restore_env("HOME", self.original_home.take());
        restore_env("no_proxy", self.original_no_proxy_lc.take());
        restore_env("NO_PROXY", self.original_no_proxy.take());
        for (key, prev) in std::mem::take(&mut self.removed_proxy_vars) {
            restore_env(key, prev);
        }
    }
}

pub(crate) fn prepare_sync_test_home(name: &str) -> SyncHomeGuard {
    let lock = sync_env_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let home = tempdir().expect("create sync test home");
    let original_home = std::env::var_os("HOME");
    #[cfg(windows)]
    let original_userprofile = std::env::var_os("USERPROFILE");
    let original_test_home = std::env::var_os("OPEN_SUNSTAR_TEST_HOME");
    let original_no_proxy = std::env::var_os("NO_PROXY");
    let original_no_proxy_lc = std::env::var_os("no_proxy");
    let removed_proxy_vars = [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ]
    .into_iter()
    .map(|key| {
        let prev = std::env::var_os(key);
        std::env::remove_var(key);
        (key, prev)
    })
    .collect();

    std::env::set_var("OPEN_SUNSTAR_TEST_HOME", home.path());
    std::env::set_var("HOME", home.path());
    #[cfg(windows)]
    std::env::set_var("USERPROFILE", home.path());
    std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
    std::env::set_var("no_proxy", "127.0.0.1,localhost");
    let _ = crate::proxy::http_client::init(None);

    crate::settings::update_settings(crate::settings::AppSettings::default())
        .expect("reset settings");

    let skills_dir =
        crate::services::skill::SkillService::get_ssot_dir().expect("create skills ssot dir");
    std::fs::write(
        skills_dir.join(format!("{name}.md")),
        format!("# {name}\n\nsync e2e test skill\n"),
    )
    .expect("write sync e2e skill");

    SyncHomeGuard {
        _lock: lock,
        _dir: home,
        original_home,
        #[cfg(windows)]
        original_userprofile,
        original_test_home,
        original_no_proxy,
        original_no_proxy_lc,
        removed_proxy_vars,
    }
}

pub(crate) fn seeded_memory_db(marker_value: &str) -> crate::database::Database {
    let db = crate::database::Database::memory().expect("memory db");
    db.init_default_official_providers()
        .expect("seed official providers");
    db.set_setting("sync_roundtrip_marker", marker_value)
        .expect("write marker setting");
    db
}

pub(crate) fn assert_marker(db: &crate::database::Database, expected: &str) {
    assert_eq!(
        db.get_setting("sync_roundtrip_marker")
            .expect("read marker setting")
            .as_deref(),
        Some(expected)
    );
}
