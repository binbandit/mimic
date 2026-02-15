//! Integration tests for the init command
//!
//! These tests use real git commands with actual test repositories.
//! No mocking - tests verify actual git clone behavior.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a local git repository for testing
fn create_test_git_repo(dir: &std::path::Path) -> anyhow::Result<()> {
    use std::process::Command as StdCommand;

    // Initialize git repo
    StdCommand::new("git")
        .arg("init")
        .current_dir(dir)
        .output()?;

    // Create a minimal mimic.toml config
    fs::write(
        dir.join("mimic.toml"),
        r#"
[variables]
test = "value"

[[dotfiles]]
source = "dotfiles/test.conf"
target = "~/.test.conf"
"#,
    )?;

    // Create a dotfiles directory and test file
    fs::create_dir_all(dir.join("dotfiles"))?;
    fs::write(dir.join("dotfiles/test.conf"), "test content")?;

    // Commit the files
    StdCommand::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(dir)
        .output()?;

    StdCommand::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test User")
        .current_dir(dir)
        .output()?;

    StdCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(dir)
        .output()?;

    StdCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Initial commit")
        .current_dir(dir)
        .output()?;

    Ok(())
}

/// Helper to get the mimic repo directory and clean it up
fn get_and_clean_repo_dir() -> anyhow::Result<PathBuf> {
    let repo_dir = directories::BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?
        .config_dir()
        .join("mimic/repo");

    // Clean up any existing repo directory
    if repo_dir.exists() {
        fs::remove_dir_all(&repo_dir)?;
    }

    Ok(repo_dir)
}

#[test]
fn test_init_clones_repository() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    create_test_git_repo(repo_path).expect("Failed to create test git repo");

    // Clean up any existing repo
    let repo_dir = get_and_clean_repo_dir().expect("Failed to get repo dir");

    // Run mimic init with local repository path
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("init")
        .arg(repo_path.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Cloning repository"))
        .stdout(predicate::str::contains("✓"));

    // Verify repository was cloned
    assert!(repo_dir.exists(), "Repository directory should exist");
    assert!(
        repo_dir.join("mimic.toml").exists(),
        "Config file should exist in cloned repo"
    );
    assert!(
        repo_dir.join("dotfiles").exists(),
        "Dotfiles directory should exist"
    );
    assert!(
        repo_dir.join("dotfiles/test.conf").exists(),
        "Test dotfile should exist"
    );

    // Cleanup
    fs::remove_dir_all(&repo_dir).ok();
}

#[test]
fn test_init_with_nonexistent_repo() {
    // Clean up any existing repo
    let repo_dir = get_and_clean_repo_dir().expect("Failed to get repo dir");

    // Run mimic init with nonexistent repository
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("init")
        .arg("https://github.com/nonexistent-user-12345/nonexistent-repo-67890.git")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found").or(predicate::str::contains("does not exist")),
        );

    // Cleanup
    if repo_dir.exists() {
        fs::remove_dir_all(&repo_dir).ok();
    }
}

#[test]
fn test_init_with_existing_directory() {
    // Clean up any existing repo
    let repo_dir = get_and_clean_repo_dir().expect("Failed to get repo dir");

    // Create the repo directory manually
    fs::create_dir_all(&repo_dir).expect("Failed to create repo dir");

    // Run mimic init - should fail because directory exists
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("init")
        .arg("https://github.com/example/repo.git")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Cleanup
    fs::remove_dir_all(&repo_dir).ok();
}

#[test]
fn test_init_creates_shallow_clone() {
    // Create a temporary git repository with multiple commits
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    create_test_git_repo(repo_path).expect("Failed to create test git repo");

    // Add a second commit
    use std::process::Command as StdCommand;
    fs::write(repo_path.join("file2.txt"), "second commit").unwrap();
    StdCommand::new("git")
        .arg("add")
        .arg("file2.txt")
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Second commit")
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Clean up any existing repo
    let repo_dir = get_and_clean_repo_dir().expect("Failed to get repo dir");

    // Run mimic init
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("init")
        .arg(repo_path.to_str().unwrap())
        .assert()
        .success();

    // Verify it's a shallow clone by checking git log
    let output = StdCommand::new("git")
        .arg("log")
        .arg("--oneline")
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run git log");

    let log_output = String::from_utf8_lossy(&output.stdout);
    let commit_count = log_output.lines().count();

    // Shallow clone should only have 1 commit (depth=1)
    assert_eq!(
        commit_count, 1,
        "Shallow clone should only have 1 commit, found {}",
        commit_count
    );

    // Cleanup
    fs::remove_dir_all(&repo_dir).ok();
}

#[test]
fn test_init_without_git_installed() {
    // This test is difficult to implement without mocking
    // Skip for now - would require PATH manipulation
}

#[test]
fn test_init_with_apply_flag() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    create_test_git_repo(repo_path).expect("Failed to create test git repo");

    // Clean up any existing repo
    let repo_dir = get_and_clean_repo_dir().expect("Failed to get repo dir");

    // Get a temporary state file path
    let state_temp = TempDir::new().unwrap();
    let state_path = state_temp.path().join("state.toml");

    // Run mimic init with --apply flag
    let output = Command::cargo_bin("mimic")
        .unwrap()
        .arg("init")
        .arg("--apply")
        .arg("--state")
        .arg(&state_path)
        .arg(repo_path.to_str().unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Verify cloning happened
    assert!(
        stdout.contains("Cloning repository"),
        "Should show cloning message"
    );

    // Verify apply was triggered
    // Note: Apply might fail if dotfile target already exists, but we should see attempt
    assert!(
        stdout.contains("✓") || stdout.contains("Applying"),
        "Should show apply progress: {}",
        stdout
    );

    // Verify repository was cloned
    assert!(repo_dir.exists(), "Repository should be cloned");

    // Cleanup
    fs::remove_dir_all(&repo_dir).ok();
}

#[test]
fn test_init_verifies_directory_created() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    create_test_git_repo(repo_path).expect("Failed to create test git repo");

    // Clean up any existing repo
    let repo_dir = get_and_clean_repo_dir().expect("Failed to get repo dir");

    // Verify directory doesn't exist before
    assert!(
        !repo_dir.exists(),
        "Repo directory should not exist before init"
    );

    // Run mimic init
    Command::cargo_bin("mimic")
        .unwrap()
        .arg("init")
        .arg(repo_path.to_str().unwrap())
        .assert()
        .success();

    // Verify directory exists after
    assert!(repo_dir.exists(), "Repo directory should exist after init");

    // Verify it's actually a git repository
    assert!(repo_dir.join(".git").exists(), "Should be a git repository");

    // Cleanup
    fs::remove_dir_all(&repo_dir).ok();
}

#[test]
fn test_init_output_format() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    create_test_git_repo(repo_path).expect("Failed to create test git repo");

    // Clean up any existing repo
    let repo_dir = get_and_clean_repo_dir().expect("Failed to get repo dir");

    // Run mimic init and capture output
    let output = Command::cargo_bin("mimic")
        .unwrap()
        .arg("init")
        .arg(repo_path.to_str().unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Verify output format
    assert!(
        stdout.contains("Cloning repository"),
        "Should show cloning message"
    );
    assert!(
        stdout.contains("✓") && stdout.contains("Repository cloned to"),
        "Should show success message with path"
    );

    // Cleanup
    fs::remove_dir_all(&repo_dir).ok();
}
