use crate::error::InstallError;
use crate::spinner::Spinner;
use crate::state::{PackageState, State};
use std::process::Command;

pub struct HomebrewManager;

/// Runs `brew list <flag> -1` and returns the installed package names.
/// Returns `Ok(None)` when the brew executable itself is missing, so callers
/// can decide whether that is an error (install/uninstall) or simply means
/// "nothing is installed" (diff/status on a machine without Homebrew).
fn try_list_brew(flag: &str) -> Result<Option<Vec<String>>, anyhow::Error> {
    let output = Command::new("brew")
        .arg("list")
        .arg(flag)
        .arg("-1")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let packages: Vec<String> = stdout
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            Ok(Some(packages))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("brew list {} failed: {}", flag, stderr))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to execute brew: {}", e)),
    }
}

fn brew_not_found() -> anyhow::Error {
    anyhow::anyhow!("Homebrew not found. Please install Homebrew from https://brew.sh")
}

impl HomebrewManager {
    pub fn new() -> Self {
        Self
    }

    pub fn list_installed(&self) -> Result<Vec<String>, anyhow::Error> {
        try_list_brew("--formula")?.ok_or_else(brew_not_found)
    }

    pub fn list_installed_casks(&self) -> Result<Vec<String>, anyhow::Error> {
        try_list_brew("--cask")?.ok_or_else(brew_not_found)
    }

    /// Check if a formula is installed. A missing brew executable means the
    /// formula is not installed rather than an error.
    pub fn is_installed(&self, name: &str) -> Result<bool, anyhow::Error> {
        Ok(try_list_brew("--formula")?
            .is_some_and(|installed| installed.iter().any(|pkg| pkg == name)))
    }

    /// Check if a cask is installed. A missing brew executable means the
    /// cask is not installed rather than an error.
    pub fn is_installed_cask(&self, name: &str) -> Result<bool, anyhow::Error> {
        Ok(try_list_brew("--cask")?
            .is_some_and(|installed| installed.iter().any(|pkg| pkg == name)))
    }

    /// List installed formulae and casks together with two brew invocations.
    /// Returns `Ok(None)` when the brew executable is missing.
    pub fn list_installed_any(&self) -> Result<Option<Vec<String>>, anyhow::Error> {
        let Some(mut packages) = try_list_brew("--formula")? else {
            return Ok(None);
        };
        packages.extend(try_list_brew("--cask")?.unwrap_or_default());
        Ok(Some(packages))
    }

    /// Check if a package is installed, routing to formula or cask based on type.
    pub fn is_installed_any(&self, name: &str, package_type: &str) -> Result<bool, anyhow::Error> {
        if package_type == "cask" {
            self.is_installed_cask(name)
        } else {
            self.is_installed(name)
        }
    }

    pub fn uninstall_many(&self, names: &[&str]) -> Result<Vec<String>, anyhow::Error> {
        if names.is_empty() {
            return Ok(Vec::new());
        }

        let spinner = Spinner::new(format!("Uninstalling {} packages...", names.len()));

        let output = Command::new("brew").arg("uninstall").args(names).output();

        match output {
            Ok(output) if output.status.success() => {
                spinner.finish_with_message(format!("✓ Uninstalled {} packages", names.len()));
                Ok(Vec::new())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);
                spinner.finish_with_error(format!("brew uninstall failed (exit {})", exit_code));
                Err(InstallError::CommandFailed {
                    command: format!("brew uninstall {}", names.join(" ")),
                    exit_code,
                    stderr: stderr.to_string(),
                }
                .into())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                spinner.finish_with_error("Homebrew not found");
                Err(anyhow::anyhow!(
                    "Homebrew not found. Please install Homebrew from https://brew.sh"
                ))
            }
            Err(e) => {
                spinner.finish_with_error(format!("Failed to execute brew: {}", e));
                Err(anyhow::anyhow!("Failed to execute brew: {}", e))
            }
        }
    }

    /// Install a single cask package. Casks must be installed one at a time
    /// because they may require interactive prompts (e.g. password for system extensions).
    pub fn install_cask(&self, name: &str, state: &mut State) -> Result<(), anyhow::Error> {
        let already_installed = self.is_installed_cask(name)?;

        if already_installed {
            if !state.packages.iter().any(|p| p.name == name) {
                state.add_package(PackageState {
                    name: name.to_string(),
                    manager: "brew".to_string(),
                });
            }
            return Ok(());
        }

        let spinner = Spinner::new(format!("Installing {} (cask)...", name));

        let output = Command::new("brew")
            .arg("install")
            .arg("--cask")
            .arg(name)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                spinner.finish_with_message(format!("✓ Installed {} (cask)", name));
                state.add_package(PackageState {
                    name: name.to_string(),
                    manager: "brew".to_string(),
                });
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);
                spinner.finish_with_error(format!("Failed to install {}", name));
                Err(InstallError::CommandFailed {
                    command: format!("brew install --cask {}", name),
                    exit_code,
                    stderr: stderr.to_string(),
                }
                .into())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                spinner.finish_with_error("Homebrew not found");
                Err(anyhow::anyhow!(
                    "Homebrew not found. Please install Homebrew from https://brew.sh"
                ))
            }
            Err(e) => {
                spinner.finish_with_error(format!("Failed to execute brew: {}", e));
                Err(anyhow::anyhow!("Failed to execute brew: {}", e))
            }
        }
    }

    /// Batch-install formula packages. Filters out already-installed formulae with a single
    /// `brew list --formula` call, then installs all remaining with one `brew install a b c` command.
    /// Returns the list of package names that were newly installed.
    pub fn install_many_formulae(
        &self,
        names: &[&str],
        state: &mut State,
    ) -> Result<Vec<String>, Vec<(String, anyhow::Error)>> {
        if names.is_empty() {
            return Ok(Vec::new());
        }

        // Single call to get all installed formulae
        let installed = self
            .list_installed()
            .map_err(|e| vec![("brew list --formula".to_string(), e)])?;

        let mut already_installed = Vec::new();
        let mut to_install = Vec::new();

        for &name in names {
            if installed.iter().any(|pkg| pkg == name) {
                already_installed.push(name);
            } else {
                to_install.push(name);
            }
        }

        // Track already-installed packages in state
        for name in &already_installed {
            if !state.packages.iter().any(|p| p.name == *name) {
                state.add_package(PackageState {
                    name: name.to_string(),
                    manager: "brew".to_string(),
                });
            }
        }

        if to_install.is_empty() {
            return Ok(Vec::new());
        }

        let spinner = Spinner::new(format!(
            "Installing {} formula{}...",
            to_install.len(),
            if to_install.len() == 1 { "" } else { "e" }
        ));

        let output = Command::new("brew")
            .arg("install")
            .args(&to_install)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let installed_names: Vec<String> =
                    to_install.iter().map(|s| s.to_string()).collect();
                spinner
                    .finish_with_message(format!("✓ Installed {} formulae", installed_names.len()));
                for name in &installed_names {
                    state.add_package(PackageState {
                        name: name.clone(),
                        manager: "brew".to_string(),
                    });
                }
                Ok(installed_names)
            }
            Ok(output) => {
                // A batch install can partially succeed (brew exits non-zero if
                // any formula fails). Record what actually made it onto disk so
                // state stays accurate for undo/clean.
                if let Ok(Some(now_installed)) = try_list_brew("--formula") {
                    for name in &to_install {
                        if now_installed.iter().any(|pkg| pkg == name)
                            && !state.packages.iter().any(|p| p.name == *name)
                        {
                            state.add_package(PackageState {
                                name: name.to_string(),
                                manager: "brew".to_string(),
                            });
                        }
                    }
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);
                spinner.finish_with_error(format!("brew install failed (exit {})", exit_code));
                Err(vec![(
                    format!("brew install {}", to_install.join(" ")),
                    InstallError::CommandFailed {
                        command: format!("brew install {}", to_install.join(" ")),
                        exit_code,
                        stderr: stderr.to_string(),
                    }
                    .into(),
                )])
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                spinner.finish_with_error("Homebrew not found");
                Err(vec![(
                    "brew".to_string(),
                    anyhow::anyhow!(
                        "Homebrew not found. Please install Homebrew from https://brew.sh"
                    ),
                )])
            }
            Err(e) => {
                spinner.finish_with_error(format!("Failed to execute brew: {}", e));
                Err(vec![(
                    "brew".to_string(),
                    anyhow::anyhow!("Failed to execute brew: {}", e),
                )])
            }
        }
    }
}

impl Default for HomebrewManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homebrew_manager_new() {
        let manager = HomebrewManager::new();
        assert!(std::mem::size_of_val(&manager) == 0);
    }

    #[test]
    fn test_homebrew_manager_default() {
        let _manager = HomebrewManager;
    }
}
