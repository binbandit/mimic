use crate::config::Config;
use crate::installer::HomebrewManager;
use colored::Colorize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Add {
        resource_type: ResourceType,
        description: String,
    },
    Modify {
        resource_type: ResourceType,
        description: String,
        reason: String,
    },
    AlreadyCorrect {
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Dotfile,
    Package,
}

impl Change {
    pub fn format(&self) -> String {
        match self {
            Change::Add {
                resource_type,
                description,
            } => {
                let symbol = "+".green().bold();
                let type_label = match resource_type {
                    ResourceType::Dotfile => "dotfile",
                    ResourceType::Package => "package",
                };
                format!("{} {} {}", symbol, type_label, description.white())
            }
            Change::Modify {
                resource_type,
                description,
                reason,
            } => {
                let symbol = "~".yellow().bold();
                let type_label = match resource_type {
                    ResourceType::Dotfile => "dotfile",
                    ResourceType::Package => "package",
                };
                format!(
                    "{} {} {} ({})",
                    symbol,
                    type_label,
                    description.white(),
                    reason.yellow()
                )
            }
            Change::AlreadyCorrect { description } => {
                let symbol = "✓".bright_black();
                format!("{} {}", symbol, description.bright_black())
            }
        }
    }
}

pub struct DiffEngine {
    homebrew: HomebrewManager,
}

impl DiffEngine {
    pub fn new() -> Self {
        Self {
            homebrew: HomebrewManager::new(),
        }
    }

    pub fn with_homebrew(homebrew: HomebrewManager) -> Self {
        Self { homebrew }
    }

    pub fn diff(&self, config: &Config) -> anyhow::Result<Vec<Change>> {
        let mut changes = Vec::new();

        for dotfile in &config.dotfiles {
            let change = self.diff_dotfile(&dotfile.source, &dotfile.target)?;
            changes.push(change);
        }

        for package in &config.packages.homebrew {
            let change = self.diff_package(&package.name)?;
            changes.push(change);
        }

        Ok(changes)
    }

    fn diff_dotfile(&self, source: &str, target: &str) -> anyhow::Result<Change> {
        let expanded_source = expand_path(source)?;
        let expanded_target = expand_path(target)?;

        if !expanded_source.exists() {
            return Err(anyhow::anyhow!(
                "Source file does not exist: {}",
                expanded_source.display()
            ));
        }

        if !expanded_target.exists() {
            return Ok(Change::Add {
                resource_type: ResourceType::Dotfile,
                description: format!(
                    "{} → {}",
                    expanded_target.display(),
                    expanded_source.display()
                ),
            });
        }

        let target_metadata = fs::symlink_metadata(&expanded_target)?;

        if !target_metadata.is_symlink() {
            return Ok(Change::Modify {
                resource_type: ResourceType::Dotfile,
                description: format!("{}", expanded_target.display()),
                reason: "exists but is not a symlink".to_string(),
            });
        }

        let current_link_target = fs::read_link(&expanded_target)?;

        let canonical_source = fs::canonicalize(&expanded_source)?;
        let canonical_current = if current_link_target.is_absolute() {
            fs::canonicalize(&current_link_target).unwrap_or(current_link_target.clone())
        } else {
            let link_parent = expanded_target.parent().unwrap_or(Path::new("."));
            let absolute_target = link_parent.join(&current_link_target);
            fs::canonicalize(&absolute_target).unwrap_or(current_link_target.clone())
        };

        if canonical_source == canonical_current {
            Ok(Change::AlreadyCorrect {
                description: format!("{}", expanded_target.display()),
            })
        } else {
            Ok(Change::Modify {
                resource_type: ResourceType::Dotfile,
                description: format!("{}", expanded_target.display()),
                reason: format!("points to wrong target: {}", current_link_target.display()),
            })
        }
    }

    fn diff_package(&self, name: &str) -> anyhow::Result<Change> {
        let is_installed = self.homebrew.is_installed(name)?;

        if is_installed {
            Ok(Change::AlreadyCorrect {
                description: format!("brew package: {}", name),
            })
        } else {
            Ok(Change::Add {
                resource_type: ResourceType::Package,
                description: name.to_string(),
            })
        }
    }
}

impl Default for DiffEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn expand_path(path: &str) -> anyhow::Result<std::path::PathBuf> {
    let expanded = shellexpand::full(path)
        .map_err(|e| anyhow::anyhow!("Failed to expand path '{}': {}", path, e))?;
    Ok(std::path::PathBuf::from(expanded.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_format_add() {
        let change = Change::Add {
            resource_type: ResourceType::Dotfile,
            description: "~/.vimrc → dotfiles/vimrc".to_string(),
        };
        let formatted = change.format();
        assert!(formatted.contains("+"));
        assert!(formatted.contains("dotfile"));
    }

    #[test]
    fn test_change_format_modify() {
        let change = Change::Modify {
            resource_type: ResourceType::Dotfile,
            description: "~/.zshrc".to_string(),
            reason: "points to wrong target".to_string(),
        };
        let formatted = change.format();
        assert!(formatted.contains("~"));
        assert!(formatted.contains("wrong target"));
    }

    #[test]
    fn test_change_format_already_correct() {
        let change = Change::AlreadyCorrect {
            description: "~/.bashrc".to_string(),
        };
        let formatted = change.format();
        assert!(formatted.contains("✓"));
    }

    #[test]
    fn test_expand_path_with_tilde() {
        let result = expand_path("~/test");
        assert!(result.is_ok());
        let expanded = result.unwrap();
        assert!(!expanded.to_string_lossy().contains("~"));
    }
}
