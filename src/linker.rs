use crate::error::LinkError;
use crate::state::{DotfileState, State};
use anyhow::Context;
use chrono::Local;
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

/// Create a backup of the target file with timestamp suffix
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

    fs::copy(target, &backup_path)
        .with_context(|| format!("Failed to create backup at {}", backup_path.display()))?;

    Ok(backup_path)
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

    // Check if target is a symlink pointing to a different source
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
            // Ask which action to apply to all
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

    if expanded_target.exists() {
        let resolution = resolve_conflict(&expanded_target, &expanded_source, apply_to_all)?;

        match resolution {
            ConflictResolution::Skip => {
                return Ok(());
            }
            ConflictResolution::Overwrite => {
                fs::remove_file(&expanded_target).with_context(|| {
                    format!(
                        "Failed to remove existing file: {}",
                        expanded_target.display()
                    )
                })?;
            }
            ConflictResolution::Backup => {
                let backup_path = backup_file(&expanded_target)?;
                backup_path_str = Some(backup_path.to_string_lossy().to_string());
                fs::remove_file(&expanded_target).with_context(|| {
                    format!(
                        "Failed to remove original file after backup: {}",
                        expanded_target.display()
                    )
                })?;
            }
            ConflictResolution::ApplyToAll(choice) => match choice {
                ApplyToAllChoice::Skip => {
                    return Ok(());
                }
                ApplyToAllChoice::Overwrite => {
                    fs::remove_file(&expanded_target).with_context(|| {
                        format!(
                            "Failed to remove existing file: {}",
                            expanded_target.display()
                        )
                    })?;
                }
                ApplyToAllChoice::Backup => {
                    let backup_path = backup_file(&expanded_target)?;
                    backup_path_str = Some(backup_path.to_string_lossy().to_string());
                    fs::remove_file(&expanded_target).with_context(|| {
                        format!(
                            "Failed to remove original file after backup: {}",
                            expanded_target.display()
                        )
                    })?;
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
