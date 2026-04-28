use crate::config::{Config, Dotfile};
use crate::expand::expand_path_str;
use crate::installer::HomebrewManager;
use crate::linker::rendered_path_for;
use crate::zerobrew::ZerobrewManager;
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
    zerobrew: ZerobrewManager,
}

impl DiffEngine {
    pub fn new() -> Self {
        Self {
            homebrew: HomebrewManager::new(),
            zerobrew: ZerobrewManager::new(),
        }
    }

    pub fn with_homebrew(homebrew: HomebrewManager) -> Self {
        Self {
            homebrew,
            zerobrew: ZerobrewManager::new(),
        }
    }

    pub fn diff(&self, config: &Config) -> anyhow::Result<Vec<Change>> {
        let mut changes = Vec::new();

        for dotfile in &config.dotfiles {
            let change = self.diff_dotfile(dotfile)?;
            changes.push(change);
        }

        let normalized_packages = config.packages.normalized();
        for package in &normalized_packages.homebrew {
            let change = self.diff_package(&package.name, &package.pkg_type)?;
            changes.push(change);
        }

        for package in &normalized_packages.zerobrew {
            let change = self.diff_zerobrew_package(&package.name)?;
            changes.push(change);
        }

        Ok(changes)
    }

    fn diff_dotfile(&self, dotfile: &Dotfile) -> anyhow::Result<Change> {
        let expanded_source = expand_path(&dotfile.source)?;
        let expanded_target = expand_path(&dotfile.target)?;

        if !expanded_source.exists() {
            return Ok(Change::Modify {
                resource_type: ResourceType::Dotfile,
                description: format!("{}", expanded_target.display()),
                reason: format!("source missing: {}", expanded_source.display()),
            });
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

        // For template dotfiles, the symlink should point to the rendered file
        // in ~/.mimic/rendered/, not the original source template.
        let expected_path = if dotfile.is_template() {
            rendered_path_for(&expanded_source)?
        } else {
            expanded_source.clone()
        };

        let canonical_expected = match fs::canonicalize(&expected_path) {
            Ok(path) => path,
            Err(_) => {
                return Ok(Change::Modify {
                    resource_type: ResourceType::Dotfile,
                    description: format!("{}", expanded_target.display()),
                    reason: format!("expected source missing: {}", expected_path.display()),
                });
            }
        };
        let canonical_current = if current_link_target.is_absolute() {
            fs::canonicalize(&current_link_target).unwrap_or(current_link_target.clone())
        } else {
            let link_parent = expanded_target.parent().unwrap_or(Path::new("."));
            let absolute_target = link_parent.join(&current_link_target);
            fs::canonicalize(&absolute_target).unwrap_or(current_link_target.clone())
        };

        if canonical_expected == canonical_current {
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

    fn diff_package(&self, name: &str, package_type: &str) -> anyhow::Result<Change> {
        let is_installed = self.homebrew.is_installed_any(name, package_type)?;
        let type_label = if package_type == "cask" {
            "cask"
        } else {
            "formula"
        };

        if is_installed {
            Ok(Change::AlreadyCorrect {
                description: format!("brew {}: {}", type_label, name),
            })
        } else {
            Ok(Change::Add {
                resource_type: ResourceType::Package,
                description: format!("{} ({})", name, type_label),
            })
        }
    }

    fn diff_zerobrew_package(&self, name: &str) -> anyhow::Result<Change> {
        let is_installed = self.zerobrew.is_installed(name)?;

        if is_installed {
            Ok(Change::AlreadyCorrect {
                description: format!("zb: {}", name),
            })
        } else {
            Ok(Change::Add {
                resource_type: ResourceType::Package,
                description: format!("{} (zb)", name),
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
    expand_path_str(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn test_template_diff_missing_rendered_file_returns_modify_not_error() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("mimic_diff_test_{}", unique));
        fs::create_dir_all(&base).unwrap();

        let source = base.join("template.conf.hbs");
        let target = base.join("target.conf");
        let actual = base.join("actual.conf");

        fs::write(&source, "name={{name}}\n").unwrap();
        fs::write(&actual, "rendered=true\n").unwrap();
        symlink(&actual, &target).unwrap();

        let dotfile = Dotfile {
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
            template: false,
            only_roles: None,
            skip_roles: None,
        };

        let engine = DiffEngine::new();
        let change = engine.diff_dotfile(&dotfile).unwrap();

        match change {
            Change::Modify { reason, .. } => {
                assert!(reason.contains("expected source missing:"));
            }
            other => panic!("expected Modify, got {:?}", other),
        }

        let _ = fs::remove_file(&target);
        let _ = fs::remove_file(&actual);
        let _ = fs::remove_file(&source);
        let _ = fs::remove_dir_all(&base);
    }
}
