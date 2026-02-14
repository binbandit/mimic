use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_test_env() -> (TempDir, PathBuf, PathBuf, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mimic.toml");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");

    fs::write(&source_path, "test content").unwrap();

    (temp_dir, config_path, source_path, target_path)
}

#[test]
fn test_undo_removes_symlink_and_restores_backup() {
    let (temp_dir, config_path, source_path, target_path) = setup_test_env();

    fs::write(&target_path, "original content").unwrap();

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

    let state_path = temp_dir.path().join("state.toml");

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

    assert!(target_path.is_symlink());

    let backup_files: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("target.txt.backup.")
        })
        .collect();
    assert_eq!(backup_files.len(), 1);

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("undo")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully undone last apply"))
        .stdout(predicate::str::contains("1 symlinks removed"))
        .stdout(predicate::str::contains("1 backups restored"));

    assert!(!target_path.is_symlink());
    assert!(target_path.exists());

    let restored_content = fs::read_to_string(&target_path).unwrap();
    assert_eq!(restored_content, "original content");
}

#[test]
fn test_undo_nothing_to_undo() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.toml");

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("undo")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to undo"));
}

#[test]
fn test_undo_missing_symlink() {
    let (temp_dir, config_path, source_path, target_path) = setup_test_env();

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

    let state_path = temp_dir.path().join("state.toml");

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

    assert!(target_path.is_symlink());

    fs::remove_file(&target_path).unwrap();

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("undo")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully undone last apply"));

    assert!(!target_path.exists());
}

#[test]
fn test_undo_multiple_dotfiles() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mimic.toml");

    let source1 = temp_dir.path().join("source1.txt");
    let target1 = temp_dir.path().join("target1.txt");
    let source2 = temp_dir.path().join("source2.txt");
    let target2 = temp_dir.path().join("target2.txt");

    fs::write(&source1, "content 1").unwrap();
    fs::write(&source2, "content 2").unwrap();
    fs::write(&target1, "original 1").unwrap();
    fs::write(&target2, "original 2").unwrap();

    let config_content = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"

[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source1.display(),
        target1.display(),
        source2.display(),
        target2.display()
    );
    fs::write(&config_path, config_content).unwrap();

    let state_path = temp_dir.path().join("state.toml");

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

    assert!(target1.is_symlink());
    assert!(target2.is_symlink());

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("undo")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("2 symlinks removed"))
        .stdout(predicate::str::contains("2 backups restored"));

    assert!(!target1.is_symlink());
    assert!(!target2.is_symlink());
    assert_eq!(fs::read_to_string(&target1).unwrap(), "original 1");
    assert_eq!(fs::read_to_string(&target2).unwrap(), "original 2");
}

#[test]
fn test_undo_no_backup() {
    let (temp_dir, config_path, source_path, target_path) = setup_test_env();

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

    let state_path = temp_dir.path().join("state.toml");

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

    assert!(target_path.is_symlink());

    Command::cargo_bin("mimic")
        .unwrap()
        .arg("undo")
        .arg("--state")
        .arg(&state_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("1 symlinks removed"))
        .stdout(predicate::str::contains("0 backups restored"));

    assert!(!target_path.exists());
}
