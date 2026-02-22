use crate::config::{Config, Dotfile};
use crate::error::LinkError;
use crate::expand::expand_path;
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

/// Recursively copy a directory and its contents
fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create directory: {}", dst.display()))?;
    for entry in
        fs::read_dir(src).with_context(|| format!("Failed to read directory: {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("Failed to copy: {}", src_path.display()))?;
        }
    }
    Ok(())
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
        // For real directories, rename is atomic on the same filesystem
        fs::rename(target, &backup_path)
            .with_context(|| format!("Failed to create backup at {}", backup_path.display()))?;
    } else if target.is_symlink() && target.is_dir() {
        // Symlink to a directory: recursively copy the contents the symlink points to
        copy_dir_all(target, &backup_path)
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

/// Result of preparing a target for symlink creation.
enum PrepareResult {
    /// Target is ready; contains optional backup path string.
    Ready(Option<String>),
    /// Conflict was resolved by skipping.
    Skipped,
}

/// Shared logic for resolving conflicts, creating backups, removing old targets,
/// and ensuring parent directories exist. Used by both regular and template dotfiles.
fn prepare_target(
    target: &Path,
    link_source: &Path,
    apply_to_all: &mut Option<ApplyToAllChoice>,
) -> anyhow::Result<PrepareResult> {
    let mut backup_path_str = None;

    if target.exists() || target.is_symlink() {
        let resolution = resolve_conflict(target, link_source, apply_to_all)?;

        match effective_choice(&resolution) {
            EffectiveChoice::Skip => return Ok(PrepareResult::Skipped),
            EffectiveChoice::Overwrite => {
                remove_target(target)?;
            }
            EffectiveChoice::Backup => {
                // Only back up if the target has readable content (not a dangling symlink)
                if target.exists() {
                    let backup_path = backup_file(target)?;
                    backup_path_str = Some(backup_path.to_string_lossy().to_string());
                }
                // Directory backups use rename, so the target may already be gone
                if target.exists() || target.is_symlink() {
                    remove_target(target)?;
                }
            }
        }
    }

    // Ensure parent directories exist
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory: {}", parent.display())
            })?;
        }
    }

    Ok(PrepareResult::Ready(backup_path_str))
}

enum EffectiveChoice {
    Skip,
    Overwrite,
    Backup,
}

fn effective_choice(resolution: &ConflictResolution) -> EffectiveChoice {
    match resolution {
        ConflictResolution::Skip => EffectiveChoice::Skip,
        ConflictResolution::Overwrite => EffectiveChoice::Overwrite,
        ConflictResolution::Backup => EffectiveChoice::Backup,
        ConflictResolution::ApplyToAll(choice) => match choice {
            ApplyToAllChoice::Skip => EffectiveChoice::Skip,
            ApplyToAllChoice::Overwrite => EffectiveChoice::Overwrite,
            ApplyToAllChoice::Backup => EffectiveChoice::Backup,
        },
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

    let backup_path_str = match prepare_target(&expanded_target, &expanded_source, apply_to_all)? {
        PrepareResult::Skipped => return Ok(()),
        PrepareResult::Ready(bp) => bp,
    };

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

    let backup_path_str = match prepare_target(&target, &temp_path, apply_to_all)? {
        PrepareResult::Skipped => return Ok(()),
        PrepareResult::Ready(bp) => bp,
    };

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
