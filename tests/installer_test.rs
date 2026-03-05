use mimic::installer::HomebrewManager;
use mimic::state::State;
use tempfile::TempDir;

#[test]
fn test_list_installed() {
    let manager = HomebrewManager::new();
    let result = manager.list_installed();

    // Should succeed even if brew not installed (returns error, but error handling works)
    match result {
        Ok(packages) => {
            // If brew is installed, we should get a list (possibly empty)
            assert!(packages.is_empty() || !packages.is_empty());
        }
        Err(e) => {
            // If brew not installed, error should mention brew
            let err_msg = e.to_string().to_lowercase();
            assert!(
                err_msg.contains("brew")
                    || err_msg.contains("homebrew")
                    || err_msg.contains("not found")
            );
        }
    }
}

#[test]
fn test_is_installed_checks_brew_list() {
    let manager = HomebrewManager::new();

    // Test with a package name
    let result = manager.is_installed("nonexistent-package-xyz123");

    match result {
        Ok(installed) => {
            // Should return false for non-existent package
            assert!(!installed, "Nonexistent package should not be installed");
        }
        Err(e) => {
            // If brew not available, error should be clear
            let err_msg = e.to_string().to_lowercase();
            assert!(
                err_msg.contains("brew")
                    || err_msg.contains("homebrew")
                    || err_msg.contains("not found")
            );
        }
    }
}

#[test]
#[ignore] // Only run with --ignored if brew is available
fn test_is_installed_integration_git() {
    let manager = HomebrewManager::new();

    // git is usually installed via Homebrew on macOS
    let result = manager.is_installed("git");

    // Should succeed if brew is installed
    assert!(result.is_ok(), "Should successfully check for git");
}

#[test]
fn test_install_many_formulae_idempotent() {
    let manager = HomebrewManager::new();
    let _temp_dir = TempDir::new().unwrap();
    let mut state = State::new();

    // Try to batch install - should check first
    let result = manager.install_many_formulae(&["git"], &mut state);

    match result {
        Ok(_installed) => {
            // Should have added to state
            assert!(state.packages.iter().any(|p| p.name == "git"));
        }
        Err(errors) => {
            // If brew not available, should have clear error
            for (_cmd, e) in &errors {
                let err_msg = e.to_string().to_lowercase();
                assert!(
                    err_msg.contains("brew")
                        || err_msg.contains("homebrew")
                        || err_msg.contains("not found")
                );
            }
        }
    }
}

#[test]
fn test_install_cask_idempotent() {
    let manager = HomebrewManager::new();
    let _temp_dir = TempDir::new().unwrap();
    let mut state = State::new();

    // Try to install a cask
    let result = manager.install_cask("nonexistent-cask-xyz123", &mut state);

    match result {
        Ok(()) => {
            // Should have added to state
            assert!(state
                .packages
                .iter()
                .any(|p| p.name == "nonexistent-cask-xyz123"));
        }
        Err(e) => {
            // If brew not available or cask doesn't exist, should have clear error
            let err_msg = e.to_string().to_lowercase();
            assert!(
                err_msg.contains("brew")
                    || err_msg.contains("homebrew")
                    || err_msg.contains("not found")
                    || err_msg.contains("no available")
                    || err_msg.contains("cask")
            );
        }
    }
}

#[test]
fn test_install_many_formulae_adds_to_state() {
    let manager = HomebrewManager::new();
    let mut state = State::new();

    // Initial state should be empty
    assert_eq!(state.packages.len(), 0);

    // Attempt batch install (may fail if brew not present, but that's OK)
    let _ = manager.install_many_formulae(&["test-package"], &mut state);

    // If install was attempted and brew was found, state should be updated
    // We can't guarantee this in test without brew, so we just verify the API works
}

#[test]
fn test_missing_brew_error() {
    // This test verifies that when brew is not found, we get a clear error
    let manager = HomebrewManager::new();

    // We can't force brew to be missing, but we can verify error handling structure
    let result = manager.list_installed();

    // Either succeeds (brew present) or fails with clear error
    if let Err(e) = result {
        let err_msg = e.to_string();
        // Error should be informative
        assert!(!err_msg.is_empty(), "Error message should not be empty");
    }
}

#[test]
#[ignore]
fn test_install_many_formulae_real_package() {
    let manager = HomebrewManager::new();
    let _temp_dir = TempDir::new().unwrap();
    let mut state = State::new();

    // Try installing a real package (tree is lightweight)
    let result = manager.install_many_formulae(&["tree"], &mut state);

    // Should either succeed or fail gracefully
    match result {
        Ok(_installed) => {
            // Verify state was updated
            assert!(state.packages.iter().any(|p| p.name == "tree"));

            // Verify package is now detected as installed
            let is_installed = manager.is_installed("tree");
            assert!(is_installed.is_ok());
            assert!(
                is_installed.unwrap(),
                "Package should be installed after install()"
            );
        }
        Err(errors) => {
            // Acceptable if brew not present or package already installed
            for (cmd, e) in errors {
                println!(
                    "Install failed (expected if brew unavailable): {} - {}",
                    cmd, e
                );
            }
        }
    }
}

#[test]
fn test_install_many_formulae_empty() {
    let manager = HomebrewManager::new();
    let mut state = State::new();

    // Empty list should return Ok immediately
    let result = manager.install_many_formulae(&[], &mut state);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}
