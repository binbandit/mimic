use mimic::config::{Config, Dotfile, Package, Packages};
use mimic::diff::{Change, DiffEngine};
use mimic::installer::HomebrewManager;
use std::fs;
use std::os::unix::fs::symlink;
use tempfile::TempDir;

#[test]
fn test_diff_detects_new_dotfile() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source_vimrc");
    let target = temp.path().join("target_vimrc");

    // Create source file
    fs::write(&source, "source content").unwrap();

    let dotfile = Dotfile {
        source: source.to_str().unwrap().to_string(),
        target: target.to_str().unwrap().to_string(),
    };

    let config = Config {
        variables: Default::default(),
        dotfiles: vec![dotfile],
        packages: Packages::default(),
    };

    let engine = DiffEngine::new();
    let changes = engine.diff(&config).unwrap();

    // Should detect that symlink needs to be created
    assert_eq!(changes.len(), 1);
    match &changes[0] {
        Change::Add { description, .. } => {
            assert!(description.contains("source_vimrc"));
            assert!(description.contains("target_vimrc"));
        }
        _ => panic!("Expected Change::Add, got {:?}", changes[0]),
    }
}

#[test]
fn test_diff_symlink_already_correct() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source_bashrc");
    let target = temp.path().join("target_bashrc");

    // Create source file and correct symlink
    fs::write(&source, "bash config").unwrap();
    symlink(&source, &target).unwrap();

    let dotfile = Dotfile {
        source: source.to_str().unwrap().to_string(),
        target: target.to_str().unwrap().to_string(),
    };

    let config = Config {
        variables: Default::default(),
        dotfiles: vec![dotfile],
        packages: Packages::default(),
    };

    let engine = DiffEngine::new();
    let changes = engine.diff(&config).unwrap();

    // Should detect that symlink is already correct
    assert_eq!(changes.len(), 1);
    match &changes[0] {
        Change::AlreadyCorrect { description } => {
            assert!(description.contains("target_bashrc"));
        }
        _ => panic!("Expected Change::AlreadyCorrect, got {:?}", changes[0]),
    }
}

#[test]
fn test_diff_wrong_symlink_target() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source_zshrc");
    let wrong_source = temp.path().join("wrong_source");
    let target = temp.path().join("target_zshrc");

    // Create both files and symlink to wrong source
    fs::write(&source, "correct zsh config").unwrap();
    fs::write(&wrong_source, "wrong config").unwrap();
    symlink(&wrong_source, &target).unwrap();

    let dotfile = Dotfile {
        source: source.to_str().unwrap().to_string(),
        target: target.to_str().unwrap().to_string(),
    };

    let config = Config {
        variables: Default::default(),
        dotfiles: vec![dotfile],
        packages: Packages::default(),
    };

    let engine = DiffEngine::new();
    let changes = engine.diff(&config).unwrap();

    // Should detect that symlink points to wrong target
    assert_eq!(changes.len(), 1);
    match &changes[0] {
        Change::Modify { description, .. } => {
            assert!(description.contains("target_zshrc"));
        }
        _ => panic!("Expected Change::Modify, got {:?}", changes[0]),
    }
}

#[test]
fn test_diff_target_is_regular_file() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source_gitconfig");
    let target = temp.path().join("target_gitconfig");

    // Create both as regular files (target is not a symlink)
    fs::write(&source, "git config").unwrap();
    fs::write(&target, "old git config").unwrap();

    let dotfile = Dotfile {
        source: source.to_str().unwrap().to_string(),
        target: target.to_str().unwrap().to_string(),
    };

    let config = Config {
        variables: Default::default(),
        dotfiles: vec![dotfile],
        packages: Packages::default(),
    };

    let engine = DiffEngine::new();
    let changes = engine.diff(&config).unwrap();

    // Should detect that target exists but is not a symlink
    assert_eq!(changes.len(), 1);
    match &changes[0] {
        Change::Modify { description, .. } => {
            assert!(description.contains("target_gitconfig"));
        }
        _ => panic!("Expected Change::Modify, got {:?}", changes[0]),
    }
}

#[test]
fn test_diff_package_not_installed() {
    let package = Package {
        name: "mimic_test_package_never_installed".to_string(),
        pkg_type: "formula".to_string(),
    };

    let config = Config {
        variables: Default::default(),
        dotfiles: vec![],
        packages: Packages {
            homebrew: vec![package],
        },
    };

    let homebrew = HomebrewManager::new();
    let engine = DiffEngine::with_homebrew(homebrew);
    let changes = engine.diff(&config).unwrap();

    // Should detect package needs to be installed
    assert_eq!(changes.len(), 1);
    match &changes[0] {
        Change::Add { description, .. } => {
            assert!(description.contains("mimic_test_package_never_installed"));
        }
        _ => panic!("Expected Change::Add, got {:?}", changes[0]),
    }
}

#[test]
fn test_diff_multiple_changes() {
    let temp = TempDir::new().unwrap();
    let source1 = temp.path().join("source1");
    let target1 = temp.path().join("target1");
    let source2 = temp.path().join("source2");
    let target2 = temp.path().join("target2");

    // Setup: one new dotfile, one already correct
    fs::write(&source1, "content1").unwrap();
    fs::write(&source2, "content2").unwrap();
    symlink(&source2, &target2).unwrap();

    let config = Config {
        variables: Default::default(),
        dotfiles: vec![
            Dotfile {
                source: source1.to_str().unwrap().to_string(),
                target: target1.to_str().unwrap().to_string(),
            },
            Dotfile {
                source: source2.to_str().unwrap().to_string(),
                target: target2.to_str().unwrap().to_string(),
            },
        ],
        packages: Packages::default(),
    };

    let engine = DiffEngine::new();
    let changes = engine.diff(&config).unwrap();

    // Should have 2 changes: 1 Add, 1 AlreadyCorrect
    assert_eq!(changes.len(), 2);

    let add_count = changes
        .iter()
        .filter(|c| matches!(c, Change::Add { .. }))
        .count();
    let correct_count = changes
        .iter()
        .filter(|c| matches!(c, Change::AlreadyCorrect { .. }))
        .count();

    assert_eq!(add_count, 1);
    assert_eq!(correct_count, 1);
}

#[test]
fn test_diff_pretty_format() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("vimrc");
    let target = temp.path().join(".vimrc");

    fs::write(&source, "vim config").unwrap();

    let dotfile = Dotfile {
        source: source.to_str().unwrap().to_string(),
        target: target.to_str().unwrap().to_string(),
    };

    let config = Config {
        variables: Default::default(),
        dotfiles: vec![dotfile],
        packages: Packages::default(),
    };

    let engine = DiffEngine::new();
    let changes = engine.diff(&config).unwrap();

    assert_eq!(changes.len(), 1);
    let formatted = changes[0].format();

    // Should contain colored output (checking for presence of arrows/symbols)
    assert!(formatted.contains("→") || formatted.contains("+"));
}
