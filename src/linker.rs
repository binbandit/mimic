use crate::error::LinkError;
use crate::state::{DotfileState, State};
use anyhow::Context;
use std::os::unix::fs::symlink;
use std::path::Path;

pub fn create_symlink(source: &Path, target: &Path, state: &mut State) -> anyhow::Result<()> {
    let expanded_source = expand_path(source)?;
    let expanded_target = expand_path(target)?;

    if !expanded_source.exists() {
        return Err(anyhow::anyhow!(
            "Source file does not exist: {}",
            expanded_source.display()
        ));
    }

    if expanded_target.exists() {
        return Err(LinkError::AlreadyExists {
            target: expanded_target.display().to_string(),
        }
        .into());
    }

    symlink(&expanded_source, &expanded_target).with_context(|| LinkError::SymlinkFailed {
        from: expanded_source.display().to_string(),
        to: expanded_target.display().to_string(),
        reason: "symlink system call failed".to_string(),
    })?;

    state.add_dotfile(DotfileState {
        source: expanded_source.to_string_lossy().to_string(),
        target: expanded_target.to_string_lossy().to_string(),
        backup_path: None,
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
