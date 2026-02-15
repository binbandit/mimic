# Organized Example

Demonstrates well-structured dotfile organization by grouping related configurations together. Shows how to maintain clean, scalable dotfile repositories.

## What This Example Shows

- **Organized directory structure** - Group dotfiles by application (zsh/, git/, nvim/, tmux/)
- **Modular configuration** - Split configs into logical pieces (aliases, environment, rc files)
- **Directory symlinking** - Link entire directories (nvim/)
- **Comprehensive configs** - Real-world, production-ready configurations
- **Scalability** - Pattern that grows well with more applications

## File Structure

```
organized/
├── mimic.toml
├── README.md
└── dotfiles/
    ├── zsh/
    │   ├── zshrc          # Main zsh config (sources aliases)
    │   ├── zshenv         # Environment variables
    │   └── aliases        # All aliases and functions
    ├── git/
    │   ├── gitconfig      # Git configuration
    │   └── gitignore_global   # Global gitignore patterns
    ├── nvim/
    │   └── init.lua       # Neovim configuration (directory symlink)
    └── tmux/
        └── tmux.conf      # Tmux configuration
```

## Why Organize This Way?

### Scalability
As your dotfiles grow, organization prevents chaos:
- Easy to find specific configs
- Clear ownership of files
- Grouped by application/purpose

### Modularity
Split large configs into logical pieces:
- **zsh/aliases** - All aliases separate from main config
- **zsh/zshenv** - Environment variables isolated
- **zsh/zshrc** - Main configuration

### Clarity
New contributors or your future self can navigate easily:
```bash
# Instead of:
dotfiles/zshrc
dotfiles/zshenv
dotfiles/gitconfig
dotfiles/gitignore
dotfiles/vimrc
dotfiles/tmux.conf

# Better:
dotfiles/zsh/zshrc
dotfiles/zsh/zshenv
dotfiles/git/gitconfig
dotfiles/git/gitignore_global
dotfiles/nvim/init.lua
dotfiles/tmux/tmux.conf
```

## Configuration Highlights

### Zsh (Split into 3 files)

**zshenv** - Environment setup:
- PATH configuration
- Editor settings
- mise/cargo activation

**zshrc** - Interactive shell:
- Completion system
- History settings
- FZF integration
- Starship prompt

**aliases** - All aliases and functions:
- Modern tool aliases (exa, bat)
- Git workflow shortcuts
- Utility functions (mkcd, extract)

### Git (Complete workflow)

**gitconfig** - Comprehensive Git setup:
- Delta pager for beautiful diffs
- Useful aliases for daily workflow
- Auto-rebase, auto-stash
- Better merge conflict resolution (zdiff3)

**gitignore_global** - Ignore patterns:
- OS-specific files (.DS_Store, Thumbs.db)
- Editor artifacts (.swp, .idea/)
- Environment files (.env)
- Language-specific (node_modules/, __pycache__/)

### Neovim (Modern minimal config)

**init.lua** - Sensible defaults:
- Relative line numbers
- System clipboard integration
- Smart case search
- Vim-like window navigation
- Highlight on yank

### Tmux (Power user setup)

**tmux.conf** - Full-featured:
- Vim-like navigation
- Mouse support
- Better key bindings
- Custom status bar
- Visual mode bindings

## Usage

### Preview changes

```bash
cd examples/organized
mimic diff
```

### Apply configuration

```bash
mimic apply
```

This creates:
- `~/.zshrc` → `dotfiles/zsh/zshrc`
- `~/.zshenv` → `dotfiles/zsh/zshenv`
- `~/.config/zsh/aliases` → `dotfiles/zsh/aliases`
- `~/.gitconfig` → `dotfiles/git/gitconfig`
- `~/.gitignore_global` → `dotfiles/git/gitignore_global`
- `~/.config/nvim/` → `dotfiles/nvim/` (entire directory)
- `~/.tmux.conf` → `dotfiles/tmux/tmux.conf`

### Verify

```bash
mimic status
```

## Expanding This Pattern

### Add new applications

```bash
mkdir dotfiles/alacritty
# Create dotfiles/alacritty/alacritty.yml
```

```toml
[[dotfiles]]
source = "dotfiles/alacritty/alacritty.yml"
target = "~/.config/alacritty/alacritty.yml"
```

### Add more zsh files

```bash
# Create dotfiles/zsh/completion.zsh
# Create dotfiles/zsh/prompt.zsh
```

Update zshrc to source them:
```bash
for file in ~/.config/zsh/*.zsh; do
    source "$file"
done
```

### Split git config further

```toml
[[dotfiles]]
source = "dotfiles/git/config"
target = "~/.gitconfig"

[[dotfiles]]
source = "dotfiles/git/config-work"
target = "~/.gitconfig-work"

[[dotfiles]]
source = "dotfiles/git/config-personal"
target = "~/.gitconfig-personal"
```

Use conditional includes in main config:
```
[includeIf "gitdir:~/work/"]
    path = ~/.gitconfig-work

[includeIf "gitdir:~/personal/"]
    path = ~/.gitconfig-personal
```

## Comparison with Flat Structure

### Flat (examples/basic/)
```
dotfiles/
├── zshrc
├── gitconfig
└── tmux.conf
```

**Pros:** Simple, minimal overhead  
**Cons:** Gets messy with many files

### Organized (this example)
```
dotfiles/
├── zsh/
│   ├── zshrc
│   ├── zshenv
│   └── aliases
├── git/
│   ├── gitconfig
│   └── gitignore_global
├── nvim/
│   └── init.lua
└── tmux/
    └── tmux.conf
```

**Pros:** Scalable, clear ownership, modular  
**Cons:** Slightly more complex structure

## When to Use Which?

**Use flat structure (basic/) when:**
- You have < 10 dotfiles
- Simple, single-file configs
- Learning mimic

**Use organized structure (this example) when:**
- You have > 10 dotfiles
- Complex, multi-file configs
- Long-term maintenance
- Team sharing dotfiles

## Next Steps

After exploring organization patterns, check out:

- **examples/multi-host/** - Manage different configs per machine
- **examples/complete/** - Full feature showcase with hooks, secrets, and more
