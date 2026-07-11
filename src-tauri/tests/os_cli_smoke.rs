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
