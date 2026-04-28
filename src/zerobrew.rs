use crate::error::InstallError;
use crate::spinner::Spinner;
use crate::state::{PackageState, State};
use std::process::Command;

pub struct ZerobrewManager;

impl ZerobrewManager {
    pub fn new() -> Self {
        Self
    }

    pub fn list_installed(&self) -> Result<Vec<String>, anyhow::Error> {
        let output = Command::new("zb").arg("list").output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let packages: Vec<String> = stdout
                    .lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();
                Ok(packages)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow::anyhow!("zb list failed: {}", stderr))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(anyhow::anyhow!(
                "zerobrew not found. Please install zerobrew from https://zerobrew.rs"
            )),
            Err(e) => Err(anyhow::anyhow!("Failed to execute zb: {}", e)),
        }
    }

    /// Check if a package is installed.
    pub fn is_installed(&self, name: &str) -> Result<bool, anyhow::Error> {
        let installed = self.list_installed()?;
        Ok(installed.iter().any(|pkg| pkg == name))
    }

    pub fn uninstall_many(&self, names: &[&str]) -> Result<Vec<String>, anyhow::Error> {
        if names.is_empty() {
            return Ok(Vec::new());
        }

        let spinner = Spinner::new(format!("Uninstalling {} packages (zb)...", names.len()));

        let output = Command::new("zb").arg("uninstall").args(names).output();

        match output {
            Ok(output) if output.status.success() => {
                spinner.finish_with_message(format!("✓ Uninstalled {} packages (zb)", names.len()));
                Ok(Vec::new())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);
                spinner.finish_with_error(format!("zb uninstall failed (exit {})", exit_code));
                Err(InstallError::CommandFailed {
                    command: format!("zb uninstall {}", names.join(" ")),
                    exit_code,
                    stderr: stderr.to_string(),
                }
                .into())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                spinner.finish_with_error("zerobrew not found");
                Err(anyhow::anyhow!(
                    "zerobrew not found. Please install zerobrew from https://zerobrew.rs"
                ))
            }
            Err(e) => {
                spinner.finish_with_error(format!("Failed to execute zb: {}", e));
                Err(anyhow::anyhow!("Failed to execute zb: {}", e))
            }
        }
    }

    /// Batch-install packages. Filters out already-installed packages with a single
    /// `zb list` call, then installs all remaining with one `zb install a b c` command.
    /// Returns the list of package names that were newly installed.
    pub fn install_many(
        &self,
        names: &[&str],
        state: &mut State,
    ) -> Result<Vec<String>, Vec<(String, anyhow::Error)>> {
        if names.is_empty() {
            return Ok(Vec::new());
        }

        // Single call to get all installed packages
        let installed = self
            .list_installed()
            .map_err(|e| vec![("zb list".to_string(), e)])?;

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
                    manager: "zb".to_string(),
                });
            }
        }

        if to_install.is_empty() {
            return Ok(Vec::new());
        }

        let spinner = Spinner::new(format!(
            "Installing {} package{}... (zb)",
            to_install.len(),
            if to_install.len() == 1 { "" } else { "s" }
        ));

        let output = Command::new("zb")
            .arg("install")
            .args(&to_install)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let installed_names: Vec<String> =
                    to_install.iter().map(|s| s.to_string()).collect();
                spinner.finish_with_message(format!(
                    "✓ Installed {} packages (zb)",
                    installed_names.len()
                ));
                for name in &installed_names {
                    state.add_package(PackageState {
                        name: name.clone(),
                        manager: "zb".to_string(),
                    });
                }
                Ok(installed_names)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);
                spinner.finish_with_error(format!("zb install failed (exit {})", exit_code));
                Err(vec![(
                    format!("zb install {}", to_install.join(" ")),
                    InstallError::CommandFailed {
                        command: format!("zb install {}", to_install.join(" ")),
                        exit_code,
                        stderr: stderr.to_string(),
                    }
                    .into(),
                )])
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                spinner.finish_with_error("zerobrew not found");
                Err(vec![(
                    "zb".to_string(),
                    anyhow::anyhow!(
                        "zerobrew not found. Please install zerobrew from https://zerobrew.rs"
                    ),
                )])
            }
            Err(e) => {
                spinner.finish_with_error(format!("Failed to execute zb: {}", e));
                Err(vec![(
                    "zb".to_string(),
                    anyhow::anyhow!("Failed to execute zb: {}", e),
                )])
            }
        }
    }
}

impl Default for ZerobrewManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zerobrew_manager_new() {
        let manager = ZerobrewManager::new();
        assert!(std::mem::size_of_val(&manager) == 0);
    }

    #[test]
    fn test_zerobrew_manager_default() {
        let _manager = ZerobrewManager::default();
    }
}
