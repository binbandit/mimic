//! Path expansion utilities.
//!
//! Replaces the `shellexpand` crate with a focused implementation that handles:
//! - Tilde (`~`) expansion to the user's home directory
//! - Environment variable expansion (`$VAR` and `${VAR}`)

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Expand shell-like variables in a path string.
///
/// Handles:
/// - `~` or `~/...` → home directory
/// - `$VAR` → environment variable value
/// - `${VAR}` → environment variable value (braced form)
fn expand_str(input: &str) -> Result<String> {
    // Handle tilde expansion first
    let after_tilde = if input == "~" {
        home_dir()?.to_string_lossy().to_string()
    } else if let Some(rest) = input.strip_prefix("~/") {
        let home = home_dir()?;
        format!("{}/{}", home.display(), rest)
    } else {
        input.to_string()
    };

    // Expand environment variables
    expand_env_vars(&after_tilde)
}

/// Expand `$VAR` and `${VAR}` patterns in a string.
fn expand_env_vars(input: &str) -> Result<String> {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if chars.peek() == Some(&'{') {
                // ${VAR} form
                chars.next(); // consume '{'
                let var_name: String = chars.by_ref().take_while(|c| *c != '}').collect();
                if var_name.is_empty() {
                    result.push_str("${}");
                } else {
                    let value = std::env::var(&var_name).with_context(|| {
                        format!("Environment variable '{}' is not set", var_name)
                    })?;
                    result.push_str(&value);
                }
            } else if chars
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
            {
                // $VAR form
                let var_name: String = chars
                    .by_ref()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                let value = std::env::var(&var_name)
                    .with_context(|| format!("Environment variable '{}' is not set", var_name))?;
                result.push_str(&value);
            } else {
                // Lone $ at end of string or before non-identifier char
                result.push('$');
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// Get the user's home directory.
fn home_dir() -> Result<PathBuf> {
    home::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))
}

/// Expand a path, handling `~`, `$VAR`, and `${VAR}`.
///
/// This is the primary entry point used by the rest of the codebase.
pub fn expand_path(path: &Path) -> Result<PathBuf> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Path contains invalid UTF-8: {}", path.display()))?;

    let expanded =
        expand_str(path_str).with_context(|| format!("Failed to expand path: {}", path_str))?;

    Ok(PathBuf::from(expanded))
}

/// Expand a path string, handling `~`, `$VAR`, and `${VAR}`.
pub fn expand_path_str(path: &str) -> Result<PathBuf> {
    let expanded = expand_str(path).with_context(|| format!("Failed to expand path: {}", path))?;
    Ok(PathBuf::from(expanded))
}

/// Expand only tilde in a string (no env vars). Used by hooks.
pub fn expand_tilde(path: &str) -> Result<String> {
    if path == "~" {
        Ok(home_dir()?.to_string_lossy().to_string())
    } else if let Some(rest) = path.strip_prefix("~/") {
        let home = home_dir()?;
        Ok(format!("{}/{}", home.display(), rest))
    } else {
        Ok(path.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let result = expand_str("~/test").unwrap();
        assert!(!result.contains('~'));
        assert!(result.ends_with("/test"));
    }

    #[test]
    fn test_expand_tilde_alone() {
        let result = expand_str("~").unwrap();
        assert!(!result.contains('~'));
        assert!(!result.is_empty());
    }

    #[test]
    fn test_expand_env_var() {
        std::env::set_var("MIMIC_TEST_VAR", "hello");
        let result = expand_str("$MIMIC_TEST_VAR/world").unwrap();
        assert_eq!(result, "hello/world");
        std::env::remove_var("MIMIC_TEST_VAR");
    }

    #[test]
    fn test_expand_env_var_braced() {
        std::env::set_var("MIMIC_TEST_BRACE", "braced");
        let result = expand_str("${MIMIC_TEST_BRACE}/path").unwrap();
        assert_eq!(result, "braced/path");
        std::env::remove_var("MIMIC_TEST_BRACE");
    }

    #[test]
    fn test_expand_absolute_path_unchanged() {
        let result = expand_str("/usr/local/bin").unwrap();
        assert_eq!(result, "/usr/local/bin");
    }

    #[test]
    fn test_expand_path_helper() {
        let result = expand_path(Path::new("~/test")).unwrap();
        assert!(!result.to_string_lossy().contains('~'));
    }

    #[test]
    fn test_expand_missing_env_var_errors() {
        let result = expand_str("$MIMIC_NONEXISTENT_VAR_12345");
        assert!(result.is_err());
    }

    #[test]
    fn test_no_expansion_needed() {
        let result = expand_str("relative/path/file.txt").unwrap();
        assert_eq!(result, "relative/path/file.txt");
    }
}
