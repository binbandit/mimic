use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use crate::config::Config;
use crate::diff::{Change, DiffEngine};
use crate::installer::HomebrewManager;
use crate::linker::{create_symlink_with_resolution, ApplyToAllChoice};
use crate::state::State;

#[derive(Parser)]
#[command(name = "mimic")]
#[command(version = "0.1.0")]
#[command(about = "Dotfile management system", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Path to config file")]
    pub config: Option<PathBuf>,

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

    #[command(about = "Show current system state (not yet implemented)")]
    Status,

    #[command(about = "Undo last apply operation")]
    Undo,
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        match &self.command {
            Commands::Apply => self.run_apply(),
            Commands::Diff => self.run_diff(),
            Commands::Status => self.run_status(),
            Commands::Undo => self.run_undo(),
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

        if let Some(config_dir) = directories::BaseDirs::new()
            .and_then(|dirs| Some(dirs.config_dir().join("mimic/config.toml")))
        {
            if config_dir.exists() {
                return Ok(config_dir);
            }
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

    fn run_diff(&self) -> anyhow::Result<()> {
        let config_path = self.find_config()?;

        if self.verbose {
            println!(
                "{} {}",
                "Loading config:".bright_black(),
                config_path.display()
            );
        }

        let config = Config::from_file(&config_path)?;
        let diff_engine = DiffEngine::new();
        let changes = diff_engine.diff(&config)?;

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
        let config_path = self.find_config()?;

        if self.verbose {
            println!(
                "{} {}",
                "Loading config:".bright_black(),
                config_path.display()
            );
        }

        let config = Config::from_file(&config_path)?;
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

        println!();
        println!("{}", "Applying changes...".bold());

        let mut apply_to_all: Option<ApplyToAllChoice> = if self.yes {
            Some(ApplyToAllChoice::Backup)
        } else {
            None
        };

        for dotfile in &config.dotfiles {
            let source = PathBuf::from(&dotfile.source);
            let target = PathBuf::from(&dotfile.target);

            if self.verbose {
                println!(
                    "  {} {} → {}",
                    "Linking:".bright_black(),
                    source.display(),
                    target.display()
                );
            }

            match create_symlink_with_resolution(&source, &target, &mut state, &mut apply_to_all) {
                Ok(()) => {
                    println!("  {} {}", "✓".green(), target.display());
                }
                Err(e) => {
                    eprintln!("  {} {} - {}", "✗".red(), target.display(), e);
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
        for package in &config.packages.homebrew {
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
}

pub fn run() -> Result<(), i32> {
    let cli = Cli::parse();

    match cli.run() {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);

            if cli.verbose {
                if let Some(source) = e.source() {
                    eprintln!();
                    eprintln!("{}", "Caused by:".bright_black());
                    eprintln!("  {}", source);
                }
            }

            Err(1)
        }
    }
}
