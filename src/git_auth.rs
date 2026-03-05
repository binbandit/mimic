//! GitHub CLI authentication helper for private repository access.
//!
//! When cloning a private HTTPS repo fails due to authentication, this module
//! provides a flow that:
//! 1. Checks if Homebrew is available
//! 2. Installs `gh` (GitHub CLI) via brew if not already present
//! 3. Runs `gh auth login` interactively
//! 4. Runs `gh auth setup-git` to configure git credential helper

use colored::Colorize;
use std::process::Command;

/// Returns true if `brew` is available on PATH.
fn is_brew_available() -> bool {
    Command::new("brew")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns true if `gh` is available on PATH.
fn is_gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns true if `gh` is already authenticated.
fn is_gh_authenticated() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Install `gh` via Homebrew.
fn install_gh_via_brew() -> anyhow::Result<()> {
    println!("  {} Installing GitHub CLI...", "→".blue());
    let output = Command::new("brew")
        .args(["install", "gh"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run brew install gh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to install gh via brew: {}",
            stderr.trim()
        ));
    }
    println!("  {} GitHub CLI installed", "✓".green());
    Ok(())
}

/// Run `gh auth login` interactively so the user can authenticate.
fn run_gh_auth_login() -> anyhow::Result<()> {
    println!("  {} Authenticating with GitHub...", "→".blue());
    let status = Command::new("gh")
        .args(["auth", "login"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run gh auth login: {}", e))?;

    if !status.success() {
        return Err(anyhow::anyhow!("gh auth login failed"));
    }
    println!("  {} Authenticated with GitHub", "✓".green());
    Ok(())
}

/// Run `gh auth setup-git` so git uses gh as a credential helper.
fn run_gh_setup_git() -> anyhow::Result<()> {
    println!("  {} Configuring git to use GitHub CLI...", "→".blue());
    let output = Command::new("gh")
        .args(["auth", "setup-git"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run gh auth setup-git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "gh auth setup-git failed: {}",
            stderr.trim()
        ));
    }
    println!("  {} Git configured for GitHub authentication", "✓".green());
    Ok(())
}

/// Returns true if the git stderr indicates an authentication/permission problem.
pub fn is_auth_error(stderr: &str) -> bool {
    stderr.contains("Authentication failed")
        || stderr.contains("Permission denied")
        || stderr.contains("could not read Username")
        || stderr.contains("terminal prompts disabled")
        || stderr.contains("Repository not found") // GitHub returns 404 for private repos you can't access
}

/// Attempt to set up GitHub authentication via the `gh` CLI.
///
/// Flow:
/// 1. If `gh` is already on PATH and authenticated, just run `setup-git` and return.
/// 2. If `gh` is on PATH but not authenticated, run `auth login` + `setup-git`.
/// 3. If `gh` is not on PATH but `brew` is, install `gh` then do step 2.
/// 4. If neither is available, return an error with manual instructions.
pub fn ensure_gh_auth() -> anyhow::Result<()> {
    println!();
    println!(
        "{}",
        "Authentication required — setting up GitHub CLI...".bold()
    );

    // Already have gh?
    if is_gh_available() {
        if !is_gh_authenticated() {
            run_gh_auth_login()?;
        }
        run_gh_setup_git()?;
        println!();
        return Ok(());
    }

    // No gh, but have brew?
    if is_brew_available() {
        install_gh_via_brew()?;
        run_gh_auth_login()?;
        run_gh_setup_git()?;
        println!();
        return Ok(());
    }

    // Neither available
    Err(anyhow::anyhow!(
        "Cannot automatically authenticate.\n\n\
         To fix:\n  \
         - Install Homebrew (https://brew.sh) then re-run this command\n  \
         - Or install the GitHub CLI manually: https://cli.github.com\n  \
         - Or configure git credentials: https://git-scm.com/docs/gitcredentials"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_auth_error_detects_patterns() {
        assert!(is_auth_error(
            "fatal: Authentication failed for 'https://github.com/foo/bar.git'"
        ));
        assert!(is_auth_error("Permission denied (publickey)."));
        assert!(is_auth_error(
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled"
        ));
        assert!(is_auth_error("fatal: terminal prompts disabled"));
        assert!(is_auth_error("ERROR: Repository not found."));
    }

    #[test]
    fn test_is_auth_error_ignores_other_errors() {
        assert!(!is_auth_error("fatal: Could not resolve host: github.com"));
        assert!(!is_auth_error("fatal: unable to access"));
        assert!(!is_auth_error(
            "warning: You appear to have cloned an empty repository"
        ));
    }
}
