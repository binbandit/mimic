use mimic::config::{Config, HostConfig, Packages};
use std::collections::HashMap;

#[test]
fn test_host_config_parsing() {
    let config_str = r#"
        [variables]
        editor = "nvim"
        
        [hosts.personal]
        roles = ["personal", "mac"]
        [hosts.personal.variables]
        email = "personal@example.com"
        
        [hosts.work]
        roles = ["work", "mac"]
        [hosts.work.variables]
        email = "work@corp.com"
    "#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert_eq!(config.hosts.len(), 2);
    assert!(config.hosts.contains_key("personal"));
    assert!(config.hosts.contains_key("work"));

    let personal = config.hosts.get("personal").unwrap();
    assert_eq!(personal.roles, vec!["personal", "mac"]);
    assert_eq!(
        personal.variables.get("email").unwrap(),
        "personal@example.com"
    );

    let work = config.hosts.get("work").unwrap();
    assert_eq!(work.roles, vec!["work", "mac"]);
    assert_eq!(work.variables.get("email").unwrap(), "work@corp.com");
}

#[test]
fn test_host_merge() {
    let mut base_vars = HashMap::new();
    base_vars.insert("editor".to_string(), "nvim".to_string());
    base_vars.insert("shell".to_string(), "fish".to_string());

    let mut host_vars = HashMap::new();
    host_vars.insert("email".to_string(), "personal@example.com".to_string());
    host_vars.insert("editor".to_string(), "vim".to_string());

    let mut hosts = HashMap::new();
    hosts.insert(
        "personal".to_string(),
        HostConfig {
            inherits: None,
            roles: vec!["personal".to_string()],
            variables: host_vars,
            dotfiles: vec![],
            packages: Packages::default(),
            hooks: vec![],
        },
    );

    let config = Config {
        variables: base_vars,
        dotfiles: vec![],
        packages: Packages::default(),
        hosts,
        hooks: vec![],
    };

    let merged = config.with_host("personal").unwrap();

    assert_eq!(merged.variables.get("editor").unwrap(), "vim");
    assert_eq!(merged.variables.get("shell").unwrap(), "fish");
    assert_eq!(
        merged.variables.get("email").unwrap(),
        "personal@example.com"
    );
}

#[test]
fn test_host_not_found() {
    let config = Config {
        variables: HashMap::new(),
        dotfiles: vec![],
        packages: Packages::default(),
        hosts: HashMap::new(),
        hooks: vec![],
    };

    let result = config.with_host("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_backward_compat_no_hosts() {
    let config_str = r#"
        [variables]
        editor = "nvim"
        
        [[dotfiles]]
        source = "vimrc"
        target = "~/.vimrc"
    "#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert!(config.hosts.is_empty());
    assert_eq!(config.variables.get("editor").unwrap(), "nvim");
    assert_eq!(config.dotfiles.len(), 1);
}

#[test]
fn test_empty_host_config() {
    let config_str = r#"
        [variables]
        editor = "nvim"
        
        [hosts.minimal]
    "#;

    let config: Config = toml::from_str(config_str).unwrap();
    let minimal = config.hosts.get("minimal").unwrap();

    assert!(minimal.roles.is_empty());
    assert!(minimal.variables.is_empty());
    assert!(minimal.dotfiles.is_empty());
    assert!(minimal.packages.homebrew.is_empty());
}

#[test]
fn test_host_names() {
    let config_str = r#"
        [hosts.alpha]
        [hosts.beta]
        [hosts.gamma]
    "#;

    let config: Config = toml::from_str(config_str).unwrap();
    let names = config.host_names();

    assert_eq!(names, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn test_host_dotfiles_merge() {
    let config_str = r#"
        [[dotfiles]]
        source = "global/gitconfig"
        target = "~/.gitconfig"
        
        [hosts.work]
        [[hosts.work.dotfiles]]
        source = "work/zshrc"
        target = "~/.zshrc"
        
        [[hosts.work.dotfiles]]
        source = "work/ssh_config"
        target = "~/.ssh/config"
    "#;

    let config: Config = toml::from_str(config_str).unwrap();
    let merged = config.with_host("work").unwrap();

    assert_eq!(merged.dotfiles.len(), 3);
    assert_eq!(merged.dotfiles[0].source, "global/gitconfig");
    assert_eq!(merged.dotfiles[1].source, "work/zshrc");
    assert_eq!(merged.dotfiles[2].source, "work/ssh_config");
}

#[test]
fn test_host_packages_merge() {
    let config_str = r#"
        [[packages.homebrew]]
        name = "git"
        type = "formula"
        
        [hosts.dev]
        [[hosts.dev.packages.homebrew]]
        name = "neovim"
        type = "formula"
        
        [[hosts.dev.packages.homebrew]]
        name = "docker"
        type = "cask"
    "#;

    let config: Config = toml::from_str(config_str).unwrap();
    let merged = config.with_host("dev").unwrap();

    assert_eq!(merged.packages.homebrew.len(), 3);
    assert_eq!(merged.packages.homebrew[0].name, "git");
    assert_eq!(merged.packages.homebrew[1].name, "neovim");
    assert_eq!(merged.packages.homebrew[2].name, "docker");
}

#[test]
fn test_host_with_roles() {
    let config_str = r#"
        [hosts.laptop]
        roles = ["personal", "portable", "mac"]
        
        [hosts.desktop]
        roles = ["work", "powerful", "linux"]
    "#;

    let config: Config = toml::from_str(config_str).unwrap();

    let laptop = config.hosts.get("laptop").unwrap();
    assert_eq!(laptop.roles, vec!["personal", "portable", "mac"]);

    let desktop = config.hosts.get("desktop").unwrap();
    assert_eq!(desktop.roles, vec!["work", "powerful", "linux"]);
}

#[test]
fn test_complex_multi_host_scenario() {
    let config_str = r#"
        [variables]
        editor = "nvim"
        shell = "fish"
        
        [[dotfiles]]
        source = "shared/gitconfig"
        target = "~/.gitconfig"
        
        [[packages.homebrew]]
        name = "git"
        type = "formula"
        
        [hosts.personal]
        roles = ["personal", "mac"]
        
        [hosts.personal.variables]
        email = "user@personal.com"
        openai_model = "gpt-5-nano"
        
        [[hosts.personal.dotfiles]]
        source = "personal/zshrc"
        target = "~/.zshrc"
        
        [[hosts.personal.packages.homebrew]]
        name = "spotify"
        type = "cask"
        
        [hosts.work]
        roles = ["work", "mac"]
        
        [hosts.work.variables]
        email = "user@corp.com"
        http_proxy = "http://localhost:3128"
        
        [[hosts.work.dotfiles]]
        source = "work/zshrc"
        target = "~/.zshrc"
        
        [[hosts.work.packages.homebrew]]
        name = "slack"
        type = "cask"
    "#;

    let config: Config = toml::from_str(config_str).unwrap();

    let personal = config.with_host("personal").unwrap();
    assert_eq!(personal.variables.get("editor").unwrap(), "nvim");
    assert_eq!(
        personal.variables.get("email").unwrap(),
        "user@personal.com"
    );
    assert_eq!(
        personal.variables.get("openai_model").unwrap(),
        "gpt-5-nano"
    );
    assert_eq!(personal.dotfiles.len(), 2);
    assert_eq!(personal.packages.homebrew.len(), 2);
    assert!(personal.packages.homebrew.iter().any(|p| p.name == "git"));
    assert!(personal
        .packages
        .homebrew
        .iter()
        .any(|p| p.name == "spotify"));

    let work = config.with_host("work").unwrap();
    assert_eq!(work.variables.get("editor").unwrap(), "nvim");
    assert_eq!(work.variables.get("email").unwrap(), "user@corp.com");
    assert_eq!(
        work.variables.get("http_proxy").unwrap(),
        "http://localhost:3128"
    );
    assert!(work.variables.get("openai_model").is_none());
    assert_eq!(work.dotfiles.len(), 2);
    assert_eq!(work.packages.homebrew.len(), 2);
    assert!(work.packages.homebrew.iter().any(|p| p.name == "git"));
    assert!(work.packages.homebrew.iter().any(|p| p.name == "slack"));
}

#[test]
fn test_host_variable_override() {
    let config_str = r#"
        [variables]
        editor = "nvim"
        theme = "dark"
        
        [hosts.special]
        [hosts.special.variables]
        editor = "emacs"
    "#;

    let config: Config = toml::from_str(config_str).unwrap();
    let merged = config.with_host("special").unwrap();

    assert_eq!(merged.variables.get("editor").unwrap(), "emacs");
    assert_eq!(merged.variables.get("theme").unwrap(), "dark");
}
