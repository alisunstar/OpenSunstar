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

pub(crate) fn prepare_sync_test_home(name: &str) -> TempDir {
    let home = tempdir().expect("create sync test home");
    disable_proxy_for_local_mock_servers();
    std::env::set_var("OPEN_SUNSTAR_TEST_HOME", home.path());
    std::env::set_var("HOME", home.path());
    #[cfg(windows)]
    std::env::set_var("USERPROFILE", home.path());

    crate::settings::update_settings(crate::settings::AppSettings::default())
        .expect("reset settings");

    let skills_dir =
        crate::services::skill::SkillService::get_ssot_dir().expect("create skills ssot dir");
    std::fs::write(
        skills_dir.join(format!("{name}.md")),
        format!("# {name}\n\nsync e2e test skill\n"),
    )
    .expect("write sync e2e skill");
    home
}

fn disable_proxy_for_local_mock_servers() {
    for key in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        std::env::remove_var(key);
    }
    std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
    std::env::set_var("no_proxy", "127.0.0.1,localhost");
    let _ = crate::proxy::http_client::init(None);
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
