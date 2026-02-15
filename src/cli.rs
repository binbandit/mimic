use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use crate::config::{should_apply_for_roles, Config};
use crate::diff::{Change, DiffEngine};
use crate::hooks;
use crate::installer::HomebrewManager;
use crate::linker::{apply_dotfile, ApplyToAllChoice};
use crate::state::State;
use crate::template::HostContext;
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

    #[arg(long, global = true, help = "Path to state file")]
    pub state: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Apply configuration changes")]
    Apply,

    #[command(about = "Show preview of changes without applying")]
    Diff,

    #[command(about = "Check for drift from last applied configuration")]
    Status,

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
    #[command(about = "List all configured hosts")]
    List,

    #[command(about = "Show merged configuration for a specific host")]
    Show {
        #[arg(help = "Name of the host to show")]
        name: String,
    },
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        match &self.command {
            Commands::Apply => self.run_apply(),
            Commands::Diff => self.run_diff(),
            Commands::Status => self.run_status(),
            Commands::Undo => self.run_undo(),
            Commands::Hosts(hosts_cmd) => self.run_hosts(hosts_cmd),
            Commands::Render { template } => self.run_render(template),
            Commands::Secrets(secrets_cmd) => self.run_secrets(secrets_cmd),
            Commands::Init { repo, apply } => self.run_init(repo, *apply),
            Commands::Edit { target } => self.run_edit(target),
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

        if let Some(config_dir) = directories::BaseDirs::new().map(|dirs| dirs.config_dir().join("mimic/config.toml"))
            && config_dir.exists() {
                return Ok(config_dir);
            }

        Err(anyhow::anyhow!(
            "Config file not found. Searched:\n  - ./mimic.toml\n  - ~/.config/mimic/config.toml\n\nUse --config to specify a custom path."
        ))
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

    fn resolve_config_and_host(&self) -> anyhow::Result<(Config, Option<String>)> {
        let config_path = self.find_config()?;

        if self.verbose {
            println!(
                "{} {}",
                "Loading config:".bright_black(),
                config_path.display()
            );
        }

        let base_config = Config::from_file(&config_path)?;

        if base_config.hosts.is_empty() {
            return Ok((base_config, None));
        }

        let host_name = if let Some(host) = &self.host {
            host.clone()
        } else {
            Self::detect_hostname()
        };

        if self.verbose {
            println!("{} {}", "Using host:".bright_black(), host_name);
        }

        let merged_config = base_config.with_host(&host_name)?;
        Ok((merged_config, Some(host_name)))
    }

    fn run_hosts(&self, cmd: &HostCommands) -> anyhow::Result<()> {
        let config_path = self.find_config()?;
        let config = Config::from_file(&config_path)?;

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
                        let roles = if host_config.roles.is_empty() {
                            "no roles".bright_black().to_string()
                        } else {
                            host_config.roles.join(", ")
                        };
                        println!("  {} ({})", name.green(), roles);
                    }
                }
                Ok(())
            }
            HostCommands::Show { name } => {
                let merged = config.with_host(name)?;

                println!("{} {}", "Host:".bold(), name.green());
                println!();

                println!("{}", "Variables:".bold());
                if merged.variables.is_empty() {
                    println!("  {}", "(none)".bright_black());
                } else {
                    for (key, value) in &merged.variables {
                        println!("  {} = {}", key, value);
                    }
                }
                println!();

                println!("{}", "Dotfiles:".bold());
                if merged.dotfiles.is_empty() {
                    println!("  {}", "(none)".bright_black());
                } else {
                    for dotfile in &merged.dotfiles {
                        println!("  {} → {}", dotfile.source, dotfile.target);
                    }
                }
                println!();

                println!("{}", "Packages:".bold());
                if merged.packages.homebrew.is_empty() {
                    println!("  {}", "(none)".bright_black());
                } else {
                    for package in &merged.packages.homebrew {
                        println!("  {} ({})", package.name, package.pkg_type);
                    }
                }

                Ok(())
            }
        }
    }

    fn run_diff(&self) -> anyhow::Result<()> {
        let (config, host_name) = self.resolve_config_and_host()?;

        let host_roles = if let Some(ref host_name) = host_name {
            config
                .hosts
                .get(host_name)
                .map(|h| h.roles.clone())
                .unwrap_or_default()
        } else {
            vec![]
        };

        let filtered_dotfiles: Vec<_> = config
            .dotfiles
            .iter()
            .filter(|df| should_apply_for_roles(&df.only_roles, &df.skip_roles, &host_roles))
            .cloned()
            .collect();

        let filtered_packages: Vec<_> = config
            .packages
            .homebrew
            .iter()
            .filter(|pkg| should_apply_for_roles(&pkg.only_roles, &pkg.skip_roles, &host_roles))
            .cloned()
            .collect();

        let filtered_config = Config {
            variables: config.variables,
            dotfiles: filtered_dotfiles,
            packages: crate::config::Packages {
                homebrew: filtered_packages,
                brew: Vec::new(),
                cask: Vec::new(),
            },
            hosts: config.hosts,
            hooks: config.hooks,
            secrets: config.secrets,
            mise: config.mise,
        };

        let diff_engine = DiffEngine::new();
        let changes = diff_engine.diff(&filtered_config)?;

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

        if add_count > 0 || modify_count > 0 {
            println!();
            println!(
                "{} {} to add, {} to modify",
                "Summary:".bold(),
                add_count.to_string().green(),
                modify_count.to_string().yellow()
            );
        }

        Ok(())
    }

    fn run_apply(&self) -> anyhow::Result<()> {
        let (config, host_name) = self.resolve_config_and_host()?;

        let diff_engine = DiffEngine::new();
        let changes = diff_engine.diff(&config)?;

        if changes.is_empty() {
            println!("{}", "No changes to apply.".bright_black());
            return Ok(());
        }

        println!("{}", "Changes to apply:".bold());
        for change in &changes {
            println!("{}", change.format());
        }

        if self.dry_run {
            println!();
            println!("{}", "Dry-run mode: No changes were made.".yellow());
            return Ok(());
        }

        if !self.yes {
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

        let state_path = self.get_state_path();
        let mut state = State::load(&state_path).unwrap_or_else(|_| State::new());

        state.active_host = host_name.clone();

        println!();
        println!("{}", "Applying changes...".bold());

        let mut apply_to_all: Option<ApplyToAllChoice> = if self.yes {
            Some(ApplyToAllChoice::Backup)
        } else {
            None
        };

        let host_ctx = if let Some(ref host_name) = host_name {
            let host_config = config.hosts.get(host_name).unwrap();
            HostContext {
                name: host_name.clone(),
                roles: host_config.roles.clone(),
            }
        } else {
            HostContext {
                name: "default".to_string(),
                roles: vec![],
            }
        };

        for dotfile in &config.dotfiles {
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
                            return Err(e);
                        }
                    }
                }
            }
        }

        let homebrew = HomebrewManager::new();
        let normalized_packages = config.packages.normalized();
        for package in &normalized_packages.homebrew {
            if !should_apply_for_roles(&package.only_roles, &package.skip_roles, &host_ctx.roles) {
                if self.verbose {
                    println!("  {} {} (role mismatch)", "↷".bright_black(), package.name);
                }
                continue;
            }

            if self.verbose {
                println!("  {} {}", "Installing:".bright_black(), package.name);
            }

            match homebrew.install(&package.name, &package.pkg_type, &mut state) {
                Ok(()) => {
                    println!("  {} brew package: {}", "✓".green(), package.name);
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
                            return Err(e);
                        }
                    }
                }
            }
        }

        if !config.hooks.is_empty() {
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

                        let canonical_expected = match source_path.canonicalize() {
                            Ok(p) => p,
                            Err(_) => {
                                drift_details.push(format!(
                                    "  {} {} (source missing: {})",
                                    "✗".red(),
                                    target_path.display(),
                                    source_path.display()
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
                                source_path.display()
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
        let mut packages_ok = 0;
        let mut packages_drift = 0;

        for package in &state.packages {
            if package.manager == "brew" {
                match homebrew.is_installed(&package.name) {
                    Ok(true) => {
                        packages_ok += 1;
                        if self.verbose {
                            println!("  {} brew: {}", "✓".green(), package.name);
                        }
                    }
                    Ok(false) => {
                        drift_details.push(format!(
                            "  {} brew package not installed: {}",
                            "✗".yellow(),
                            package.name
                        ));
                        packages_drift += 1;
                    }
                    Err(e) => {
                        drift_details.push(format!(
                            "  {} error checking {}: {}",
                            "✗".red(),
                            package.name,
                            e
                        ));
                        packages_drift += 1;
                    }
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
            std::process::exit(1);
        } else {
            println!("{}", "✓ All resources in sync".green().bold());
        }

        Ok(())
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

            if target.exists() || target.is_symlink() {
                match std::fs::remove_file(&target) {
                    Ok(()) => {
                        symlinks_removed += 1;
                        println!("  {} Removed symlink: {}", "✓".green(), target.display());
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Failed to remove symlink {}: {}", target.display(), e);
                        eprintln!("  {} {}", "✗".red(), error_msg);
                        errors.push(error_msg);
                    }
                }
            } else if self.verbose {
                println!(
                    "  {} Symlink already removed: {}",
                    "○".bright_black(),
                    target.display()
                );
            }

            if let Some(backup_path_str) = &dotfile.backup_path {
                let backup_path = PathBuf::from(backup_path_str);

                if backup_path.exists() {
                    match std::fs::copy(&backup_path, &target) {
                        Ok(_) => {
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

        let config_path = self.find_config()?;

        if self.verbose {
            println!(
                "{} {}",
                "Loading config:".bright_black(),
                config_path.display()
            );
        }

        let base_config = Config::from_file(&config_path)?;

        let host_name = if let Some(host) = &self.host {
            host.clone()
        } else {
            Self::detect_hostname()
        };

        let merged_config = if !base_config.hosts.is_empty() {
            if self.verbose {
                println!("{} {}", "Using host:".bright_black(), host_name);
            }
            base_config.with_host(&host_name)?
        } else {
            base_config
        };

        let host_ctx = if !merged_config.hosts.is_empty() {
            let host_config = merged_config.hosts.get(&host_name).unwrap();
            HostContext {
                name: host_name.clone(),
                roles: host_config.roles.clone(),
            }
        } else {
            HostContext {
                name: "default".to_string(),
                roles: vec![],
            }
        };

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
                let config = Config::from_file(&self.find_config()?)?;
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

                    println!("export {}=\"{}\"", env_var, value);
                }

                Ok(())
            }
        }
    }

    fn run_init(&self, repo: &str, apply_after: bool) -> anyhow::Result<()> {
        use crate::secrets_scan::scan_for_secrets;
        use dialoguer::Confirm;
        use std::fs;
        use std::process::Command;

        let repo_dir = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?
            .config_dir()
            .join("mimic/repo");

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

        let output = Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(repo)
            .arg(&repo_dir)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                println!(
                    "  {} Repository cloned to {}",
                    "✓".green(),
                    repo_dir.display().to_string().green()
                );
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stderr.contains("not found") || stderr.contains("does not exist") {
                    return Err(anyhow::anyhow!(
                        "Git clone failed: Repository not found\n\nTo fix:\n  - Verify the repository URL is correct\n  - Ensure you have access to the repository\n  - Try cloning manually: git clone {}",
                        repo
                    ));
                } else if stderr.contains("Authentication failed")
                    || stderr.contains("Permission denied")
                {
                    return Err(anyhow::anyhow!(
                        "Git clone failed: Authentication failed\n\nTo fix:\n  - Ensure you have access to the repository\n  - Check your SSH keys or credentials\n  - Try using HTTPS URL instead of SSH (or vice versa)"
                    ));
                } else if stderr.contains("Could not resolve host") {
                    return Err(anyhow::anyhow!(
                        "Git clone failed: Network error\n\nTo fix:\n  - Check your internet connection\n  - Verify the repository host is correct"
                    ));
                } else {
                    return Err(anyhow::anyhow!(
                        "Git clone failed\n\n{}\n\nTo fix:\n  - Try cloning manually: git clone {}",
                        stderr.trim(),
                        repo
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(anyhow::anyhow!(
                    "Git is not installed or not in PATH\n\nTo fix:\n  - Install git: brew install git (macOS)\n  - Or: apt install git (Linux)"
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to execute git: {}", e));
            }
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

        let secrets = scan_for_secrets(&paths_to_scan)?;

        println!();

        if !secrets.is_empty() {
            println!("{}", "⚠ Secrets detected in repository".yellow().bold());
            println!(
                "  {} secret(s) found - review output above",
                secrets.len().to_string().yellow()
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
                command: Commands::Apply,
                config: Some(config_path),
                host: self.host.clone(),
                yes: true,
                dry_run: self.dry_run,
                verbose: self.verbose,
                state: self.state.clone(),
            };

            apply_cli.run_apply()?;
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

    fn run_edit(&self, target: &str) -> anyhow::Result<()> {
        use std::env;
        use std::process::Command;

        let expanded_target = if target.starts_with("~/") {
            if let Some(home) = directories::BaseDirs::new() {
                home.home_dir()
                    .join(&target[2..])
                    .to_string_lossy()
                    .to_string()
            } else {
                target.to_string()
            }
        } else {
            target.to_string()
        };

        let state_path = self.get_state_path();
        let mut source_path: Option<String> = None;

        if state_path.exists()
            && let Ok(state) = State::load(&state_path) {
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
            let config = Config::from_file(&config_path)?;

            let config_dir = config_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Config file has no parent directory"))?;

            for dotfile in &config.dotfiles {
                let config_target = if dotfile.target.starts_with("~/") {
                    if let Some(home) = directories::BaseDirs::new() {
                        home.home_dir()
                            .join(&dotfile.target[2..])
                            .to_string_lossy()
                            .to_string()
                    } else {
                        dotfile.target.clone()
                    }
                } else {
                    dotfile.target.clone()
                };

                if config_target == expanded_target || dotfile.target == target {
                    let source = PathBuf::from(&dotfile.source);
                    let resolved_source = if source.is_absolute() {
                        source
                    } else {
                        config_dir.join(source)
                    };
                    source_path = Some(resolved_source.to_string_lossy().to_string());
                    if self.verbose {
                        println!(
                            "{} Found in config: {}",
                            "→".bright_black(),
                            resolved_source.display()
                        );
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
}

pub fn run() -> Result<(), i32> {
    let cli = Cli::parse();

    match cli.run() {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);

            if cli.verbose
                && let Some(source) = e.source() {
                    eprintln!();
                    eprintln!("{}", "Caused by:".bright_black());
                    eprintln!("  {}", source);
                }

            Err(1)
        }
    }
}
