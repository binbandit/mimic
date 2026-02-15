# Quick Migration Checklist

Use this checklist when converting your dotfiles to mimic.

## Pre-Migration

- [ ] Identify all your current dotfiles (`ls -la ~ | grep "^\."`)
- [ ] Check if you have an existing dotfiles repository
- [ ] Backup your current dotfiles to a safe location
- [ ] Install mimic (`cargo install mimic` or build from source)
- [ ] Install Homebrew if not already installed

## Repository Setup

- [ ] Create or navigate to your dotfiles directory
- [ ] Create a `dotfiles/` subdirectory for actual config files
- [ ] Copy dotfiles into the repository (remove leading dots)
- [ ] Organize files into logical structure

Example structure:
```
~/dotfiles/
├── mimic.toml
├── dotfiles/
│   ├── zshrc
│   ├── vimrc
│   ├── gitconfig
│   └── tmux.conf
└── README.md
```

## Configuration

- [ ] Create `mimic.toml` in repository root
- [ ] Add `[variables]` section with your personal info
- [ ] Add `[[dotfiles]]` entries for each file/directory
- [ ] Add `[packages]` section with brew/cask lists
- [ ] Verify all source paths are relative to config file location

Minimal mimic.toml:
```toml
[variables]
email = "your@email.com"

[[dotfiles]]
source = "dotfiles/zshrc"
target = "~/.zshrc"

[packages]
brew = ["git", "neovim"]
cask = ["visual-studio-code"]
```

## Testing

- [ ] Run `mimic diff` to preview changes
- [ ] Review all symlinks that will be created
- [ ] Review all packages that will be installed
- [ ] Run `mimic apply --dry-run` to simulate without changes
- [ ] Verify no unexpected changes appear

## Apply Configuration

- [ ] Run `mimic apply` (without --yes for interactive mode)
- [ ] Review each conflict prompt carefully
- [ ] Choose `[b]ackup` for conflicts to preserve originals
- [ ] Wait for all packages to install
- [ ] Check for any error messages

## Verification

- [ ] Run `mimic status` to verify no drift
- [ ] Check symlinks are correct: `ls -la ~ | grep "->"`
- [ ] Test each application to ensure configs work
- [ ] Verify packages are installed: `brew list`
- [ ] Check that backups were created: `ls -la ~ | grep ".backup."`

## Git Setup (Recommended)

- [ ] Initialize git if not already: `git init`
- [ ] Create `.gitignore` file
- [ ] Add all files: `git add .`
- [ ] Create first commit: `git commit -m "Initial mimic configuration"`
- [ ] Push to remote: `git remote add origin <url> && git push -u origin main`

## Cleanup

- [ ] Test everything for a few days
- [ ] Once confident, remove backup files (or keep them safe)
- [ ] Document any custom setup in README
- [ ] Share your dotfiles repo (optional)

## Multi-Machine Setup

If setting up on additional machines:

- [ ] Clone your dotfiles repo OR use `mimic init --apply <repo-url>`
- [ ] Review the configuration for machine-specific needs
- [ ] Use `--host` flag if you have machine-specific configs
- [ ] Run `mimic apply`
- [ ] Verify with `mimic status`

## Troubleshooting Reference

| Problem | Solution |
|---------|----------|
| Symlink points to wrong path | Check source paths are relative to config file |
| File already exists | Choose `[b]ackup` option during apply |
| Package fails to install | Verify Homebrew installed, check package name |
| Permission denied | Don't use system directories, use user directories |
| Config not found | Check file is named `mimic.toml` exactly |

## Common Commands Reference

```bash
# Preview changes
mimic diff

# Apply with prompts
mimic apply

# Apply automatically (use with caution)
mimic apply --yes

# Check for drift
mimic status

# Undo last apply
mimic undo

# Edit a dotfile
mimic edit ~/.zshrc
```

## Next Steps

After successful migration:

1. Set up automated syncing (git hooks, cron, etc.)
2. Explore advanced features (hosts, templates, hooks)
3. Share your setup with the community
4. Keep your configuration up to date

## Getting Help

- Check the main README.md for detailed documentation
- Review examples/ directory for reference configurations
- Open an issue on GitHub for bugs or questions
- Check ARCHITECTURE.md for how mimic works internally
