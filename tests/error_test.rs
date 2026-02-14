use mimic::error::*;
use std::io;

#[test]
fn test_config_error_display() {
    let err = ConfigError::NotFound("mimic.toml".to_string());
    assert_eq!(
        format!("{}", err),
        "Configuration file not found: mimic.toml"
    );
}

#[test]
fn test_config_error_parse() {
    let err = ConfigError::Parse {
        path: "mimic.toml".to_string(),
        details: "invalid TOML syntax".to_string(),
    };
    assert_eq!(
        format!("{}", err),
        "Failed to parse mimic.toml: invalid TOML syntax"
    );
}

#[test]
fn test_config_error_io() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let err = ConfigError::Io(io_err);
    assert!(format!("{}", err).contains("I/O error"));
}

#[test]
fn test_link_error_exists() {
    let err = LinkError::AlreadyExists {
        target: "/home/user/.vimrc".to_string(),
    };
    assert_eq!(
        format!("{}", err),
        "Link target already exists: /home/user/.vimrc"
    );
}

#[test]
fn test_link_error_symlink_failed() {
    let err = LinkError::SymlinkFailed {
        from: "/dotfiles/vimrc".to_string(),
        to: "/home/user/.vimrc".to_string(),
        reason: "permission denied".to_string(),
    };
    assert_eq!(
        format!("{}", err),
        "Failed to create symlink from /dotfiles/vimrc to /home/user/.vimrc: permission denied"
    );
}

#[test]
fn test_install_error_command_failed() {
    let err = InstallError::CommandFailed {
        command: "brew install neovim".to_string(),
        exit_code: 1,
        stderr: "Error: package not found".to_string(),
    };
    assert_eq!(
        format!("{}", err),
        "Command 'brew install neovim' failed with exit code 1: Error: package not found"
    );
}

#[test]
fn test_install_error_missing_condition() {
    let err = InstallError::MissingCondition {
        package: "neovim".to_string(),
        condition: "os = macos".to_string(),
    };
    assert_eq!(
        format!("{}", err),
        "Cannot install 'neovim': condition 'os = macos' not met"
    );
}

#[test]
fn test_state_error_serialization() {
    let err = StateError::Serialization("invalid JSON format".to_string());
    assert_eq!(
        format!("{}", err),
        "Failed to serialize state: invalid JSON format"
    );
}

#[test]
fn test_state_error_io() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err = StateError::Io(io_err);
    assert!(format!("{}", err).contains("I/O error"));
}

#[test]
fn test_error_from_conversions() {
    // Test that ConfigError can be converted From io::Error
    let io_err = io::Error::new(io::ErrorKind::NotFound, "test");
    let _config_err: ConfigError = io_err.into();

    // Test that StateError can be converted From io::Error
    let io_err2 = io::Error::new(io::ErrorKind::NotFound, "test");
    let _state_err: StateError = io_err2.into();
}
