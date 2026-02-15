use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Hook {
    #[serde(rename = "rustup")]
    Rustup {
        toolchains: Vec<String>,
        components: Vec<String>,
        targets: Vec<String>,
        #[serde(default)]
        default: Option<String>,
        #[serde(default)]
        only_roles: Option<Vec<String>>,
        #[serde(default)]
        skip_roles: Option<Vec<String>>,
    },
    #[serde(rename = "cargo-install")]
    CargoInstall {
        packages: Vec<CargoPackage>,
        #[serde(default)]
        only_roles: Option<Vec<String>>,
        #[serde(default)]
        skip_roles: Option<Vec<String>>,
    },
    #[serde(rename = "mise")]
    Mise {
        #[serde(default)]
        only_roles: Option<Vec<String>>,
        #[serde(default)]
        skip_roles: Option<Vec<String>>,
    },
    #[serde(rename = "pnpm-global")]
    PnpmGlobal {
        packages: Vec<String>,
        #[serde(default)]
        only_roles: Option<Vec<String>>,
        #[serde(default)]
        skip_roles: Option<Vec<String>>,
    },
    #[serde(rename = "uv-python")]
    UvPython {
        version: String,
        symlinks: HashMap<String, String>,
        #[serde(default)]
        only_roles: Option<Vec<String>>,
        #[serde(default)]
        skip_roles: Option<Vec<String>>,
    },
    #[serde(rename = "command")]
    Command {
        name: String,
        command: String,
        on_failure: FailureMode,
        #[serde(default)]
        only_roles: Option<Vec<String>>,
        #[serde(default)]
        skip_roles: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CargoPackage {
    pub name: String,
    pub git: String,
    #[serde(default)]
    pub bin: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum FailureMode {
    #[default]
    Continue,
    Fail,
}

impl Hook {
    pub fn name(&self) -> String {
        match self {
            Hook::Rustup { .. } => "rustup".to_string(),
            Hook::CargoInstall { .. } => "cargo-install".to_string(),
            Hook::Mise { .. } => "mise".to_string(),
            Hook::PnpmGlobal { .. } => "pnpm-global".to_string(),
            Hook::UvPython { .. } => "uv-python".to_string(),
            Hook::Command { name, .. } => name.clone(),
        }
    }

    pub fn only_roles(&self) -> &Option<Vec<String>> {
        match self {
            Hook::Rustup { only_roles, .. } => only_roles,
            Hook::CargoInstall { only_roles, .. } => only_roles,
            Hook::Mise { only_roles, .. } => only_roles,
            Hook::PnpmGlobal { only_roles, .. } => only_roles,
            Hook::UvPython { only_roles, .. } => only_roles,
            Hook::Command { only_roles, .. } => only_roles,
        }
    }

    pub fn skip_roles(&self) -> &Option<Vec<String>> {
        match self {
            Hook::Rustup { skip_roles, .. } => skip_roles,
            Hook::CargoInstall { skip_roles, .. } => skip_roles,
            Hook::Mise { skip_roles, .. } => skip_roles,
            Hook::PnpmGlobal { skip_roles, .. } => skip_roles,
            Hook::UvPython { skip_roles, .. } => skip_roles,
            Hook::Command { skip_roles, .. } => skip_roles,
        }
    }
}

/// Execute all hooks in sequence, filtering by roles
pub fn execute_hooks(hooks: &[Hook], host_roles: &[String], verbose: bool) -> anyhow::Result<()> {
    if hooks.is_empty() {
        return Ok(());
    }

    for hook in hooks {
        if !crate::config::should_apply_for_roles(hook.only_roles(), hook.skip_roles(), host_roles)
        {
            if verbose {
                println!("  {} {} (role mismatch)", "↷".bright_black(), hook.name());
            }
            continue;
        }

        execute_hook(hook, verbose)?;
    }

    Ok(())
}

/// Execute a single hook
fn execute_hook(hook: &Hook, verbose: bool) -> anyhow::Result<()> {
    println!();
    println!("{} {}", "→ Hook:".bright_cyan(), hook.name());

    let result = match hook {
        Hook::Rustup {
            toolchains,
            components,
            targets,
            default,
            ..
        } => execute_rustup_hook(toolchains, components, targets, default.as_deref(), verbose),
        Hook::CargoInstall { packages, .. } => execute_cargo_install_hook(packages, verbose),
        Hook::Mise { .. } => execute_mise_hook(verbose),
        Hook::PnpmGlobal { packages, .. } => execute_pnpm_global_hook(packages, verbose),
        Hook::UvPython {
            version, symlinks, ..
        } => execute_uv_python_hook(version, symlinks, verbose),
        Hook::Command {
            command,
            on_failure,
            ..
        } => execute_command_hook(command, on_failure, verbose),
    };

    match result {
        Ok(()) => {
            println!("  {} Hook completed", "✓".green());
            Ok(())
        }
        Err(e) => {
            let should_fail = match hook {
                Hook::Command { on_failure, .. } => matches!(on_failure, FailureMode::Fail),
                _ => false,
            };

            if should_fail {
                Err(e)
            } else {
                eprintln!("  {} Hook failed (continuing): {}", "⚠".yellow(), e);
                Ok(())
            }
        }
    }
}

fn execute_rustup_hook(
    toolchains: &[String],
    components: &[String],
    targets: &[String],
    default: Option<&str>,
    verbose: bool,
) -> anyhow::Result<()> {
    if !command_exists("rustup") {
        println!("  {} Installing rustup...", "→".bright_black());
        let status = Command::new("sh")
            .arg("-c")
            .arg("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --no-modify-path")
            .stdout(if verbose { Stdio::inherit() } else { Stdio::null() })
            .stderr(if verbose { Stdio::inherit() } else { Stdio::null() })
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to install rustup"));
        }
    }

    for toolchain in toolchains {
        if verbose {
            println!(
                "  {} Installing {} toolchain...",
                "→".bright_black(),
                toolchain
            );
        }
        let status = Command::new("rustup")
            .args(["toolchain", "install", toolchain])
            .stdout(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stderr(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to install {} toolchain", toolchain));
        }
    }

    for toolchain in toolchains {
        for component in components {
            if verbose {
                println!(
                    "  {} Adding {} to {}...",
                    "→".bright_black(),
                    component,
                    toolchain
                );
            }
            let status = Command::new("rustup")
                .args(["component", "add", "--toolchain", toolchain, component])
                .stdout(if verbose {
                    Stdio::inherit()
                } else {
                    Stdio::null()
                })
                .stderr(if verbose {
                    Stdio::inherit()
                } else {
                    Stdio::null()
                })
                .status()?;

            if !status.success() {
                eprintln!(
                    "  {} Failed to add {} to {} (continuing)",
                    "⚠".yellow(),
                    component,
                    toolchain
                );
            }
        }
    }

    for toolchain in toolchains {
        for target in targets {
            if verbose {
                println!(
                    "  {} Adding target {} to {}...",
                    "→".bright_black(),
                    target,
                    toolchain
                );
            }
            let status = Command::new("rustup")
                .args(["target", "add", "--toolchain", toolchain, target])
                .stdout(if verbose {
                    Stdio::inherit()
                } else {
                    Stdio::null()
                })
                .stderr(if verbose {
                    Stdio::inherit()
                } else {
                    Stdio::null()
                })
                .status()?;

            if !status.success() {
                eprintln!(
                    "  {} Failed to add target {} to {} (continuing)",
                    "⚠".yellow(),
                    target,
                    toolchain
                );
            }
        }
    }

    if let Some(default_toolchain) = default {
        if verbose {
            println!(
                "  {} Setting default toolchain to {}...",
                "→".bright_black(),
                default_toolchain
            );
        }
        let status = Command::new("rustup")
            .args(["default", default_toolchain])
            .stdout(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stderr(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Failed to set default toolchain to {}",
                default_toolchain
            ));
        }
    }

    Ok(())
}

fn execute_cargo_install_hook(packages: &[CargoPackage], verbose: bool) -> anyhow::Result<()> {
    if !command_exists("cargo") {
        return Err(anyhow::anyhow!("cargo not found - install Rust first"));
    }

    for package in packages {
        if verbose {
            println!("  {} Installing {}...", "→".bright_black(), package.name);
        }

        let mut args = vec!["install", &package.name, "--git", &package.git, "--locked"];

        if let Some(bin) = &package.bin {
            args.extend(["--bin", bin]);
        }

        let status = Command::new("cargo")
            .args(&args)
            .stdout(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stderr(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .status()?;

        if !status.success() {
            eprintln!(
                "  {} Failed to install {} (continuing)",
                "⚠".yellow(),
                package.name
            );
        }
    }

    Ok(())
}

fn execute_mise_hook(verbose: bool) -> anyhow::Result<()> {
    if !command_exists("mise") {
        return Err(anyhow::anyhow!("mise not found - install mise first"));
    }

    if verbose {
        println!("  {} Running mise install...", "→".bright_black());
    }

    let status = Command::new("mise")
        .arg("install")
        .stdout(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stderr(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("mise install failed"));
    }

    Ok(())
}

fn execute_pnpm_global_hook(packages: &[String], verbose: bool) -> anyhow::Result<()> {
    if !command_exists("pnpm") {
        return Err(anyhow::anyhow!("pnpm not found - install pnpm first"));
    }

    for package in packages {
        if verbose {
            println!("  {} Installing {}...", "→".bright_black(), package);
        }

        let status = Command::new("pnpm")
            .args(["add", "-g", package])
            .stdout(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stderr(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .status()?;

        if !status.success() {
            eprintln!(
                "  {} Failed to install {} (continuing)",
                "⚠".yellow(),
                package
            );
        }
    }

    Ok(())
}

fn execute_uv_python_hook(
    version: &str,
    symlinks: &HashMap<String, String>,
    verbose: bool,
) -> anyhow::Result<()> {
    if !command_exists("uv") {
        return Err(anyhow::anyhow!("uv not found - install uv first"));
    }

    if verbose {
        println!("  {} Installing Python {}...", "→".bright_black(), version);
    }

    let status = Command::new("uv")
        .args(["python", "install", version])
        .stdout(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stderr(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to install Python {}", version));
    }

    let output = Command::new("uv")
        .args(["python", "find", version])
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to find Python {}", version));
    }

    let python_bin = String::from_utf8(output.stdout)?.trim().to_string();

    for (name, target) in symlinks {
        let expanded_target = shellexpand::tilde(target).to_string();

        if verbose {
            println!(
                "  {} Creating symlink {} → {}",
                "→".bright_black(),
                name,
                expanded_target
            );
        }

        if let Some(parent) = std::path::Path::new(&expanded_target).parent() {
            std::fs::create_dir_all(parent)?;
        }

        if std::path::Path::new(&expanded_target).exists() {
            std::fs::remove_file(&expanded_target)?;
        }

        std::os::unix::fs::symlink(&python_bin, &expanded_target)?;
    }

    Ok(())
}

fn execute_command_hook(
    command: &str,
    _on_failure: &FailureMode,
    verbose: bool,
) -> anyhow::Result<()> {
    if verbose {
        println!("  {} Running: {}", "→".bright_black(), command);
    }

    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stderr(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Command failed: {}", command));
    }

    Ok(())
}

/// Check if a command exists in PATH
fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_name() {
        let hook = Hook::Rustup {
            toolchains: vec![],
            components: vec![],
            targets: vec![],
            default: None,
            only_roles: None,
            skip_roles: None,
        };
        assert_eq!(hook.name(), "rustup");

        let hook = Hook::Command {
            name: "test".to_string(),
            command: "echo test".to_string(),
            on_failure: FailureMode::Continue,
            only_roles: None,
            skip_roles: None,
        };
        assert_eq!(hook.name(), "test");
    }

    #[test]
    fn test_command_exists() {
        assert!(command_exists("sh"));
        assert!(!command_exists(
            "this_command_definitely_does_not_exist_12345"
        ));
    }

    #[test]
    fn test_failure_mode_default() {
        let mode = FailureMode::default();
        assert_eq!(mode, FailureMode::Continue);
    }
}
