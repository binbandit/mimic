use mimic::config::Config;
use std::path::Path;

#[test]
fn test_parse_valid_config() {
    let toml_str = r#"
        [variables]
        email = "user@example.com"
        name = "John Doe"

        [[dotfiles]]
        source = "zsh/zshrc"
        target = "~/.zshrc"

        [[dotfiles]]
        source = "vim/vimrc"
        target = "~/.vimrc"

        [[packages.homebrew]]
        name = "git"
        type = "formula"

        [[packages.homebrew]]
        name = "docker"
        type = "cask"
    "#;

    let config = Config::from_str(toml_str).expect("Failed to parse valid config");

    assert_eq!(config.variables.len(), 2);
    assert_eq!(
        config.variables.get("email"),
        Some(&"user@example.com".to_string())
    );
    assert_eq!(config.variables.get("name"), Some(&"John Doe".to_string()));

    assert_eq!(config.dotfiles.len(), 2);
    assert_eq!(config.dotfiles[0].source, "zsh/zshrc");
    assert_eq!(config.dotfiles[0].target, "~/.zshrc");

    assert_eq!(config.packages.homebrew.len(), 2);
    assert_eq!(config.packages.homebrew[0].name, "git");
    assert_eq!(config.packages.homebrew[0].pkg_type, "formula");
}

#[test]
fn test_parse_minimal_config() {
    let toml_str = r#"
        [variables]
        
        [[dotfiles]]
        source = "bashrc"
        target = "~/.bashrc"
    "#;

    let config = Config::from_str(toml_str).expect("Failed to parse minimal config");

    assert_eq!(config.variables.len(), 0);
    assert_eq!(config.dotfiles.len(), 1);
    assert_eq!(config.packages.homebrew.len(), 0);
}

#[test]
fn test_parse_empty_config() {
    let toml_str = r#"
        [variables]
    "#;

    let config = Config::from_str(toml_str).expect("Failed to parse empty config");

    assert_eq!(config.variables.len(), 0);
    assert_eq!(config.dotfiles.len(), 0);
}

#[test]
fn test_parse_missing_dotfile_source() {
    let toml_str = r#"
        [[dotfiles]]
        target = "~/.zshrc"
    "#;

    let result = Config::from_str(toml_str);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(error_msg.contains("source") || error_msg.contains("missing field"));
}

#[test]
fn test_parse_missing_dotfile_target() {
    let toml_str = r#"
        [[dotfiles]]
        source = "zsh/zshrc"
    "#;

    let result = Config::from_str(toml_str);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(error_msg.contains("target") || error_msg.contains("missing field"));
}

#[test]
fn test_parse_invalid_toml_syntax() {
    let toml_str = r#"
        [variables
        email = "invalid
    "#;

    let result = Config::from_str(toml_str);
    assert!(result.is_err());
}

#[test]
fn test_parse_missing_package_name() {
    let toml_str = r#"
        [[packages.homebrew]]
        type = "formula"
    "#;

    let result = Config::from_str(toml_str);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(error_msg.contains("name") || error_msg.contains("missing field"));
}

#[test]
fn test_parse_missing_package_type() {
    let toml_str = r#"
        [[packages.homebrew]]
        name = "git"
    "#;

    let result = Config::from_str(toml_str);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(error_msg.contains("type") || error_msg.contains("missing field"));
}

#[test]
fn test_parse_config_with_only_packages() {
    let toml_str = r#"
        [[packages.homebrew]]
        name = "neovim"
        type = "formula"
    "#;

    let config = Config::from_str(toml_str).expect("Failed to parse config with only packages");

    assert_eq!(config.dotfiles.len(), 0);
    assert_eq!(config.packages.homebrew.len(), 1);
    assert_eq!(config.packages.homebrew[0].name, "neovim");
}

#[test]
fn test_parse_config_from_file() {
    use std::fs;
    use std::io::Write;

    let temp_path = "/tmp/test_config.toml";
    let toml_str = r#"
        [variables]
        email = "test@example.com"

        [[dotfiles]]
        source = "test/config"
        target = "~/.testrc"
    "#;

    let mut file = fs::File::create(temp_path).expect("Failed to create temp file");
    file.write_all(toml_str.as_bytes())
        .expect("Failed to write temp file");

    let config = Config::from_file(temp_path).expect("Failed to parse config from file");

    assert_eq!(
        config.variables.get("email"),
        Some(&"test@example.com".to_string())
    );
    assert_eq!(config.dotfiles.len(), 1);

    fs::remove_file(temp_path).ok();
}

#[test]
fn test_parse_simple_package_format() {
    let toml_str = r#"
        [packages]
        brew = ["git", "neovim", "ripgrep"]
        cask = ["visual-studio-code", "docker"]
    "#;

    let config = Config::from_str(toml_str).expect("Failed to parse simple package format");

    assert_eq!(config.packages.brew.len(), 3);
    assert_eq!(config.packages.cask.len(), 2);
    assert_eq!(config.packages.brew[0], "git");
    assert_eq!(config.packages.cask[0], "visual-studio-code");
}

#[test]
fn test_normalized_simple_format() {
    let toml_str = r#"
        [packages]
        brew = ["git", "neovim"]
        cask = ["docker"]
    "#;

    let config = Config::from_str(toml_str).expect("Failed to parse simple format");
    let normalized = config.packages.normalized();

    assert_eq!(normalized.homebrew.len(), 3);
    assert_eq!(normalized.homebrew[0].name, "git");
    assert_eq!(normalized.homebrew[0].pkg_type, "formula");
    assert_eq!(normalized.homebrew[1].name, "neovim");
    assert_eq!(normalized.homebrew[1].pkg_type, "formula");
    assert_eq!(normalized.homebrew[2].name, "docker");
    assert_eq!(normalized.homebrew[2].pkg_type, "cask");
}

#[test]
fn test_backwards_compatible_verbose_format() {
    let toml_str = r#"
        [[packages.homebrew]]
        name = "git"
        type = "formula"

        [[packages.homebrew]]
        name = "docker"
        type = "cask"
    "#;

    let config = Config::from_str(toml_str).expect("Failed to parse verbose format");

    assert_eq!(config.packages.homebrew.len(), 2);
    assert_eq!(config.packages.homebrew[0].name, "git");
    assert_eq!(config.packages.homebrew[0].pkg_type, "formula");
    assert_eq!(config.packages.homebrew[1].name, "docker");
    assert_eq!(config.packages.homebrew[1].pkg_type, "cask");
}

#[test]
fn test_mixed_package_formats() {
    let toml_str = r#"
        [packages]
        brew = ["git", "neovim"]
        cask = ["docker"]

        [[packages.homebrew]]
        name = "tmux"
        type = "formula"
        only_roles = ["work"]
    "#;

    let config = Config::from_str(toml_str).expect("Failed to parse mixed format");
    let normalized = config.packages.normalized();

    assert_eq!(normalized.homebrew.len(), 4);
    assert_eq!(normalized.homebrew[0].name, "tmux");
    assert_eq!(
        normalized.homebrew[0].only_roles,
        Some(vec!["work".to_string()])
    );
    assert_eq!(normalized.homebrew[1].name, "git");
    assert_eq!(normalized.homebrew[1].pkg_type, "formula");
    assert!(normalized.homebrew[1].only_roles.is_none());
}

#[test]
fn test_resolve_source_paths_makes_relative_paths_absolute() {
    let toml_str = r#"
        [[dotfiles]]
        source = "dotfiles/zshrc"
        target = "~/.zshrc"

        [[dotfiles]]
        source = "vim/vimrc"
        target = "~/.vimrc"
    "#;

    let mut config = Config::from_str(toml_str).unwrap();
    config.resolve_source_paths(Path::new("/home/user/configs"));

    assert_eq!(
        config.dotfiles[0].source,
        "/home/user/configs/dotfiles/zshrc"
    );
    assert_eq!(config.dotfiles[1].source, "/home/user/configs/vim/vimrc");
}

#[test]
fn test_resolve_source_paths_preserves_absolute_paths() {
    let toml_str = r#"
        [[dotfiles]]
        source = "/absolute/path/zshrc"
        target = "~/.zshrc"
    "#;

    let mut config = Config::from_str(toml_str).unwrap();
    config.resolve_source_paths(Path::new("/home/user/configs"));

    assert_eq!(config.dotfiles[0].source, "/absolute/path/zshrc");
}

#[test]
fn test_resolve_source_paths_preserves_tilde_paths() {
    let toml_str = r#"
        [[dotfiles]]
        source = "~/my-dotfiles/zshrc"
        target = "~/.zshrc"
    "#;

    let mut config = Config::from_str(toml_str).unwrap();
    config.resolve_source_paths(Path::new("/home/user/configs"));

    assert_eq!(config.dotfiles[0].source, "~/my-dotfiles/zshrc");
}

#[test]
fn test_resolve_source_paths_preserves_env_var_paths() {
    let toml_str = r#"
        [[dotfiles]]
        source = "$HOME/dotfiles/zshrc"
        target = "~/.zshrc"
    "#;

    let mut config = Config::from_str(toml_str).unwrap();
    config.resolve_source_paths(Path::new("/home/user/configs"));

    assert_eq!(config.dotfiles[0].source, "$HOME/dotfiles/zshrc");
}

#[test]
fn test_resolve_source_paths_in_host_dotfiles() {
    let toml_str = r#"
        [[dotfiles]]
        source = "common/zshrc"
        target = "~/.zshrc"

        [hosts.laptop]
        roles = ["personal"]

        [[hosts.laptop.dotfiles]]
        source = "laptop/gitconfig"
        target = "~/.gitconfig"
    "#;

    let mut config = Config::from_str(toml_str).unwrap();
    config.resolve_source_paths(Path::new("/home/user/configs"));

    assert_eq!(config.dotfiles[0].source, "/home/user/configs/common/zshrc");

    let laptop = config.hosts.get("laptop").unwrap();
    assert_eq!(
        laptop.dotfiles[0].source,
        "/home/user/configs/laptop/gitconfig"
    );
}

#[test]
fn test_from_file_resolves_relative_source_paths() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("myconfigs");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("mimic.toml");
    fs::write(
        &config_path,
        r#"
[[dotfiles]]
source = "dotfiles/zshrc"
target = "~/.zshrc"

[[dotfiles]]
source = "/absolute/vimrc"
target = "~/.vimrc"
"#,
    )
    .unwrap();

    let config = Config::from_file(&config_path).unwrap();

    // Relative path should be resolved against config file's directory
    let resolved = &config.dotfiles[0].source;
    assert!(
        Path::new(resolved).is_absolute(),
        "Relative source path should be resolved to absolute, got: {}",
        resolved
    );
    assert!(
        resolved.ends_with("myconfigs/dotfiles/zshrc"),
        "Should resolve relative to config dir, got: {}",
        resolved
    );

    // Absolute path should remain unchanged
    assert_eq!(config.dotfiles[1].source, "/absolute/vimrc");
}

#[test]
fn test_from_file_resolves_host_dotfile_paths() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("myconfigs");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("mimic.toml");
    fs::write(
        &config_path,
        r#"
[hosts.work]
roles = ["work"]

[[hosts.work.dotfiles]]
source = "work/gitconfig"
target = "~/.gitconfig"
"#,
    )
    .unwrap();

    let config = Config::from_file(&config_path).unwrap();

    let work = config.hosts.get("work").unwrap();
    let resolved = &work.dotfiles[0].source;
    assert!(
        Path::new(resolved).is_absolute(),
        "Host dotfile relative source path should be resolved to absolute, got: {}",
        resolved
    );
    assert!(
        resolved.ends_with("myconfigs/work/gitconfig"),
        "Should resolve relative to config dir, got: {}",
        resolved
    );
}

#[test]
fn test_with_host_preserves_resolved_paths() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("myconfigs");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("mimic.toml");
    fs::write(
        &config_path,
        r#"
[[dotfiles]]
source = "common/zshrc"
target = "~/.zshrc"

[hosts.work]
roles = ["work"]

[[hosts.work.dotfiles]]
source = "work/gitconfig"
target = "~/.gitconfig"
"#,
    )
    .unwrap();

    let config = Config::from_file(&config_path).unwrap();
    let merged = config.with_host("work").unwrap();

    // Both base and host dotfiles should have resolved paths
    assert_eq!(merged.dotfiles.len(), 2);
    for dotfile in &merged.dotfiles {
        assert!(
            Path::new(&dotfile.source).is_absolute(),
            "Merged dotfile source should be absolute, got: {}",
            dotfile.source
        );
    }
}
