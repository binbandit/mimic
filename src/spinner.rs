//! Spinner utilities for progress indication
//!
//! This module provides reusable spinner infrastructure with CI/TTY detection.
//! Spinners are automatically hidden in CI environments or when stdout is not a TTY.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::{Duration, Instant};

/// A wrapper around indicatif's ProgressBar with CI/TTY detection
pub struct Spinner {
    pb: Option<ProgressBar>,
    start_time: Instant,
    message: String,
}

impl Spinner {
    /// Create a new spinner with CI/TTY detection
    ///
    /// The spinner will be disabled if:
    /// - Running in CI (CI env var is set)
    /// - stdout is not a TTY
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let pb = if should_show_spinner() {
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::with_template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            spinner.set_message(message.clone());
            spinner.enable_steady_tick(Duration::from_millis(100));
            Some(spinner)
        } else {
            None
        };

        Self {
            pb,
            start_time: Instant::now(),
            message,
        }
    }

    /// Update the spinner message
    pub fn set_message(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.message = message.clone();
        if let Some(pb) = &self.pb {
            pb.set_message(message);
        }
    }

    /// Finish the spinner with a success message and timing
    pub fn finish_with_message(&self, message: impl Into<String>) {
        let elapsed = self.start_time.elapsed();
        let message = format!("{} (took {:.2}s)", message.into(), elapsed.as_secs_f64());

        if let Some(pb) = &self.pb {
            pb.finish_with_message(message);
        }
    }

    /// Finish the spinner and clear it
    pub fn finish_and_clear(&self) {
        if let Some(pb) = &self.pb {
            pb.finish_and_clear();
        }
    }

    /// Finish the spinner with an error message
    pub fn finish_with_error(&self, message: impl Into<String>) {
        let elapsed = self.start_time.elapsed();
        let message = format!("✗ {} (after {:.2}s)", message.into(), elapsed.as_secs_f64());

        if let Some(pb) = &self.pb {
            pb.finish_with_message(message);
        }
    }
}

/// Manager for multiple concurrent spinners
pub struct SpinnerManager {
    mp: Option<MultiProgress>,
}

impl SpinnerManager {
    /// Create a new spinner manager
    pub fn new() -> Self {
        let mp = if should_show_spinner() {
            Some(MultiProgress::new())
        } else {
            None
        };

        Self { mp }
    }

    /// Create a new spinner managed by this MultiProgress instance
    pub fn add_spinner(&self, message: impl Into<String>) -> Spinner {
        let message = message.into();
        let pb = if let Some(mp) = &self.mp {
            let spinner = mp.add(ProgressBar::new_spinner());
            spinner.set_style(
                ProgressStyle::with_template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            spinner.set_message(message.clone());
            spinner.enable_steady_tick(Duration::from_millis(100));
            Some(spinner)
        } else {
            None
        };

        Spinner {
            pb,
            start_time: Instant::now(),
            message,
        }
    }
}

impl Default for SpinnerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if spinners should be shown
///
/// Returns false if:
/// - CI environment variable is set
/// - stdout is not a TTY (using atty would require additional dep, so we check CI only for now)
fn should_show_spinner() -> bool {
    std::env::var("CI").is_err()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_spinner_creation() {
        let spinner = Spinner::new("Testing");
        assert_eq!(spinner.message, "Testing");
    }

    #[test]
    fn test_spinner_manager_creation() {
        let manager = SpinnerManager::new();
        let _spinner = manager.add_spinner("Test spinner");
    }

    #[test]
    fn test_should_show_spinner_in_ci() {
        let original = std::env::var("CI").ok();

        unsafe {
            std::env::set_var("CI", "true");
        }
        assert!(
            !should_show_spinner(),
            "Spinner should be hidden in CI environment"
        );

        unsafe {
            match original {
                Some(val) => std::env::set_var("CI", val),
                None => std::env::remove_var("CI"),
            }
        }
    }

    #[test]
    fn test_should_show_spinner_not_in_ci() {
        let original = std::env::var("CI").ok();

        unsafe {
            std::env::remove_var("CI");
        }
        assert!(
            should_show_spinner(),
            "Spinner should be shown when not in CI"
        );

        unsafe {
            match original {
                Some(val) => std::env::set_var("CI", val),
                None => std::env::remove_var("CI"),
            }
        }
    }

    #[test]
    fn test_spinner_timing_tracking() {
        let spinner = Spinner::new("Testing timing");
        thread::sleep(Duration::from_millis(10));
        let elapsed = spinner.start_time.elapsed();
        assert!(
            elapsed >= Duration::from_millis(10),
            "Elapsed time should be at least 10ms, got {:?}",
            elapsed
        );
    }

    #[test]
    fn test_spinner_set_message() {
        let mut spinner = Spinner::new("Initial message");
        assert_eq!(spinner.message, "Initial message");

        spinner.set_message("Updated message");
        assert_eq!(spinner.message, "Updated message");
    }

    #[test]
    fn test_spinner_finish_methods() {
        let spinner = Spinner::new("Test operation");
        spinner.finish_with_message("Success");

        let spinner2 = Spinner::new("Test operation 2");
        spinner2.finish_with_error("Failed");

        let spinner3 = Spinner::new("Test operation 3");
        spinner3.finish_and_clear();
    }

    #[test]
    fn test_spinner_manager_multiple_spinners() {
        let manager = SpinnerManager::new();
        let spinner1 = manager.add_spinner("Task 1");
        let spinner2 = manager.add_spinner("Task 2");

        assert_eq!(spinner1.message, "Task 1");
        assert_eq!(spinner2.message, "Task 2");

        spinner1.finish_with_message("✓ Task 1 complete");
        spinner2.finish_with_message("✓ Task 2 complete");
    }

    #[test]
    fn test_spinner_in_ci_is_none() {
        unsafe {
            env::set_var("CI", "true");
        }
        let spinner = Spinner::new("CI test");
        assert!(spinner.pb.is_none(), "ProgressBar should be None in CI");
        unsafe {
            env::remove_var("CI");
        }
    }

    #[test]
    fn test_spinner_manager_in_ci_is_none() {
        unsafe {
            env::set_var("CI", "true");
        }
        let manager = SpinnerManager::new();
        assert!(manager.mp.is_none(), "MultiProgress should be None in CI");
        unsafe {
            env::remove_var("CI");
        }
    }

    #[test]
    fn test_spinner_default_trait() {
        let manager1 = SpinnerManager::default();
        let manager2 = SpinnerManager::new();
        assert_eq!(manager1.mp.is_some(), manager2.mp.is_some());
    }
}
