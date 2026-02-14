use mimic::linker::create_symlink;
use mimic::state::State;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_create_symlink() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.txt");
    let target = temp.path().join("target.txt");

    // Create source file
    fs::write(&source, "test content").unwrap();

    // Create symlink
    let mut state = State::new();
    create_symlink(&source, &target, &mut state).unwrap();

    // Verify symlink exists and points to source
    assert!(target.exists());
    assert!(target.is_symlink());
    let link_target = fs::read_link(&target).unwrap();
    assert_eq!(link_target, source);

    // Verify state was updated
    assert_eq!(state.dotfiles.len(), 1);
    assert_eq!(state.dotfiles[0].source, source.to_str().unwrap());
    assert_eq!(state.dotfiles[0].target, target.to_str().unwrap());
}

#[test]
fn test_path_expansion() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.txt");
    fs::write(&source, "test content").unwrap();

    let home_dir = std::env::var("HOME").unwrap();
    let unique_name = format!("test_target_{}.txt", std::process::id());
    let target_path = PathBuf::from(&home_dir).join(&unique_name);

    let target_with_tilde = format!("~/{}", unique_name);
    let mut state = State::new();

    let result = create_symlink(&source, &PathBuf::from(&target_with_tilde), &mut state);

    assert!(
        result.is_ok(),
        "Failed to create symlink: {:?}",
        result.err()
    );

    assert_eq!(state.dotfiles.len(), 1);
    let stored_target = &state.dotfiles[0].target;
    assert!(
        !stored_target.contains('~'),
        "Path should be expanded, not contain '~'"
    );
    assert_eq!(stored_target, target_path.to_str().unwrap());

    fs::remove_file(&target_path).ok();
}

#[test]
fn test_missing_source_error() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("nonexistent.txt");
    let target = temp.path().join("target.txt");

    let mut state = State::new();
    let result = create_symlink(&source, &target, &mut state);

    // Should fail with error
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("source") || err_msg.contains("exist"));

    // State should not be modified
    assert_eq!(state.dotfiles.len(), 0);
}

#[test]
fn test_target_exists_detection() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.txt");
    let target = temp.path().join("target.txt");

    // Create both files
    fs::write(&source, "source content").unwrap();
    fs::write(&target, "existing target").unwrap();

    let mut state = State::new();
    let result = create_symlink(&source, &target, &mut state);

    // Should fail because target exists
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exists") || err_msg.contains("target"),
        "Error message should mention target exists: {}",
        err_msg
    );

    // State should not be modified
    assert_eq!(state.dotfiles.len(), 0);
}

#[test]
fn test_state_integration() {
    let temp = TempDir::new().unwrap();
    let source1 = temp.path().join("source1.txt");
    let source2 = temp.path().join("source2.txt");
    let target1 = temp.path().join("target1.txt");
    let target2 = temp.path().join("target2.txt");

    fs::write(&source1, "content1").unwrap();
    fs::write(&source2, "content2").unwrap();

    let mut state = State::new();

    // Create multiple symlinks
    create_symlink(&source1, &target1, &mut state).unwrap();
    create_symlink(&source2, &target2, &mut state).unwrap();

    // Verify state tracks both
    assert_eq!(state.dotfiles.len(), 2);
    assert_eq!(state.dotfiles[0].source, source1.to_str().unwrap());
    assert_eq!(state.dotfiles[0].target, target1.to_str().unwrap());
    assert_eq!(state.dotfiles[1].source, source2.to_str().unwrap());
    assert_eq!(state.dotfiles[1].target, target2.to_str().unwrap());

    // Verify both symlinks work
    assert!(target1.is_symlink());
    assert!(target2.is_symlink());
}

#[test]
fn test_symlink_to_existing_symlink_fails() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.txt");
    let target = temp.path().join("target.txt");

    fs::write(&source, "content").unwrap();

    // Create a symlink manually
    symlink(&source, &target).unwrap();

    // Try to create symlink to same target
    let mut state = State::new();
    let result = create_symlink(&source, &target, &mut state);

    // Should fail because target exists (even though it's a symlink)
    assert!(result.is_err());
    assert_eq!(state.dotfiles.len(), 0);
}
