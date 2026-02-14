use mimic::config::Config;

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
