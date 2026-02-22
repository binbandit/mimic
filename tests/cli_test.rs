use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("apply"))
        .stdout(predicate::str::contains("diff"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("undo"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("mimic"));
}

#[test]
fn test_cli_diff_missing_config() {
    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("diff").arg("--config").arg("/nonexistent.toml");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read config file"));
}

#[test]
fn test_cli_diff_with_valid_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_path = temp_dir.path().join("vimrc");

    // Create source file
    fs::write(&source_path, "set number\n").unwrap();

    // Create config
    let config_content = format!(
        r#"
[variables]
email = "test@example.com"

[[dotfiles]]
source = "{}"
target = "{}/.vimrc"
"#,
        source_path.display(),
        temp_dir.path().display()
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("diff").arg("--config").arg(config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("+"))
        .stdout(predicate::str::contains("dotfile"));
}

#[test]
fn test_cli_diff_no_changes() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_path = temp_dir.path().join("bashrc");
    let target_path = temp_dir.path().join(".bashrc");

    // Create source file
    fs::write(&source_path, "export PATH=$PATH:/usr/local/bin\n").unwrap();

    // Create target symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source_path, &target_path).unwrap();

    // Create config
    let config_content = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source_path.display(),
        target_path.display()
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("diff").arg("--config").arg(config_path);

    cmd.assert().success().stdout(predicate::str::contains("✓"));
}

#[test]
fn test_cli_apply_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_path = temp_dir.path().join("zshrc");

    // Create source file
    fs::write(&source_path, "export EDITOR=vim\n").unwrap();

    // Create config
    let config_content = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}/.zshrc"
"#,
        source_path.display(),
        temp_dir.path().display()
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Dry-run mode"));

    // Verify symlink was NOT created
    let target_path = temp_dir.path().join(".zshrc");
    assert!(!target_path.exists());
}

#[test]
fn test_cli_apply_with_yes_flag() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let source_path = temp_dir.path().join("tmux.conf");
    let target_path = temp_dir.path().join(".tmux.conf");
    let state_path = temp_dir.path().join("state.toml");

    // Create source file
    fs::write(&source_path, "set -g mouse on\n").unwrap();

    // Create config
    let config_content = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source_path.display(),
        target_path.display()
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--yes")
        .arg("--state")
        .arg(&state_path);

    cmd.assert().success();

    // Verify symlink was created
    #[cfg(unix)]
    {
        assert!(target_path.exists());
        assert!(target_path.is_symlink());
        let link_target = fs::read_link(&target_path).unwrap();
        assert_eq!(
            fs::canonicalize(&link_target).unwrap(),
            fs::canonicalize(&source_path).unwrap()
        );
    }

    // Verify state file was created
    assert!(state_path.exists());
}

#[test]
fn test_cli_status_no_state() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("nonexistent_state.toml");

    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("--state").arg(&state_path).arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No state file found"));
}

#[test]
fn test_cli_undo_nothing_to_undo() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("nonexistent_state.toml");

    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("--state").arg(&state_path).arg("undo");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Nothing to undo"));
}

#[test]
fn test_cli_apply_missing_config() {
    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("apply").arg("--config").arg("/nonexistent.toml");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read config file"));
}

#[test]
fn test_cli_global_flags() {
    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
fn test_cli_default_config_search() {
    // Test that CLI searches for config in default locations when not specified
    let temp_dir = TempDir::new().unwrap();
    let cwd_config = temp_dir.path().join("mimic.toml");
    let source_path = temp_dir.path().join("gitconfig");

    // Create source file
    fs::write(&source_path, "[user]\nname = test\n").unwrap();

    // Create config in CWD
    let config_content = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}/.gitconfig"
"#,
        source_path.display(),
        temp_dir.path().display()
    );
    fs::write(&cwd_config, config_content).unwrap();

    let mut cmd = Command::cargo_bin("mimic").unwrap();
    cmd.current_dir(temp_dir.path()).arg("diff");

    // Should succeed by finding mimic.toml in CWD
    cmd.assert().success();
}
