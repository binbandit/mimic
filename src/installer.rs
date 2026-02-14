use crate::error::InstallError;
use crate::state::{PackageState, State};
use std::process::Command;

pub struct HomebrewManager;

impl HomebrewManager {
    pub fn new() -> Self {
        Self
    }

    pub fn list_installed(&self) -> Result<Vec<String>, anyhow::Error> {
        let output = Command::new("brew")
            .arg("list")
            .arg("--formula")
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
                Ok(packages)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow::anyhow!("brew list failed: {}", stderr))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(anyhow::anyhow!(
                "Homebrew not found. Please install Homebrew from https://brew.sh"
            )),
            Err(e) => Err(anyhow::anyhow!("Failed to execute brew: {}", e)),
        }
    }

    pub fn is_installed(&self, name: &str) -> Result<bool, anyhow::Error> {
        let installed = self.list_installed()?;
        Ok(installed.iter().any(|pkg| pkg == name))
    }

    pub fn install(
        &self,
        name: &str,
        _package_type: &str,
        state: &mut State,
    ) -> Result<(), anyhow::Error> {
        let already_installed = self.is_installed(name)?;

        if already_installed {
            if !state.packages.iter().any(|p| p.name == name) {
                state.add_package(PackageState {
                    name: name.to_string(),
                    manager: "brew".to_string(),
                });
            }
            return Ok(());
        }

        let output = Command::new("brew").arg("install").arg(name).output();

        match output {
            Ok(output) if output.status.success() => {
                state.add_package(PackageState {
                    name: name.to_string(),
                    manager: "brew".to_string(),
                });
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);
                Err(InstallError::CommandFailed {
                    command: format!("brew install {}", name),
                    exit_code,
                    stderr: stderr.to_string(),
                }
                .into())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(anyhow::anyhow!(
                "Homebrew not found. Please install Homebrew from https://brew.sh"
            )),
            Err(e) => Err(anyhow::anyhow!("Failed to execute brew: {}", e)),
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
        let _manager = HomebrewManager::default();
    }
}
