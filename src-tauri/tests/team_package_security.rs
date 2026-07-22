use open_sunstar_lib::team_config::{
    enforce_team_package_security, validate_team_package_security,
};
use std::fs;

#[test]
fn blocks_plaintext_secrets_without_returning_the_secret_in_the_report() {
    let root = tempfile::tempdir().expect("temp team package");
    let secret = "sk-proj-ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcd";
    fs::write(
        root.path().join("mcp.json"),
        format!(r#"{{"env":{{"OPENAI_API_KEY":"{secret}"}}}}"#),
    )
    .expect("write fixture");

    let report = validate_team_package_security(root.path()).expect("scan package");
    assert!(report.blocked);
    assert!(report.findings.iter().any(|finding| {
        finding.category == "hardcoded-secret" || finding.category == "team-secret"
    }));
    let serialized = serde_json::to_string(&report).expect("serialize report");
    assert!(!serialized.contains(secret));
}

#[test]
fn blocks_private_keys_and_hidden_secret_files() {
    let root = tempfile::tempdir().expect("temp team package");
    fs::write(
        root.path().join(".env"),
        "PRIVATE_KEY=-----BEGIN PRIVATE KEY-----abcdefghijklmnopqrstuvwxyz1234567890",
    )
    .expect("write fixture");

    let report = validate_team_package_security(root.path()).expect("scan package");
    assert!(report.blocked);
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.rule_id == "team-private-key"));
}

#[test]
fn accepts_credential_slots_environment_templates_and_keychain_references() {
    let root = tempfile::tempdir().expect("temp team package");
    fs::write(
        root.path().join("team.toml"),
        r#"
[[credential_slots]]
id = "TEAM_GITHUB"
kind = "oauth"
provider = "github"

[bindings]
token = "${TEAM_GITHUB}"
local_ref = "keychain://ref/team/TEAM_GITHUB"
"#,
    )
    .expect("write fixture");

    let report = validate_team_package_security(root.path()).expect("scan package");
    assert!(
        !report.blocked,
        "safe placeholders must not be blocked: {:?}",
        report.findings
    );
}

#[test]
fn enforcement_returns_a_secret_free_blocking_error() {
    let root = tempfile::tempdir().expect("temp team package");
    let secret = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
    fs::write(
        root.path().join("agent.json"),
        format!(r#"{{"GITHUB_TOKEN":"{secret}"}}"#),
    )
    .expect("write fixture");

    let error = enforce_team_package_security(root.path()).expect_err("package must be blocked");
    assert!(error.to_string().contains("team_package_security_blocked"));
    assert!(!error.to_string().contains(secret));
}
