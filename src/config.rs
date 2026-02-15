use crate::hooks::Hook;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub dotfiles: Vec<Dotfile>,

    #[serde(default)]
    pub packages: Packages,

    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,

    #[serde(default)]
    pub hooks: Vec<Hook>,

    #[serde(default)]
    pub secrets: HashMap<String, SecretMetadata>,

    #[serde(default)]
    pub mise: MiseSection,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct HostConfig {
    #[serde(default)]
    pub inherits: Option<String>,

    #[serde(default)]
    pub roles: Vec<String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub dotfiles: Vec<Dotfile>,

    #[serde(default)]
    pub packages: Packages,

    #[serde(default)]
    pub hooks: Vec<Hook>,

    #[serde(default)]
    pub secrets: HashMap<String, SecretMetadata>,

    #[serde(default)]
    pub mise: MiseSection,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecretMetadata {
    pub description: Option<String>,
    pub env_var: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MiseSection {
    #[serde(default)]
    pub tools: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Dotfile {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub template: bool,
    #[serde(default)]
    pub only_roles: Option<Vec<String>>,
    #[serde(default)]
    pub skip_roles: Option<Vec<String>>,
}

impl Dotfile {
    /// Check if this dotfile should be treated as a template
    pub fn is_template(&self) -> bool {
        self.template || self.source.ends_with(".tmpl") || self.source.ends_with(".hbs")
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Packages {
    #[serde(default)]
    pub homebrew: Vec<Package>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Package {
    pub name: String,

    #[serde(rename = "type")]
    pub pkg_type: String,

    #[serde(default)]
    pub only_roles: Option<Vec<String>>,

    #[serde(default)]
    pub skip_roles: Option<Vec<String>>,
}

/// Check if a resource should be applied based on role filtering
///
/// # Arguments
/// * `only_roles` - If set, resource applies ONLY if host has at least one matching role
/// * `skip_roles` - If set, resource is skipped if host has any matching role
/// * `host_roles` - The roles assigned to the current host
///
/// # Returns
/// * `true` if the resource should be applied
/// * `false` if the resource should be skipped
///
/// # Logic
/// 1. If `skip_roles` matches any host role, return false (skip)
/// 2. If `only_roles` is set and non-empty, return true only if at least one role matches
/// 3. If `only_roles` is empty or None, return true (no restrictions)
pub fn should_apply_for_roles(
    only_roles: &Option<Vec<String>>,
    skip_roles: &Option<Vec<String>>,
    host_roles: &[String],
) -> bool {
    // If skip_roles matches, don't apply
    if let Some(skip) = skip_roles {
        if skip.iter().any(|role| host_roles.contains(role)) {
            return false;
        }
    }

    // If only_roles is set, must match at least one
    if let Some(only) = only_roles {
        if only.is_empty() {
            return true; // Empty only_roles means apply to all
        }
        return only.iter().any(|role| host_roles.contains(role));
    }

    // No restrictions = apply
    true
}

impl Config {
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let config: Config =
            toml::from_str(content).map_err(|e| anyhow::anyhow!("TOML parse error: {}", e))?;
        Ok(config)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content =
            fs::read_to_string(path.as_ref()).map_err(|e| anyhow::anyhow!("IO error: {}", e))?;
        Self::from_str(&content)
    }

    /// Merge the base config with a specific host configuration
    pub fn with_host(&self, host_name: &str) -> anyhow::Result<Config> {
        let host = self
            .hosts
            .get(host_name)
            .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in config", host_name))?;

        // Deep merge: base + host overrides
        let mut merged_vars = self.variables.clone();
        for (key, value) in &host.variables {
            merged_vars.insert(key.clone(), value.clone());
        }

        let mut merged_dotfiles = self.dotfiles.clone();
        merged_dotfiles.extend(host.dotfiles.clone());

        let mut merged_packages = self.packages.clone();
        merged_packages
            .homebrew
            .extend(host.packages.homebrew.clone());

        let mut merged_hooks = self.hooks.clone();
        merged_hooks.extend(host.hooks.clone());

        let mut merged_secrets = self.secrets.clone();
        for (key, value) in &host.secrets {
            merged_secrets.insert(key.clone(), value.clone());
        }

        let mut merged_mise = self.mise.clone();
        for (key, value) in &host.mise.tools {
            merged_mise.tools.insert(key.clone(), value.clone());
        }

        Ok(Config {
            variables: merged_vars,
            dotfiles: merged_dotfiles,
            packages: merged_packages,
            hosts: self.hosts.clone(),
            hooks: merged_hooks,
            secrets: merged_secrets,
            mise: merged_mise,
        })
    }

    /// Get list of configured host names
    pub fn host_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.hosts.keys().cloned().collect();
        names.sort();
        names
    }
}
