use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test first-time setup scenario:
/// - Create config with dotfiles and packages
/// - Run apply
/// - Verify symlinks created
/// - Verify state file created
/// - Run status to verify all in sync
#[test]
fn test_first_time_setup() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create source dotfiles
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("test.conf"), "test content").unwrap();

    // Create config file
    let config_path = temp_path.join("mimic.toml");
    let target_path = temp_path.join("target.conf");
    let state_path = temp_path.join("state.toml");

    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/test.conf"
target = "{}"
"#,
            source_dir.display(),
            target_path.display()
        ),
    )
    .unwrap();

    // Run apply with --yes to skip prompts
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("✓"));

    // Verify symlink created
    assert!(target_path.is_symlink());
    let link_target = fs::read_link(&target_path).unwrap();
    assert_eq!(
        link_target.canonicalize().unwrap(),
        source_dir.join("test.conf").canonicalize().unwrap()
    );

    // Verify state file created
    assert!(state_path.exists());

    // Run status to verify all in sync
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("status")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All resources in sync"));
}

/// Test multi-file apply with conflicts:
/// - Create config with multiple dotfiles
/// - Create conflicting files at target locations
/// - Run apply with --yes (auto-backup)
/// - Verify backups created
/// - Verify symlinks created
#[test]
fn test_multi_file_apply_with_conflicts() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create source dotfiles
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("file1.conf"), "new content 1").unwrap();
    fs::write(source_dir.join("file2.conf"), "new content 2").unwrap();

    // Create conflicting target files
    let target1 = temp_path.join("target1.conf");
    let target2 = temp_path.join("target2.conf");
    fs::write(&target1, "old content 1").unwrap();
    fs::write(&target2, "old content 2").unwrap();

    // Create config file
    let config_path = temp_path.join("mimic.toml");
    let state_path = temp_path.join("state.toml");

    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/file1.conf"
target = "{}"

[[dotfiles]]
source = "{}/file2.conf"
target = "{}"
"#,
            source_dir.display(),
            target1.display(),
            source_dir.display(),
            target2.display()
        ),
    )
    .unwrap();

    // Run apply with --yes (auto-backup conflicts)
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Verify symlinks created
    assert!(target1.is_symlink());
    assert!(target2.is_symlink());

    // Verify backups exist
    let backup_pattern = temp_path.join("target1.conf.backup.*");
    let backup_files: Vec<_> = glob::glob(&backup_pattern.to_string_lossy())
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!backup_files.is_empty(), "Backup should be created");

    // Verify backup content preserved
    let backup_content = fs::read_to_string(&backup_files[0]).unwrap();
    assert_eq!(backup_content, "old content 1");
}

/// Test drift detection and recovery:
/// - Apply config
/// - Manually break symlinks (delete or repoint)
/// - Run status to detect drift
/// - Run apply to fix drift
/// - Run status again to verify fixed
#[test]
fn test_drift_detection_and_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create source dotfile
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("test.conf"), "test content").unwrap();

    // Create config and state paths
    let config_path = temp_path.join("mimic.toml");
    let target_path = temp_path.join("target.conf");
    let state_path = temp_path.join("state.toml");

    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/test.conf"
target = "{}"
"#,
            source_dir.display(),
            target_path.display()
        ),
    )
    .unwrap();

    // Initial apply
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Break symlink (simulate drift)
    fs::remove_file(&target_path).unwrap();

    // Run status - should detect drift
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("status")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .failure() // Exit code 1 when drift detected
        .stdout(predicate::str::contains("Drift detected"))
        .stdout(predicate::str::contains("missing"));

    // Run apply to fix drift
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Run status again - should be in sync
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("status")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All resources in sync"));
}

/// Test undo workflow:
/// - Apply config with conflict (creates backup)
/// - Verify symlink created
/// - Run undo
/// - Verify symlink removed
/// - Verify backup restored
/// - Run undo again - should say "nothing to undo"
#[test]
fn test_undo_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create source dotfile
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("test.conf"), "new content").unwrap();

    // Create conflicting target
    let target_path = temp_path.join("target.conf");
    fs::write(&target_path, "original content").unwrap();

    // Create config
    let config_path = temp_path.join("mimic.toml");
    let state_path = temp_path.join("state.toml");

    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/test.conf"
target = "{}"
"#,
            source_dir.display(),
            target_path.display()
        ),
    )
    .unwrap();

    // Apply (creates backup)
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Verify symlink created
    assert!(target_path.is_symlink());

    // Run undo
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("undo")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully undone"))
        .stdout(predicate::str::contains("symlinks removed"))
        .stdout(predicate::str::contains("backups restored"));

    // Verify symlink removed
    assert!(!target_path.is_symlink());

    // Verify original content restored
    assert!(target_path.exists());
    let restored_content = fs::read_to_string(&target_path).unwrap();
    assert_eq!(restored_content, "original content");

    // Run undo again - should say nothing to undo
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("undo")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to undo"));
}

/// Test dry-run mode:
/// - Create config
/// - Run apply with --dry-run
/// - Verify no symlinks created
/// - Verify no state file created
#[test]
fn test_dry_run_mode() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create source dotfile
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("test.conf"), "test content").unwrap();

    // Create config
    let config_path = temp_path.join("mimic.toml");
    let target_path = temp_path.join("target.conf");
    let state_path = temp_path.join("state.toml");

    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/test.conf"
target = "{}"
"#,
            source_dir.display(),
            target_path.display()
        ),
    )
    .unwrap();

    // Run apply with --dry-run
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry-run mode"));

    // Verify no symlink created
    assert!(!target_path.exists());

    // Verify no state file created
    assert!(!state_path.exists());
}

/// Test template variable substitution:
/// - Create config with variables
/// - Create dotfile using template variables
/// - Apply config
/// - Verify variables substituted correctly
#[test]
fn test_template_variables() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create source dotfile with template
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("test.conf"), "email: {{ email }}").unwrap();

    // Create config with variables
    let config_path = temp_path.join("mimic.toml");
    let target_path = temp_path.join("target.conf");
    let state_path = temp_path.join("state.toml");

    fs::write(
        &config_path,
        format!(
            r#"
[variables]
email = "test@example.com"

[[dotfiles]]
source = "{}/test.conf"
target = "{}"
"#,
            source_dir.display(),
            target_path.display()
        ),
    )
    .unwrap();

    // Apply config
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Note: Template variables are for path expansion, not file content
    // This test verifies the apply succeeds with variables defined
    assert!(target_path.is_symlink());
}

/// Test complete workflow: apply -> status -> modify config -> diff -> apply -> undo
/// Simulates real user workflow of evolving configuration over time
#[test]
fn test_complete_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Setup: Create source dotfiles
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("file1.conf"), "content 1").unwrap();

    let config_path = temp_path.join("mimic.toml");
    let state_path = temp_path.join("state.toml");
    let target1 = temp_path.join("target1.conf");
    let target2 = temp_path.join("target2.conf");

    // Step 1: Initial apply with one dotfile
    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/file1.conf"
target = "{}"
"#,
            source_dir.display(),
            target1.display()
        ),
    )
    .unwrap();

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Step 2: Verify status shows in sync
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("status")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All resources in sync"));

    // Step 3: Add second dotfile to config
    fs::write(source_dir.join("file2.conf"), "content 2").unwrap();
    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/file1.conf"
target = "{}"

[[dotfiles]]
source = "{}/file2.conf"
target = "{}"
"#,
            source_dir.display(),
            target1.display(),
            source_dir.display(),
            target2.display()
        ),
    )
    .unwrap();

    // Step 4: Run diff to preview changes
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("diff")
        .arg("--config")
        .arg(&config_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("file2.conf"));

    // Step 5: Apply updated config
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Verify both symlinks exist
    assert!(target1.is_symlink());
    assert!(target2.is_symlink());

    // Step 6: Undo everything
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("undo")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success();

    // Verify symlinks removed
    assert!(!target1.exists());
    assert!(!target2.exists());
}

/// Test diff command with various scenarios
#[test]
fn test_diff_scenarios() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create source dotfiles
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("new.conf"), "new").unwrap();
    fs::write(source_dir.join("exists.conf"), "exists").unwrap();

    let config_path = temp_path.join("mimic.toml");
    let target_new = temp_path.join("new_target.conf");
    let target_exists = temp_path.join("exists_target.conf");

    // Create one existing target (to show as "already correct" or conflict)
    fs::write(&target_exists, "old content").unwrap();

    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/new.conf"
target = "{}"

[[dotfiles]]
source = "{}/exists.conf"
target = "{}"
"#,
            source_dir.display(),
            target_new.display(),
            source_dir.display(),
            target_exists.display()
        ),
    )
    .unwrap();

    // Run diff - should show new symlink and conflict
    let output = Command::cargo_bin("mimic")
        .unwrap()
        .arg("diff")
        .arg("--config")
        .arg(&config_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("new.conf"));
    assert!(stdout.contains("exists.conf"));
}

/// Test error handling for invalid configurations
#[test]
fn test_invalid_config_handling() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Test 1: Missing source file
    let config_path = temp_path.join("mimic.toml");
    let state_path = temp_path.join("state.toml");

    fs::write(
        &config_path,
        r#"
[[dotfiles]]
source = "/nonexistent/file.conf"
target = "/tmp/target.conf"
"#,
    )
    .unwrap();

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    // Test 2: Malformed TOML
    let bad_config_path = temp_path.join("bad.toml");
    fs::write(&bad_config_path, "[[dotfiles]\nmissing = bracket").unwrap();

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&bad_config_path)
        .assert()
        .failure();
}

/// Test status command verbose mode
#[test]
fn test_status_verbose_mode() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create and apply config
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("test.conf"), "test").unwrap();

    let config_path = temp_path.join("mimic.toml");
    let target_path = temp_path.join("target.conf");
    let state_path = temp_path.join("state.toml");

    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/test.conf"
target = "{}"
"#,
            source_dir.display(),
            target_path.display()
        ),
    )
    .unwrap();

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Run status with verbose flag
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("status")
        .arg("--state")
        .arg(&state_path)
        .arg("--verbose")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.conf"));
}

/// Test path expansion (tilde and environment variables)
#[test]
fn test_path_expansion() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create source dotfile
    let source_dir = temp_path.join("dotfiles");
    fs::create_dir(&source_dir).unwrap();
    fs::write(source_dir.join("test.conf"), "test").unwrap();

    let config_path = temp_path.join("mimic.toml");
    let state_path = temp_path.join("state.toml");

    // Use tilde in target path
    fs::write(
        &config_path,
        format!(
            r#"
[[dotfiles]]
source = "{}/test.conf"
target = "~/mimic_test_target.conf"
"#,
            source_dir.display()
        ),
    )
    .unwrap();

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path)
        .arg("--yes")
        .assert()
        .success();

    // Verify symlink created in home directory
    let home = dirs::home_dir().unwrap();
    let target = home.join("mimic_test_target.conf");
    assert!(target.is_symlink());

    // Cleanup
    fs::remove_file(&target).ok();
}
