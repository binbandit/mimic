# Multi-Host Configuration Example

This example demonstrates how to manage multiple machines (personal laptop, work laptop, home server, etc.) from a single `mimic.toml` configuration file.

## How It Works

The configuration has:
- **Base configuration**: Variables, dotfiles, and packages shared across all hosts
- **Host-specific overrides**: Each `[hosts.name]` section adds or overrides the base config

When you run `mimic apply`, it:
1. Auto-detects your hostname (or uses `--host` flag)
2. Merges base config + host-specific config
3. Applies the merged configuration

## Usage

### Auto-detect hostname
```bash
mimic apply
```

### Explicit host selection
```bash
mimic apply --host personal-macbook
mimic apply --host work-macbook
```

### List configured hosts
```bash
mimic hosts list
```

### Show merged config for a host
```bash
mimic hosts show personal-macbook
```

## Configuration Structure

### Base Config (Applied to All Hosts)
- Shared variables like `editor`, `shell`, `theme`
- Common dotfiles like `.gitconfig`, `.vimrc`
- Universal packages like `git`, `curl`

### Host-Specific Config
Each host can:
- Add new variables (e.g., `email`, `openai_model`)
- Override base variables (e.g., different `editor`)
- Add host-specific dotfiles (e.g., work vs personal `.zshrc`)
- Install additional packages (e.g., `spotify` on personal, `slack` on work)

## Roles

Hosts can have roles for documentation/organization:
- `personal-macbook`: `["personal", "mac", "portable"]`
- `work-macbook`: `["work", "mac", "portable"]`
- `home-desktop`: `["personal", "linux", "desktop", "powerful"]`
- `home-server`: `["server", "linux", "headless"]`

Roles are currently informational but could be used for future features like role-based filtering.

## Real-World Scenarios

### Scenario 1: Work and Personal Laptops
Both need:
- Git, common CLI tools
- Neovim, basic dotfiles

But differ in:
- Email addresses (personal vs corporate)
- Packages (Spotify vs Slack)
- SSH configs (different keys)

### Scenario 2: Development Machine and Server
Dev machine:
- Full IDE setup, GUI apps
- Rich terminal config

Server:
- Minimal packages (htop, tmux)
- Headless-optimized dotfiles
- Monitoring tools

## Migration from Single Config

If you have an existing `mimic.toml`, it continues to work. To add multi-host support:

1. Keep your base config as-is
2. Add `[hosts.hostname]` sections
3. Move host-specific items into their sections
4. Run `mimic apply` (auto-detects hostname)
