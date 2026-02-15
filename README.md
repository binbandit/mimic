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
- **Secrets detection** - Scans files for API keys, tokens, and credentials (via ripsecrets)
- **Visual feedback** - Progress spinners show operation timing (auto-hidden in CI)
- **Better errors** - Full error chains with "To fix:" sections for actionable guidance

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

## Converting Your Existing Dotfiles to mimic

This guide walks you through converting any existing dotfiles setup to work with mimic.

### Step 1: Understand Your Current Setup

First, identify where your dotfiles currently live:

```bash
# Find all your dotfiles
ls -la ~ | grep "^\."

# Common locations:
# - Scattered in home directory: ~/.vimrc, ~/.zshrc, etc.
# - In a dotfiles directory: ~/dotfiles/, ~/.dotfiles/
# - In version control: Already in a git repository
```

### Step 2: Organize Your Dotfiles Repository

Create or use an existing directory for your dotfiles:

```bash
# If starting fresh
mkdir -p ~/dotfiles
cd ~/dotfiles

# If you already have a git repo
cd ~/dotfiles  # or wherever your repo is
```

**Recommended structure:**
```
~/dotfiles/
├── mimic.toml           # Configuration file
├── dotfiles/            # Your actual dotfiles
│   ├── zshrc
│   ├── vimrc
│   ├── gitconfig
│   └── tmux.conf
└── README.md
```

### Step 3: Copy Your Dotfiles

Move your dotfiles from home directory into your repository:

```bash
cd ~/dotfiles

# Create the dotfiles directory
mkdir -p dotfiles

# Copy your dotfiles (keep originals for now)
cp ~/.zshrc dotfiles/zshrc
cp ~/.vimrc dotfiles/vimrc
cp ~/.gitconfig dotfiles/gitconfig
cp ~/.tmux.conf dotfiles/tmux.conf

# For directories (like .config subdirectories)
mkdir -p dotfiles/nvim
cp -r ~/.config/nvim/* dotfiles/nvim/
```

**Important:** Remove the leading dot from filenames in your repository. mimic will create the symlinks with the dot in your home directory.

### Step 4: Create mimic.toml

Create a `mimic.toml` file in your dotfiles repository:

```toml
# Personal variables (optional but useful)
[variables]
email = "your.email@example.com"
name = "Your Name"
editor = "vim"

# Dotfile entries - one for each file or directory
[[dotfiles]]
source = "dotfiles/zshrc"
target = "~/.zshrc"

[[dotfiles]]
source = "dotfiles/vimrc"
target = "~/.vimrc"

[[dotfiles]]
source = "dotfiles/gitconfig"
target = "~/.gitconfig"

[[dotfiles]]
source = "dotfiles/tmux.conf"
target = "~/.tmux.conf"

# For .config subdirectories
[[dotfiles]]
source = "dotfiles/nvim"
target = "~/.config/nvim"

# Packages you want installed
[packages]
brew = ["git", "neovim", "tmux"]
cask = ["visual-studio-code"]
```

### Step 5: Preview the Changes

Before applying, see what mimic will do:

```bash
cd ~/dotfiles
mimic diff --config mimic.toml
```

This shows:
- Which symlinks will be created
- Which packages will be installed
- Any conflicts with existing files

### Step 6: Backup Your Originals

mimic will create backups automatically, but you can backup manually first:

```bash
# Backup all dotfiles you're about to replace
mkdir -p ~/dotfiles-backup
cp ~/.zshrc ~/dotfiles-backup/
cp ~/.vimrc ~/dotfiles-backup/
cp ~/.gitconfig ~/dotfiles-backup/
# ... etc
```

### Step 7: Test with Dry Run

See exactly what will happen without making changes:

```bash
mimic apply --dry-run --config ~/dotfiles/mimic.toml
```

### Step 8: Apply the Configuration

When ready, apply the configuration:

```bash
cd ~/dotfiles
mimic apply --config mimic.toml
```

mimic will:
1. Show you what changes will be made
2. Prompt for confirmation at each conflict
3. Automatically backup any existing files
4. Create symlinks from your home directory to the repository
5. Install missing packages

**Handling conflicts:**
- `[s]kip` - Keep existing file, don't create symlink
- `[o]verwrite` - Replace with symlink
- `[b]ackup` - Backup existing file, create symlink
- `[a]pply to all` - Use same choice for remaining conflicts

### Step 9: Verify Everything Works

Check that all symlinks are created correctly:

```bash
# Check symlink targets
ls -la ~/ | grep "\->"

# Example output:
# .zshrc -> /Users/you/dotfiles/dotfiles/zshrc
# .vimrc -> /Users/you/dotfiles/dotfiles/vimrc

# Verify status
mimic status
```

### Step 10: Commit to Git

If using git (recommended), commit your configuration:

```bash
cd ~/dotfiles
git init  # if not already a repo
git add .
git commit -m "Add mimic configuration"

# Push to GitHub (optional but recommended)
git remote add origin https://github.com/yourusername/dotfiles.git
git push -u origin main
```

## Common Migration Scenarios

### Scenario 1: Migrating from GNU Stow

If you're using GNU Stow with a structure like:
```
~/dotfiles/
├── vim/.vimrc
├── zsh/.zshrc
└── git/.gitconfig
```

Convert to mimic structure:
```bash
# Flatten the structure
mkdir -p ~/dotfiles/dotfiles
mv vim/.vimrc dotfiles/vimrc
mv zsh/.zshrc dotfiles/zshrc
mv git/.gitconfig dotfiles/gitconfig
```

Then create `mimic.toml` as shown in Step 4.

### Scenario 2: Migrating from Chezmoi

If you're using chezmoi with files like:
```
~/.local/share/chezmoi/
├── dot_vimrc
├── dot_zshrc
└── dot_gitconfig
```

Convert to mimic:
```bash
# Copy and rename files
mkdir -p ~/dotfiles/dotfiles
cp ~/.local/share/chezmoi/dot_vimrc ~/dotfiles/dotfiles/vimrc
cp ~/.local/share/chezmoi/dot_zshrc ~/dotfiles/dotfiles/zshrc
cp ~/.local/share/chezmoi/dot_gitconfig ~/dotfiles/dotfiles/gitconfig
```

For templated files in chezmoi, you'll need to manually resolve the templates or wait for mimic's template feature (Phase 2).

### Scenario 3: Dotfiles Scattered in Home Directory

If your dotfiles are directly in `~` without a repository:

```bash
# Create repository
mkdir -p ~/dotfiles/dotfiles
cd ~/dotfiles

# Copy all dotfiles
cp ~/.zshrc dotfiles/zshrc
cp ~/.bashrc dotfiles/bashrc
cp ~/.vimrc dotfiles/vimrc
cp ~/.gitconfig dotfiles/gitconfig
cp ~/.tmux.conf dotfiles/tmux.conf

# Copy .config directories
mkdir -p dotfiles/config
cp -r ~/.config/nvim dotfiles/config/nvim
cp -r ~/.config/kitty dotfiles/config/kitty

# Create config file
cat > mimic.toml << 'EOF'
[variables]
email = "your@email.com"

[[dotfiles]]
source = "dotfiles/zshrc"
target = "~/.zshrc"

[[dotfiles]]
source = "dotfiles/vimrc"
target = "~/.vimrc"

# Add more as needed...
EOF

# Preview and apply
mimic diff
mimic apply
```

### Scenario 4: Machine-Specific Dotfiles

If you have different configurations for different machines (work vs. home):

```toml
# mimic.toml - base configuration
[variables]
email = "personal@email.com"

[[dotfiles]]
source = "dotfiles/zshrc"
target = "~/.zshrc"

# Host-specific configuration
[hosts.work]
variables = { email = "work@company.com" }

[[hosts.work.dotfiles]]
source = "dotfiles/work-gitconfig"
target = "~/.gitconfig"

[[hosts.work.packages.homebrew]]
name = "docker"
type = "cask"

[hosts.personal]
variables = { email = "personal@email.com" }

[[hosts.personal.dotfiles]]
source = "dotfiles/personal-gitconfig"
target = "~/.gitconfig"
```

Then apply with:
```bash
mimic apply --host work    # On work machine
mimic apply --host personal # On personal machine
```

## Troubleshooting Migration

### Problem: Symlinks Point to Wrong Location

**Symptom:** `ls -la ~/.zshrc` shows symlink to wrong path

**Fix:**
```bash
# Remove incorrect symlink
mimic undo

# Check your source paths in mimic.toml
# source paths are relative to the config file location
# If config is in ~/dotfiles/mimic.toml, then:
# source = "dotfiles/zshrc" means ~/dotfiles/dotfiles/zshrc

# Fix paths and reapply
mimic apply
```

### Problem: File Already Exists

**Symptom:** mimic says "Target already exists"

**Fix:**
- Choose `[b]ackup` to backup the existing file
- Or choose `[a]pply to all` then `backup all` to handle all conflicts at once
- Original files are saved as `.backup.{timestamp}`

### Problem: Package Installation Fails

**Symptom:** Homebrew package won't install

**Fix:**
```bash
# Check Homebrew is installed
brew --version

# If not installed:
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Check package name is correct
brew search <package-name>

# Update package name in mimic.toml if needed
```

### Problem: Permission Denied

**Symptom:** Can't create symlink due to permissions

**Fix:**
```bash
# Check target directory permissions
ls -la ~/

# Ensure you own the directory
# For system directories, you may need to use different target paths

# Instead of system-wide directories, use user directories:
target = "~/.local/bin/something"  # Good
# Not: target = "/usr/local/bin/something"  # Requires sudo
```

## Tips for a Smooth Migration

1. **Start Small** - Begin with just 2-3 dotfiles, verify they work, then add more
2. **Keep Backups** - Don't delete your original files until you're confident everything works
3. **Test in Dry Run** - Always use `--dry-run` first when experimenting
4. **Use Git** - Keep your dotfiles in git so you can revert changes easily
5. **Document** - Add comments in mimic.toml to explain what each entry does
6. **One Machine First** - Test on one machine completely before syncing to others
7. **Check Symlinks** - Use `ls -la ~ | grep "\->"` to verify symlinks are correct

## Quick Start

### Single-Command Bootstrap

The fastest way to get started with an existing dotfiles repository:

```bash
mimic init --apply https://github.com/user/dotfiles
```

This command:
1. Clones the repository to `~/.config/mimic/repo/`
2. Automatically applies the configuration with auto-backup of conflicts
3. Installs declared packages
4. Shows progress with timing information

### Manual Setup

#### 1. Create a configuration file

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

[packages]
brew = ["git", "neovim"]
```

#### 2. Preview changes

See what mimic would do without making changes:

```bash
mimic diff
```

#### 3. Apply configuration

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
- Display progress spinners with timing for each operation

#### 4. Verify everything is in sync

```bash
mimic status
```

#### 5. Edit dotfiles easily

Open the source file for any managed dotfile:

```bash
mimic edit ~/.zshrc
```

#### 6. Undo if needed

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

### `mimic init`

Bootstrap a new dotfiles setup by cloning a repository to `~/.config/mimic/repo/`.

```bash
mimic init <REPO> [OPTIONS]
```

**Arguments:**
- `<REPO>` - Repository URL (HTTPS or SSH)

**Options:**
- `--apply` - Automatically apply configuration after cloning (uses `--yes`)

**Examples:**

```bash
# Clone repository only
mimic init https://github.com/user/dotfiles

# Clone and immediately apply
mimic init --apply https://github.com/user/dotfiles

# With SSH URL
mimic init --apply git@github.com:user/dotfiles.git
```

**Behavior:**
- Clones with `--depth 1` for faster downloads
- Stores repository in `~/.config/mimic/repo/`
- When using `--apply`, automatically runs `mimic apply --yes` with the cloned config
- Shows progress with timing information

**Error handling:**
- Checks if repository directory already exists
- Validates git is installed
- Provides actionable error messages for common issues (auth failures, network errors)

### `mimic edit`

Open the source file for a dotfile target in your editor.

```bash
mimic edit <TARGET>
```

**Arguments:**
- `<TARGET>` - Target path of the dotfile (e.g., `~/.zshrc`)

**Options:**
- `--config <PATH>` - Path to config file (if state file doesn't exist)
- `--verbose, -v` - Show source path and editor name

**Examples:**

```bash
# Edit zshrc source file
mimic edit ~/.zshrc

# Works with both tilde and absolute paths
mimic edit ~/.gitconfig
mimic edit /Users/username/.vimrc

# With verbose output
mimic edit ~/.zshrc --verbose
```

**Behavior:**
- First checks state file (`~/.config/mimic/state.toml`) for source path
- Falls back to config file if state doesn't exist
- Opens file in `$EDITOR`, or falls back to `vim`, then `nano`
- Resolves relative source paths relative to config file location

**Error handling:**
- Provides clear error if target not found in state or config
- Guides user to set `$EDITOR` if no editor available
- Shows actionable steps for common issues

## Configuration Reference

### File structure

```toml
[variables]
key = "value"

[[dotfiles]]
source = "path/to/source"
target = "path/to/target"

# Simple format (recommended)
[packages]
brew = ["git", "neovim", "ripgrep"]
cask = ["visual-studio-code", "docker"]

# Or use verbose format for role filtering
[[packages.homebrew]]
name = "package-name"
type = "formula"  # or "cask"
only_roles = ["work"]  # optional: only install on hosts with these roles
skip_roles = ["server"]  # optional: skip on hosts with these roles
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

**Simple format (recommended):**

```toml
[packages]
brew = ["git", "neovim", "tmux", "ripgrep"]
cask = ["visual-studio-code", "docker"]
```

**Verbose format (for role filtering):**

```toml
[[packages.homebrew]]
name = "git"
type = "formula"  # "formula" for CLI tools, "cask" for GUI apps
only_roles = ["work"]  # optional: only install on hosts with these roles
skip_roles = ["server"]  # optional: skip on hosts with these roles
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

## Visual Feedback

### Progress Spinners

mimic shows progress spinners during long-running operations:

- **Cloning repositories** - `mimic init` shows timing for git clone
- **Creating symlinks** - Each dotfile operation displays progress
- **Installing packages** - Homebrew installations show timing
- **CI/non-TTY detection** - Spinners automatically hidden in CI environments

**Example output:**
```
⠋ Cloning repository from https://github.com/user/dotfiles...
✓ Repository cloned to ~/.config/mimic/repo (took 2.34s)

⠋ Linking ~/.zshrc...
✓ Linked ~/.zshrc (took 0.08s)

⠋ Installing neovim...
✓ Installed neovim (took 8.12s)
```

Spinners detect CI environments via the `CI` environment variable and automatically disable themselves for clean log output.

## Error Handling

mimic provides detailed error messages with full error chains and actionable guidance.

**Error message structure:**
- **Main error** - What went wrong
- **Caused by chain** - Technical details and root cause
- **To fix section** - Actionable steps with commands

**Example:**
```
Error: Failed to read config file: /path/to/mimic.toml

Caused by:
  → No such file or directory (os error 2)

To fix:
  - Check that the file exists
  - Verify you have read permissions
  - Ensure the path is correct
```

**BrokenPipe handling:** When piping output to commands like `head` or closing a pager, mimic exits cleanly without showing errors.

**Partial failures:** The `apply` command continues processing after individual failures and shows a summary at the end, allowing you to see and fix all issues at once rather than iterating one error at a time.

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
