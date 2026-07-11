use crate::config::{Config, Dotfile};
use crate::expand::expand_path_str;
use crate::installer::HomebrewManager;
use crate::linker::rendered_path_for;
use crate::template::HostContext;
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
    Remove {
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
            Change::Remove {
                resource_type,
                description,
                reason,
            } => {
                let symbol = "-".red().bold();
                let type_label = match resource_type {
                    ResourceType::Dotfile => "dotfile",
                    ResourceType::Package => "package",
                };
                format!(
                    "{} {} {} ({})",
                    symbol,
                    type_label,
                    description.white(),
                    reason.red()
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
        let mut changes = self.diff_dotfiles(config, None)?;
        changes.extend(self.diff_packages(config)?);
        Ok(changes)
    }

    /// Diff only the dotfiles section. When `host` is provided, template
    /// dotfiles whose symlink is correct are additionally checked for content
    /// drift: the template is re-rendered and compared against the live
    /// rendered file, catching hand-edits and template/variable changes.
    pub fn diff_dotfiles(
        &self,
        config: &Config,
        host: Option<&HostContext>,
    ) -> anyhow::Result<Vec<Change>> {
        let mut changes = Vec::new();
        for dotfile in &config.dotfiles {
            let mut change = self.diff_dotfile(dotfile)?;
            if let (Change::AlreadyCorrect { description }, Some(host_ctx)) = (&change, host)
                && dotfile.is_template()
                && let Some(reason) = self.template_drift_reason(dotfile, config, host_ctx)
            {
                change = Change::Modify {
                    resource_type: ResourceType::Dotfile,
                    description: description.clone(),
                    reason,
                };
            }
            changes.push(change);
        }
        Ok(changes)
    }

    /// Diff only the packages section (homebrew + zerobrew).
    ///
    /// Installed-package lists are fetched once per manager instead of once
    /// per package — `brew list` costs a subprocess each time, so this is the
    /// difference between 2 and N spawns. A missing manager binary means
    /// nothing is installed.
    pub fn diff_packages(&self, config: &Config) -> anyhow::Result<Vec<Change>> {
        let mut changes = Vec::new();
        let normalized_packages = config.packages.normalized();

        let (want_formulae, want_casks) = normalized_packages
            .homebrew
            .iter()
            .fold((false, false), |(f, c), p| {
                (f || p.pkg_type != "cask", c || p.pkg_type == "cask")
            });
        let installed_formulae: Vec<String> = if want_formulae {
            self.homebrew
                .try_list_installed_formulae()?
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let installed_casks: Vec<String> = if want_casks {
            self.homebrew
                .try_list_installed_casks()?
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        for package in &normalized_packages.homebrew {
            let (installed, type_label) = if package.pkg_type == "cask" {
                (&installed_casks, "cask")
            } else {
                (&installed_formulae, "formula")
            };

            changes.push(if installed.iter().any(|p| p == &package.name) {
                Change::AlreadyCorrect {
                    description: format!("brew {}: {}", type_label, package.name),
                }
            } else {
                Change::Add {
                    resource_type: ResourceType::Package,
                    description: format!("{} ({})", package.name, type_label),
                }
            });
        }

        if !normalized_packages.zerobrew.is_empty() {
            let installed_zb = self.zerobrew.try_list_installed()?.unwrap_or_default();
            for package in &normalized_packages.zerobrew {
                changes.push(if installed_zb.iter().any(|p| p == &package.name) {
                    Change::AlreadyCorrect {
                        description: format!("zb: {}", package.name),
                    }
                } else {
                    Change::Add {
                        resource_type: ResourceType::Package,
                        description: format!("{} (zb)", package.name),
                    }
                });
            }
        }

        Ok(changes)
    }

    /// Compare the live rendered file against a fresh render of the template.
    /// Returns a human-readable drift reason, or None when in sync.
    fn template_drift_reason(
        &self,
        dotfile: &Dotfile,
        config: &Config,
        host: &HostContext,
    ) -> Option<String> {
        let source = expand_path(&dotfile.source).ok()?;
        let rendered_path = rendered_path_for(&source).ok()?;
        let live = fs::read_to_string(&rendered_path).ok()?;

        let fresh = match crate::template::render_file(&source, &config.variables, host) {
            Ok(fresh) => fresh,
            Err(e) => return Some(format!("template render failed: {}", e)),
        };

        if live == fresh {
            return None;
        }

        Some(
            "rendered content drifted (live file edited or template/variables changed)".to_string(),
        )
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
            // exists() follows symlinks, so a dangling link lands here too —
            // that's drift (the linked file vanished), not a fresh install.
            if expanded_target.is_symlink() {
                return Ok(Change::Modify {
                    resource_type: ResourceType::Dotfile,
                    description: format!("{}", expanded_target.display()),
                    reason: "broken symlink (points to a missing file)".to_string(),
                });
            }
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
