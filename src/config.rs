use crate::hooks::Hook;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub extends: Vec<ExtendsRepo>,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub dotfiles: Vec<Dotfile>,

    #[serde(default)]
    pub packages: Packages,

    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,

    #[serde(default)]
    pub hooks: Vec<Hook>,

    #[serde(default)]
    pub secrets: HashMap<String, SecretMetadata>,

    #[serde(default)]
    pub mise: MiseSection,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExtendsRepo {
    pub repo: String,

    #[serde(default)]
    pub branch: Option<String>,

    #[serde(default = "default_extends_config")]
    pub config: String,
}

fn default_extends_config() -> String {
    "mimic.toml".to_string()
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct HostConfig {
    #[serde(default)]
    pub inherits: Option<String>,

    #[serde(default)]
    pub roles: Vec<String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub dotfiles: Vec<Dotfile>,

    #[serde(default)]
    pub packages: Packages,

    #[serde(default)]
    pub hooks: Vec<Hook>,

    #[serde(default)]
    pub secrets: HashMap<String, SecretMetadata>,

    #[serde(default)]
    pub mise: MiseSection,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecretMetadata {
    pub description: Option<String>,
    pub env_var: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MiseSection {
    #[serde(default)]
    pub tools: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Dotfile {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub template: bool,
    #[serde(default)]
    pub only_roles: Option<Vec<String>>,
    #[serde(default)]
    pub skip_roles: Option<Vec<String>>,
}

impl Dotfile {
    /// Check if this dotfile should be treated as a template
    pub fn is_template(&self) -> bool {
        self.template || self.source.ends_with(".tmpl") || self.source.ends_with(".hbs")
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Packages {
    #[serde(default)]
    pub homebrew: Vec<Package>,

    /// Simple format: list of formula names
    #[serde(default)]
    pub brew: Vec<String>,

    /// Simple format: list of cask names
    #[serde(default)]
    pub cask: Vec<String>,
}

impl Packages {
    /// Merge simple format (brew/cask) into homebrew Vec<Package>
    /// This allows both formats to coexist, deduplicating by name
    pub fn normalized(&self) -> Self {
        let mut homebrew = self.homebrew.clone();

        // Add brew formulas (skip if already present in homebrew)
        for name in &self.brew {
            if !homebrew.iter().any(|p| p.name == *name) {
                homebrew.push(Package {
                    name: name.clone(),
                    pkg_type: "formula".to_string(),
                    only_roles: None,
                    skip_roles: None,
                });
            }
        }

        // Add casks (skip if already present in homebrew)
        for name in &self.cask {
            if !homebrew.iter().any(|p| p.name == *name) {
                homebrew.push(Package {
                    name: name.clone(),
                    pkg_type: "cask".to_string(),
                    only_roles: None,
                    skip_roles: None,
                });
            }
        }

        Packages {
            homebrew,
            brew: Vec::new(),
            cask: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Package {
    pub name: String,

    #[serde(rename = "type")]
    pub pkg_type: String,

    #[serde(default)]
    pub only_roles: Option<Vec<String>>,

    #[serde(default)]
    pub skip_roles: Option<Vec<String>>,
}

/// Check if a resource should be applied based on role filtering
///
/// # Arguments
/// * `only_roles` - If set, resource applies ONLY if host has at least one matching role
/// * `skip_roles` - If set, resource is skipped if host has any matching role
/// * `host_roles` - The roles assigned to the current host
///
/// # Returns
/// * `true` if the resource should be applied
/// * `false` if the resource should be skipped
///
/// # Logic
/// 1. If `skip_roles` matches any host role, return false (skip)
/// 2. If `only_roles` is set and non-empty, return true only if at least one role matches
/// 3. If `only_roles` is empty or None, return true (no restrictions)
pub fn should_apply_for_roles(
    only_roles: &Option<Vec<String>>,
    skip_roles: &Option<Vec<String>>,
    host_roles: &[String],
) -> bool {
    if let Some(skip) = skip_roles
        && skip.iter().any(|role| host_roles.contains(role))
    {
        return false;
    }

    if let Some(only) = only_roles {
        if only.is_empty() {
            return true;
        }
        return only.iter().any(|role| host_roles.contains(role));
    }

    true
}

impl Config {
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let config: Config =
            toml::from_str(content).map_err(|e| anyhow::anyhow!("TOML parse error: {}", e))?;
        Ok(config)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut loading_stack = Vec::new();
        Self::from_file_internal(path.as_ref(), &mut loading_stack)
    }

    fn from_file_internal(
        path_ref: &Path,
        loading_stack: &mut Vec<PathBuf>,
    ) -> anyhow::Result<Self> {
        use anyhow::Context;
        let resolved_path = path_ref
            .canonicalize()
            .unwrap_or_else(|_| path_ref.to_path_buf());

        if let Some(cycle_start) = loading_stack.iter().position(|p| p == &resolved_path) {
            let mut chain: Vec<String> = loading_stack[cycle_start..]
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            chain.push(resolved_path.display().to_string());
            return Err(anyhow::anyhow!(
                "Cyclic config extends detected:\n  - {}",
                chain.join("\n  - ")
            ));
        }

        loading_stack.push(resolved_path.clone());

        let result = (|| {
            let content = fs::read_to_string(path_ref).with_context(|| {
            format!(
                "Failed to read config file: {}\n\nTo fix:\n  - Check that the file exists\n  - Verify you have read permissions\n  - Ensure the path is correct",
                path_ref.display()
            )
        })?;
            let mut config = Self::from_str(&content)?;

            // Resolve relative source paths against the config file's directory
            if let Some(config_dir) = path_ref.parent() {
                // Canonicalize the config dir so relative paths become fully absolute.
                // If the parent is empty (relative config path like "mimic.toml"),
                // canonicalize fails, so fall back to the current working directory.
                let config_dir = config_dir
                    .canonicalize()
                    .or_else(|_| std::env::current_dir())
                    .unwrap_or_else(|_| config_dir.to_path_buf());
                config.resolve_source_paths(&config_dir);
                config = config.resolve_extends(loading_stack)?;
            }

            Ok(config)
        })();

        loading_stack.pop();
        result
    }

    fn resolve_extends(mut self, loading_stack: &mut Vec<PathBuf>) -> anyhow::Result<Self> {
        if self.extends.is_empty() {
            return Ok(self);
        }

        let extends = self.extends.clone();
        self.extends.clear();

        let mut merged = Config::default();

        for extend in extends {
            let extended_config_path = Self::ensure_extended_repo(&extend)?;
            let extended =
                Self::from_file_internal(&extended_config_path, loading_stack).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to load extended config from {}: {}",
                        extended_config_path.display(),
                        e
                    )
                })?;
            merged = Self::merge(merged, extended);
        }

        Ok(Self::merge(merged, self))
    }

    fn merge(base: Config, overlay: Config) -> Config {
        let mut variables = base.variables;
        variables.extend(overlay.variables);

        let mut dotfiles = base.dotfiles;
        dotfiles.extend(overlay.dotfiles);

        let mut merged_packages = base.packages.normalized();
        let overlay_packages = overlay.packages.normalized();
        for pkg in overlay_packages.homebrew {
            if let Some(existing) = merged_packages
                .homebrew
                .iter_mut()
                .find(|existing| existing.name == pkg.name && existing.pkg_type == pkg.pkg_type)
            {
                *existing = pkg;
            } else {
                merged_packages.homebrew.push(pkg);
            }
        }

        let mut hosts = base.hosts;
        hosts.extend(overlay.hosts);

        let mut hooks = base.hooks;
        hooks.extend(overlay.hooks);

        let mut secrets = base.secrets;
        secrets.extend(overlay.secrets);

        let mut mise = base.mise;
        mise.tools.extend(overlay.mise.tools);

        Config {
            extends: Vec::new(),
            variables,
            dotfiles,
            packages: merged_packages,
            hosts,
            hooks,
            secrets,
            mise,
        }
    }

    fn ensure_extended_repo(extend: &ExtendsRepo) -> anyhow::Result<PathBuf> {
        let base_dirs = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?;

        let repo_dir = base_dirs
            .config_dir()
            .join("mimic/extends")
            .join(Self::extends_repo_id(
                &extend.repo,
                extend.branch.as_deref(),
            ));

        if repo_dir.exists() {
            Self::git_update_extended_repo(&repo_dir, extend)?;
        } else {
            if let Some(parent) = repo_dir.parent() {
                fs::create_dir_all(parent)?;
            }
            Self::git_clone_extended_repo(&repo_dir, extend)?;
        }

        let config_path = repo_dir.join(&extend.config);
        if !config_path.exists() {
            return Err(anyhow::anyhow!(
                "Extended config file not found: {}\n\nTo fix:\n  - Verify the `config` path in [[extends]]\n  - Ensure repository '{}' contains that file",
                config_path.display(),
                extend.repo
            ));
        }

        Ok(config_path)
    }

    fn git_clone_extended_repo(repo_dir: &Path, extend: &ExtendsRepo) -> anyhow::Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth").arg("1");
        if let Some(branch) = &extend.branch {
            cmd.arg("--branch").arg(branch).arg("--single-branch");
        }
        cmd.arg(&extend.repo).arg(repo_dir);

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "Git is not installed or not in PATH\n\nTo fix:\n  - Install git: brew install git (macOS)\n  - Or: apt install git (Linux)"
                )
            } else {
                anyhow::anyhow!("Failed to execute git clone for '{}': {}", extend.repo, e)
            }
        })?;
        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);

        // If it looks like an auth problem, try gh-based authentication and retry
        if crate::git_auth::is_auth_error(&stderr) {
            crate::git_auth::ensure_gh_auth()?;

            // Clean up any partial clone
            if repo_dir.exists() {
                fs::remove_dir_all(repo_dir).ok();
            }

            let mut retry_cmd = Command::new("git");
            retry_cmd.arg("clone").arg("--depth").arg("1");
            if let Some(branch) = &extend.branch {
                retry_cmd
                    .arg("--branch")
                    .arg(branch)
                    .arg("--single-branch");
            }
            retry_cmd.arg(&extend.repo).arg(repo_dir);

            let retry_output = retry_cmd.output().map_err(|e| {
                anyhow::anyhow!("Failed to execute git clone for '{}': {}", extend.repo, e)
            })?;

            if retry_output.status.success() {
                return Ok(());
            }

            let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
            return Err(anyhow::anyhow!(Self::format_extended_repo_error(
                "clone",
                extend,
                &retry_stderr
            )));
        }

        Err(anyhow::anyhow!(Self::format_extended_repo_error(
            "clone", extend, &stderr
        )))
    }

    fn git_update_extended_repo(repo_dir: &Path, extend: &ExtendsRepo) -> anyhow::Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(repo_dir).arg("pull").arg("--ff-only");
        if let Some(branch) = &extend.branch {
            cmd.arg("origin").arg(branch);
        }

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "Git is not installed or not in PATH\n\nTo fix:\n  - Install git: brew install git (macOS)\n  - Or: apt install git (Linux)"
                )
            } else {
                anyhow::anyhow!(
                    "Failed to execute git pull for extended repo '{}': {}",
                    extend.repo,
                    e
                )
            }
        })?;
        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(Self::format_extended_repo_error(
            "update", extend, &stderr
        )))
    }

    fn format_extended_repo_error(action: &str, extend: &ExtendsRepo, stderr: &str) -> String {
        let trimmed = stderr.trim();

        if trimmed.contains("Authentication failed")
            || trimmed.contains("Permission denied")
            || trimmed.contains("could not read Username")
            || trimmed.contains("Repository not found")
        {
            return format!(
                "Failed to {} extended repo '{}': authentication failed\n\nTo fix:\n  - Use an SSH URL and ensure your key is loaded (e.g. git@github.com:org/repo.git)\n  - Or configure git credential helper for HTTPS private repositories\n  - Verify your account has access to the repository\n\nGit output:\n{}",
                action, extend.repo, trimmed
            );
        }

        format!(
            "Failed to {} extended repo '{}':\n{}",
            action, extend.repo, trimmed
        )
    }

    fn extends_repo_id(repo: &str, branch: Option<&str>) -> String {
        let raw = if let Some(branch) = branch {
            format!("{}#{}", repo, branch)
        } else {
            repo.to_string()
        };

        let mut hasher = DefaultHasher::new();
        raw.hash(&mut hasher);
        let digest = format!("{:016x}", hasher.finish());

        let mut id: String = raw
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect();

        if id.is_empty() {
            id = "repo".to_string();
        }

        if id.len() > 80 {
            id.truncate(80);
        }

        format!("{}_{}", id, digest)
    }

    /// Resolve all relative dotfile source paths against the given base directory.
    /// This ensures source paths work regardless of the current working directory.
    pub fn resolve_source_paths(&mut self, base_dir: &Path) {
        for dotfile in &mut self.dotfiles {
            dotfile.source = Self::resolve_relative_path(&dotfile.source, base_dir);
        }
        for host_config in self.hosts.values_mut() {
            for dotfile in &mut host_config.dotfiles {
                dotfile.source = Self::resolve_relative_path(&dotfile.source, base_dir);
            }
        }
    }

    /// If a path is relative (doesn't start with `/` or `~` or `$`), join it with the base directory.
    fn resolve_relative_path(path: &str, base_dir: &Path) -> String {
        let p = Path::new(path);
        if p.is_absolute() || path.starts_with('~') || path.starts_with('$') {
            path.to_string()
        } else {
            base_dir.join(p).to_string_lossy().to_string()
        }
    }

    /// Merge the base config with a specific host configuration
    pub fn with_host(&self, host_name: &str) -> anyhow::Result<Config> {
        let host = self
            .hosts
            .get(host_name)
            .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in config", host_name))?;

        let mut merged_vars = self.variables.clone();
        for (key, value) in &host.variables {
            merged_vars.insert(key.clone(), value.clone());
        }

        let mut merged_dotfiles = self.dotfiles.clone();
        merged_dotfiles.extend(host.dotfiles.clone());

        let mut merged_packages = self.packages.normalized();
        let host_packages = host.packages.normalized();
        for pkg in host_packages.homebrew {
            if !merged_packages.homebrew.iter().any(|p| p.name == pkg.name) {
                merged_packages.homebrew.push(pkg);
            }
        }

        let mut merged_hooks = self.hooks.clone();
        merged_hooks.extend(host.hooks.clone());

        let mut merged_secrets = self.secrets.clone();
        for (key, value) in &host.secrets {
            merged_secrets.insert(key.clone(), value.clone());
        }

        let mut merged_mise = self.mise.clone();
        for (key, value) in &host.mise.tools {
            merged_mise.tools.insert(key.clone(), value.clone());
        }

        Ok(Config {
            extends: Vec::new(),
            variables: merged_vars,
            dotfiles: merged_dotfiles,
            packages: merged_packages,
            hosts: self.hosts.clone(),
            hooks: merged_hooks,
            secrets: merged_secrets,
            mise: merged_mise,
        })
    }

    /// Get list of configured host names
    pub fn host_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.hosts.keys().cloned().collect();
        names.sort();
        names
    }
}
