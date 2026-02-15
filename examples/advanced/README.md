# Advanced Example

This example demonstrates a more sophisticated mimic configuration with organized directory structure and machine-specific configurations.

## Directory Structure

```
advanced/
├── mimic.toml              # Main configuration
├── dotfiles/               # Organized by application
│   ├── zsh/
│   │   ├── zshrc
│   │   └── zshenv
│   ├── git/
│   │   ├── gitconfig
│   │   └── gitignore_global
│   ├── vim/
│   │   └── vimrc
│   └── tmux/
│       └── tmux.conf
└── machines/               # Machine-specific configs
    ├── work-laptop.toml
    └── home-desktop.toml
```

## Features Demonstrated

### Organized Dotfiles
Dotfiles are grouped by application in subdirectories for better organization:
- `dotfiles/zsh/` - Shell configuration files
- `dotfiles/git/` - Git configuration and global gitignore
- `dotfiles/vim/` - Vim editor configuration
- `dotfiles/tmux/` - Tmux terminal multiplexer config

### Advanced Configurations
- **Zsh**: Modern aliases (exa, bat, ripgrep), FZF integration, starship prompt
- **Git**: SSH signing, rerere, custom aliases, VS Code as difftool
- **Vim**: Persistent undo, backup directories, window navigation, visual mode improvements
- **Tmux**: Vim-style keybindings, custom prefix, pane splitting shortcuts

### Machine-Specific Configs
The `machines/` directory contains tailored configurations for different environments:

**work-laptop.toml** - Minimal setup for work
- Work email
- Essential tools only
- Docker for containers

**home-desktop.toml** - Full development setup
- Personal email
- Complete toolchain
- VS Code integration

## Usage

Use the main configuration:
```bash
mimic apply --config mimic.toml
```

Or use a machine-specific configuration:
```bash
mimic apply --config machines/work-laptop.toml
```

The machine-specific configs share the same dotfiles but with different variables and package selections.

## Variables

Variables can be overridden in machine-specific configs:
- `email` - Different for work vs personal
- `editor` - vim vs VS Code depending on machine
- `shell` - Shell preference

## Modern CLI Tools

This configuration replaces traditional tools with modern alternatives:
- `exa` instead of `ls` - Better colors and git integration
- `bat` instead of `cat` - Syntax highlighting
- `ripgrep` instead of `grep` - Faster searching
- `fzf` for fuzzy finding files and history
