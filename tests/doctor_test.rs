//! Tests for `mimic doctor` and rendered-template drift detection.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn apply(config: &std::path::Path, state: &std::path::Path) {
    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("apply")
        .arg("--config")
        .arg(config)
        .arg("--yes")
        .arg("--state")
        .arg(state);
    cmd.assert().success();
}

fn doctor(config: &std::path::Path, state: &std::path::Path) -> assert_cmd::assert::Assert {
    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("doctor")
        .arg("--config")
        .arg(config)
        .arg("--state")
        .arg(state);
    cmd.assert()
}

/// Remove the rendered files recorded in state (tests write into the real
/// ~/.mimic/rendered directory).
fn cleanup_rendered(state_path: &std::path::Path) {
    if let Ok(state) = mimic::state::State::load(state_path) {
        for dotfile in &state.dotfiles {
            if let Some(rendered) = &dotfile.rendered_path {
                let _ = fs::remove_file(rendered);
            }
        }
    }
}

#[test]
fn test_doctor_healthy_after_apply() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    let state_path = temp.path().join("state.toml");
    let source = temp.path().join("vimrc");
    fs::write(&source, "set number\n").unwrap();

    let config = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source.display(),
        temp.path().join(".vimrc").display()
    );
    fs::write(&config_path, config).unwrap();

    apply(&config_path, &state_path);

    doctor(&config_path, &state_path)
        .success()
        .stdout(predicate::str::contains("No problems"))
        .stdout(predicate::str::contains("dotfile(s) linked correctly"));
}

#[test]
fn test_doctor_detects_hijacked_target() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    let state_path = temp.path().join("state.toml");
    let source = temp.path().join("vimrc");
    let target = temp.path().join(".vimrc");
    fs::write(&source, "set number\n").unwrap();

    let config = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source.display(),
        target.display()
    );
    fs::write(&config_path, config).unwrap();

    apply(&config_path, &state_path);

    // Replace the managed symlink with a plain file
    fs::remove_file(&target).unwrap();
    fs::write(&target, "hand-rolled\n").unwrap();

    doctor(&config_path, &state_path)
        .failure()
        .stdout(predicate::str::contains("Problems:"))
        .stdout(predicate::str::contains("not a symlink"));
}

#[test]
fn test_doctor_warns_on_unmatched_hostname() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    let state_path = temp.path().join("state.toml");

    fs::write(
        &config_path,
        r#"
[hosts.zzz-unlikely-hostname]
roles = ["work"]
"#,
    )
    .unwrap();

    doctor(&config_path, &state_path)
        .success()
        .stdout(predicate::str::contains("no [hosts] entry matches"));
}

#[test]
fn test_doctor_reports_broken_config() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    let state_path = temp.path().join("state.toml");

    fs::write(&config_path, "this is not valid toml [[[").unwrap();

    doctor(&config_path, &state_path)
        .failure()
        .stdout(predicate::str::contains("Problems:"));
}

#[test]
fn test_diff_and_doctor_detect_rendered_template_drift() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    let state_path = temp.path().join("state.toml");
    let source = temp.path().join("gitconfig.hbs");
    let target = temp.path().join(".gitconfig");
    fs::write(&source, "editor={{variables.editor}}\n").unwrap();

    let config = format!(
        r#"
[variables]
editor = "nvim"

[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source.display(),
        target.display()
    );
    fs::write(&config_path, config).unwrap();

    apply(&config_path, &state_path);

    // In sync right after apply
    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("diff")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("drifted").not());

    // Hand-edit the live file (writes through the symlink into the rendered file)
    fs::write(&target, "editor=code\n").unwrap();

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("diff")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(&state_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("drifted"));

    doctor(&config_path, &state_path)
        .failure()
        .stdout(predicate::str::contains("drifted"));

    cleanup_rendered(&state_path);
}
