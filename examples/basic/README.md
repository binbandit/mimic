# Basic Example

A simple starter configuration demonstrating core mimic features with minimal complexity. Perfect for getting started with dotfile management.

## What This Example Shows

- **Flat directory structure** - Simple layout with all dotfiles in one directory
- **Simple package format** - Using `brew` and `cask` arrays for Homebrew packages
- **Essential dotfiles** - Common shell, git, and terminal multiplexer configs
- **Variable usage** - Declaring variables (though not templating them yet)

## File Structure

```
basic/
├── mimic.toml          # Configuration declaring dotfiles and packages
├── README.md           # This file
└── dotfiles/
    ├── zshrc           # Zsh shell configuration
    ├── gitconfig       # Git configuration
    └── tmux.conf       # Tmux terminal multiplexer config
```

## Configuration Highlights

### Variables

While this example doesn't use templates, it declares variables that could be referenced later:

```toml
[variables]
email = "user@example.com"
name = "Your Name"
editor = "nvim"
```

### Dotfiles

Three essential configuration files:

- **zshrc** - Shell configuration with history settings, modern aliases (exa, bat), and git shortcuts
- **gitconfig** - Git with delta pager, auto-setup remote, and useful aliases
- **tmux.conf** - Tmux with vim-like keybindings, mouse support, and custom status bar

### Packages

Modern CLI tools via Homebrew:

- **git** - Version control
- **neovim** - Modern vim-based editor
- **tmux** - Terminal multiplexer
- **ripgrep** - Fast text search (rg)
- **fd** - Fast file finder
- **bat** - cat clone with syntax highlighting
- **exa** - Modern ls replacement
- **visual-studio-code** - GUI code editor (cask)

## Usage

### Preview changes

See what mimic will do without applying:

```bash
cd examples/basic
mimic diff
```

### Apply configuration

Create symlinks and install packages:

```bash
mimic apply
```

This will:
1. Create `~/.zshrc` → `dotfiles/zshrc`
2. Create `~/.gitconfig` → `dotfiles/gitconfig`
3. Create `~/.tmux.conf` → `dotfiles/tmux.conf`
4. Install missing Homebrew packages
5. Backup any existing files

### Verify status

Check if everything matches the configuration:

```bash
mimic status
```

### Undo changes

Remove symlinks and restore backups:

```bash
mimic undo
```

## Next Steps

After trying this basic example, explore:

- **examples/templates/** - Learn to use variable templating with `.tmpl` files
- **examples/organized/** - See how to organize dotfiles by application
- **examples/multi-host/** - Manage multiple machines with different configs
- **examples/complete/** - Full feature showcase including hooks and secrets

## Customization

To adapt this for your use:

1. Edit `mimic.toml` variables with your information
2. Modify dotfiles to match your preferences
3. Add/remove packages based on your needs
4. Run `mimic apply` to sync changes
