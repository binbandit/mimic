use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotfileState {
    pub source: String,
    pub target: String,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageState {
    pub name: String,
    pub manager: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub applied_commit: Option<String>,
    pub applied_at: DateTime<Utc>,
    pub dotfiles: Vec<DotfileState>,
    pub packages: Vec<PackageState>,
}

impl State {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            applied_commit: None,
            applied_at: Utc::now(),
            dotfiles: Vec::new(),
            packages: Vec::new(),
        }
    }

    /// Load state from file. Returns empty state if file doesn't exist.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let state: State = toml::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(state)
    }

    /// Save state to file using atomic write pattern
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let temp_path = path.with_extension("tmp");

        let mut temp_file = fs::File::create(&temp_path)?;
        temp_file.write_all(content.as_bytes())?;
        temp_file.sync_all()?;
        drop(temp_file);

        fs::rename(&temp_path, path)?;

        Ok(())
    }

    /// Add a dotfile to the state
    pub fn add_dotfile(&mut self, dotfile: DotfileState) {
        self.dotfiles.push(dotfile);
        self.applied_at = Utc::now();
    }

    /// Add a package to the state
    pub fn add_package(&mut self, package: PackageState) {
        self.packages.push(package);
        self.applied_at = Utc::now();
    }

    /// Remove a dotfile by source path
    pub fn remove_dotfile(&mut self, source: &str) {
        self.dotfiles.retain(|d| d.source != source);
        self.applied_at = Utc::now();
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.applied_commit = None;
        self.dotfiles.clear();
        self.packages.clear();
        self.applied_at = Utc::now();
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_new() {
        let state = State::new();
        assert!(state.applied_commit.is_none());
        assert_eq!(state.dotfiles.len(), 0);
        assert_eq!(state.packages.len(), 0);
    }

    #[test]
    fn test_add_operations() {
        let mut state = State::new();

        state.add_dotfile(DotfileState {
            source: "test".to_string(),
            target: "test_target".to_string(),
            backup_path: None,
        });

        state.add_package(PackageState {
            name: "test_pkg".to_string(),
            manager: "brew".to_string(),
        });

        assert_eq!(state.dotfiles.len(), 1);
        assert_eq!(state.packages.len(), 1);
    }
}
