# mimic

A declarative dotfile management system with symlink management, package installation, and drift detection.

## What is mimic?

mimic is a CLI tool for managing your development environment configuration. It creates symlinks for your dotfiles, installs packages via Homebrew, and tracks everything in a state file. If anything drifts from the declared configuration (deleted symlinks, missing packages), mimic detects and fixes it.

**Key features:**
- **Declarative configuration** - Define your desired state in a TOML file
- **Safe operations** - Automatic backups, dry-run mode, interactive conflict resolution
- **State tracking** - Always knows what it manages, enabling safe undo operations
- **Drift detection** - Verify your system matches the declared configuration
- **Template variables** - Customize configs with user-defined and system variables

## Installation

### From source

```bash
git clone https://github.com/yourusername/mimic.git
cd mimic
cargo install --path .
```

### From crates.io

```bash
cargo install mimic
```

## Quick Start

### 1. Create a configuration file

Create `mimic.toml` in your dotfiles repository:

```toml
[variables]
email = "you@example.com"
name = "Your Name"
editor = "nvim"

[[dotfiles]]
source = "dotfiles/zshrc"
target = "~/.zshrc"

[[dotfiles]]
source = "dotfiles/gitconfig"
target = "~/.gitconfig"

[[packages.homebrew]]
name = "git"
type = "formula"

[[packages.homebrew]]
name = "neovim"
type = "formula"
```

### 2. Preview changes

See what mimic would do without making changes:

```bash
mimic diff
```

### 3. Apply configuration

Create symlinks and install packages:

```bash
mimic apply
```

Mimic will:
- Show you what changes will be made
- Prompt for confirmation (unless you use `--yes`)
- Create backups if files already exist at target locations
- Install missing packages
- Save state for drift detection and undo

### 4. Verify everything is in sync

```bash
mimic status
```

### 5. Undo if needed

Remove symlinks and restore backups:

```bash
mimic undo
```

## Commands

### `mimic apply`

Apply the configuration by creating symlinks and installing packages.

```bash
mimic apply [OPTIONS]
```

**Options:**
- `--config <PATH>` - Path to config file (default: `./mimic.toml` or `~/.config/mimic/config.toml`)
- `--state <PATH>` - Path to state file (default: `~/.config/mimic/state.toml`)
- `--yes, -y` - Skip confirmation prompts, auto-backup conflicts
- `--dry-run, -n` - Show what would happen without making changes
- `--verbose, -v` - Show detailed output

**Examples:**

```bash
mimic apply

mimic apply --dry-run

mimic apply --yes --config ~/dotfiles/mimic.toml

mimic apply --verbose
```

### `mimic diff`

Preview changes without applying them. Shows what would be added or modified.

```bash
mimic diff [OPTIONS]
```

**Options:**
- `--config <PATH>` - Path to config file
- `--verbose, -v` - Show detailed output

**Output:**
- `+ dotfile` - New symlink would be created (green)
- `+ package` - Package would be installed (green)
- `~ dotfile` - Symlink would be modified (yellow)
- `✓` - Already correct (gray)

**Examples:**

```bash
mimic diff

mimic diff --config machines/work.toml
```

### `mimic status`

Check if the system matches the last applied configuration. Detects drift like missing symlinks or uninstalled packages.

```bash
mimic status [OPTIONS]
```

**Options:**
- `--state <PATH>` - Path to state file
- `--verbose, -v` - Show all resources individually

**Exit codes:**
- `0` - All resources in sync
- `1` - Drift detected
- `2` - Error

**Examples:**

```bash
mimic status

mimic status --verbose
```

### `mimic undo`

Undo the last apply operation by removing symlinks and restoring backups.

```bash
mimic undo [OPTIONS]
```

**Options:**
- `--state <PATH>` - Path to state file
- `--verbose, -v` - Show detailed output

**Examples:**

```bash
mimic undo

mimic undo --verbose
```

## Configuration Reference

### File structure

```toml
[variables]
key = "value"

[[dotfiles]]
source = "path/to/source"
target = "path/to/target"

[[packages.homebrew]]
name = "package-name"
type = "formula"  # or "cask"
```

### Variables section

Define custom variables that can be used in templates (future feature) and help document your configuration.

```toml
[variables]
email = "user@example.com"
name = "User Name"
editor = "nvim"
shell = "zsh"
```

**System variables** (automatically available):
- `{{ hostname }}` - System hostname
- `{{ username }}` - Current username
- `{{ os }}` - Operating system (e.g., "macos", "linux")
- `{{ arch }}` - CPU architecture (e.g., "aarch64", "x86_64")

### Dotfiles

Each dotfile entry creates a symlink from target to source.

```toml
[[dotfiles]]
source = "dotfiles/zshrc"    # Path to source file (relative to config or absolute)
target = "~/.zshrc"          # Where to create symlink (~ expands to home dir)
```

**Path expansion:**
- `~` expands to home directory
- Environment variables like `$HOME` are expanded
- Relative paths in `source` are relative to the config file location

### Packages

Packages are installed via Homebrew (macOS/Linux).

```toml
[[packages.homebrew]]
name = "git"           # Package name as it appears in brew
type = "formula"       # "formula" for CLI tools, "cask" for GUI apps
```

**Package behavior:**
- mimic installs declared packages if missing
- mimic **never** uninstalls packages (safe by design)
- To remove a package from management, remove it from config and run `mimic undo`

## Configuration File Discovery

mimic searches for configuration files in this order:

1. `--config` flag if provided
2. `./mimic.toml` in current directory
3. `~/.config/mimic/config.toml`

## State File

mimic tracks applied configuration in a state file (default: `~/.config/mimic/state.toml`).

The state file contains:
- Symlinks created (source, target, backup path)
- Packages installed (name, manager)
- Last apply timestamp

**Important:** Don't edit the state file manually. Use `mimic apply` and `mimic undo`.

## Conflict Resolution

When applying configuration, mimic may find existing files at target locations.

**Interactive mode** (default):
- Shows each conflict
- Prompts: [s]kip, [o]verwrite, [b]ackup, [a]pply to all
- Backup creates `{filename}.backup.{timestamp}`

**Automatic mode** (`--yes` flag):
- Always creates backups
- No prompts, safe for scripts

## Examples

See the `examples/` directory for complete working examples:

- **`examples/basic/`** - Simple configuration with common dotfiles and packages
- **`examples/advanced/`** - Organized structure with machine-specific configs

Run an example:

```bash
cd examples/basic
mimic apply --config mimic.toml
```

## Design Philosophy

mimic follows these principles:

1. **Declarative** - State what you want, not how to get there
2. **Safe by default** - Backups, dry-run, confirmations
3. **Additive only** - Install declared packages, never auto-uninstall
4. **Transparent** - State file shows exactly what's managed
5. **Recoverable** - Undo restores previous state

## Roadmap

See the project plan for Phase 2 features:
- Multi-machine support with host-specific configurations
- Template file generation with variable substitution in file contents
- Additional package managers (apt, dnf, pacman)
- Git integration for dotfile repo management
- Pre/post hooks for custom scripts

## License

MIT

## Contributing

Contributions welcome! Please open an issue or pull request.
