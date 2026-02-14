use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub dotfiles: Vec<Dotfile>,

    #[serde(default)]
    pub packages: Packages,
}

#[derive(Debug, Deserialize)]
pub struct Dotfile {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Packages {
    #[serde(default)]
    pub homebrew: Vec<Package>,
}

#[derive(Debug, Deserialize)]
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
}
