# Basic Example

This example demonstrates a simple mimic configuration with common dotfiles and packages.

## Contents

- **dotfiles/zshrc** - Zsh shell configuration with history and aliases
- **dotfiles/gitconfig** - Git configuration with user info and aliases
- **dotfiles/vimrc** - Vim editor configuration
- **mimic.toml** - Configuration file declaring dotfiles and packages

## Variables

The configuration uses these variables:
- `email` - User email address
- `name` - User's full name  
- `editor` - Preferred text editor

## Packages

The configuration installs these Homebrew packages:
- `git` - Version control
- `neovim` - Modern vim editor
- `ripgrep` - Fast text search

## Usage

From this directory:

```bash
mimic apply --config mimic.toml
```

This will:
1. Create symlinks for dotfiles in your home directory
2. Install declared Homebrew packages if missing
3. Create backups if files already exist at target locations

To see what would change without applying:

```bash
mimic diff --config mimic.toml
```

To check if everything is in sync:

```bash
mimic status
```

To undo the last apply:

```bash
mimic undo
```
