use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::config;
use crate::config::{Config, should_apply_for_roles};
use crate::diff::{Change, DiffEngine};
use crate::git_auth;
use crate::hooks;
use crate::installer::HomebrewManager;
use crate::linker::{ApplyToAllChoice, apply_dotfile};
use crate::state::State;
use crate::template::HostContext;
use crate::zerobrew::ZerobrewManager;
use anyhow::Context;

#[derive(Parser)]
#[command(name = "mimic")]
#[command(version = "0.1.0")]
#[command(about = "Dotfile management system", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Path to config file")]
    pub config: Option<PathBuf>,

    #[arg(short = 'H', long, global = true, help = "Select host configuration")]
    pub host: Option<String>,

    #[arg(short, long, global = true, help = "Skip confirmation prompts")]
    pub yes: bool,

    #[arg(
        short = 'n',
        long,
        global = true,
        help = "Show what would be done without doing it"
    )]
    pub dry_run: bool,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(
        long,
        global = true,
        help = "Never touch the network: use cached extends repos as-is"
    )]
    pub offline: bool,

    #[arg(long, global = true, help = "Path to state file")]
    pub state: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        help = "Use a specific initialized dotfiles branch"
    )]
    pub branch: Option<String>,
}

/// Outcome of matching the requested/detected hostname against `[hosts.*]`.
enum HostResolution {
    /// Config has no host entries at all — single-machine setup.
    NoHosts(Config),
    /// A host entry matched; `merged` is the effective config for it.
    Matched { key: String, merged: Config },
    /// Hostname was auto-detected but matches nothing — defaults apply.
    Unmatched { requested: String, base: Config },
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplySection {
    Dotfiles,
    Packages,
    Hooks,
    Mise,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Apply configuration changes")]
    Apply {
        #[arg(
            long,
            value_enum,
            value_delimiter = ',',
            help = "Apply only these sections (dotfiles, packages, hooks, mise)"
        )]
        only: Vec<ApplySection>,

        #[arg(
            long,
            value_enum,
            value_delimiter = ',',
            help = "Skip these sections (dotfiles, packages, hooks, mise)"
        )]
        skip: Vec<ApplySection>,
    },

    #[command(about = "Show preview of changes without applying")]
    Diff,

    #[command(about = "Check for drift from last applied configuration")]
    Status,

    #[command(about = "Run read-only health checks on config, links, and packages")]
    Doctor,

    #[command(about = "Undo last apply operation")]
    Undo,

    #[command(about = "Manage host configurations", subcommand)]
    Hosts(HostCommands),

    #[command(about = "Render a template file to preview output")]
    Render {
        #[arg(help = "Path to template file")]
        template: String,
    },

    #[command(about = "Manage secrets in macOS Keychain", subcommand)]
    Secrets(SecretsCommands),

    #[command(about = "Initialize from a git repository")]
    Init {
        #[arg(help = "Repository URL to clone")]
        repo: String,

        #[arg(long, help = "Automatically apply configuration after cloning")]
        apply: bool,
    },

    #[command(about = "Open the source file for a target in your editor")]
    Edit {
        #[arg(help = "Target path to edit (e.g., ~/.zshrc)")]
        target: String,
    },

    #[command(about = "Remove brew packages not listed in config")]
    Clean,
}

#[derive(Subcommand)]
pub enum SecretsCommands {
    #[command(about = "Store a secret")]
    Set {
        #[arg(help = "Secret key name")]
        key: String,

        #[arg(long, help = "Read value from stdin")]
        stdin: bool,
    },

    #[command(about = "Retrieve a secret")]
    Get {
        #[arg(help = "Secret key name")]
        key: String,
    },

    #[command(about = "List all secrets")]
    List,

    #[command(about = "Remove a secret")]
    Rm {
        #[arg(help = "Secret key name")]
        key: String,
    },

    #[command(about = "Export secrets as shell environment variables")]
    Export,
}

#[derive(Subcommand)]
pub enum HostCommands {
    #[command(about = "List all configured hosts with their effective roles")]
    List,

    #[command(about = "Show merged (effective) configuration for a specific host")]
    Show {
        #[arg(help = "Name of the host to show")]
        name: String,

        #[arg(long, help = "Show the host's own section without merging defaults")]
        raw: bool,

        #[arg(long, help = "Show only what the host overrides or adds vs defaults")]
        diff: bool,
    },
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        match &self.command {
            Commands::Apply { only, skip } => self.run_apply(only, skip),
            Commands::Diff => self.run_diff(),
            Commands::Status => self.run_status(),
            Commands::Doctor => self.run_doctor(),
            Commands::Undo => self.run_undo(),
            Commands::Hosts(hosts_cmd) => self.run_hosts(hosts_cmd),
            Commands::Render { template } => self.run_render(template),
            Commands::Secrets(secrets_cmd) => self.run_secrets(secrets_cmd),
            Commands::Init { repo, apply } => self.run_init(repo, *apply),
            Commands::Edit { target } => self.run_edit(target),
            Commands::Clean => self.run_clean(),
        }
    }

    fn find_config(&self) -> anyhow::Result<PathBuf> {
        if let Some(config_path) = &self.config {
            return Ok(config_path.clone());
        }

        let cwd_config = PathBuf::from("mimic.toml");
        if cwd_config.exists() {
            return Ok(cwd_config);
        }

        if let Some(base_dirs) = directories::BaseDirs::new() {
            let home = base_dirs.home_dir();
            let mut candidates = Vec::new();

            if let Some(branch) = &self.branch {
                candidates.push(
                    base_dirs
                        .config_dir()
                        .join("mimic/repos")
                        .join(branch)
                        .join("mimic.toml"),
                );
            }

            candidates.extend([
                home.join("mimic.toml"),
                base_dirs.config_dir().join("mimic/mimic.toml"),
                base_dirs.config_dir().join("mimic/config.toml"),
                base_dirs.config_dir().join("mimic/repo/mimic.toml"),
                home.join(".dots/mimic.toml"),
                home.join(".dotfiles/mimic.toml"),
            ]);

            for candidate in candidates {
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }

        let branch_hint = self
            .branch
            .as_ref()
            .map(|branch| format!("\n  - ~/.config/mimic/repos/{}/mimic.toml", branch))
            .unwrap_or_default();

        Err(anyhow::anyhow!(
            "Config file not found. Searched:\n  - ./mimic.toml\n  - ~/mimic.toml\n  - ~/.config/mimic/mimic.toml\n  - ~/.config/mimic/config.toml\n  - ~/.config/mimic/repo/mimic.toml{}\n  - ~/.dots/mimic.toml\n  - ~/.dotfiles/mimic.toml\n\nUse --config to specify a custom path, or run 'mimic init <repo>' to get started.",
            branch_hint
        ))
    }

    fn repo_dir(&self) -> anyhow::Result<PathBuf> {
        let base_dirs = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?;

        let repo_dir = if let Some(branch) = &self.branch {
            base_dirs.config_dir().join("mimic/repos").join(branch)
        } else {
            base_dirs.config_dir().join("mimic/repo")
        };

        Ok(repo_dir)
    }

    fn get_state_path(&self) -> PathBuf {
        if let Some(state_path) = &self.state {
            return state_path.clone();
        }

        if let Some(config_dir) = directories::BaseDirs::new() {
            return config_dir.config_dir().join("mimic").join("state.toml");
        }

        PathBuf::from(".mimic/state.toml")
    }

    fn detect_hostname() -> String {
        whoami::hostname().unwrap_or_else(|_| "unknown".to_string())
    }

    fn load_options(&self) -> crate::config::LoadOptions {
        crate::config::LoadOptions {
            offline: self.offline,
        }
    }

    /// Load a config file honoring global flags (--offline).
    fn load_config(&self, path: &Path) -> anyhow::Result<Config> {
        if self.verbose {
            println!("{} {}", "Loading config:".bright_black(), path.display());
        }
        Config::from_file_with_options(path, self.load_options())
    }

    fn host_not_found_error(name: &str, config: &Config) -> anyhow::Error {
        anyhow::anyhow!(
            "Host '{}' not found in config\n\nAvailable hosts: {}",
            name,
            config.host_names().join(", ")
        )
    }

    fn unmatched_host_message(requested: &str) -> String {
        format!(
            "no [hosts] entry matches '{}' — applying defaults only. Add [hosts.{}] or list it under an existing host's aliases",
            requested, requested
        )
    }

    /// Resolve the requested/detected hostname against the config's host
    /// entries. Owns the resolution policy so every command (apply, diff,
    /// render, doctor) reports the same outcome; callers decide how to
    /// present it.
    fn resolve_host(&self, base_config: Config) -> anyhow::Result<HostResolution> {
        if base_config.hosts.is_empty() {
            return Ok(HostResolution::NoHosts(base_config));
        }

        let requested = self.host.clone().unwrap_or_else(Self::detect_hostname);

        match base_config.resolve_host_name(&requested)? {
            Some(key) => {
                let merged = base_config.with_host(&key)?;
                Ok(HostResolution::Matched { key, merged })
            }
            // An explicitly requested host that doesn't exist is an error;
            // an unmatched *detected* hostname just means "defaults only".
            None if self.host.is_some() => {
                Err(Self::host_not_found_error(&requested, &base_config))
            }
            None => Ok(HostResolution::Unmatched {
                requested,
                base: base_config,
            }),
        }
    }

    fn resolve_config_and_host(&self) -> anyhow::Result<(Config, Option<String>)> {
        let config_path = self.find_config()?;
        let base_config = self.load_config(&config_path)?;

        match self.resolve_host(base_config)? {
            HostResolution::NoHosts(config) => Ok((config, None)),
            HostResolution::Matched { key, merged } => {
                if self.verbose {
                    println!("{} {}", "Using host:".bright_black(), key);
                }
                Ok((merged, Some(key)))
            }
            HostResolution::Unmatched { requested, base } => {
                eprintln!(
                    "{} {}",
                    "Warning:".yellow().bold(),
                    Self::unmatched_host_message(&requested)
                );
                Ok((base, Some(requested)))
            }
        }
    }

    fn run_hosts(&self, cmd: &HostCommands) -> anyhow::Result<()> {
        let config_path = self.find_config()?;
        let config = self.load_config(&config_path)?;

        match cmd {
            HostCommands::List => {
                if config.hosts.is_empty() {
                    println!("{}", "No hosts configured.".yellow());
                    println!("Add a [hosts.name] section to your config file.");
                    return Ok(());
                }

                println!("{}", "Configured hosts:".bold());
                for name in config.host_names() {
                    if let Some(host_config) = config.hosts.get(&name) {
                        // Effective roles, including roles inherited from parents
                        let resolved_roles = config
                            .resolved_host_roles(&name)
                            .unwrap_or_else(|_| host_config.roles.clone());
                        let roles = if resolved_roles.is_empty() {
                            "no roles".bright_black().to_string()
                        } else {
                            resolved_roles.join(", ")
                        };

                        let mut annotations = Vec::new();
                        if let Some(parent) = &host_config.inherits {
                            annotations.push(format!("inherits {}", parent));
                        }
                        if !host_config.aliases.is_empty() {
                            annotations
                                .push(format!("aliases: {}", host_config.aliases.join(", ")));
                        }
                        let suffix = if annotations.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", annotations.join("; "))
                                .bright_black()
                                .to_string()
                        };

                        println!("  {} ({}){}", name.green(), roles, suffix);
                    }
                }
                Ok(())
            }
            HostCommands::Show { name, raw, diff } => {
                let resolved_name = config
                    .resolve_host_name(name)?
                    .ok_or_else(|| Self::host_not_found_error(name, &config))?;

                if *raw {
                    return Self::show_host_raw(&config, &resolved_name);
                }
                if *diff {
                    return Self::show_host_diff(&config, &resolved_name);
                }
                Self::show_host_merged(&config, &resolved_name)
            }
        }
    }

    fn sorted_vars(vars: &std::collections::HashMap<String, String>) -> Vec<(&String, &String)> {
        let mut entries: Vec<_> = vars.iter().collect();
        entries.sort_by_key(|(k, _)| k.as_str());
        entries
    }

    fn print_host_header(config: &Config, name: &str) {
        println!("{} {}", "Host:".bold(), name.green());
        let roles = config.resolved_host_roles(name).unwrap_or_default();
        if !roles.is_empty() {
            println!("{} {}", "Roles:".bold(), roles.join(", "));
        }
        if let Some(host) = config.hosts.get(name) {
            if let Some(parent) = &host.inherits {
                println!("{} {}", "Inherits:".bold(), parent);
            }
            if !host.aliases.is_empty() {
                println!("{} {}", "Aliases:".bold(), host.aliases.join(", "));
            }
        }
        println!();
    }

    /// Print the Variables / Dotfiles / Packages blocks shared by the merged
    /// and raw views of `hosts show`.
    fn print_host_sections(
        variables: &std::collections::HashMap<String, String>,
        dotfiles: &[config::Dotfile],
        packages: &config::Packages,
    ) {
        println!("{}", "Variables:".bold());
        if variables.is_empty() {
            println!("  {}", "(none)".bright_black());
        } else {
            for (key, value) in Self::sorted_vars(variables) {
                println!("  {} = {}", key, value);
            }
        }
        println!();

        println!("{}", "Dotfiles:".bold());
        if dotfiles.is_empty() {
            println!("  {}", "(none)".bright_black());
        } else {
            for dotfile in dotfiles {
                println!("  {} → {}", dotfile.source, dotfile.target);
            }
        }
        println!();

        println!("{}", "Packages:".bold());
        let packages = packages.normalized();
        if packages.homebrew.is_empty() && packages.zerobrew.is_empty() {
            println!("  {}", "(none)".bright_black());
        } else {
            for package in &packages.homebrew {
                println!("  {} ({})", package.name, package.pkg_type);
            }
            for package in &packages.zerobrew {
                println!("  {} (zb)", package.name);
            }
        }
    }

    fn show_host_merged(config: &Config, name: &str) -> anyhow::Result<()> {
        let merged = config.with_host(name)?;

        Self::print_host_header(config, name);
        Self::print_host_sections(&merged.variables, &merged.dotfiles, &merged.packages);

        if !merged.hooks.is_empty() {
            println!();
            println!("{}", "Hooks:".bold());
            for hook in &merged.hooks {
                println!("  {}", hook.name());
            }
        }

        if !merged.mise.tools.is_empty() {
            println!();
            println!("{}", "Mise tools:".bold());
            for (tool, version) in Self::sorted_vars(&merged.mise.tools) {
                println!("  {} = {}", tool, version);
            }
        }

        Ok(())
    }

    /// Show only the host's own section, without merging global defaults or
    /// the inheritance chain.
    fn show_host_raw(config: &Config, name: &str) -> anyhow::Result<()> {
        let host = config
            .hosts
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in config", name))?;

        println!(
            "{} {} {}",
            "Host:".bold(),
            name.green(),
            "(raw)".bright_black()
        );
        if let Some(parent) = &host.inherits {
            println!("{} {}", "Inherits:".bold(), parent);
        }
        if !host.aliases.is_empty() {
            println!("{} {}", "Aliases:".bold(), host.aliases.join(", "));
        }
        if !host.roles.is_empty() {
            println!("{} {}", "Roles:".bold(), host.roles.join(", "));
        }
        println!();

        Self::print_host_sections(&host.variables, &host.dotfiles, &host.packages);

        Ok(())
    }

    /// Lines for map entries the host chain added (`+`) or overrode (`~`)
    /// relative to the base map. Used for both variables and mise tools.
    fn map_override_lines(
        merged: &std::collections::HashMap<String, String>,
        base: &std::collections::HashMap<String, String>,
    ) -> Vec<String> {
        Self::sorted_vars(merged)
            .into_iter()
            .filter_map(|(key, value)| match base.get(key) {
                None => Some(format!("  {} {} = {}", "+".green(), key, value)),
                Some(default) if default != value => Some(format!(
                    "  {} {} = {} {}",
                    "~".yellow(),
                    key,
                    value,
                    format!("(default: {})", default).bright_black()
                )),
                Some(_) => None,
            })
            .collect()
    }

    /// Show only what the host's inheritance chain overrides or adds compared
    /// to the global defaults.
    fn show_host_diff(config: &Config, name: &str) -> anyhow::Result<()> {
        let merged = config.with_host(name)?;

        println!(
            "{} {} {}",
            "Host:".bold(),
            name.green(),
            "(overrides vs defaults)".bright_black()
        );
        println!();

        // Host-chain dotfiles and hooks are appended after the defaults, so
        // everything past the base length was added by this host's chain.
        // Packages merge by name+type instead, so those are set-diffed.
        let dotfile_lines: Vec<String> = merged.dotfiles[config.dotfiles.len()..]
            .iter()
            .map(|d| format!("  {} {} → {}", "+".green(), d.source, d.target))
            .collect();

        let base_packages = config.packages.normalized();
        let merged_packages = merged.packages.normalized();
        let pkg_lines: Vec<String> = [
            (&merged_packages.homebrew, &base_packages.homebrew, "brew"),
            (&merged_packages.zerobrew, &base_packages.zerobrew, "zb"),
        ]
        .into_iter()
        .flat_map(|(list, base_list, label)| {
            list.iter()
                .filter(|pkg| {
                    !base_list
                        .iter()
                        .any(|b| b.name == pkg.name && b.pkg_type == pkg.pkg_type)
                })
                .map(move |pkg| {
                    format!(
                        "  {} {} ({} {})",
                        "+".green(),
                        pkg.name,
                        label,
                        pkg.pkg_type
                    )
                })
        })
        .collect();

        let hook_lines: Vec<String> = merged.hooks[config.hooks.len()..]
            .iter()
            .map(|hook| format!("  {} {}", "+".green(), hook.name()))
            .collect();

        let sections = [
            (
                "Variables:",
                Self::map_override_lines(&merged.variables, &config.variables),
            ),
            ("Dotfiles:", dotfile_lines),
            ("Packages:", pkg_lines),
            ("Hooks:", hook_lines),
            (
                "Mise tools:",
                Self::map_override_lines(&merged.mise.tools, &config.mise.tools),
            ),
        ];

        if sections.iter().all(|(_, lines)| lines.is_empty()) {
            println!(
                "{}",
                "(no overrides — host uses defaults as-is)".bright_black()
            );
            return Ok(());
        }

        for (label, lines) in sections {
            if lines.is_empty() {
                continue;
            }
            println!("{}", label.bold());
            for line in lines {
                println!("{}", line);
            }
            println!();
        }

        Ok(())
    }

    /// Get the roles for the resolved host, including inherited roles.
    fn get_host_roles(config: &Config, host_name: &Option<String>) -> Vec<String> {
        if let Some(name) = host_name {
            config.resolved_host_roles(name).unwrap_or_default()
        } else {
            vec![]
        }
    }

    /// Build a HostContext from the resolved host, with safe fallback.
    fn build_host_context(config: &Config, host_name: &Option<String>) -> HostContext {
        if let Some(name) = host_name
            && config.hosts.contains_key(name)
        {
            return HostContext {
                name: name.clone(),
                roles: config.resolved_host_roles(name).unwrap_or_default(),
            };
        }
        HostContext {
            name: host_name.as_deref().unwrap_or("default").to_string(),
            roles: vec![],
        }
    }

    /// Filter a config by roles, returning only dotfiles and packages that apply.
    fn filter_config_by_roles(config: Config, host_roles: &[String]) -> Config {
        let filtered_dotfiles: Vec<_> = config
            .dotfiles
            .iter()
            .filter(|df| should_apply_for_roles(&df.only_roles, &df.skip_roles, host_roles))
            .cloned()
            .collect();

        // Normalize first so brew/cask/zb shorthand lists are included
        let normalized = config.packages.normalized();
        let filtered_packages: Vec<_> = normalized
            .homebrew
            .iter()
            .filter(|pkg| should_apply_for_roles(&pkg.only_roles, &pkg.skip_roles, host_roles))
            .cloned()
            .collect();
        let filtered_zb: Vec<_> = normalized
            .zerobrew
            .iter()
            .filter(|pkg| should_apply_for_roles(&pkg.only_roles, &pkg.skip_roles, host_roles))
            .cloned()
            .collect();

        Config {
            extends: Vec::new(),
            variables: config.variables,
            dotfiles: filtered_dotfiles,
            packages: crate::config::Packages {
                homebrew: filtered_packages,
                brew: Vec::new(),
                cask: Vec::new(),
                zerobrew: filtered_zb,
                zb: Vec::new(),
            },
            hosts: config.hosts,
            hooks: config.hooks,
            secrets: config.secrets,
            mise: config.mise,
        }
    }

    fn run_diff(&self) -> anyhow::Result<()> {
        let (config, host_name) = self.resolve_config_and_host()?;
        let host_ctx = Self::build_host_context(&config, &host_name);
        let host_roles = Self::get_host_roles(&config, &host_name);
        let filtered_config = Self::filter_config_by_roles(config, &host_roles);

        let state = State::load(self.get_state_path()).unwrap_or_else(|_| State::new());

        let diff_engine = DiffEngine::new();
        let mut changes = diff_engine.diff_dotfiles(&filtered_config, Some(&host_ctx))?;
        changes.extend(diff_engine.diff_packages(&filtered_config)?);
        changes.extend(Self::orphan_changes(&Self::find_orphans(
            &state,
            &filtered_config,
        )));

        if changes.is_empty() {
            println!("{}", "No changes detected.".bright_black());
            return Ok(());
        }

        println!("{}", "Changes:".bold());
        for change in &changes {
            println!("{}", change.format());
        }

        let add_count = changes
            .iter()
            .filter(|c| matches!(c, Change::Add { .. }))
            .count();
        let modify_count = changes
            .iter()
            .filter(|c| matches!(c, Change::Modify { .. }))
            .count();
        let remove_count = changes
            .iter()
            .filter(|c| matches!(c, Change::Remove { .. }))
            .count();

        if add_count > 0 || modify_count > 0 || remove_count > 0 {
            println!();
            println!(
                "{} {} to add, {} to modify, {} to remove",
                "Summary:".bold(),
                add_count.to_string().green(),
                modify_count.to_string().yellow(),
                remove_count.to_string().red()
            );
        }

        Ok(())
    }

    /// Save state, printing a warning instead of failing. Used on early exits
    /// from apply so already-completed work is never lost from tracking.
    fn save_state_best_effort(state: &State, state_path: &Path) {
        if let Err(e) = state.save(state_path) {
            eprintln!("{} Failed to save state: {}", "Warning:".yellow(), e);
        }
    }

    /// Compute state entries whose target is no longer produced by the
    /// (role-filtered) config — these were linked by a previous apply and
    /// should be cleaned up.
    fn find_orphans(state: &State, filtered_config: &Config) -> Vec<crate::state::DotfileState> {
        let desired: std::collections::HashSet<String> = filtered_config
            .dotfiles
            .iter()
            .filter_map(|d| crate::expand::expand_path_str(&d.target).ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        state
            .dotfiles
            .iter()
            .filter(|d| !desired.contains(&d.target))
            .cloned()
            .collect()
    }

    /// Preview entries for orphaned state dotfiles, shown by diff and apply.
    fn orphan_changes(orphans: &[crate::state::DotfileState]) -> Vec<Change> {
        orphans
            .iter()
            .map(|orphan| Change::Remove {
                resource_type: crate::diff::ResourceType::Dotfile,
                description: orphan.target.clone(),
                reason: "removed from config".to_string(),
            })
            .collect()
    }

    /// Restore a backup into a now-free target slot. Directory backups were
    /// created with rename, so they move back; file backups copy, then the
    /// backup file is removed best-effort (the restore already succeeded).
    fn restore_backup(backup_path: &Path, target: &Path) -> std::io::Result<()> {
        if backup_path.is_dir() {
            std::fs::rename(backup_path, target)
        } else {
            std::fs::copy(backup_path, target)?;
            let _ = std::fs::remove_file(backup_path);
            Ok(())
        }
    }

    /// Remove symlinks recorded in state whose dotfile entry no longer exists
    /// in the config: delete the mimic-created symlink, restore any backup,
    /// clean up the rendered file, and untrack the entry.
    fn cleanup_orphans(&self, orphans: &[crate::state::DotfileState], state: &mut State) {
        for orphan in orphans {
            let target = PathBuf::from(&orphan.target);
            let present = target.exists() || target.is_symlink();

            if present && !Self::is_mimic_symlink(&target, orphan) {
                println!(
                    "  {} Untracking {}: not a mimic-managed symlink (left in place)",
                    "⚠".yellow(),
                    target.display()
                );
                state.remove_dotfile(&orphan.target);
                continue;
            }

            if present {
                if let Err(e) = std::fs::remove_file(&target) {
                    eprintln!(
                        "  {} Failed to remove orphaned symlink {}: {}",
                        "✗".red(),
                        target.display(),
                        e
                    );
                    continue; // keep tracking it so a later apply can retry
                }
                println!(
                    "  {} Removed orphaned symlink: {}",
                    "✓".green(),
                    target.display()
                );

                if let Some(backup_path) = orphan.backup_path.as_deref().map(Path::new)
                    && backup_path.exists()
                {
                    match Self::restore_backup(backup_path, &target) {
                        Ok(()) => {
                            println!("  {} Restored backup: {}", "✓".green(), target.display());
                        }
                        Err(e) => {
                            eprintln!(
                                "  {} Failed to restore backup for {}: {}",
                                "✗".red(),
                                target.display(),
                                e
                            );
                        }
                    }
                }
            }

            // Remove the rendered file unless another state entry still uses it
            if let Some(rendered) = &orphan.rendered_path {
                let still_used = state.dotfiles.iter().any(|d| {
                    d.target != orphan.target
                        && d.rendered_path.as_deref() == Some(rendered.as_str())
                });
                if !still_used {
                    let rendered_path = PathBuf::from(rendered);
                    if rendered_path.exists()
                        && let Err(e) = std::fs::remove_file(&rendered_path)
                        && self.verbose
                    {
                        eprintln!(
                            "  {} Failed to remove rendered file {}: {}",
                            "⚠".yellow(),
                            rendered_path.display(),
                            e
                        );
                    }
                }
            }

            state.remove_dotfile(&orphan.target);
        }
    }

    fn run_apply(&self, only: &[ApplySection], skip: &[ApplySection]) -> anyhow::Result<()> {
        let enabled = |section: ApplySection| {
            (only.is_empty() || only.contains(&section)) && !skip.contains(&section)
        };

        let (config, host_name) = self.resolve_config_and_host()?;
        let host_ctx = Self::build_host_context(&config, &host_name);

        // Filter config by roles for diff preview (must match what we actually apply)
        let host_roles = Self::get_host_roles(&config, &host_name);
        let filtered_for_diff = Self::filter_config_by_roles(config.clone(), &host_roles);

        let state_path = self.get_state_path();
        let mut state = State::load(&state_path).unwrap_or_else(|_| State::new());

        let orphans = if enabled(ApplySection::Dotfiles) {
            Self::find_orphans(&state, &filtered_for_diff)
        } else {
            Vec::new()
        };

        let diff_engine = DiffEngine::new();
        let mut changes = Vec::new();
        if enabled(ApplySection::Dotfiles) {
            changes.extend(diff_engine.diff_dotfiles(&filtered_for_diff, Some(&host_ctx))?);
        }
        if enabled(ApplySection::Packages) {
            changes.extend(diff_engine.diff_packages(&filtered_for_diff)?);
        }
        changes.extend(Self::orphan_changes(&orphans));

        // With an explicit --only hooks/mise, an empty diff is expected — the
        // point is to run just those sections.
        let sections_without_diff = !only.is_empty()
            && ((enabled(ApplySection::Hooks) && !config.hooks.is_empty())
                || (enabled(ApplySection::Mise) && !config.mise.tools.is_empty()));

        if changes.is_empty() && !sections_without_diff {
            println!("{}", "No changes to apply.".bright_black());
            return Ok(());
        }

        if !changes.is_empty() {
            println!("{}", "Changes to apply:".bold());
            for change in &changes {
                println!("{}", change.format());
            }
        }

        if self.dry_run {
            println!();
            println!("{}", "Dry-run mode: No changes were made.".yellow());
            return Ok(());
        }

        if !self.yes && !changes.is_empty() {
            println!();
            use dialoguer::Confirm;
            let proceed = Confirm::new()
                .with_prompt("Apply these changes?")
                .default(false)
                .interact()?;

            if !proceed {
                println!("{}", "Aborted.".yellow());
                return Ok(());
            }
        }

        state.active_host = host_name.clone();

        println!();
        println!("{}", "Applying changes...".bold());

        let mut apply_to_all: Option<ApplyToAllChoice> = if self.yes {
            Some(ApplyToAllChoice::Backup)
        } else {
            None
        };

        if !orphans.is_empty() {
            self.cleanup_orphans(&orphans, &mut state);
        }

        let dotfiles_to_link = if enabled(ApplySection::Dotfiles) {
            config.dotfiles.as_slice()
        } else {
            &[]
        };
        for dotfile in dotfiles_to_link {
            if !should_apply_for_roles(&dotfile.only_roles, &dotfile.skip_roles, &host_ctx.roles) {
                if self.verbose {
                    println!(
                        "  {} {} (role mismatch)",
                        "↷".bright_black(),
                        dotfile.target
                    );
                }
                continue;
            }

            if self.verbose {
                println!(
                    "  {} {} → {}",
                    "Linking:".bright_black(),
                    dotfile.source,
                    dotfile.target
                );
            }

            match apply_dotfile(dotfile, &config, &host_ctx, &mut state, &mut apply_to_all) {
                Ok(()) => {
                    println!("  {} {}", "✓".green(), dotfile.target);
                }
                Err(e) => {
                    eprintln!("  {} {} - {}", "✗".red(), dotfile.target, e);
                    if !self.yes {
                        use dialoguer::Confirm;
                        let continue_on_error = Confirm::new()
                            .with_prompt("Continue with remaining dotfiles?")
                            .default(true)
                            .interact()?;

                        if !continue_on_error {
                            // Persist what was already applied so undo/status
                            // still know about it.
                            Self::save_state_best_effort(&state, &state_path);
                            return Err(e);
                        }
                    }
                }
            }
        }

        let homebrew = HomebrewManager::new();
        let normalized_packages = config.packages.normalized();

        // Partition packages into formulae (batch install) and casks (install one at a time)
        let mut formulae: Vec<&str> = Vec::new();
        let mut casks: Vec<&config::Package> = Vec::new();

        let packages_to_install = if enabled(ApplySection::Packages) {
            normalized_packages.homebrew.as_slice()
        } else {
            &[]
        };
        for package in packages_to_install {
            if !should_apply_for_roles(&package.only_roles, &package.skip_roles, &host_ctx.roles) {
                if self.verbose {
                    println!("  {} {} (role mismatch)", "↷".bright_black(), package.name);
                }
                continue;
            }

            if package.pkg_type == "cask" {
                casks.push(package);
            } else {
                formulae.push(&package.name);
            }
        }

        // Batch install all formulae with a single `brew install x y z` call
        if !formulae.is_empty() {
            if self.verbose {
                println!(
                    "  {} {} formulae: {}",
                    "Installing:".bright_black(),
                    formulae.len(),
                    formulae.join(", ")
                );
            }

            match homebrew.install_many_formulae(&formulae, &mut state) {
                Ok(installed) => {
                    for name in &installed {
                        println!("  {} brew formula: {}", "✓".green(), name);
                    }
                    // Also print already-installed formulae that were skipped
                    for name in &formulae {
                        if !installed.iter().any(|i| i == name) {
                            println!("  {} brew formula: {}", "✓".green(), name);
                        }
                    }
                }
                Err(errors) => {
                    for (cmd, e) in &errors {
                        eprintln!("  {} {} - {}", "✗".red(), cmd, e);
                    }
                    if !self.yes {
                        use dialoguer::Confirm;
                        let continue_on_error = Confirm::new()
                            .with_prompt("Continue with remaining packages?")
                            .default(true)
                            .interact()?;

                        if !continue_on_error {
                            // Persist what was already applied so undo/status
                            // still know about it.
                            Self::save_state_best_effort(&state, &state_path);
                            return Err(errors.into_iter().next().unwrap().1);
                        }
                    }
                }
            }
        }

        // Install casks one at a time (they may need interactive prompts)
        for package in &casks {
            if self.verbose {
                println!("  {} {} (cask)", "Installing:".bright_black(), package.name);
            }

            match homebrew.install_cask(&package.name, &mut state) {
                Ok(()) => {
                    println!("  {} brew cask: {}", "✓".green(), package.name);
                }
                Err(e) => {
                    eprintln!("  {} {} - {}", "✗".red(), package.name, e);
                    if !self.yes {
                        use dialoguer::Confirm;
                        let continue_on_error = Confirm::new()
                            .with_prompt("Continue with remaining packages?")
                            .default(true)
                            .interact()?;

                        if !continue_on_error {
                            // Persist what was already applied so undo/status
                            // still know about it.
                            Self::save_state_best_effort(&state, &state_path);
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Install zerobrew packages
        let zb_packages: Vec<&str> = if enabled(ApplySection::Packages) {
            normalized_packages
                .zerobrew
                .iter()
                .filter(|p| should_apply_for_roles(&p.only_roles, &p.skip_roles, &host_ctx.roles))
                .map(|p| p.name.as_str())
                .collect()
        } else {
            Vec::new()
        };

        if !zb_packages.is_empty() {
            if self.verbose {
                println!(
                    "  {} {} zerobrew packages: {}",
                    "Installing:".bright_black(),
                    zb_packages.len(),
                    zb_packages.join(", ")
                );
            }

            let zerobrew = ZerobrewManager::new();
            match zerobrew.install_many(&zb_packages, &mut state) {
                Ok(installed) => {
                    for name in &installed {
                        println!("  {} zb: {}", "✓".green(), name);
                    }
                    for name in &zb_packages {
                        if !installed.iter().any(|i| i == name) {
                            println!("  {} zb: {}", "✓".green(), name);
                        }
                    }
                }
                Err(errors) => {
                    for (cmd, e) in &errors {
                        eprintln!("  {} {} - {}", "✗".red(), cmd, e);
                    }
                    if !self.yes {
                        use dialoguer::Confirm;
                        let continue_on_error = Confirm::new()
                            .with_prompt("Continue with remaining packages?")
                            .default(true)
                            .interact()?;

                        if !continue_on_error {
                            // Persist what was already applied so undo/status
                            // still know about it.
                            Self::save_state_best_effort(&state, &state_path);
                            return Err(errors.into_iter().next().unwrap().1);
                        }
                    }
                }
            }
        }

        // Write the declared [mise] tools to mise's config before hooks run,
        // so a `mise` hook installs the tool set from this apply, not a stale one.
        if enabled(ApplySection::Mise) && !config.mise.tools.is_empty() {
            println!();
            if let Err(e) = crate::mise::generate_mise_config(&config) {
                eprintln!("  {} Failed to write mise config: {}", "✗".red(), e);
                Self::save_state_best_effort(&state, &state_path);
                return Err(e);
            }
        }

        if enabled(ApplySection::Hooks) && !config.hooks.is_empty() {
            println!();
            println!("{}", "Running activation hooks...".bright_cyan().bold());

            match hooks::execute_hooks(&config.hooks, &host_ctx.roles, self.verbose) {
                Ok(()) => {
                    println!();
                    println!("{}", "✓ All hooks completed successfully".green());
                    state.hooks = config.hooks.clone();
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("{} Hook execution failed: {}", "✗".red(), e);
                    if !self.yes {
                        use dialoguer::Confirm;
                        let continue_on_error = Confirm::new()
                            .with_prompt("Continue with saving state?")
                            .default(true)
                            .interact()?;

                        if !continue_on_error {
                            // Persist what was already applied so undo/status
                            // still know about it.
                            Self::save_state_best_effort(&state, &state_path);
                            return Err(e);
                        }
                    }
                }
            }
        }

        state
            .save(&state_path)
            .map_err(|e| anyhow::anyhow!("Failed to save state: {}", e))?;

        println!();
        println!("{}", "✓ Successfully applied configuration".green().bold());
        println!(
            "  {}: {}",
            "State saved to".bright_black(),
            state_path.display()
        );

        Ok(())
    }

    fn run_status(&self) -> anyhow::Result<()> {
        let state_path = self.get_state_path();

        if !state_path.exists() {
            println!("{}", "No state file found.".yellow());
            println!("  Run 'mimic apply' to initialize.");
            return Ok(());
        }

        if self.verbose {
            println!(
                "{} {}",
                "Loading state:".bright_black(),
                state_path.display()
            );
        }

        let state = State::load(&state_path)?;

        if state.dotfiles.is_empty() && state.packages.is_empty() {
            println!("{}", "No resources managed.".bright_black());
            return Ok(());
        }

        println!("{}", "Status Report".bold());
        println!();

        let mut dotfiles_ok = 0;
        let mut dotfiles_drift = 0;
        let mut drift_details = Vec::new();

        for dotfile in &state.dotfiles {
            let target_path = PathBuf::from(&dotfile.target);
            let source_path = PathBuf::from(&dotfile.source);

            // For template dotfiles, the symlink points to the rendered file,
            // not the original source template.
            let expected_path = if let Some(ref rendered) = dotfile.rendered_path {
                PathBuf::from(rendered)
            } else {
                source_path.clone()
            };

            if !target_path.exists() {
                drift_details.push(format!(
                    "  {} {} (missing)",
                    "✗".red(),
                    target_path.display()
                ));
                dotfiles_drift += 1;
            } else if !target_path.is_symlink() {
                drift_details.push(format!(
                    "  {} {} (not a symlink)",
                    "✗".red(),
                    target_path.display()
                ));
                dotfiles_drift += 1;
            } else {
                match std::fs::read_link(&target_path) {
                    Ok(actual_target) => {
                        let canonical_actual = match actual_target.canonicalize() {
                            Ok(p) => p,
                            Err(_) => {
                                drift_details.push(format!(
                                    "  {} {} (broken link)",
                                    "✗".red(),
                                    target_path.display()
                                ));
                                dotfiles_drift += 1;
                                continue;
                            }
                        };

                        let canonical_expected = match expected_path.canonicalize() {
                            Ok(p) => p,
                            Err(_) => {
                                drift_details.push(format!(
                                    "  {} {} (source missing: {})",
                                    "✗".red(),
                                    target_path.display(),
                                    expected_path.display()
                                ));
                                dotfiles_drift += 1;
                                continue;
                            }
                        };

                        if canonical_actual != canonical_expected {
                            drift_details.push(format!(
                                "  {} {} (points to {} instead of {})",
                                "✗".yellow(),
                                target_path.display(),
                                actual_target.display(),
                                expected_path.display()
                            ));
                            dotfiles_drift += 1;
                        } else {
                            dotfiles_ok += 1;
                            if self.verbose {
                                println!("  {} {}", "✓".green(), target_path.display());
                            }
                        }
                    }
                    Err(e) => {
                        drift_details.push(format!(
                            "  {} {} (error reading link: {})",
                            "✗".red(),
                            target_path.display(),
                            e
                        ));
                        dotfiles_drift += 1;
                    }
                }
            }
        }

        let homebrew = HomebrewManager::new();
        let zerobrew = ZerobrewManager::new();
        let mut packages_ok = 0;
        let mut packages_drift = 0;

        // Fetch each manager's installed list once instead of shelling out per
        // package. Brew entries can be formulae or casks, so both lists are
        // included. A missing manager binary means nothing is installed.
        let brew_installed: Option<anyhow::Result<Vec<String>>> = state
            .packages
            .iter()
            .any(|p| p.manager == "brew")
            .then(|| homebrew.list_installed_any())
            .map(|r| r.map(|opt| opt.unwrap_or_default()));
        let zb_installed: Option<anyhow::Result<Vec<String>>> = state
            .packages
            .iter()
            .any(|p| p.manager == "zb")
            .then(|| zerobrew.try_list_installed())
            .map(|r| r.map(|opt| opt.unwrap_or_default()));

        for package in &state.packages {
            let lookup = match package.manager.as_str() {
                "brew" => brew_installed.as_ref(),
                "zb" => zb_installed.as_ref(),
                _ => None,
            };
            let Some(result) = lookup else { continue };

            match result {
                Ok(installed) if installed.iter().any(|p| p == &package.name) => {
                    packages_ok += 1;
                    if self.verbose {
                        println!("  {} {}: {}", "✓".green(), package.manager, package.name);
                    }
                }
                Ok(_) => {
                    drift_details.push(format!(
                        "  {} {} package not installed: {}",
                        "✗".yellow(),
                        package.manager,
                        package.name
                    ));
                    packages_drift += 1;
                }
                Err(e) => {
                    drift_details.push(format!(
                        "  {} error checking {} ({}): {}",
                        "✗".red(),
                        package.name,
                        package.manager,
                        e
                    ));
                    packages_drift += 1;
                }
            }
        }

        let total_dotfiles = dotfiles_ok + dotfiles_drift;
        let total_packages = packages_ok + packages_drift;

        if !self.verbose {
            if dotfiles_ok == total_dotfiles && total_dotfiles > 0 {
                println!(
                    "  {} {}/{} dotfiles in sync",
                    "✓".green(),
                    dotfiles_ok,
                    total_dotfiles
                );
            } else if total_dotfiles > 0 {
                println!(
                    "  {} {}/{} dotfiles in sync",
                    "✗".yellow(),
                    dotfiles_ok,
                    total_dotfiles
                );
            }

            if packages_ok == total_packages && total_packages > 0 {
                println!(
                    "  {} {}/{} packages installed",
                    "✓".green(),
                    packages_ok,
                    total_packages
                );
            } else if total_packages > 0 {
                println!(
                    "  {} {}/{} packages installed",
                    "✗".yellow(),
                    packages_ok,
                    total_packages
                );
            }
        }

        if !drift_details.is_empty() {
            println!();
            println!("{}", "Drift detected:".yellow().bold());
            for detail in drift_details {
                println!("{}", detail);
            }
        }

        println!();
        if dotfiles_drift > 0 || packages_drift > 0 {
            println!(
                "{}",
                "Run 'mimic apply' to reconcile drift.".yellow().bold()
            );
            return Err(anyhow::anyhow!("__drift_detected__"));
        } else {
            println!("{}", "✓ All resources in sync".green().bold());
        }

        Ok(())
    }

    /// Read-only health check: config parses, hostname resolves, symlinks are
    /// intact, rendered files aren't orphaned, templates haven't drifted, and
    /// packages match the config. Never mutates anything; exits non-zero when
    /// problems are found.
    fn run_doctor(&self) -> anyhow::Result<()> {
        println!("{}", "mimic doctor".bold());
        println!();

        let mut problems: Vec<String> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();

        fn ok(msg: &str) {
            println!("  {} {}", "✓".green(), msg);
        }

        // Config parses (and extends resolve)
        let loaded = match self.find_config() {
            Ok(path) => match self.load_config(&path) {
                Ok(config) => {
                    ok(&format!("config loads: {}", path.display()));
                    Some(config)
                }
                Err(e) => {
                    problems.push(format!("config failed to load: {:#}", e));
                    None
                }
            },
            Err(e) => {
                problems.push(format!("{:#}", e));
                None
            }
        };

        // Hostname resolves to a host entry (same policy as apply/diff/render)
        let mut resolved: Option<(Config, Option<String>)> = None;
        if let Some(base) = loaded {
            match self.resolve_host(base) {
                Ok(HostResolution::NoHosts(config)) => {
                    ok("no [hosts] entries — single-machine config");
                    resolved = Some((config, None));
                }
                Ok(HostResolution::Matched { key, merged }) => {
                    let roles = merged.resolved_host_roles(&key).unwrap_or_default();
                    let roles_desc = if roles.is_empty() {
                        "no roles".to_string()
                    } else {
                        format!("roles: {}", roles.join(", "))
                    };
                    ok(&format!("hostname matches host '{}' ({})", key, roles_desc));
                    resolved = Some((merged, Some(key)));
                }
                Ok(HostResolution::Unmatched { requested, base }) => {
                    warnings.push(Self::unmatched_host_message(&requested));
                    resolved = Some((base, None));
                }
                Err(e) => problems.push(e.to_string()),
            }
        }

        let state_path = self.get_state_path();
        let state = State::load(&state_path).unwrap_or_else(|_| State::new());

        if let Some((config, host_name)) = &resolved {
            if let (Some(active), Some(current)) = (&state.active_host, host_name)
                && active != current
            {
                warnings.push(format!(
                    "state was last applied as host '{}', but current host resolves to '{}'",
                    active, current
                ));
            }

            let host_ctx = Self::build_host_context(config, host_name);
            let host_roles = Self::get_host_roles(config, host_name);
            let filtered = Self::filter_config_by_roles(config.clone(), &host_roles);

            // Dotfile link health + template drift (via the diff engine)
            let diff_engine = DiffEngine::new();
            match diff_engine.diff_dotfiles(&filtered, Some(&host_ctx)) {
                Ok(changes) => {
                    // The diff engine already classifies dotfile trouble:
                    // broken symlinks, hijacked targets, and template drift
                    // all arrive as Modify with a reason. Add just means the
                    // link hasn't been created yet.
                    let mut healthy = 0;
                    for change in changes {
                        match change {
                            Change::AlreadyCorrect { .. } => healthy += 1,
                            Change::Add { description, .. } => {
                                warnings.push(format!(
                                    "not applied yet: {} (run 'mimic apply --only dotfiles')",
                                    description
                                ));
                            }
                            Change::Modify {
                                description,
                                reason,
                                ..
                            } => {
                                problems.push(format!("{} — {}", description, reason));
                            }
                            Change::Remove { .. } => {}
                        }
                    }
                    if healthy > 0 {
                        ok(&format!("{} dotfile(s) linked correctly", healthy));
                    }
                }
                Err(e) => problems.push(format!("dotfile check failed: {}", e)),
            }

            // State entries whose target left the config
            for orphan in Self::find_orphans(&state, &filtered) {
                warnings.push(format!(
                    "orphaned state entry: {} (removed from config; next 'mimic apply' cleans it up)",
                    orphan.target
                ));
            }

            // Rendered files no longer referenced by state or config templates
            if let Ok(rendered_dir) = crate::linker::rendered_dir()
                && rendered_dir.is_dir()
            {
                let mut expected: std::collections::HashSet<PathBuf> = state
                    .dotfiles
                    .iter()
                    .filter_map(|d| d.rendered_path.as_ref())
                    .map(PathBuf::from)
                    .collect();
                for dotfile in filtered.dotfiles.iter().filter(|d| d.is_template()) {
                    if let Ok(source) = crate::expand::expand_path_str(&dotfile.source)
                        && let Ok(rendered) = crate::linker::rendered_path_for(&source)
                    {
                        expected.insert(rendered);
                    }
                }

                if let Ok(entries) = std::fs::read_dir(&rendered_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        if !expected.contains(&entry.path()) {
                            warnings.push(format!(
                                "orphaned rendered file: {} (safe to delete)",
                                entry.path().display()
                            ));
                        }
                    }
                }
            }

            // Declared packages that aren't installed
            match diff_engine.diff_packages(&filtered) {
                Ok(changes) => {
                    let missing: Vec<String> = changes
                        .iter()
                        .filter_map(|c| match c {
                            Change::Add { description, .. } => Some(description.clone()),
                            _ => None,
                        })
                        .collect();
                    let total = changes.len();
                    if missing.is_empty() {
                        if total > 0 {
                            ok(&format!("{} package(s) installed", total));
                        }
                    } else {
                        for name in missing {
                            warnings.push(format!(
                                "package declared but not installed: {} (run 'mimic apply --only packages')",
                                name
                            ));
                        }
                    }
                }
                Err(e) => warnings.push(format!("package check failed: {}", e)),
            }

            // Installed-but-undeclared packages (informational)
            match Self::unmanaged_packages(config) {
                Ok((formulas, casks, zb)) => {
                    let total = formulas.len() + casks.len() + zb.len();
                    if total == 0 {
                        ok("no unmanaged packages");
                    } else {
                        println!(
                            "  {} {} unmanaged package(s) not in config (run 'mimic clean --dry-run' to list)",
                            "○".bright_black(),
                            total
                        );
                    }
                }
                Err(e) => {
                    if self.verbose {
                        println!(
                            "  {} skipped unmanaged package check: {}",
                            "○".bright_black(),
                            e
                        );
                    }
                }
            }
        }

        println!();
        if !warnings.is_empty() {
            println!("{}", "Warnings:".yellow().bold());
            for warning in &warnings {
                println!("  {} {}", "⚠".yellow(), warning);
            }
            println!();
        }
        if !problems.is_empty() {
            println!("{}", "Problems:".red().bold());
            for problem in &problems {
                println!("  {} {}", "✗".red(), problem);
            }
            println!();
            println!(
                "{}",
                format!(
                    "{} problem(s), {} warning(s) found.",
                    problems.len(),
                    warnings.len()
                )
                .red()
                .bold()
            );
            return Err(anyhow::anyhow!("__drift_detected__"));
        }

        if warnings.is_empty() {
            println!("{}", "✓ No problems found".green().bold());
        } else {
            println!(
                "{}",
                format!("No problems, {} warning(s).", warnings.len())
                    .yellow()
                    .bold()
            );
        }
        Ok(())
    }

    /// Check whether `target` is still the symlink mimic created for this
    /// state entry, i.e. it points at the recorded source or rendered file.
    fn is_mimic_symlink(target: &Path, dotfile: &crate::state::DotfileState) -> bool {
        if !target.is_symlink() {
            return false;
        }
        let Ok(dest) = std::fs::read_link(target) else {
            return false;
        };
        let matches = |recorded: &str| {
            if dest == Path::new(recorded) {
                return true;
            }
            match (
                std::fs::canonicalize(&dest),
                std::fs::canonicalize(recorded),
            ) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            }
        };
        matches(&dotfile.source) || dotfile.rendered_path.as_deref().is_some_and(matches)
    }

    fn run_undo(&self) -> anyhow::Result<()> {
        let state_path = self.get_state_path();

        let state = match State::load(&state_path) {
            Ok(state) if state.dotfiles.is_empty() && state.packages.is_empty() => {
                println!("{}", "Nothing to undo.".yellow());
                return Ok(());
            }
            Ok(state) => state,
            Err(_) => {
                println!("{}", "Nothing to undo.".yellow());
                return Ok(());
            }
        };

        if self.verbose {
            println!(
                "{} {}",
                "Loading state from:".bright_black(),
                state_path.display()
            );
        }

        println!("{}", "Undoing last apply operation...".bold());
        println!();

        let mut symlinks_removed = 0;
        let mut backups_restored = 0;
        let mut errors = Vec::new();

        for dotfile in &state.dotfiles {
            let target = PathBuf::from(&dotfile.target);

            if self.verbose {
                println!("  {} {}", "Processing:".bright_black(), target.display());
            }

            // Only remove the target if it is still the symlink mimic created
            // (pointing at the recorded source or rendered file). If the user
            // replaced it with a real file/directory or a different link,
            // leave it alone instead of destroying their data.
            let mut target_slot_free = true;
            if target.exists() || target.is_symlink() {
                if Self::is_mimic_symlink(&target, dotfile) {
                    match std::fs::remove_file(&target) {
                        Ok(()) => {
                            symlinks_removed += 1;
                            println!("  {} Removed symlink: {}", "✓".green(), target.display());
                        }
                        Err(e) => {
                            target_slot_free = false;
                            let error_msg =
                                format!("Failed to remove symlink {}: {}", target.display(), e);
                            eprintln!("  {} {}", "✗".red(), error_msg);
                            errors.push(error_msg);
                        }
                    }
                } else {
                    target_slot_free = false;
                    println!(
                        "  {} Skipping {}: no longer a mimic-managed symlink (modified outside mimic)",
                        "⚠".yellow(),
                        target.display()
                    );
                }
            } else if self.verbose {
                println!(
                    "  {} Symlink already removed: {}",
                    "○".bright_black(),
                    target.display()
                );
            }

            if !target_slot_free {
                // Don't restore a backup over something we refused to remove.
                if dotfile.backup_path.is_some() {
                    println!(
                        "  {} Backup for {} left in place: {}",
                        "○".bright_black(),
                        target.display(),
                        dotfile.backup_path.as_deref().unwrap_or_default()
                    );
                }
                continue;
            }

            if let Some(backup_path_str) = &dotfile.backup_path {
                let backup_path = PathBuf::from(backup_path_str);

                if backup_path.exists() {
                    match Self::restore_backup(&backup_path, &target) {
                        Ok(()) => {
                            backups_restored += 1;
                            println!(
                                "  {} Restored backup: {} → {}",
                                "✓".green(),
                                backup_path.display(),
                                target.display()
                            );
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Failed to restore backup from {} to {}: {}",
                                backup_path.display(),
                                target.display(),
                                e
                            );
                            eprintln!("  {} {}", "✗".red(), error_msg);
                            errors.push(error_msg);
                        }
                    }
                } else if self.verbose {
                    println!(
                        "  {} Backup not found: {}",
                        "○".bright_black(),
                        backup_path.display()
                    );
                }
            }

            if let Some(rendered_path_str) = &dotfile.rendered_path {
                let rendered_path = PathBuf::from(rendered_path_str);
                if rendered_path.exists() {
                    match std::fs::remove_file(&rendered_path) {
                        Ok(()) => {
                            if self.verbose {
                                println!(
                                    "  {} Cleaned up rendered file: {}",
                                    "✓".green(),
                                    rendered_path.display()
                                );
                            }
                        }
                        Err(e) => {
                            if self.verbose {
                                eprintln!(
                                    "  {} Failed to clean up rendered file {}: {}",
                                    "⚠".yellow(),
                                    rendered_path.display(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        let package_count = state.packages.len();

        let mut new_state = State::new();
        new_state.clear();

        if let Err(e) = new_state.save(&state_path) {
            eprintln!("{} Failed to clear state file: {}", "Warning:".yellow(), e);
        }

        println!();
        if errors.is_empty() {
            println!("{}", "✓ Successfully undone last apply".green().bold());
            println!("  {} symlinks removed", symlinks_removed);
            println!("  {} backups restored", backups_restored);
        } else {
            println!("{}", "⚠ Undo completed with errors".yellow().bold());
            println!("  {} symlinks removed", symlinks_removed);
            println!("  {} backups restored", backups_restored);
            println!("  {} errors occurred", errors.len());
        }

        if package_count > 0 {
            println!(
                "  {} {} installed packages preserved (use 'mimic clean' to remove)",
                "ℹ".bright_black(),
                package_count
            );
        }

        if self.verbose {
            println!(
                "  {}: {}",
                "State cleared in".bright_black(),
                state_path.display()
            );
        }

        Ok(())
    }

    fn run_render(&self, template: &str) -> anyhow::Result<()> {
        use crate::template::render_file;

        // Honors --host, so any host's output can be previewed from any
        // machine (e.g. `mimic render tpl.hbs --host elara`).
        let (merged_config, host_name) = self.resolve_config_and_host()?;
        let host_ctx = Self::build_host_context(&merged_config, &host_name);

        let template_path = PathBuf::from(template);
        let rendered = render_file(&template_path, &merged_config.variables, &host_ctx)?;

        println!("{}", rendered);
        Ok(())
    }

    fn run_secrets(&self, cmd: &SecretsCommands) -> anyhow::Result<()> {
        use crate::secrets;

        match cmd {
            SecretsCommands::Set { key, stdin } => {
                let value = if *stdin {
                    use std::io::Read;
                    let mut buf = String::new();
                    std::io::stdin().read_to_string(&mut buf)?;
                    buf.trim().to_string()
                } else {
                    use dialoguer::Password;
                    Password::new()
                        .with_prompt(format!("Enter value for '{}'", key))
                        .interact()?
                };

                secrets::set_secret(key, &value)?;
                println!("{} Secret '{}' stored in keychain", "✓".green(), key);
                Ok(())
            }

            SecretsCommands::Get { key } => {
                let value = secrets::get_secret(key)?;
                println!("{}", value);
                Ok(())
            }

            SecretsCommands::List => {
                let secrets_list = secrets::list_secrets()?;
                if secrets_list.is_empty() {
                    println!("{}", "No secrets stored".bright_black());
                } else {
                    println!("{}", "Stored secrets:".bold());
                    for key in secrets_list {
                        println!("  • {}", key);
                    }
                }
                Ok(())
            }

            SecretsCommands::Rm { key } => {
                secrets::remove_secret(key)?;
                println!("{} Secret '{}' removed from keychain", "✓".green(), key);
                Ok(())
            }

            SecretsCommands::Export => {
                let config = self.load_config(&self.find_config()?)?;
                let all_secrets = secrets::get_all_secrets();

                if all_secrets.is_empty() {
                    if self.verbose {
                        eprintln!("{}", "No secrets available to export".yellow());
                    }
                    return Ok(());
                }

                for (key, value) in &all_secrets {
                    let env_var = if let Some(metadata) = config.secrets.get(key) {
                        metadata
                            .env_var
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| key.to_uppercase())
                    } else {
                        key.to_uppercase()
                    };

                    // Single-quote the value so shell metacharacters in the
                    // secret ($, `, ", \) survive `eval "$(mimic secrets export)"`.
                    println!("export {}='{}'", env_var, value.replace('\'', r"'\''"));
                }

                Ok(())
            }
        }
    }

    fn run_init(&self, repo: &str, apply_after: bool) -> anyhow::Result<()> {
        use crate::secrets_scan::scan_for_secrets;
        use dialoguer::Confirm;
        use std::fs;

        let repo_dir = self.repo_dir()?;

        if repo_dir.exists() {
            return Err(anyhow::anyhow!(
                "Repository directory already exists: {}\n\nTo fix:\n  - Remove the existing directory: rm -rf {}\n  - Or use a different location",
                repo_dir.display(),
                repo_dir.display()
            ));
        }

        let parent_dir = repo_dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid repository directory"))?;
        fs::create_dir_all(parent_dir).with_context(|| {
            format!(
                "Failed to create mimic config directory: {}",
                parent_dir.display()
            )
        })?;

        println!("{}", "Cloning repository...".bold());

        let clone_result = Self::try_git_clone(repo, &repo_dir, self.branch.as_deref());

        match clone_result {
            Ok(()) => {
                println!(
                    "  {} Repository cloned to {}",
                    "✓".green(),
                    repo_dir.display().to_string().green()
                );
            }
            Err(ref e) if format!("{e}").contains("__auth_retry__") => {
                // Auth failed — attempt gh-based authentication then retry
                git_auth::ensure_gh_auth()?;

                println!("{}", "Retrying clone...".bold());

                // Clean up partial clone if any
                if repo_dir.exists() {
                    fs::remove_dir_all(&repo_dir).ok();
                }

                Self::try_git_clone(repo, &repo_dir, self.branch.as_deref())
                    .map_err(|retry_err| {
                        // Strip the internal retry marker so it never reaches
                        // the user-facing message.
                        let msg = retry_err.to_string();
                        let msg = msg
                            .strip_prefix("__auth_retry__: ")
                            .unwrap_or(&msg)
                            .to_string();
                        anyhow::anyhow!(
                            "Git clone failed after authentication\n\n{}\n\nTo fix:\n  - Verify you have access to the repository\n  - Try cloning manually: {}",
                            msg,
                            Self::manual_clone_command(repo, self.branch.as_deref())
                        )
                    })?;

                println!(
                    "  {} Repository cloned to {}",
                    "✓".green(),
                    repo_dir.display().to_string().green()
                );
            }
            Err(e) => return Err(e),
        }

        println!();
        println!("{}", "Scanning for secrets...".bold());
        println!();

        let mut paths_to_scan = Vec::new();
        for entry in walkdir::WalkDir::new(&repo_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                paths_to_scan.push(entry.path().to_path_buf());
            }
        }

        let secret_count = scan_for_secrets(&paths_to_scan)?;

        println!();

        if secret_count > 0 {
            println!("{}", "⚠ Secrets detected in repository".yellow().bold());
            println!(
                "  {} secret(s) found - review output above",
                secret_count.to_string().yellow()
            );
            println!();
            println!(
                "{}",
                "Repository supports .secretsignore file to exclude false positives".bright_black()
            );
            println!();

            if !self.yes {
                let proceed = Confirm::new()
                    .with_prompt("Secrets detected. Continue?")
                    .default(false)
                    .interact()?;

                if !proceed {
                    println!("{}", "Cleaning up...".yellow());
                    fs::remove_dir_all(&repo_dir).ok();
                    println!("{}", "Aborted.".yellow());
                    return Ok(());
                }
            } else {
                println!(
                    "{}",
                    "Warning: --yes flag enabled, continuing despite secrets"
                        .yellow()
                        .bold()
                );
            }
        } else {
            println!("  {} No secrets detected", "✓".green());
        }

        if apply_after {
            println!();
            println!("{}", "→ Applying configuration...".cyan());

            let config_path = repo_dir.join("mimic.toml");
            if !config_path.exists() {
                return Err(anyhow::anyhow!(
                    "Configuration file not found: {}\n\nTo fix:\n  - Ensure the repository contains a mimic.toml file\n  - Check that you cloned the correct repository",
                    config_path.display()
                ));
            }

            let apply_cli = Cli {
                command: Commands::Apply {
                    only: Vec::new(),
                    skip: Vec::new(),
                },
                config: Some(config_path),
                host: self.host.clone(),
                yes: true,
                dry_run: self.dry_run,
                verbose: self.verbose,
                offline: self.offline,
                state: self.state.clone(),
                branch: self.branch.clone(),
            };

            apply_cli.run_apply(&[], &[])?;
        }

        println!();
        println!("{}", "✓ Initialization complete".green().bold());
        println!(
            "  {}: {}",
            "Repository cloned to".bright_black(),
            repo_dir.display()
        );

        Ok(())
    }

    fn manual_clone_command(repo: &str, branch: Option<&str>) -> String {
        if let Some(branch) = branch {
            format!(
                "git clone --depth 1 --branch {} --single-branch {}",
                branch, repo
            )
        } else {
            format!("git clone --depth 1 {}", repo)
        }
    }

    /// Attempt a git clone, returning a sentinel error for auth failures so the
    /// caller can offer interactive gh-based authentication.
    fn try_git_clone(
        repo: &str,
        repo_dir: &std::path::Path,
        branch: Option<&str>,
    ) -> anyhow::Result<()> {
        use std::process::Command;

        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth").arg("1");
        if let Some(branch) = branch {
            cmd.arg("--branch").arg(branch).arg("--single-branch");
        }
        cmd.arg(repo).arg(repo_dir);

        let output = cmd.output();

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);

                if git_auth::is_auth_error(&stderr) {
                    // Signal to caller that auth-based retry is appropriate
                    Err(anyhow::anyhow!("__auth_retry__: {}", stderr.trim()))
                } else if stderr.contains("Could not resolve host") {
                    Err(anyhow::anyhow!(
                        "Git clone failed: Network error\n\nTo fix:\n  - Check your internet connection\n  - Verify the repository host is correct"
                    ))
                } else {
                    Err(anyhow::anyhow!(
                        "Git clone failed\n\n{}\n\nTo fix:\n  - Try cloning manually: {}",
                        stderr.trim(),
                        Self::manual_clone_command(repo, branch)
                    ))
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(anyhow::anyhow!(
                "Git is not installed or not in PATH\n\nTo fix:\n  - Install git: brew install git (macOS)\n  - Or: apt install git (Linux)"
            )),
            Err(e) => Err(anyhow::anyhow!("Failed to execute git: {}", e)),
        }
    }

    fn run_edit(&self, target: &str) -> anyhow::Result<()> {
        use std::env;
        use std::process::Command;

        // Use the same expansion rules as apply (~ and env vars) so lookups
        // match the fully-expanded targets stored in state.
        let expanded_target = crate::expand::expand_path_str(target)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| target.to_string());

        let state_path = self.get_state_path();
        let mut source_path: Option<String> = None;

        if state_path.exists()
            && let Ok(state) = State::load(&state_path)
        {
            for dotfile in &state.dotfiles {
                if dotfile.target == expanded_target || dotfile.target == target {
                    source_path = Some(dotfile.source.clone());
                    if self.verbose {
                        println!("{} Found in state: {}", "→".bright_black(), dotfile.source);
                    }
                    break;
                }
            }
        }

        if source_path.is_none() {
            let config_path = self.find_config().with_context(|| {
                format!(
                    "Target '{}' not found in state and no config file available\n\nTo fix:\n  - Run 'mimic apply' first to track dotfiles in state\n  - Or ensure mimic.toml exists and contains the target",
                    target
                )
            })?;
            let config = self.load_config(&config_path)?;

            for dotfile in &config.dotfiles {
                let config_target = crate::expand::expand_path_str(&dotfile.target)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| dotfile.target.clone());

                if config_target == expanded_target || dotfile.target == target {
                    // Source paths are already resolved to absolute by Config::from_file
                    source_path = Some(dotfile.source.clone());
                    if self.verbose {
                        println!("{} Found in config: {}", "→".bright_black(), dotfile.source);
                    }
                    break;
                }
            }
        }

        let source = source_path.ok_or_else(|| {
            anyhow::anyhow!(
                "Target '{}' not found in state or config\n\nTo fix:\n  - Check that the target path is correct\n  - Verify it's defined in your mimic.toml\n  - Run 'mimic apply' to track it in state",
                target
            )
        })?;

        let editor = env::var("EDITOR")
            .ok()
            .or_else(|| {
                Command::new("which")
                    .arg("vim")
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|_| "vim".to_string())
            })
            .or_else(|| {
                Command::new("which")
                    .arg("nano")
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|_| "nano".to_string())
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No editor found\n\nTo fix:\n  - Set the EDITOR environment variable: export EDITOR=vim\n  - Or install vim: brew install vim\n  - Or install nano: brew install nano"
                )
            })?;

        if self.verbose {
            println!("{} Opening with editor: {}", "→".bright_black(), editor);
            println!("{} Source file: {}", "→".bright_black(), source);
        }

        let status = Command::new(&editor)
            .arg(&source)
            .status()
            .with_context(|| format!("Failed to execute editor: {}", editor))?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Editor exited with non-zero status\n\nTo fix:\n  - Check that the file exists: {}\n  - Verify the editor works: {} --version",
                source,
                editor
            ));
        }

        Ok(())
    }

    /// Installed-but-undeclared packages: brew leaves, casks, and (when
    /// configured) zerobrew packages that don't appear in the config.
    fn unmanaged_packages(
        config: &Config,
    ) -> anyhow::Result<(Vec<String>, Vec<String>, Vec<String>)> {
        // Config entries may use tap-qualified names (sometap/foo); brew
        // reports short names, so compare on the final path segment.
        fn short_name(name: &str) -> &str {
            name.rsplit('/').next().unwrap_or(name)
        }

        let normalized = config.packages.normalized();
        let config_formulas: std::collections::HashSet<&str> = normalized
            .homebrew
            .iter()
            .filter(|p| p.pkg_type == "formula")
            .map(|p| short_name(&p.name))
            .collect();
        let config_casks: std::collections::HashSet<&str> = normalized
            .homebrew
            .iter()
            .filter(|p| p.pkg_type == "cask")
            .map(|p| short_name(&p.name))
            .collect();
        let config_zb: std::collections::HashSet<&str> = normalized
            .zerobrew
            .iter()
            .map(|p| short_name(&p.name))
            .collect();

        let homebrew = HomebrewManager::new();
        // Only leaves count as removable: `brew list` includes auto-installed
        // dependencies, which must not be uninstalled out from under the
        // configured packages that need them.
        let installed_formulas = homebrew.list_leaves()?;
        let installed_casks = homebrew.list_installed_casks()?;

        let extra_formulas: Vec<String> = installed_formulas
            .into_iter()
            .filter(|p| !config_formulas.contains(p.as_str()))
            .collect();
        let extra_casks: Vec<String> = installed_casks
            .into_iter()
            .filter(|p| !config_casks.contains(p.as_str()))
            .collect();

        // For zerobrew: only attempt list if there are zb packages configured, to avoid
        // failing the whole command if zb is not installed when not in use.
        let zerobrew = ZerobrewManager::new();
        let extra_zb: Vec<String> = if !config_zb.is_empty() {
            zerobrew
                .list_installed()?
                .into_iter()
                .filter(|p| !config_zb.contains(short_name(p)))
                .collect()
        } else {
            Vec::new()
        };

        Ok((extra_formulas, extra_casks, extra_zb))
    }

    fn run_clean(&self) -> anyhow::Result<()> {
        let (config, _host_name) = self.resolve_config_and_host()?;

        let (extra_formulas, extra_casks, extra_zb) = Self::unmanaged_packages(&config)?;
        let homebrew = HomebrewManager::new();
        let zerobrew = ZerobrewManager::new();

        if extra_formulas.is_empty() && extra_casks.is_empty() && extra_zb.is_empty() {
            println!(
                "{}",
                "No extra packages to remove. System matches config.".green()
            );
            return Ok(());
        }

        println!("{}", "Packages not in config:".bold());
        for name in &extra_formulas {
            println!("  {} {} (formula)", "✗".yellow(), name);
        }
        for name in &extra_casks {
            println!("  {} {} (cask)", "✗".yellow(), name);
        }
        for name in &extra_zb {
            println!("  {} {} (zb)", "✗".yellow(), name);
        }
        println!();
        println!(
            "  {} formulas, {} casks, {} zb packages to remove",
            extra_formulas.len(),
            extra_casks.len(),
            extra_zb.len()
        );

        let total = extra_formulas.len() + extra_casks.len() + extra_zb.len();

        // Dry run is a preview: never prompt, never touch anything.
        if self.dry_run {
            println!();
            for name in &extra_formulas {
                println!(
                    "  {} Would uninstall {} (formula)",
                    "→".bright_black(),
                    name
                );
            }
            for name in &extra_casks {
                println!("  {} Would uninstall {} (cask)", "→".bright_black(), name);
            }
            for name in &extra_zb {
                println!("  {} Would uninstall {} (zb)", "→".bright_black(), name);
            }
            println!();
            println!(
                "{}",
                format!("Dry run: {} packages would be removed", total)
                    .green()
                    .bold()
            );
            return Ok(());
        }

        if !self.yes {
            use dialoguer::Confirm;
            let confirmed = Confirm::new()
                .with_prompt("Uninstall these packages?")
                .default(false)
                .interact()?;

            if !confirmed {
                println!("{}", "Cancelled.".bright_black());
                return Ok(());
            }
        }

        println!();
        let mut all_brew: Vec<&str> = extra_formulas.iter().map(|s| s.as_str()).collect();
        all_brew.extend(extra_casks.iter().map(|s| s.as_str()));

        let mut uninstall_errors: Vec<anyhow::Error> = Vec::new();

        if !all_brew.is_empty() {
            match homebrew.uninstall_many(&all_brew) {
                Ok(_) => {
                    println!(
                        "{}",
                        format!("✓ Removed {} brew packages", all_brew.len())
                            .green()
                            .bold()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "{}",
                        format!("✗ Failed to uninstall brew packages: {}", e)
                            .red()
                            .bold()
                    );
                    uninstall_errors.push(e);
                }
            }
        }

        if !extra_zb.is_empty() {
            let zb_refs: Vec<&str> = extra_zb.iter().map(|s| s.as_str()).collect();
            match zerobrew.uninstall_many(&zb_refs) {
                Ok(_) => {
                    println!(
                        "{}",
                        format!("✓ Removed {} zb packages", extra_zb.len())
                            .green()
                            .bold()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "{}",
                        format!("✗ Failed to uninstall zb packages: {}", e)
                            .red()
                            .bold()
                    );
                    uninstall_errors.push(e);
                }
            }
        }

        if !uninstall_errors.is_empty() {
            return Err(anyhow::anyhow!(
                "clean finished with {} error(s); see output above",
                uninstall_errors.len()
            ));
        }

        Ok(())
    }
}

pub fn run() -> Result<(), i32> {
    let cli = Cli::parse();

    match cli.run() {
        Ok(()) => Ok(()),
        Err(e) => {
            // Drift detection returns a silent exit code 1
            // (the user-facing message was already printed)
            if e.to_string() == "__drift_detected__" {
                return Err(1);
            }

            eprintln!("{} {}", "Error:".red().bold(), e);

            if cli.verbose
                && let Some(source) = e.source()
            {
                eprintln!();
                eprintln!("{}", "Caused by:".bright_black());
                eprintln!("  {}", source);
            }

            Err(1)
        }
    }
}
