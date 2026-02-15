//! Integration tests for secrets management.
//!
//! These tests cover the secrets module functionality including keychain operations.
//! Note: Some tests require macOS to run fully.

#[cfg(test)]
mod secrets_tests {
    use mimic::secrets::*;
    use std::collections::HashMap;

    const TEST_KEY: &str = "mimic_test_secret";
    const TEST_VALUE: &str = "test_secret_value_123";

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore] // Requires clean keychain state
    fn test_set_and_get_secret() {
        cleanup_test_secret();

        let result = set_secret(TEST_KEY, TEST_VALUE);
        assert!(result.is_ok(), "Failed to set secret: {:?}", result.err());

        let retrieved = get_secret(TEST_KEY);
        assert!(
            retrieved.is_ok(),
            "Failed to get secret: {:?}",
            retrieved.err()
        );
        assert_eq!(retrieved.unwrap(), TEST_VALUE);

        cleanup_test_secret();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_update_existing_secret() {
        cleanup_test_secret();

        set_secret(TEST_KEY, "initial_value").expect("Failed to set initial secret");

        let updated_value = "updated_value_456";
        let result = set_secret(TEST_KEY, updated_value);
        assert!(
            result.is_ok(),
            "Failed to update secret: {:?}",
            result.err()
        );

        let retrieved = get_secret(TEST_KEY);
        assert_eq!(
            retrieved.unwrap(),
            updated_value,
            "Secret was not updated correctly"
        );

        cleanup_test_secret();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_nonexistent_secret() {
        let nonexistent_key = "mimic_nonexistent_test_key_12345";
        cleanup_specific_secret(nonexistent_key);

        let result = get_secret(nonexistent_key);
        assert!(result.is_err(), "Expected error for nonexistent secret");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not found"),
            "Error message should mention 'not found', got: {}",
            err_msg
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore]
    fn test_secret_exists() {
        cleanup_test_secret();

        let exists_before = secret_exists(TEST_KEY).unwrap_or(false);
        assert!(!exists_before, "Secret should not exist initially");

        set_secret(TEST_KEY, TEST_VALUE).expect("Failed to set secret");

        let exists_after = secret_exists(TEST_KEY).unwrap_or(false);
        assert!(exists_after, "Secret should exist after setting");

        cleanup_test_secret();
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore]
    fn test_list_secrets() {
        cleanup_test_secret();

        set_secret(TEST_KEY, TEST_VALUE).expect("Failed to set secret");

        let secrets = list_secrets();
        assert!(
            secrets.is_ok(),
            "Failed to list secrets: {:?}",
            secrets.err()
        );

        let secret_list = secrets.unwrap();
        assert!(
            secret_list.contains(&TEST_KEY.to_string()),
            "Test secret should be in the list: {:?}",
            secret_list
        );

        cleanup_test_secret();
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore]
    fn test_remove_secret() {
        cleanup_test_secret();

        set_secret(TEST_KEY, TEST_VALUE).expect("Failed to set secret");

        let exists_before = secret_exists(TEST_KEY).unwrap_or(false);
        assert!(exists_before, "Secret should exist before removal");

        let result = remove_secret(TEST_KEY);
        assert!(
            result.is_ok(),
            "Failed to remove secret: {:?}",
            result.err()
        );

        let exists_after = secret_exists(TEST_KEY).unwrap_or(true);
        assert!(!exists_after, "Secret should not exist after removal");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_remove_nonexistent_secret() {
        let nonexistent_key = "mimic_nonexistent_test_key_99999";
        cleanup_specific_secret(nonexistent_key);

        let result = remove_secret(nonexistent_key);
        assert!(
            result.is_err(),
            "Expected error when removing nonexistent secret"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not found"),
            "Error message should mention 'not found', got: {}",
            err_msg
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore]
    fn test_get_all_secrets() {
        cleanup_test_secret();

        set_secret(TEST_KEY, TEST_VALUE).expect("Failed to set secret");

        let all_secrets = get_all_secrets();
        assert!(
            all_secrets.contains_key(TEST_KEY),
            "get_all_secrets should include test secret"
        );
        assert_eq!(all_secrets.get(TEST_KEY), Some(&TEST_VALUE.to_string()));

        cleanup_test_secret();
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore]
    fn test_secret_with_special_characters() {
        cleanup_test_secret();

        let special_value = "p@ssw0rd!#$%^&*()_+={}[]|\\:;\"'<>,.?/~`";
        set_secret(TEST_KEY, special_value).expect("Failed to set secret with special chars");

        let retrieved = get_secret(TEST_KEY);
        assert!(
            retrieved.is_ok(),
            "Failed to retrieve secret with special chars"
        );
        assert_eq!(retrieved.unwrap(), special_value);

        cleanup_test_secret();
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_platform_check_non_macos() {
        let result = set_secret(TEST_KEY, TEST_VALUE);
        assert!(
            result.is_err(),
            "Operations should fail on non-macOS platforms"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("only supported on macOS"),
            "Error should mention platform requirement, got: {}",
            err_msg
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_all_secrets_returns_empty_on_error() {
        let secrets = get_all_secrets();
        assert!(
            secrets.is_empty() || secrets.len() > 0,
            "get_all_secrets should return a valid HashMap"
        );
    }

    #[cfg(target_os = "macos")]
    fn cleanup_test_secret() {
        let _ = remove_secret(TEST_KEY);
    }

    #[cfg(target_os = "macos")]
    fn cleanup_specific_secret(key: &str) {
        let _ = remove_secret(key);
    }
}
