use crate::config::{Config, Dotfile};
use crate::error::LinkError;
use crate::state::{DotfileState, State};
use crate::template::{render_file, HostContext};
use anyhow::Context;
use chrono::Local;
use colored::Colorize;
use dialoguer::Select;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// Strategy for resolving symlink conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Skip,
    Overwrite,
    Backup,
    ApplyToAll(ApplyToAllChoice),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyToAllChoice {
    Skip,
    Overwrite,
    Backup,
}

/// Create a backup of the target file or directory with timestamp suffix
fn backup_file(target: &Path) -> anyhow::Result<PathBuf> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!(
        "{}.backup.{}",
        target
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid target path"))?
            .to_string_lossy(),
        timestamp
    );
    let backup_path = target.with_file_name(backup_name);

    if target.is_dir() && !target.is_symlink() {
        // For directories, rename is atomic and works across the same filesystem
        fs::rename(target, &backup_path)
            .with_context(|| format!("Failed to create backup at {}", backup_path.display()))?;
    } else {
        fs::copy(target, &backup_path)
            .with_context(|| format!("Failed to create backup at {}", backup_path.display()))?;
    }

    Ok(backup_path)
}

/// Remove a target path, handling files, symlinks, and directories
fn remove_target(target: &Path) -> anyhow::Result<()> {
    if target.is_dir() && !target.is_symlink() {
        fs::remove_dir_all(target)
            .with_context(|| format!("Failed to remove directory: {}", target.display()))?;
    } else {
        fs::remove_file(target)
            .with_context(|| format!("Failed to remove file: {}", target.display()))?;
    }
    Ok(())
}

/// Resolve a symlink conflict interactively or with apply-to-all strategy
fn resolve_conflict(
    target: &Path,
    source: &Path,
    apply_to_all: &mut Option<ApplyToAllChoice>,
) -> anyhow::Result<ConflictResolution> {
    // If apply-to-all is set, use that choice
    if let Some(choice) = apply_to_all {
        return Ok(match choice {
            ApplyToAllChoice::Skip => ConflictResolution::Skip,
            ApplyToAllChoice::Overwrite => ConflictResolution::Overwrite,
            ApplyToAllChoice::Backup => ConflictResolution::Backup,
        });
    }

    let is_symlink_to_different = if target.is_symlink() {
        let current_source = fs::read_link(target)?;
        current_source != source
    } else {
        false
    };

    let message = if is_symlink_to_different {
        format!(
            "Target {} is a symlink to a different source. How do you want to proceed?",
            target.display()
        )
    } else {
        format!(
            "Target {} already exists. How do you want to proceed?",
            target.display()
        )
    };

    let choices = vec![
        "[s]kip - Leave existing file/symlink",
        "[o]verwrite - Replace with new symlink",
        "[b]ackup - Backup original, create symlink",
        "[a]pply to all remaining - Use this choice for all conflicts",
    ];

    let selection = Select::new()
        .with_prompt(&message)
        .items(&choices)
        .default(0)
        .interact()?;

    match selection {
        0 => Ok(ConflictResolution::Skip),
        1 => Ok(ConflictResolution::Overwrite),
        2 => Ok(ConflictResolution::Backup),
        3 => {
            let apply_choices = vec!["[s]kip all", "[o]verwrite all", "[b]ackup all"];

            let apply_selection = Select::new()
                .with_prompt("Select action to apply to all remaining conflicts")
                .items(&apply_choices)
                .default(0)
                .interact()?;

            let choice = match apply_selection {
                0 => ApplyToAllChoice::Skip,
                1 => ApplyToAllChoice::Overwrite,
                2 => ApplyToAllChoice::Backup,
                _ => unreachable!(),
            };

            *apply_to_all = Some(choice);

            Ok(ConflictResolution::ApplyToAll(choice))
        }
        _ => unreachable!(),
    }
}

pub fn create_symlink(source: &Path, target: &Path, state: &mut State) -> anyhow::Result<()> {
    create_symlink_with_resolution(source, target, state, &mut None)
}

pub fn create_symlink_with_resolution(
    source: &Path,
    target: &Path,
    state: &mut State,
    apply_to_all: &mut Option<ApplyToAllChoice>,
) -> anyhow::Result<()> {
    let expanded_source = expand_path(source)?;
    let expanded_target = expand_path(target)?;

    if !expanded_source.exists() {
        return Err(anyhow::anyhow!(
            "Source file does not exist: {}",
            expanded_source.display()
        ));
    }

    let mut backup_path_str = None;

    if expanded_target.exists() || expanded_target.is_symlink() {
        let resolution = resolve_conflict(&expanded_target, &expanded_source, apply_to_all)?;

        match resolution {
            ConflictResolution::Skip => {
                return Ok(());
            }
            ConflictResolution::Overwrite => {
                remove_target(&expanded_target)?;
            }
            ConflictResolution::Backup => {
                // Only back up if the target has readable content (not a dangling symlink)
                if expanded_target.exists() {
                    let backup_path = backup_file(&expanded_target)?;
                    backup_path_str = Some(backup_path.to_string_lossy().to_string());
                }
                // Directory backups use rename, so the target is already gone
                if expanded_target.exists() || expanded_target.is_symlink() {
                    remove_target(&expanded_target)?;
                }
            }
            ConflictResolution::ApplyToAll(choice) => match choice {
                ApplyToAllChoice::Skip => {
                    return Ok(());
                }
                ApplyToAllChoice::Overwrite => {
                    remove_target(&expanded_target)?;
                }
                ApplyToAllChoice::Backup => {
                    // Only back up if the target has readable content (not a dangling symlink)
                    if expanded_target.exists() {
                        let backup_path = backup_file(&expanded_target)?;
                        backup_path_str = Some(backup_path.to_string_lossy().to_string());
                    }
                    // Directory backups use rename, so the target is already gone
                    if expanded_target.exists() || expanded_target.is_symlink() {
                        remove_target(&expanded_target)?;
                    }
                }
            },
        }
    }

    symlink(&expanded_source, &expanded_target).with_context(|| LinkError::SymlinkFailed {
        from: expanded_source.display().to_string(),
        to: expanded_target.display().to_string(),
        reason: "symlink system call failed".to_string(),
    })?;

    state.add_dotfile(DotfileState {
        source: expanded_source.to_string_lossy().to_string(),
        target: expanded_target.to_string_lossy().to_string(),
        backup_path: backup_path_str,
        rendered_path: None,
    });

    Ok(())
}

fn expand_path(path: &Path) -> anyhow::Result<std::path::PathBuf> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Path contains invalid UTF-8: {}", path.display()))?;

    let expanded = shellexpand::full(path_str)
        .with_context(|| format!("Failed to expand path: {}", path_str))?;

    Ok(std::path::PathBuf::from(expanded.as_ref()))
}

pub fn apply_dotfile(
    dotfile: &Dotfile,
    config: &Config,
    host_context: &HostContext,
    state: &mut State,
    apply_to_all: &mut Option<ApplyToAllChoice>,
) -> anyhow::Result<()> {
    if dotfile.is_template() {
        apply_template_dotfile(dotfile, config, host_context, state, apply_to_all)
    } else {
        apply_regular_dotfile(dotfile, state, apply_to_all)
    }
}

fn apply_regular_dotfile(
    dotfile: &Dotfile,
    state: &mut State,
    apply_to_all: &mut Option<ApplyToAllChoice>,
) -> anyhow::Result<()> {
    let source = PathBuf::from(&dotfile.source);
    let target = PathBuf::from(&dotfile.target);
    create_symlink_with_resolution(&source, &target, state, apply_to_all)
}

fn apply_template_dotfile(
    dotfile: &Dotfile,
    config: &Config,
    host_context: &HostContext,
    state: &mut State,
    apply_to_all: &mut Option<ApplyToAllChoice>,
) -> anyhow::Result<()> {
    let source = expand_path(&PathBuf::from(&dotfile.source))?;
    let target = expand_path(&PathBuf::from(&dotfile.target))?;

    let rendered = render_file(&source, &config.variables, host_context)?;

    let rendered_dir = directories::BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .home_dir()
        .join(".mimic/rendered");

    std::fs::create_dir_all(&rendered_dir)?;

    let filename = source
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .trim_end_matches(".tmpl")
        .trim_end_matches(".hbs");
    let temp_path = rendered_dir.join(filename);

    std::fs::write(&temp_path, rendered)?;

    println!("  {} Rendered: {}", "→".bright_black(), temp_path.display());

    let mut backup_path_str = None;

    if target.exists() || target.is_symlink() {
        let resolution = resolve_conflict(&target, &temp_path, apply_to_all)?;

        match resolution {
            ConflictResolution::Skip => {
                return Ok(());
            }
            ConflictResolution::Overwrite => {
                remove_target(&target)?;
            }
            ConflictResolution::Backup => {
                // Only back up if the target has readable content (not a dangling symlink)
                if target.exists() {
                    let backup_path = backup_file(&target)?;
                    backup_path_str = Some(backup_path.to_string_lossy().to_string());
                }
                // Directory backups use rename, so the target is already gone
                if target.exists() || target.is_symlink() {
                    remove_target(&target)?;
                }
            }
            ConflictResolution::ApplyToAll(choice) => match choice {
                ApplyToAllChoice::Skip => {
                    return Ok(());
                }
                ApplyToAllChoice::Overwrite => {
                    remove_target(&target)?;
                }
                ApplyToAllChoice::Backup => {
                    // Only back up if the target has readable content (not a dangling symlink)
                    if target.exists() {
                        let backup_path = backup_file(&target)?;
                        backup_path_str = Some(backup_path.to_string_lossy().to_string());
                    }
                    // Directory backups use rename, so the target is already gone
                    if target.exists() || target.is_symlink() {
                        remove_target(&target)?;
                    }
                }
            },
        }
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }

    symlink(&temp_path, &target).with_context(|| LinkError::SymlinkFailed {
        from: temp_path.display().to_string(),
        to: target.display().to_string(),
        reason: "symlink system call failed".to_string(),
    })?;

    state.add_dotfile(DotfileState {
        source: source.to_string_lossy().to_string(),
        target: target.to_string_lossy().to_string(),
        backup_path: backup_path_str,
        rendered_path: Some(temp_path.to_string_lossy().to_string()),
    });

    Ok(())
}
