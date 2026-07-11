//! Tests for `mimic apply --only/--skip` section scoping and orphaned
//! target cleanup.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

struct Fixture {
    temp: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            temp: TempDir::new().unwrap(),
        }
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.temp.path().join(name)
    }

    /// Config with one dotfile and one command hook that creates a marker file.
    fn write_config_with_hook(&self) -> std::path::PathBuf {
        let source = self.path("zshrc");
        fs::write(&source, "export EDITOR=vim\n").unwrap();

        let config_path = self.path("config.toml");
        let config = format!(
            r#"
[[dotfiles]]
source = "{}"
target = "{}"

[[hooks]]
type = "command"
name = "marker"
command = "touch {}"
on_failure = "continue"
"#,
            source.display(),
            self.path(".zshrc").display(),
            self.path("hook_ran").display()
        );
        fs::write(&config_path, config).unwrap();
        config_path
    }

    fn apply(&self, config: &std::path::Path, extra_args: &[&str]) -> assert_cmd::assert::Assert {
        let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
        cmd.arg("apply")
            .arg("--config")
            .arg(config)
            .arg("--yes")
            .arg("--state")
            .arg(self.path("state.toml"));
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd.assert()
    }
}

#[test]
fn test_apply_only_dotfiles_skips_hooks() {
    let fx = Fixture::new();
    let config = fx.write_config_with_hook();

    fx.apply(&config, &["--only", "dotfiles"]).success();

    assert!(fx.path(".zshrc").is_symlink(), "dotfile should be linked");
    assert!(!fx.path("hook_ran").exists(), "hook must not run");
}

#[test]
fn test_apply_only_hooks_runs_hooks_without_linking() {
    let fx = Fixture::new();
    let config = fx.write_config_with_hook();

    fx.apply(&config, &["--only", "hooks"]).success();

    assert!(!fx.path(".zshrc").exists(), "dotfile must not be linked");
    assert!(fx.path("hook_ran").exists(), "hook should run");
}

#[test]
fn test_apply_skip_hooks() {
    let fx = Fixture::new();
    let config = fx.write_config_with_hook();

    fx.apply(&config, &["--skip", "hooks"]).success();

    assert!(fx.path(".zshrc").is_symlink(), "dotfile should be linked");
    assert!(!fx.path("hook_ran").exists(), "hook must not run");
}

#[test]
fn test_apply_removes_orphaned_targets() {
    let fx = Fixture::new();

    let source_a = fx.path("file_a");
    let source_b = fx.path("file_b");
    fs::write(&source_a, "a\n").unwrap();
    fs::write(&source_b, "b\n").unwrap();

    let config_path = fx.path("config.toml");
    let both = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"

[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source_a.display(),
        fx.path(".file_a").display(),
        source_b.display(),
        fx.path(".file_b").display()
    );
    fs::write(&config_path, both).unwrap();

    fx.apply(&config_path, &[]).success();
    assert!(fx.path(".file_a").is_symlink());
    assert!(fx.path(".file_b").is_symlink());

    // Drop file_b from the config; the old symlink must be cleaned up
    let only_a = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source_a.display(),
        fx.path(".file_a").display()
    );
    fs::write(&config_path, only_a).unwrap();

    fx.apply(&config_path, &[])
        .success()
        .stdout(predicate::str::contains("removed from config"))
        .stdout(predicate::str::contains("Removed orphaned symlink"));

    assert!(fx.path(".file_a").is_symlink(), "kept dotfile stays linked");
    assert!(
        !fx.path(".file_b").exists() && !fx.path(".file_b").is_symlink(),
        "orphaned symlink should be removed"
    );

    let state = mimic::state::State::load(fx.path("state.toml")).unwrap();
    let targets: Vec<&str> = state.dotfiles.iter().map(|d| d.target.as_str()).collect();
    assert!(targets.iter().any(|t| t.ends_with(".file_a")));
    assert!(
        !targets.iter().any(|t| t.ends_with(".file_b")),
        "orphan should be untracked"
    );
}

#[test]
fn test_apply_leaves_non_mimic_orphan_in_place() {
    let fx = Fixture::new();

    let source = fx.path("file_a");
    fs::write(&source, "a\n").unwrap();

    let config_path = fx.path("config.toml");
    let config = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source.display(),
        fx.path(".file_a").display()
    );
    fs::write(&config_path, config).unwrap();

    fx.apply(&config_path, &[]).success();

    // User replaces the symlink with a real file, then removes it from config
    fs::remove_file(fx.path(".file_a")).unwrap();
    fs::write(fx.path(".file_a"), "user content\n").unwrap();
    fs::write(&config_path, "").unwrap();

    fx.apply(&config_path, &[]).success();

    assert!(
        fx.path(".file_a").exists(),
        "user-owned file must not be deleted"
    );
    assert_eq!(
        fs::read_to_string(fx.path(".file_a")).unwrap(),
        "user content\n"
    );
}

#[test]
fn test_diff_shows_orphan_removals() {
    let fx = Fixture::new();

    let source = fx.path("file_a");
    fs::write(&source, "a\n").unwrap();

    let config_path = fx.path("config.toml");
    let config = format!(
        r#"
[[dotfiles]]
source = "{}"
target = "{}"
"#,
        source.display(),
        fx.path(".file_a").display()
    );
    fs::write(&config_path, config).unwrap();
    fx.apply(&config_path, &[]).success();

    fs::write(&config_path, "").unwrap();

    let mut cmd = assert_cmd::cargo_bin_cmd!("mimic");
    cmd.arg("diff")
        .arg("--config")
        .arg(&config_path)
        .arg("--state")
        .arg(fx.path("state.toml"));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("removed from config"));

    assert!(
        fx.path(".file_a").is_symlink(),
        "diff must not touch the filesystem"
    );
}
