use mimic::config::Config;

#[test]
fn test_parse_rustup_hook() {
    let toml = r#"
[[hooks]]
type = "rustup"
toolchains = ["stable", "nightly"]
components = ["rustfmt", "clippy"]
targets = ["x86_64-unknown-linux-musl"]
default = "nightly"
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.hooks.len(), 1);
}

#[test]
fn test_parse_cargo_install_hook() {
    let toml = r#"
[[hooks]]
type = "cargo-install"
packages = [
    { name = "tap", git = "https://github.com/crazywolf132/tap.git" },
    { name = "sg", git = "https://github.com/sage-scm/sage.git", bin = "sg" },
]
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.hooks.len(), 1);
}

#[test]
fn test_parse_mise_hook() {
    let toml = r#"
[[hooks]]
type = "mise"
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.hooks.len(), 1);
}

#[test]
fn test_parse_pnpm_global_hook() {
    let toml = r#"
[[hooks]]
type = "pnpm-global"
packages = ["typescript", "@biomejs/biome"]
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.hooks.len(), 1);
}

#[test]
fn test_parse_uv_python_hook() {
    let toml = r#"
[[hooks]]
type = "uv-python"
version = "3.12"

[hooks.symlinks]
python = "~/.local/bin/python"
python3 = "~/.local/bin/python3"
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.hooks.len(), 1);
}

#[test]
fn test_parse_command_hook() {
    let toml = r#"
[[hooks]]
type = "command"
name = "test-hook"
command = "echo hello"
on_failure = "continue"
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.hooks.len(), 1);
}

#[test]
fn test_parse_multiple_hooks() {
    let toml = r#"
[[hooks]]
type = "rustup"
toolchains = ["stable"]
components = ["rustfmt"]
targets = []

[[hooks]]
type = "mise"

[[hooks]]
type = "command"
name = "custom"
command = "echo test"
on_failure = "fail"
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.hooks.len(), 3);
}

#[test]
fn test_empty_hooks() {
    let toml = r#"
[[dotfiles]]
source = "test"
target = "test"
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.hooks.len(), 0);
}
