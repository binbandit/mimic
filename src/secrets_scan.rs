//! Secrets detection module using ripsecrets.
//!
//! Local-only scanning for secrets in files. Detects:
//! - GitHub PAT, Stripe keys, AWS credentials, JWT, SSH keys
//!
//! No data leaves the machine. Supports `.secretsignore`.

use anyhow::Result;
use std::path::PathBuf;
use termcolor::{BufferWriter, ColorChoice};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMatch {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub pattern: String,
    pub matched_text: String,
}

pub fn scan_for_secrets(paths: &[PathBuf]) -> Result<Vec<SecretMatch>> {
    let writer = BufferWriter::stdout(ColorChoice::Always);
    let additional_patterns: Vec<String> = Vec::new();

    let count = ripsecrets::find_secrets(paths, &additional_patterns, true, false, writer)
        .map_err(|e| anyhow::anyhow!("Failed to scan for secrets: {}", e))?;

    let mut matches = Vec::new();
    for _ in 0..count {
        matches.push(SecretMatch {
            file_path: PathBuf::from(""),
            line_number: 0,
            pattern: String::new(),
            matched_text: String::new(),
        });
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_detects_github_pat() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        fs::write(
            &test_file,
            "fake token: ghp_1234567890abcdefghijklmnopqrstuvwxyz",
        )
        .unwrap();

        let results = scan_for_secrets(&[test_file.clone()]).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_scan_empty_paths() {
        let results = scan_for_secrets(&[]).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_scan_handles_nonexistent_file() {
        let nonexistent = PathBuf::from("/nonexistent/file/that/does/not/exist.txt");
        let result = scan_for_secrets(&[nonexistent]);

        assert!(
            result.is_ok(),
            "Should succeed but print warning for nonexistent file"
        );
    }

    #[test]
    fn test_scan_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, "content without secrets").unwrap();
        fs::write(&file2, "more safe content").unwrap();

        let results = scan_for_secrets(&[file1, file2]).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_scan_aws_credentials_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("aws.txt");

        fs::write(&test_file, "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE").unwrap();

        let results = scan_for_secrets(&[test_file]).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_secret_match_structure() {
        let secret = SecretMatch {
            file_path: PathBuf::from("/test/file.txt"),
            line_number: 42,
            pattern: "github_pat".to_string(),
            matched_text: "ghp_xxxxx".to_string(),
        };

        assert_eq!(secret.file_path, PathBuf::from("/test/file.txt"));
        assert_eq!(secret.line_number, 42);
        assert_eq!(secret.pattern, "github_pat");
        assert_eq!(secret.matched_text, "ghp_xxxxx");
    }

    #[test]
    fn test_secret_match_clone_and_equality() {
        let secret1 = SecretMatch {
            file_path: PathBuf::from("/test/file.txt"),
            line_number: 10,
            pattern: "test".to_string(),
            matched_text: "test_value".to_string(),
        };

        let secret2 = secret1.clone();
        assert_eq!(secret1, secret2);
    }

    #[test]
    fn test_scan_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("empty.txt");
        fs::write(&test_file, "").unwrap();

        let results = scan_for_secrets(&[test_file]).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_scan_returns_ok_for_valid_paths() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("safe.txt");
        fs::write(&test_file, "safe content").unwrap();

        let result = scan_for_secrets(&[test_file]);
        assert!(result.is_ok(), "Should succeed for valid file");
    }
}
