# Comparison with Other Dotfile Managers

This guide helps you understand how mimic compares to other popular dotfile management tools and why you might choose one over another.

## Quick Comparison Table

| Feature | mimic | GNU Stow | Chezmoi | Nix/Home Manager | YADM |
|---------|-------|----------|---------|------------------|------|
| **Complexity** | Low | Low | Medium | High | Low |
| **Learning Curve** | Gentle | Minimal | Moderate | Steep | Gentle |
| **Package Management** | Yes (Homebrew) | No | No | Yes (Nix) | No |
| **Templating** | Planned | No | Yes | Yes | Yes |
| **Multi-machine** | Yes (hosts) | Manual | Yes | Yes | Yes |
| **State Tracking** | Yes | No | Yes | Yes | No |
| **Secrets Management** | Detection only | No | Yes | Yes | Yes |
| **Language** | Rust | Perl | Go | Nix | Bash |
| **Cross-platform** | macOS/Linux | Unix-like | All | Nix systems | Unix-like |

## Detailed Comparisons

### mimic vs GNU Stow

**GNU Stow** is a symlink farm manager that's been around since the 1990s.

**Similarities:**
- Both use symlinks to manage dotfiles
- Both are relatively simple conceptually
- Both work well with git

**mimic Advantages:**
- Package management integration (Homebrew)
- State file tracks what's managed (enables undo, drift detection)
- Interactive conflict resolution with automatic backups
- Built-in `status` command for drift detection
- Better error messages with actionable guidance
- Progress feedback with timing

**Stow Advantages:**
- Extremely mature and stable
- Can manage any symlink farm (not just dotfiles)
- Simpler mental model (just directories)
- No configuration file needed

**When to use Stow:**
- You only need symlink management (no packages)
- You prefer minimal tooling
- You already have a Stow setup that works

**When to use mimic:**
- You want integrated package management
- You want drift detection and state tracking
- You need conflict resolution
- You want better feedback and error messages

**Migration from Stow:**

Stow structure:
```
~/dotfiles/
├── vim/.vimrc
├── zsh/.zshrc
└── git/.gitconfig
```

Convert to mimic:
```bash
# Flatten structure
mkdir -p ~/dotfiles/dotfiles
mv vim/.vimrc dotfiles/vimrc
mv zsh/.zshrc dotfiles/zshrc
mv git/.gitconfig dotfiles/gitconfig

# Create mimic.toml
cat > mimic.toml << 'EOF'
[[dotfiles]]
source = "dotfiles/vimrc"
target = "~/.vimrc"

[[dotfiles]]
source = "dotfiles/zshrc"
target = "~/.zshrc"

[[dotfiles]]
source = "dotfiles/gitconfig"
target = "~/.gitconfig"
EOF
```

---

### mimic vs Chezmoi

**Chezmoi** is a popular, feature-rich dotfile manager written in Go.

**Similarities:**
- Both track state
- Both detect drift
- Both support multi-machine configurations
- Both have good error messages

**mimic Advantages:**
- Simpler configuration (TOML vs Chezmoi's special syntax)
- No file renaming (`dot_vimrc` → just `vimrc`)
- Source files stay readable and editable
- Integrated package management
- Secrets detection (warns before commits)
- Faster (Rust vs Go)
- Cleaner git diffs (no renamed files)

**Chezmoi Advantages:**
- More mature (established 2018)
- Full templating support (Go templates)
- Built-in secret management (integration with password managers)
- More package managers (apt, yum, etc.)
- File attributes management
- Script execution hooks
- Larger community

**When to use Chezmoi:**
- You need mature templating now
- You need cross-platform package management
- You want integration with password managers
- You need to manage file permissions/attributes

**When to use mimic:**
- You want simpler, more transparent configuration
- You prefer files stay readable in the repo
- You want cleaner git history
- You primarily use Homebrew
- You value simplicity over features

**Migration from Chezmoi:**

Chezmoi structure:
```
~/.local/share/chezmoi/
├── dot_vimrc
├── dot_zshrc
└── dot_gitconfig.tmpl
```

Convert to mimic:
```bash
# Copy and rename
mkdir -p ~/dotfiles/dotfiles
cp ~/.local/share/chezmoi/dot_vimrc ~/dotfiles/dotfiles/vimrc
cp ~/.local/share/chezmoi/dot_zshrc ~/dotfiles/dotfiles/zshrc

# For templates, manually resolve or wait for mimic template support
cp ~/.local/share/chezmoi/dot_gitconfig.tmpl ~/dotfiles/dotfiles/gitconfig
# Edit gitconfig to replace {{ .email }} with actual value

# Create mimic.toml
cat > mimic.toml << 'EOF'
[variables]
email = "your@email.com"

[[dotfiles]]
source = "dotfiles/vimrc"
target = "~/.vimrc"

[[dotfiles]]
source = "dotfiles/zshrc"
target = "~/.zshrc"

[[dotfiles]]
source = "dotfiles/gitconfig"
target = "~/.gitconfig"
EOF
```

---

### mimic vs Nix/Home Manager

**Nix/Home Manager** is a declarative system configuration tool.

**Similarities:**
- Both are declarative
- Both manage packages and configuration
- Both support multiple machines
- Both track state

**mimic Advantages:**
- Much simpler to learn (hours vs weeks)
- No new language to learn (TOML vs Nix language)
- Works with existing package managers (Homebrew)
- Doesn't take over your entire system
- Can edit files directly (no read-only filesystem)
- Faster iteration (no rebuild step)
- Works on any macOS/Linux system

**Nix Advantages:**
- Complete system reproducibility
- Atomic rollbacks
- Multiple versions of packages simultaneously
- Works across all systems (macOS, Linux, NixOS)
- Declarative package builds
- Isolated environments
- More mature ecosystem

**When to use Nix:**
- You want complete system reproducibility
- You need atomic rollbacks
- You're willing to invest time learning
- You want declarative package building
- You're using NixOS or multiple Linux distros

**When to use mimic:**
- You want something simple that works now
- You just need dotfile and package management
- You're happy with Homebrew
- You don't want to learn a new language
- You want to iterate quickly

**Migration from Nix:**

Nix is fundamentally different - it manages much more than dotfiles. If you're moving away from Nix, you'll need to:

1. Extract your dotfile configurations from Nix expressions
2. Manually manage packages that Nix was handling
3. Lose some reproducibility guarantees

This is typically not recommended unless you're simplifying your setup intentionally.

---

### mimic vs YADM

**YADM** (Yet Another Dotfiles Manager) uses git directly to manage dotfiles.

**Similarities:**
- Both are relatively simple
- Both work well with git
- Both support templates and multi-machine

**mimic Advantages:**
- Explicit configuration file (mimic.toml)
- Package management integration
- State tracking and drift detection
- Conflict resolution
- Files live in a repository directory (cleaner separation)
- Undo functionality
- Better error messages

**YADM Advantages:**
- No separate repository needed (uses `$HOME` as git repo)
- Built-in encryption for sensitive files
- Bootstrap scripts
- More mature (established 2015)

**When to use YADM:**
- You want to use `$HOME` directly as a git repo
- You need built-in encryption
- You like managing dotfiles with git commands directly

**When to use mimic:**
- You want clear separation between repo and home
- You want integrated package management
- You want drift detection
- You prefer declarative configuration

---

## Philosophy Comparison

### mimic Philosophy

**Transparency over magic**
- Explicit configuration (mimic.toml)
- Clear symlinks (visible with `ls -la`)
- Source files remain readable in repo

**Safety first**
- Automatic backups
- Dry-run mode
- Interactive conflict resolution
- Undo functionality

**Simplicity and practicality**
- TOML configuration (easy to read/write)
- Works with existing tools (Homebrew, git)
- Fast learning curve

### When mimic is the Right Choice

Choose mimic if you:
- Want something simple that works now
- Value transparency and clarity
- Need package management integrated
- Want good error messages and feedback
- Are primarily on macOS with Homebrew
- Prefer explicit over implicit
- Don't need advanced templating yet
- Want to learn the tool in < 1 hour

### When to Choose Something Else

**Choose Nix/Home Manager if:**
- You need complete system reproducibility
- You want atomic rollbacks
- You're willing to invest weeks learning
- You need this across many Linux distros

**Choose Chezmoi if:**
- You need mature templating now
- You need cross-platform package management (apt, yum, etc.)
- You want password manager integration
- File permissions management is important

**Choose GNU Stow if:**
- You only need symlink management
- You want the absolute simplest tool
- You're managing more than just dotfiles

**Choose YADM if:**
- You want to use `$HOME` as your git repo
- You need built-in encryption
- You like managing dotfiles with git directly

---

## Feature Roadmap

mimic is actively developed. Planned features include:

**Phase 2 (Planned):**
- Template file generation (Handlebars)
- Additional package managers (apt, dnf, pacman)
- Pre/post hooks for custom scripts
- Git integration helpers

**Phase 3 (Future):**
- Secrets management (not just detection)
- File permission management
- Script execution
- Plugin system

---

## Migration Decision Matrix

Use this matrix to decide if migrating to mimic makes sense:

| Current Tool | Complexity of Migration | Recommended? | Notes |
|--------------|------------------------|--------------|-------|
| No tool (scattered dotfiles) | Easy | ✅ Yes | Significant improvement |
| GNU Stow | Easy | ✅ Yes | If you want package management |
| Chezmoi | Medium | 🤔 Maybe | Loss of some features, gain simplicity |
| Nix/Home Manager | Hard | ❌ No | Unless simplifying intentionally |
| YADM | Medium | 🤔 Maybe | If you want better structure |
| Custom scripts | Easy-Medium | ✅ Yes | More maintainable than scripts |

---

## Conclusion

**mimic is designed for pragmatic developers who want:**
- Simple, transparent dotfile management
- Integrated package installation
- Good error messages and feedback
- Fast learning curve
- Safety and recoverability

**It's not designed for:**
- Complete system reproducibility (use Nix)
- Complex cross-platform setups (use Chezmoi)
- Maximum simplicity (use Stow)

Choose the tool that matches your needs and complexity tolerance. There's no "best" tool - only the best tool for your situation.
