# Multi-Host Example

Manage multiple machines from a single configuration. Perfect for developers with personal laptops, work machines, and servers that need different dotfiles and packages.

## What This Example Shows

- **Multi-machine management** - One config for all your machines
- **Host-specific overrides** - Different dotfiles/packages per machine
- **Role-based organization** - Group hosts by characteristics (personal, work, server)
- **Template variables per host** - Different email addresses, GitHub usernames, etc.
- **Shared base configuration** - Common dotfiles and packages across all hosts

## File Structure

```
multi-host/
├── mimic.toml          # Single config with base + per-host sections
├── README.md           # This file
└── dotfiles/
    ├── gitconfig.tmpl           # Shared, rendered with host variables
    ├── tmux.conf                # Shared across all hosts
    ├── personal-zshrc           # Personal laptop only
    ├── work-zshrc               # Work laptop only (proxy settings)
    ├── work-ssh-config          # Work SSH (corporate proxy)
    ├── server-bashrc            # Server only (minimal)
    └── server-ssh-config        # Server SSH (homelab)
```

## Configured Hosts

### personal-macbook
**Roles:** personal, mac, desktop

**Purpose:** Personal development laptop

**Customizations:**
- Personal email address
- Personal GitHub username
- Spotify and Discord installed
- Standard zsh configuration

### work-laptop
**Roles:** work, mac, desktop

**Purpose:** Corporate work laptop

**Customizations:**
- Corporate email address
- Work GitHub account
- HTTP proxy settings in shell
- Corporate SSH config with proxy
- Slack, Zoom, Docker installed
- VPN connection aliases

### home-server
**Roles:** server, linux, headless

**Purpose:** Home lab server (headless)

**Customizations:**
- Admin email
- Minimal bash configuration
- Server-optimized SSH config
- Auto-attach to tmux on login
- Server tools only (htop, ncdu, iotop)

## How Multi-Host Works

### Base Configuration (Applied to All)

Defined at the root level:
```toml
[variables]
email = "default@example.com"
name = "Default User"
editor = "nvim"

[[dotfiles]]
source = "dotfiles/gitconfig.tmpl"
target = "~/.gitconfig"
```

Every host gets these unless overridden.

### Host-Specific Overrides

Each `[hosts.name]` section:
- **Merges with base** - Adds to or overrides base config
- **Has its own variables** - Different email, name, etc.
- **Can add dotfiles** - Host-specific shell configs
- **Can add packages** - Spotify on personal, Slack on work

Example:
```toml
[hosts.work-laptop]
roles = ["work", "mac", "desktop"]

[hosts.work-laptop.variables]
email = "jane.developer@company.com"
http_proxy = "http://proxy.company.com:8080"

[[hosts.work-laptop.dotfiles]]
source = "dotfiles/work-zshrc"
target = "~/.zshrc"

[[hosts.work-laptop.packages.homebrew]]
name = "slack"
type = "cask"
```

### Template Rendering

Templates use host-specific variables:

**dotfiles/gitconfig.tmpl:**
```toml
[user]
    name = {{ name }}
    email = {{ email }}
```

**On personal-macbook renders to:**
```toml
[user]
    name = Jane Developer
    email = jane@personal.com
```

**On work-laptop renders to:**
```toml
[user]
    name = Jane Developer
    email = jane.developer@company.com
```

## Usage

### List available hosts

```bash
cd examples/multi-host
mimic hosts list
```

Output:
```
Available hosts:
- home-server
- personal-macbook
- work-laptop
```

### Show merged config for a host

See what a specific host will actually apply:

```bash
mimic hosts show personal-macbook
```

### Apply for specific host

```bash
mimic apply --host personal-macbook
mimic apply --host work-laptop
mimic apply --host home-server
```

### Auto-detect hostname

If your machine's hostname matches a configured host:

```bash
mimic apply
```

mimic will automatically select the matching host configuration.

## Real-World Scenarios

### Scenario 1: Work and Personal Email

**Problem:** You need different Git email addresses on work vs personal machines.

**Solution:**
```toml
[variables]
email = "default@example.com"

[[dotfiles]]
source = "dotfiles/gitconfig.tmpl"
target = "~/.gitconfig"

[hosts.work]
[hosts.work.variables]
email = "you@company.com"

[hosts.personal]
[hosts.personal.variables]
email = "you@personal.com"
```

The template `{{ email }}` renders differently per host.

### Scenario 2: Corporate Proxy

**Problem:** Work laptop needs proxy settings, personal laptop doesn't.

**Solution:**
```toml
[hosts.work-laptop]
[hosts.work-laptop.variables]
http_proxy = "http://proxy.company.com:8080"

[[hosts.work-laptop.dotfiles]]
source = "dotfiles/work-zshrc"
target = "~/.zshrc"
```

**work-zshrc includes:**
```bash
export HTTP_PROXY="http://proxy.company.com:8080"
export HTTPS_PROXY="$HTTP_PROXY"
```

### Scenario 3: Different Apps Per Machine

**Problem:** Spotify on personal, Slack on work, minimal tools on server.

**Solution:**
```toml
[[hosts.personal-macbook.packages.homebrew]]
name = "spotify"
type = "cask"

[[hosts.work-laptop.packages.homebrew]]
name = "slack"
type = "cask"

[hosts.home-server.packages]
brew = ["htop", "ncdu"]
```

### Scenario 4: SSH Key Management

**Problem:** Different SSH keys for personal, work, and server.

**Solution:**
```toml
[[hosts.personal-macbook.dotfiles]]
source = "dotfiles/personal-ssh-config"
target = "~/.ssh/config"

[[hosts.work-laptop.dotfiles]]
source = "dotfiles/work-ssh-config"
target = "~/.ssh/config"

[[hosts.home-server.dotfiles]]
source = "dotfiles/server-ssh-config"
target = "~/.ssh/config"
```

Each SSH config references different identity files.

## Roles

Roles organize hosts by shared characteristics:

```toml
[hosts.personal-macbook]
roles = ["personal", "mac", "desktop"]

[hosts.work-laptop]
roles = ["work", "mac", "desktop"]

[hosts.home-server]
roles = ["server", "linux", "headless"]
```

### Use Cases for Roles

1. **Platform-specific packages**
   ```toml
   [[packages.homebrew]]
   name = "some-mac-app"
   only_roles = ["mac"]
   ```

2. **GUI vs headless**
   ```toml
   [[packages.homebrew]]
   name = "visual-studio-code"
   skip_roles = ["headless"]
   ```

3. **Work vs personal tools**
   ```toml
   [[packages.homebrew]]
   name = "spotify"
   skip_roles = ["work"]
   ```

## Expanding to More Hosts

To add a new machine:

1. **Add host section:**
   ```toml
   [hosts.gaming-pc]
   roles = ["personal", "linux", "desktop"]
   
   [hosts.gaming-pc.variables]
   email = "you@personal.com"
   ```

2. **Add host-specific dotfiles if needed:**
   ```toml
   [[hosts.gaming-pc.dotfiles]]
   source = "dotfiles/gaming-bashrc"
   target = "~/.bashrc"
   ```

3. **Add host-specific packages:**
   ```toml
   [hosts.gaming-pc.packages]
   brew = ["steam", "discord"]
   ```

4. **Apply on that machine:**
   ```bash
   mimic apply --host gaming-pc
   ```

## Next Steps

After mastering multi-host configurations, explore:

- **examples/complete/** - See multi-host combined with hooks, secrets, and all features
