use mimic::linker::{create_symlink_with_resolution, ApplyToAllChoice};
use mimic::state::State;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_conflict_skip() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let target = temp_dir.path().join("target.txt");

    fs::write(&source, "source content").unwrap();
    fs::write(&target, "original target content").unwrap();

    let mut state = State::new();
    let mut apply_to_all = Some(ApplyToAllChoice::Skip);

    let result = create_symlink_with_resolution(&source, &target, &mut state, &mut apply_to_all);

    assert!(result.is_ok());
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "original target content"
    );
    assert!(!target.is_symlink());
    assert_eq!(state.dotfiles.len(), 0);
}

#[test]
fn test_conflict_overwrite() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let target = temp_dir.path().join("target.txt");

    fs::write(&source, "source content").unwrap();
    fs::write(&target, "original target content").unwrap();

    let mut state = State::new();
    let mut apply_to_all = Some(ApplyToAllChoice::Overwrite);

    let result = create_symlink_with_resolution(&source, &target, &mut state, &mut apply_to_all);

    assert!(result.is_ok());
    assert!(target.is_symlink());
    assert_eq!(fs::read_link(&target).unwrap(), source);
    assert_eq!(state.dotfiles.len(), 1);
    assert!(state.dotfiles[0].backup_path.is_none());

    let read_entries: Vec<_> = fs::read_dir(temp_dir.path()).unwrap().collect();
    let backup_exists = read_entries.iter().any(|e| {
        e.as_ref()
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".backup.")
    });
    assert!(!backup_exists, "No backup should be created for overwrite");
}

#[test]
fn test_conflict_creates_backup() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let target = temp_dir.path().join("target.txt");

    fs::write(&source, "source content").unwrap();
    fs::write(&target, "original target content").unwrap();

    let mut state = State::new();
    let mut apply_to_all = Some(ApplyToAllChoice::Backup);

    let result = create_symlink_with_resolution(&source, &target, &mut state, &mut apply_to_all);

    assert!(result.is_ok());
    assert!(target.is_symlink());
    assert_eq!(fs::read_link(&target).unwrap(), source);
    assert_eq!(state.dotfiles.len(), 1);
    assert!(state.dotfiles[0].backup_path.is_some());

    let backup_path = PathBuf::from(state.dotfiles[0].backup_path.as_ref().unwrap());
    assert!(backup_path.exists(), "Backup file should exist");
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "original target content",
        "Backup should contain original content"
    );

    assert!(
        backup_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("target.txt.backup."),
        "Backup should have correct naming format"
    );
}

#[test]
fn test_no_conflict_when_target_missing() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let target = temp_dir.path().join("target.txt");

    fs::write(&source, "source content").unwrap();

    let mut state = State::new();
    let mut apply_to_all = None;

    let result = create_symlink_with_resolution(&source, &target, &mut state, &mut apply_to_all);

    assert!(result.is_ok());
    assert!(target.is_symlink());
    assert_eq!(fs::read_link(&target).unwrap(), source);
    assert_eq!(state.dotfiles.len(), 1);
    assert!(state.dotfiles[0].backup_path.is_none());
}

#[test]
fn test_conflict_with_existing_symlink() {
    let temp_dir = TempDir::new().unwrap();
    let source1 = temp_dir.path().join("source1.txt");
    let source2 = temp_dir.path().join("source2.txt");
    let target = temp_dir.path().join("target.txt");

    fs::write(&source1, "source1 content").unwrap();
    fs::write(&source2, "source2 content").unwrap();

    std::os::unix::fs::symlink(&source1, &target).unwrap();

    let mut state = State::new();
    let mut apply_to_all = Some(ApplyToAllChoice::Overwrite);

    let result = create_symlink_with_resolution(&source2, &target, &mut state, &mut apply_to_all);

    assert!(result.is_ok());
    assert!(target.is_symlink());
    assert_eq!(fs::read_link(&target).unwrap(), source2);
    assert_eq!(state.dotfiles.len(), 1);
}

#[test]
fn test_backup_timestamp_format() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let target = temp_dir.path().join("vimrc");

    fs::write(&source, "source content").unwrap();
    fs::write(&target, "original vimrc content").unwrap();

    let mut state = State::new();
    let mut apply_to_all = Some(ApplyToAllChoice::Backup);

    let result = create_symlink_with_resolution(&source, &target, &mut state, &mut apply_to_all);

    assert!(result.is_ok());

    let backup_path = PathBuf::from(state.dotfiles[0].backup_path.as_ref().unwrap());
    let backup_name = backup_path.file_name().unwrap().to_string_lossy();

    assert!(
        backup_name.starts_with("vimrc.backup."),
        "Backup name '{}' should start with 'vimrc.backup.'",
        backup_name
    );

    let timestamp_part = backup_name.strip_prefix("vimrc.backup.").unwrap();
    assert_eq!(
        timestamp_part.len(),
        15,
        "Timestamp should be YYYYMMDD_HHMMSS format (15 chars)"
    );
}

#[test]
fn test_apply_to_all_consistency() {
    let temp_dir = TempDir::new().unwrap();

    let source1 = temp_dir.path().join("source1.txt");
    let target1 = temp_dir.path().join("target1.txt");
    let source2 = temp_dir.path().join("source2.txt");
    let target2 = temp_dir.path().join("target2.txt");

    fs::write(&source1, "source1").unwrap();
    fs::write(&target1, "existing1").unwrap();
    fs::write(&source2, "source2").unwrap();
    fs::write(&target2, "existing2").unwrap();

    let mut state = State::new();
    let mut apply_to_all = Some(ApplyToAllChoice::Backup);

    create_symlink_with_resolution(&source1, &target1, &mut state, &mut apply_to_all).unwrap();
    create_symlink_with_resolution(&source2, &target2, &mut state, &mut apply_to_all).unwrap();

    assert_eq!(state.dotfiles.len(), 2);
    assert!(state.dotfiles[0].backup_path.is_some());
    assert!(state.dotfiles[1].backup_path.is_some());

    assert!(target1.is_symlink());
    assert!(target2.is_symlink());
}
