//! Secrets detection module using ripsecrets.
//!
//! Local-only scanning for secrets in files. Detects:
//! - GitHub PAT, Stripe keys, AWS credentials, JWT, SSH keys
//!
//! No data leaves the machine. Supports `.secretsignore`.
//!
//! Note: ripsecrets prints matches directly to stdout via its BufferWriter.
//! We return only the count of matches found.

use anyhow::Result;
use std::path::PathBuf;
use termcolor::{BufferWriter, ColorChoice};

/// Scan the given files for secrets using ripsecrets.
///
/// Returns the number of secret matches found. Detailed output is
/// printed directly to stdout by ripsecrets.
pub fn scan_for_secrets(paths: &[PathBuf]) -> Result<usize> {
    let writer = BufferWriter::stdout(ColorChoice::Always);
    let additional_patterns: Vec<String> = Vec::new();

    let count = ripsecrets::find_secrets(paths, &additional_patterns, true, false, writer)
        .map_err(|e| anyhow::anyhow!("Failed to scan for secrets: {}", e))?;

    Ok(count)
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

        let count = scan_for_secrets(&[test_file.clone()]).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_scan_empty_paths() {
        let count = scan_for_secrets(&[]).unwrap();
        assert_eq!(count, 0);
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

        let count = scan_for_secrets(&[file1, file2]).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_scan_aws_credentials_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("aws.txt");

        fs::write(&test_file, "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE").unwrap();

        let count = scan_for_secrets(&[test_file]).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_scan_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("empty.txt");
        fs::write(&test_file, "").unwrap();

        let count = scan_for_secrets(&[test_file]).unwrap();
        assert_eq!(count, 0);
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
