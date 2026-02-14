use std::io;
use thiserror::Error;

pub type Result<T> = anyhow::Result<T>;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    NotFound(String),

    #[error("Failed to parse {path}: {details}")]
    Parse { path: String, details: String },

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum LinkError {
    #[error("Link target already exists: {target}")]
    AlreadyExists { target: String },

    #[error("Failed to create symlink from {from} to {to}: {reason}")]
    SymlinkFailed {
        from: String,
        to: String,
        reason: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum InstallError {
    #[error("Command '{command}' failed with exit code {exit_code}: {stderr}")]
    CommandFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    #[error("Cannot install '{package}': condition '{condition}' not met")]
    MissingCondition { package: String, condition: String },

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum StateError {
    #[error("Failed to serialize state: {0}")]
    Serialization(String),

    #[error("Failed to deserialize state: {0}")]
    Deserialization(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}
