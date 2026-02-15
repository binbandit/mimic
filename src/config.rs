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
}

#[derive(Debug, Deserialize, Clone)]
pub struct Dotfile {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub template: bool,
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

        Ok(Config {
            variables: merged_vars,
            dotfiles: merged_dotfiles,
            packages: merged_packages,
            hosts: self.hosts.clone(), // Preserve for later use
        })
    }

    /// Get list of configured host names
    pub fn host_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.hosts.keys().cloned().collect();
        names.sort();
        names
    }
}
