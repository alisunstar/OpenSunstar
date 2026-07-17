use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn os_bin() -> &'static str {
    env!("CARGO_BIN_EXE_os")
}

fn isolated_home() -> TempDir {
    tempfile::tempdir().expect("create isolated CLI home")
}

fn os_command(home: &TempDir) -> Command {
    let mut cmd = Command::new(os_bin());
    cmd.env("OPEN_SUNSTAR_TEST_HOME", home.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("NO_COLOR", "1")
        .env_remove("HTTP_PROXY")
        .env_remove("http_proxy")
        .env_remove("HTTPS_PROXY")
        .env_remove("https_proxy")
        .env_remove("ALL_PROXY")
        .env_remove("all_proxy")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost");
    cmd
}

fn assert_success_json(output: Output) -> Value {
    assert!(
        output.status.success(),
        "command failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "stdout was not valid JSON: {err}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn os_cli_emits_machine_readable_version_and_config_path() {
    let home = isolated_home();

    let version = assert_success_json(
        os_command(&home)
            .args(["--json", "version"])
            .output()
            .expect("run os version"),
    );
    assert!(version.get("appVersion").and_then(Value::as_str).is_some());
    assert!(version
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .is_some());

    let path = assert_success_json(
        os_command(&home)
            .args(["--json", "config", "path"])
            .output()
            .expect("run os config path"),
    );
    let config_dir = path
        .get("config_dir")
        .and_then(Value::as_str)
        .expect("config_dir field");
    assert!(
        config_dir.contains(".OpenSunstar"),
        "unexpected config dir: {config_dir}"
    );
}

#[test]
fn os_doctor_json_bootstraps_and_reports_database_status() {
    let home = isolated_home();

    let bootstrap = assert_success_json(
        os_command(&home)
            .args(["--json", "doctor", "--init"])
            .output()
            .expect("run os doctor --init"),
    );
    assert_eq!(
        bootstrap.get("created").and_then(Value::as_bool),
        Some(true)
    );
    let db_path = bootstrap
        .get("db_path")
        .and_then(Value::as_str)
        .expect("db_path field");
    assert!(
        std::path::Path::new(db_path).exists(),
        "database should exist"
    );

    let report = assert_success_json(
        os_command(&home)
            .args(["--json", "doctor"])
            .output()
            .expect("run os doctor"),
    );
    assert!(matches!(
        report.get("status").and_then(Value::as_str),
        Some("ok" | "issues")
    ));
    assert_eq!(
        report.pointer("/database/exists").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        report
            .pointer("/database/readable")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(report.get("tools").and_then(Value::as_array).is_some());
}

#[test]
fn arbitrary_repository_can_be_inspected_then_adopted_idempotently() {
    let home = isolated_home();
    let repository = tempfile::tempdir().expect("create repository");
    std::fs::write(
        repository.path().join("AGENTS.md"),
        "# Agent instructions\n",
    )
    .expect("write repository marker");
    let path = repository.path().to_string_lossy().to_string();

    let unmanaged = assert_success_json(
        os_command(&home)
            .args(["--json", "project", "status", "--project-path", &path])
            .output()
            .expect("inspect arbitrary repository"),
    );
    assert_eq!(
        unmanaged.get("managed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        unmanaged.pointer("/project/path").and_then(Value::as_str),
        Some(path.as_str())
    );

    let first_scan = assert_success_json(
        os_command(&home)
            .args([
                "--json",
                "project",
                "scan",
                "--project-path",
                &path,
                "--save",
            ])
            .output()
            .expect("adopt repository"),
    );
    let first_id = first_scan
        .get("project_id")
        .and_then(Value::as_str)
        .expect("saved scan project id")
        .to_string();
    assert_eq!(
        first_scan.get("registered_now").and_then(Value::as_bool),
        Some(true)
    );

    let second_scan = assert_success_json(
        os_command(&home)
            .args([
                "--json",
                "project",
                "scan",
                "--project-path",
                &path,
                "--save",
            ])
            .output()
            .expect("repeat repository adoption"),
    );
    assert_eq!(
        second_scan.get("project_id").and_then(Value::as_str),
        Some(first_id.as_str())
    );
    assert_eq!(
        second_scan.get("registered_now").and_then(Value::as_bool),
        Some(false)
    );

    let config_path = assert_success_json(
        os_command(&home)
            .args(["--json", "config", "path"])
            .output()
            .expect("locate CLI database"),
    );
    let database_path = std::path::Path::new(
        config_path
            .get("config_dir")
            .and_then(Value::as_str)
            .expect("config directory"),
    )
    .join("OpenSunstar.db");
    let connection = rusqlite::Connection::open(database_path).expect("open CLI database");
    let saved_detection_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_sdd_detections WHERE project_id = ?1",
            [&first_id],
            |row| row.get(0),
        )
        .expect("count saved framework detections");
    assert_eq!(saved_detection_count, 7);

    let managed = assert_success_json(
        os_command(&home)
            .args(["--json", "project", "status", "--project-path", &path])
            .output()
            .expect("inspect adopted repository"),
    );
    assert_eq!(managed.get("managed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        managed.pointer("/project/id").and_then(Value::as_str),
        Some(first_id.as_str())
    );

    let readiness_output = os_command(&home)
        .args([
            "--json",
            "readiness",
            "score",
            "--project-path",
            &path,
            "--app",
            "codex",
        ])
        .output()
        .expect("score adopted repository");
    let readiness: Value = serde_json::from_slice(&readiness_output.stdout)
        .expect("managed readiness should emit JSON");
    assert_eq!(
        readiness.get("managed").and_then(Value::as_bool),
        Some(true)
    );
    assert!(readiness.get("score").is_some_and(Value::is_number));
    assert!(readiness
        .get("details")
        .and_then(Value::as_array)
        .expect("managed readiness details")
        .iter()
        .any(|item| item.get("status").and_then(Value::as_str) == Some("missing")));

    let health = assert_success_json(
        os_command(&home)
            .args(["--json", "asset", "health", "--project-id", &first_id])
            .output()
            .expect("inspect adopted project health by id"),
    );
    assert_eq!(health.get("managed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        health.get("assessment_state").and_then(Value::as_str),
        Some("unknown")
    );
}

#[test]
fn unmanaged_readiness_is_unscored_and_asset_health_accepts_path() {
    let home = isolated_home();
    let repository = tempfile::tempdir().expect("create repository");
    std::fs::write(
        repository.path().join("AGENTS.md"),
        "# Agent instructions\n",
    )
    .expect("write repository marker");
    let path = repository.path().to_string_lossy().to_string();

    let default_status = assert_success_json(
        os_command(&home)
            .current_dir(repository.path())
            .args(["--json", "project", "status"])
            .output()
            .expect("inspect current repository without explicit path"),
    );
    assert_eq!(
        default_status.get("managed").and_then(Value::as_bool),
        Some(false)
    );

    let readiness_output = os_command(&home)
        .args([
            "--json",
            "readiness",
            "score",
            "--project-path",
            &path,
            "--app",
            "codex",
        ])
        .output()
        .expect("score unmanaged repository");
    let readiness: Value = serde_json::from_slice(&readiness_output.stdout)
        .expect("unmanaged readiness should still emit JSON");
    assert_eq!(
        readiness.get("assessment_state").and_then(Value::as_str),
        Some("unmanaged")
    );
    assert!(readiness.get("score").is_some_and(Value::is_null));
    assert!(readiness
        .get("details")
        .and_then(Value::as_array)
        .expect("readiness details")
        .iter()
        .all(|item| item.get("status").and_then(Value::as_str) != Some("missing")));

    let health = assert_success_json(
        os_command(&home)
            .args(["--json", "asset", "health", "--project-path", &path])
            .output()
            .expect("inspect asset health by repository path"),
    );
    assert_eq!(health.get("managed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        health.get("assessment_state").and_then(Value::as_str),
        Some("unmanaged")
    );
    assert_eq!(
        health
            .get("records")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
}
