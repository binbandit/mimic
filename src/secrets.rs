//! Secrets management using macOS Keychain.
//!
//! This module provides secure storage and retrieval of secrets using the macOS Keychain
//! via the `security` command-line tool. Secrets are stored under the service name "mimic"
//! and can be accessed in templates or exported as environment variables.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;

const SERVICE_NAME: &str = "mimic";

/// Set a secret in macOS Keychain.
///
/// If the secret already exists, it will be updated. Otherwise, a new entry is created.
///
/// # Arguments
/// * `key` - The secret key name (used as the account name in Keychain)
/// * `value` - The secret value to store
///
/// # Errors
/// Returns an error if the keychain operation fails or if not on macOS.
pub fn set_secret(key: &str, value: &str) -> Result<()> {
    check_platform()?;

    // Try to update first (handles both create and update cases)
    let output = Command::new("security")
        .args([
            "add-generic-password",
            "-U", // Update if exists, create if not
            "-s",
            SERVICE_NAME,
            "-a",
            key,
            "-w",
            value,
        ])
        .output()
        .context("Failed to execute security command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to store secret in keychain: {}",
            stderr
        ));
    }

    Ok(())
}

/// Get a secret from macOS Keychain.
///
/// # Arguments
/// * `key` - The secret key name to retrieve
///
/// # Returns
/// The secret value as a String
///
/// # Errors
/// Returns an error if the secret is not found or if the keychain operation fails.
pub fn get_secret(key: &str) -> Result<String> {
    check_platform()?;

    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            SERVICE_NAME,
            "-a",
            key,
            "-w", // Return password only
        ])
        .output()
        .context("Failed to execute security command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("could not be found") {
            return Err(anyhow::anyhow!("Secret '{}' not found in keychain", key));
        }
        return Err(anyhow::anyhow!(
            "Failed to retrieve secret from keychain: {}",
            stderr
        ));
    }

    let secret = String::from_utf8(output.stdout)
        .context("Secret value is not valid UTF-8")?
        .trim()
        .to_string();

    Ok(secret)
}

/// List all secrets stored by mimic in the Keychain.
///
/// # Returns
/// A vector of secret key names
///
/// # Errors
/// Returns an error if the keychain query fails.
pub fn list_secrets() -> Result<Vec<String>> {
    check_platform()?;

    let output = Command::new("security")
        .args(["find-generic-password", "-s", SERVICE_NAME, "-a"])
        .output();

    // If this fails, try alternative approach using dump-keychain
    if output.is_err() || !output.as_ref().unwrap().status.success() {
        return list_secrets_via_dump();
    }

    list_secrets_via_dump()
}

/// List secrets by dumping keychain and parsing output.
fn list_secrets_via_dump() -> Result<Vec<String>> {
    let output = Command::new("security")
        .args(["dump-keychain"])
        .output()
        .context("Failed to execute security dump-keychain")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse keychain dump output to find entries for our service
    let mut secrets = Vec::new();
    let mut in_mimic_entry = false;
    let mut current_account = None;

    for line in stdout.lines() {
        // Look for service attribute matching "mimic"
        if line.contains("\"svce\"<blob>") && line.contains(SERVICE_NAME) {
            in_mimic_entry = true;
        }

        // If we're in a mimic entry, extract the account name
        if in_mimic_entry && line.contains("\"acct\"<blob>=") {
            if let Some(account) = extract_account_name(line) {
                current_account = Some(account);
            }
        }

        // End of entry - check if we should add it
        if line.trim() == "}" {
            if let Some(account) = current_account.take() {
                secrets.push(account);
            }
            in_mimic_entry = false;
        }
    }

    Ok(secrets)
}

/// Extract account name from keychain dump line.
fn extract_account_name(line: &str) -> Option<String> {
    // Format: "acct"<blob>="account_name"
    if let Some(start) = line.find("\"acct\"<blob>=") {
        let rest = &line[start + 13..]; // Skip past "acct"<blob>=
        if let Some(quote_start) = rest.find('"') {
            let after_quote = &rest[quote_start + 1..];
            if let Some(quote_end) = after_quote.find('"') {
                return Some(after_quote[..quote_end].to_string());
            }
        }
    }
    None
}

/// Remove a secret from macOS Keychain.
///
/// # Arguments
/// * `key` - The secret key name to remove
///
/// # Errors
/// Returns an error if the secret is not found or if the keychain operation fails.
pub fn remove_secret(key: &str) -> Result<()> {
    check_platform()?;

    let output = Command::new("security")
        .args(["delete-generic-password", "-s", SERVICE_NAME, "-a", key])
        .output()
        .context("Failed to execute security command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("could not be found") {
            return Err(anyhow::anyhow!("Secret '{}' not found in keychain", key));
        }
        return Err(anyhow::anyhow!(
            "Failed to remove secret from keychain: {}",
            stderr
        ));
    }

    Ok(())
}

/// Check if a secret exists in the Keychain.
///
/// # Arguments
/// * `key` - The secret key name to check
///
/// # Returns
/// `true` if the secret exists, `false` otherwise
pub fn secret_exists(key: &str) -> Result<bool> {
    check_platform()?;

    let output = Command::new("security")
        .args(["find-generic-password", "-s", SERVICE_NAME, "-a", key])
        .output()
        .context("Failed to execute security command")?;

    Ok(output.status.success())
}

/// Get all secrets as a HashMap for use in template context.
///
/// This function retrieves all secrets managed by mimic and returns them as a map.
/// If any individual secret fails to retrieve, it is silently skipped.
///
/// # Returns
/// A HashMap mapping secret keys to their values
pub fn get_all_secrets() -> HashMap<String, String> {
    let mut secrets = HashMap::new();

    // Skip if not on macOS
    if check_platform().is_err() {
        return secrets;
    }

    if let Ok(keys) = list_secrets() {
        for key in keys {
            if let Ok(value) = get_secret(&key) {
                secrets.insert(key, value);
            }
        }
    }

    secrets
}

/// Check if running on macOS.
///
/// # Errors
/// Returns an error with a helpful message if not on macOS.
fn check_platform() -> Result<()> {
    if cfg!(not(target_os = "macos")) {
        return Err(anyhow::anyhow!(
            "Secrets management is currently only supported on macOS.\n\
             The macOS Keychain is used for secure storage.\n\
             Support for other platforms (Linux Secret Service, Windows Credential Manager) \
             may be added in the future."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_account_name() {
        let line = r#"    "acct"<blob>="my_secret_key""#;
        let result = extract_account_name(line);
        assert_eq!(result, Some("my_secret_key".to_string()));
    }

    #[test]
    fn test_extract_account_name_with_underscores() {
        let line = r#"    "acct"<blob>="openai_api_key""#;
        let result = extract_account_name(line);
        assert_eq!(result, Some("openai_api_key".to_string()));
    }

    #[test]
    fn test_extract_account_name_no_match() {
        let line = r#"    "svce"<blob>="mimic""#;
        let result = extract_account_name(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_service_name_constant() {
        assert_eq!(SERVICE_NAME, "mimic");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_check_platform_macos() {
        assert!(check_platform().is_ok());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_check_platform_non_macos() {
        let result = check_platform();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("only supported on macOS"));
    }
}
