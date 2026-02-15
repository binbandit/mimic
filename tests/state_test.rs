use mimic::state::{DotfileState, PackageState, State};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_state_persistence() {
    // Create temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.toml");

    // Create a new state with some data
    let mut state = State::new();
    state.applied_commit = Some("abc123".to_string());
    state.add_dotfile(DotfileState {
        source: "~/.vimrc".to_string(),
        target: "/home/user/.vimrc".to_string(),
        backup_path: Some("/home/user/.vimrc.backup".to_string()),
        rendered_path: None,
    });
    state.add_package(PackageState {
        name: "git".to_string(),
        manager: "brew".to_string(),
    });

    // Save to file
    state.save(&state_path).unwrap();

    // Verify file exists
    assert!(state_path.exists());

    // Load from file
    let loaded_state = State::load(&state_path).unwrap();

    // Verify all data persisted correctly
    assert_eq!(loaded_state.applied_commit, Some("abc123".to_string()));
    assert_eq!(loaded_state.dotfiles.len(), 1);
    assert_eq!(loaded_state.dotfiles[0].source, "~/.vimrc");
    assert_eq!(loaded_state.packages.len(), 1);
    assert_eq!(loaded_state.packages[0].name, "git");
}

#[test]
fn test_state_missing_file() {
    // Try to load from non-existent path
    let path = PathBuf::from("/tmp/nonexistent_mimic_state_test.toml");

    // Should return empty state, not error
    let state = State::load(&path).unwrap();

    assert!(state.applied_commit.is_none());
    assert_eq!(state.dotfiles.len(), 0);
    assert_eq!(state.packages.len(), 0);
}

#[test]
fn test_state_add_remove_dotfile() {
    let mut state = State::new();

    // Add dotfile
    state.add_dotfile(DotfileState {
        source: "~/.bashrc".to_string(),
        target: "/home/user/.bashrc".to_string(),
        backup_path: None,
        rendered_path: None,
    });

    assert_eq!(state.dotfiles.len(), 1);

    // Remove dotfile
    state.remove_dotfile("~/.bashrc");

    assert_eq!(state.dotfiles.len(), 0);
}

#[test]
fn test_state_clear() {
    let mut state = State::new();
    state.applied_commit = Some("xyz789".to_string());
    state.add_dotfile(DotfileState {
        source: "~/.zshrc".to_string(),
        target: "/home/user/.zshrc".to_string(),
        backup_path: None,
        rendered_path: None,
    });

    // Clear should reset everything
    state.clear();

    assert!(state.applied_commit.is_none());
    assert_eq!(state.dotfiles.len(), 0);
    assert_eq!(state.packages.len(), 0);
}

#[test]
fn test_state_atomic_write() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.toml");

    let mut state = State::new();
    state.applied_commit = Some("atomic_test".to_string());

    // Save should be atomic (no partial writes)
    state.save(&state_path).unwrap();

    // Verify no .tmp files left behind
    let tmp_files: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "tmp"))
        .collect();

    assert_eq!(tmp_files.len(), 0, "Temporary files should be cleaned up");

    // Verify state file exists and is readable
    let loaded = State::load(&state_path).unwrap();
    assert_eq!(loaded.applied_commit, Some("atomic_test".to_string()));
}

#[test]
fn test_state_multiple_dotfiles() {
    let mut state = State::new();

    // Add multiple dotfiles
    for i in 0..5 {
        state.add_dotfile(DotfileState {
            source: format!("~/.config/file{}", i),
            target: format!("/home/user/.config/file{}", i),
            backup_path: None,
            rendered_path: None,
        });
    }

    assert_eq!(state.dotfiles.len(), 5);

    // Remove one
    state.remove_dotfile("~/.config/file2");

    assert_eq!(state.dotfiles.len(), 4);
    assert!(!state.dotfiles.iter().any(|d| d.source == "~/.config/file2"));
}
