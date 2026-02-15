# Complete Example - Full Feature Showcase

Comprehensive demonstration of ALL mimic features in a single, real-world configuration. This is the kitchen sink example showing everything mimic can do.

## What This Example Shows

- **✓ Templates** - Variable substitution with Handlebars
- **✓ Secrets** - Secure credential management via macOS Keychain
- **✓ Multi-host** - Different configs for personal vs work machines
- **✓ Packages** - Homebrew formula and cask installation
- **✓ Hooks** - Post-install automation (rustup, mise, cargo-install, pnpm-global, uv-python, command)
- **✓ Role-based filtering** - Apply dotfiles/packages based on machine roles
- **✓ Mise integration** - Declarative version management
- **✓ Mixed format packages** - Both simple and verbose package declarations

## File Structure

```
complete/
├── mimic.toml              # Full-featured configuration
├── README.md               # This file
├── templates/
│   ├── gitconfig.tmpl      # Template using variables and GitHub username
│   ├── zshrc.tmpl          # Shell config with template variables
│   └── npmrc.tmpl          # NPM config with secrets
└── dotfiles/
    ├── tmux.conf           # Static dotfile (no templating)
    ├── dev-aliases         # Development-specific aliases
    └── work-aliases        # Work-specific aliases
```

## Feature Breakdown

### 1. Variables

User-defined values for template rendering:

```toml
[variables]
name = "Alex Developer"
email = "alex@example.com"
editor = "nvim"
github_username = "alexdev"
```

Used in templates:
- `{{ name }}` - Full name for Git
- `{{ email }}` - Email address
- `{{ editor }}` - Preferred editor
- `{{ github_username }}` - GitHub URL rewriting

### 2. Secrets

Secure credential management without committing sensitive data:

```toml
[secrets]
github_token = { description = "GitHub PAT for API access", env_var = "GITHUB_TOKEN" }
openai_api_key = { description = "OpenAI API key", env_var = "OPENAI_API_KEY" }
```

**Usage in templates:**
```
//registry.npmjs.org/:_authToken={{ secrets.github_token }}
```

**Managing secrets:**
```bash
mimic secrets set github_token
mimic secrets set openai_api_key
mimic secrets list
mimic secrets export
```

Secrets are stored in macOS Keychain (encrypted, secure).

### 3. Templates

Three template files demonstrating variable and secret substitution:

**gitconfig.tmpl** - Personal Git configuration:
```toml
[user]
    name = {{ name }}
    email = {{ email }}

[github]
    user = {{ github_username }}

[url "git@github.com:{{ github_username }}/"]
    insteadOf = https://github.com/{{ github_username }}/
```

**zshrc.tmpl** - Shell with environment variables:
```bash
export EDITOR="{{ editor }}"
export GIT_AUTHOR_NAME="{{ name }}"
export GIT_AUTHOR_EMAIL="{{ email }}"
export GITHUB_USER="{{ github_username }}"
```

**npmrc.tmpl** - NPM config with secrets:
```
//registry.npmjs.org/:_authToken={{ secrets.github_token }}
save-exact=true
```

### 4. Packages

Modern development tools:

**CLI tools:**
- git, neovim, tmux
- ripgrep, fd, bat, exa
- delta (better git diffs)
- fzf (fuzzy finder)

**GUI apps:**
- visual-studio-code

**Host-specific:**
- docker (dev laptop only)
- slack, zoom (work machine only)

### 5. Hooks

Post-installation automation in order:

#### rustup Hook
Install Rust toolchains and components:
```toml
[[hooks]]
type = "rustup"
toolchains = ["stable", "nightly"]
components = ["rustfmt", "clippy", "rust-analyzer"]
default = "stable"
```

Installs:
- stable and nightly Rust
- rustfmt (code formatter)
- clippy (linter)
- rust-analyzer (LSP)

#### mise Hook
Activate mise for version management:
```toml
[[hooks]]
type = "mise"
```

Reads `[mise]` section and installs tools:
```toml
[mise]
tools = { node = "20", python = "3.12", go = "1.21" }
```

#### cargo-install Hook
Install Rust binaries from git:
```toml
[[hooks]]
type = "cargo-install"
packages = [
    { name = "tap", git = "https://github.com/crazywolf132/tap.git" },
]
```

#### pnpm-global Hook
Install global Node.js packages:
```toml
[[hooks]]
type = "pnpm-global"
packages = ["typescript", "@biomejs/biome", "prettier"]
```

#### uv-python Hook (dev laptop only)
Install Python via uv and create symlinks:
```toml
[[hooks]]
type = "uv-python"
version = "3.12"
symlinks = { python3 = "~/.local/bin/python3" }
```

Only runs on dev-laptop (role filtering).

#### command Hook
Custom setup script:
```toml
[[hooks]]
type = "command"
name = "setup-complete"
command = "echo 'Development environment fully configured!'"
on_failure = "continue"
```

### 6. Multi-Host Configuration

Two hosts with different personalities:

#### dev-laptop (Personal)
**Roles:** personal, mac, desktop

**Customizations:**
- Personal email
- Docker installed
- Python via uv-python
- Development aliases

#### work-machine (Corporate)
**Roles:** work, mac, desktop

**Customizations:**
- Corporate email
- HTTP proxy settings
- Work-specific secrets (JIRA, AWS)
- Slack and Zoom
- Work aliases (VPN, deployments)

### 7. Role-Based Filtering

Resources can be filtered by host roles:

```toml
[[hooks]]
type = "uv-python"
version = "3.12"
only_roles = ["personal"]
```

Only installs on hosts with "personal" role.

## Usage

### Initial Setup

1. **Clone or create this config:**
   ```bash
   cd examples/complete
   ```

2. **Set up secrets:**
   ```bash
   mimic secrets set github_token
   mimic secrets set openai_api_key
   ```

3. **Preview for a host:**
   ```bash
   mimic diff --host dev-laptop
   mimic diff --host work-machine
   ```

4. **Apply:**
   ```bash
   mimic apply --host dev-laptop
   ```

### What Happens During Apply

1. **Templates rendered** with variables and secrets
2. **Dotfiles symlinked** to home directory
3. **Packages installed** via Homebrew
4. **Hooks executed** in order:
   - Rust toolchains installed
   - mise tools installed (Node, Python, Go)
   - Cargo packages compiled from git
   - pnpm global packages installed
   - uv-python installed (if on dev-laptop)
   - Custom setup command runs

### Commands

**List hosts:**
```bash
mimic hosts list
```

**Show merged config:**
```bash
mimic hosts show dev-laptop
```

**Manage secrets:**
```bash
mimic secrets set github_token
mimic secrets get github_token
mimic secrets list
mimic secrets export
```

**Render a template:**
```bash
mimic render templates/gitconfig.tmpl
```

**Check status:**
```bash
mimic status
```

**Undo:**
```bash
mimic undo
```

## Real-World Workflow

### Day 1: Personal Laptop Setup

```bash
cd ~/dotfiles
mimic secrets set github_token
mimic secrets set openai_api_key
mimic apply --host dev-laptop
```

Result:
- Git configured with personal email
- Rust, Node, Python, Go installed
- VS Code, Docker installed
- Custom dev aliases available

### Day 2: Work Machine Setup

```bash
cd ~/dotfiles
mimic secrets set github_token
mimic secrets set jira_token
mimic secrets set aws_access_key
mimic apply --host work-machine
```

Result:
- Git configured with work email
- HTTP proxy configured
- Slack and Zoom installed
- Work aliases for VPN and deployments
- Same dev tools as personal laptop

### Day 30: Add New Tool

Edit `mimic.toml`:
```toml
[packages]
brew = [..., "lazygit"]
```

Apply changes:
```bash
mimic apply
```

Only installs the new package, everything else stays the same.

## Customization Guide

### Add New Variable

```toml
[variables]
timezone = "America/New_York"
```

Use in template:
```bash
export TZ="{{ timezone }}"
```

### Add New Secret

```toml
[secrets]
api_key = { description = "Service API key", env_var = "SERVICE_API_KEY" }
```

Set value:
```bash
mimic secrets set api_key
```

Use in template:
```
API_KEY={{ secrets.api_key }}
```

### Add New Hook

```toml
[[hooks]]
type = "command"
name = "install-custom-tool"
command = "curl -sSL https://tool.com/install.sh | sh"
on_failure = "fail"
```

### Add New Host

```toml
[hosts.gaming-pc]
roles = ["personal", "linux", "desktop"]

[hosts.gaming-pc.variables]
email = "alex@personal.com"

[[hosts.gaming-pc.packages.homebrew]]
name = "steam"
type = "cask"
```

## Learning Path

1. **Start with basic/** - Learn core dotfile symlinking
2. **Try templates/** - Understand variable substitution
3. **Explore organized/** - See scalable file organization
4. **Study multi-host/** - Manage multiple machines
5. **Master complete/** (this example) - Combine everything

## Summary

This example combines:
- ✓ Templates with variables
- ✓ Secrets management
- ✓ Multi-host with role filtering
- ✓ Comprehensive package management
- ✓ All 6 hook types
- ✓ Mise integration
- ✓ Real-world dotfiles

It represents a production-ready dotfile configuration that a professional developer might actually use across multiple machines.
