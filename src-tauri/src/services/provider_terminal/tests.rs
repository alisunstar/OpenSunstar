use super::*;
use std::path::Path;
#[test]
fn resolve_launch_cwd_accepts_existing_directory() {
    let resolved = resolve_launch_cwd(Some(std::env::temp_dir().to_string_lossy().into_owned()))
        .expect("temp dir should resolve")
        .expect("temp dir should be present");

    assert!(resolved.is_dir());
}

#[test]
fn resolve_launch_cwd_rejects_missing_directory() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let missing = std::env::temp_dir().join(format!("OpenSunstar-missing-{unique}"));

    let error = resolve_launch_cwd(Some(missing.to_string_lossy().into_owned()))
        .expect_err("missing directory should fail");

    assert!(error.contains("目录不存在"));
}

#[test]
fn build_shell_cd_command_quotes_spaces_and_single_quotes() {
    let command = build_shell_cd_command(Some(Path::new("/tmp/project O'Brien")));

    assert_eq!(command, "cd '/tmp/project O'\"'\"'Brien' || exit 1\n");
}

#[cfg(target_os = "macos")]
#[test]
fn iterm2_applescript_cold_start_avoids_current_window_before_one_exists() {
    let script = build_macos_iterm2_applescript(Path::new("/tmp/OpenSunstar_launcher.sh"));

    let cold_start_branch = script
        .split("else\n        activate")
        .nth(1)
        .expect("cold start branch should be present")
        .split("    end if\n    tell current session")
        .next()
        .expect("cold start branch should end before writing command");

    assert!(cold_start_branch.contains("repeat while (count of windows) = 0"));
    assert!(cold_start_branch.contains("create window with default profile"));
    assert!(!cold_start_branch.contains("tell current window"));
    assert!(!cold_start_branch.contains("create tab with default profile"));
}

#[cfg(target_os = "macos")]
#[test]
fn iterm2_applescript_keeps_new_tab_behavior_for_existing_windows() {
    let script = build_macos_iterm2_applescript(Path::new("/tmp/OpenSunstar_launcher.sh"));

    let running_branch = script
        .split("if was_running then")
        .nth(1)
        .expect("already-running branch should be present")
        .split("else\n        activate")
        .next()
        .expect("already-running branch should end before cold start branch");

    assert!(running_branch.contains("if (count of windows) = 0 then"));
    assert!(running_branch.contains("create window with default profile"));
    assert!(running_branch.contains("create tab with default profile"));
}

#[test]
fn build_windows_cwd_command_str_uses_cd_for_drive_paths() {
    let command = build_windows_cwd_command_str(r"C:\work\repo");

    assert_eq!(command, "cd /d \"C:\\work\\repo\" || exit /b 1\r\n");
}

#[test]
fn build_windows_cwd_command_str_uses_pushd_for_unc_paths() {
    let command = build_windows_cwd_command_str(r"\\wsl$\Ubuntu\home\coder\repo");

    assert_eq!(
        command,
        "pushd \"\\\\wsl$\\Ubuntu\\home\\coder\\repo\" || exit /b 1\r\n"
    );
}

#[test]
fn build_windows_cwd_command_str_escapes_batch_metacharacters() {
    let command = build_windows_cwd_command_str(r"\\server\share\100%&(test)");

    assert_eq!(
        command,
        "pushd \"\\\\server\\share\\100%%^&^(test^)\" || exit /b 1\r\n"
    );
}
