//! Tests for host alias/fuzzy resolution, hosts list/show views,
//! render --host, and offline mode.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn write_config(temp: &TempDir, content: &str) -> std::path::PathBuf {
    let config_path = temp.path().join("config.toml");
    fs::write(&config_path, content).unwrap();
    config_path
}

#[test]
fn test_apply_with_unknown_explicit_host_errors() {
    let temp = TempDir::new().unwrap();
    let config_path = write_config(
        &temp,
        r#"
[hosts.work]
roles = ["work"]
"#,
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("apply")
        .arg("--config")
        .arg(&config_path)
        .arg("--host")
        .arg("nonexistent")
        .arg("--yes")
        .arg("--state")
        .arg(temp.path().join("state.toml"));

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found in config"))
        .stderr(predicate::str::contains("Available hosts: work"));
}

#[test]
fn test_diff_warns_when_detected_hostname_has_no_entry() {
    let temp = TempDir::new().unwrap();
    let config_path = write_config(
        &temp,
        r#"
[hosts.zzz-unlikely-hostname]
roles = ["work"]
"#,
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("diff")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(temp.path().join("state.toml"));

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("no [hosts] entry matches"));
}

#[test]
fn test_host_alias_resolution_via_render() {
    let temp = TempDir::new().unwrap();
    let template = temp.path().join("editor.hbs");
    fs::write(&template, "editor={{variables.editor}}").unwrap();

    let config_path = write_config(
        &temp,
        r#"
[variables]
editor = "nvim"

[hosts.work-laptop]
aliases = ["wrk"]

[hosts.work-laptop.variables]
editor = "code"
"#,
    );

    // Resolve by alias
    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("render")
        .arg(&template)
        .arg("--config")
        .arg(&config_path)
        .arg("--host")
        .arg("wrk");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("editor=code"));

    // Resolve by first label of a fully-qualified name
    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("render")
        .arg(&template)
        .arg("--config")
        .arg(&config_path)
        .arg("--host")
        .arg("work-laptop.localdomain");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("editor=code"));
}

#[test]
fn test_render_as_other_host_from_any_machine() {
    let temp = TempDir::new().unwrap();
    let template = temp.path().join("cfg.hbs");
    fs::write(
        &template,
        "host={{host.name}} work={{includes host.roles \"work\"}}",
    )
    .unwrap();

    let config_path = write_config(
        &temp,
        r#"
[hosts.elara]
roles = ["work"]
"#,
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("render")
        .arg(&template)
        .arg("--config")
        .arg(&config_path)
        .arg("--host")
        .arg("elara");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("host=elara work=true"));
}

#[test]
fn test_hosts_list_shows_inherited_roles_and_aliases() {
    let temp = TempDir::new().unwrap();
    let config_path = write_config(
        &temp,
        r#"
[hosts.parent]
roles = ["base"]

[hosts.child]
inherits = "parent"
aliases = ["kid"]
roles = ["dev"]
"#,
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("hosts")
        .arg("list")
        .arg("--config")
        .arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("base, dev"))
        .stdout(predicate::str::contains("inherits parent"))
        .stdout(predicate::str::contains("aliases: kid"));
}

#[test]
fn test_hosts_show_diff_lists_only_overrides() {
    let temp = TempDir::new().unwrap();
    let config_path = write_config(
        &temp,
        r#"
[variables]
editor = "nvim"
shell = "fish"

[hosts.work]

[hosts.work.variables]
editor = "code"
extra = "yes"
"#,
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("hosts")
        .arg("show")
        .arg("work")
        .arg("--diff")
        .arg("--config")
        .arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("editor = code"))
        .stdout(predicate::str::contains("(default: nvim)"))
        .stdout(predicate::str::contains("extra = yes"))
        .stdout(predicate::str::contains("shell").not());
}

#[test]
fn test_hosts_show_raw_skips_defaults() {
    let temp = TempDir::new().unwrap();
    let config_path = write_config(
        &temp,
        r#"
[variables]
editor = "nvim"

[hosts.work.variables]
extra = "yes"
"#,
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("hosts")
        .arg("show")
        .arg("work")
        .arg("--raw")
        .arg("--config")
        .arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("extra = yes"))
        .stdout(predicate::str::contains("editor").not());
}

#[test]
fn test_offline_mode_fails_on_uncached_extends() {
    let temp = TempDir::new().unwrap();
    // Unique repo URL guarantees the extends cache has never seen it
    let config_path = write_config(
        &temp,
        &format!(
            r#"
[[extends]]
repo = "https://invalid.example/never-cached-{}.git"
"#,
            std::process::id()
        ),
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("diff")
        .arg("--offline")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(temp.path().join("state.toml"));

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("never been fetched"));
}
