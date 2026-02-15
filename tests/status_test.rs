use mimic::state::{DotfileState, PackageState, State};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test state file with dotfiles and packages
fn create_test_state(state_path: &PathBuf) -> anyhow::Result<State> {
    let mut state = State::new();

    state.add_dotfile(DotfileState {
        source: "/tmp/test_source1".to_string(),
        target: "/tmp/test_target1".to_string(),
        backup_path: None,
    });

    state.add_dotfile(DotfileState {
        source: "/tmp/test_source2".to_string(),
        target: "/tmp/test_target2".to_string(),
        backup_path: None,
    });

    state.add_package(PackageState {
        name: "test_package".to_string(),
        manager: "brew".to_string(),
    });

    state.save(state_path)?;
    Ok(state)
}

#[test]
fn test_status_detects_broken_symlink() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let state_path = temp_dir.path().join("state.toml");

    // Create state file
    let mut state = State::new();

    let source = temp_dir.path().join("source.txt");
    let target = temp_dir.path().join("target.txt");
    fs::write(&source, "test content")?;

    // Create symlink
    symlink(&source, &target)?;

    // Add to state
    state.add_dotfile(DotfileState {
        source: source.to_string_lossy().to_string(),
        target: target.to_string_lossy().to_string(),
        backup_path: None,
    });
    state.save(&state_path)?;

    // Now delete the symlink to simulate drift
    fs::remove_file(&target)?;

    // Status check should detect missing symlink
    let loaded_state = State::load(&state_path)?;
    assert_eq!(loaded_state.dotfiles.len(), 1);

    let dotfile = &loaded_state.dotfiles[0];
    let target_path = PathBuf::from(&dotfile.target);
    assert!(!target_path.exists(), "Symlink should be missing");

    Ok(())
}

#[test]
fn test_status_detects_wrong_symlink_target() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let state_path = temp_dir.path().join("state.toml");

    // Create state file
    let mut state = State::new();

    let source = temp_dir.path().join("source.txt");
    let wrong_source = temp_dir.path().join("wrong_source.txt");
    let target = temp_dir.path().join("target.txt");

    fs::write(&source, "test content")?;
    fs::write(&wrong_source, "wrong content")?;

    // Create symlink to wrong source
    symlink(&wrong_source, &target)?;

    // Add to state with correct source
    state.add_dotfile(DotfileState {
        source: source.to_string_lossy().to_string(),
        target: target.to_string_lossy().to_string(),
        backup_path: None,
    });
    state.save(&state_path)?;

    // Status check should detect wrong target
    let loaded_state = State::load(&state_path)?;
    let dotfile = &loaded_state.dotfiles[0];
    let target_path = PathBuf::from(&dotfile.target);

    assert!(target_path.is_symlink(), "Target should be a symlink");

    let actual_target = fs::read_link(&target_path)?;
    let expected_source = PathBuf::from(&dotfile.source);

    assert_ne!(
        actual_target, expected_source,
        "Symlink should point to wrong target"
    );

    Ok(())
}

#[test]
fn test_status_detects_all_in_sync() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let state_path = temp_dir.path().join("state.toml");

    // Create state file
    let mut state = State::new();

    let source = temp_dir.path().join("source.txt");
    let target = temp_dir.path().join("target.txt");

    fs::write(&source, "test content")?;
    symlink(&source, &target)?;

    // Add to state
    state.add_dotfile(DotfileState {
        source: source.to_string_lossy().to_string(),
        target: target.to_string_lossy().to_string(),
        backup_path: None,
    });
    state.save(&state_path)?;

    // Status check should detect everything in sync
    let loaded_state = State::load(&state_path)?;
    let dotfile = &loaded_state.dotfiles[0];
    let target_path = PathBuf::from(&dotfile.target);

    assert!(target_path.is_symlink(), "Target should be a symlink");

    let actual_target = fs::read_link(&target_path)?;
    let expected_source = PathBuf::from(&dotfile.source);

    assert_eq!(
        actual_target, expected_source,
        "Symlink should point to correct target"
    );

    Ok(())
}

#[test]
fn test_status_with_empty_state() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let state_path = temp_dir.path().join("state.toml");

    // Create empty state
    let state = State::new();
    state.save(&state_path)?;

    let loaded_state = State::load(&state_path)?;
    assert_eq!(loaded_state.dotfiles.len(), 0);
    assert_eq!(loaded_state.packages.len(), 0);

    Ok(())
}

#[test]
fn test_status_with_nonexistent_state_file() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let state_path = temp_dir.path().join("nonexistent.toml");

    // Should return empty state without error
    let state = State::load(&state_path)?;
    assert_eq!(state.dotfiles.len(), 0);
    assert_eq!(state.packages.len(), 0);

    Ok(())
}
