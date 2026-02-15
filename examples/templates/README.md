# Templates Example

Demonstrates template rendering with Handlebars syntax and variable substitution. Learn how to create dynamic dotfiles that adapt to different values.

## What This Example Shows

- **Template files** - Using `.tmpl` extension for Handlebars templating
- **Variable substitution** - Replacing `{{ variable }}` placeholders with actual values
- **Automatic template detection** - Files ending in `.tmpl` or `.hbs` are automatically templated
- **Explicit template flag** - Using `template = true` in dotfile declarations
- **Rendered examples** - See actual output in the `rendered/` directory

## File Structure

```
templates/
├── mimic.toml              # Configuration with variables and template dotfiles
├── README.md               # This file
├── dotfiles/
│   ├── gitconfig.tmpl      # Template using {{ name }}, {{ email }}, {{ github_username }}
│   ├── zshrc.tmpl          # Template using {{ editor }}, {{ email }}, {{ name }}
│   └── ssh-config.tmpl     # Static config (no variables, but shows .tmpl extension)
└── rendered/               # Example output after template rendering
    ├── gitconfig
    └── zshrc
```

## Template Syntax

mimic uses Handlebars templating. Basic variable substitution:

```handlebars
{{ variable_name }}
```

### Available Variables

**User-defined variables** (from `[variables]` section):
```toml
[variables]
name = "Jane Developer"
email = "jane@example.com"
editor = "nvim"
github_username = "janedeveloper"
```

**System variables** (automatically available):
- `{{ hostname }}` - System hostname
- `{{ username }}` - Current username
- `{{ os }}` - Operating system (macos, linux)
- `{{ arch }}` - CPU architecture (aarch64, x86_64)

## Template Examples

### gitconfig.tmpl

Personalizes Git configuration with your name, email, and GitHub username:

```toml
[user]
    name = {{ name }}
    email = {{ email }}

[github]
    user = {{ github_username }}

[url "git@github.com:{{ github_username }}/"]
    insteadOf = https://github.com/{{ github_username }}/
```

**Renders to:**
```toml
[user]
    name = Jane Developer
    email = jane@example.com

[github]
    user = janedeveloper

[url "git@github.com:janedeveloper/"]
    insteadOf = https://github.com/janedeveloper/
```

### zshrc.tmpl

Sets editor preference and Git environment variables:

```bash
export EDITOR="{{ editor }}"
export GIT_AUTHOR_EMAIL="{{ email }}"
export GIT_AUTHOR_NAME="{{ name }}"
export GITHUB_USER="{{ github_username }}"

alias vim='{{ editor }}'
```

**Renders to:**
```bash
export EDITOR="nvim"
export GIT_AUTHOR_EMAIL="jane@example.com"
export GIT_AUTHOR_NAME="Jane Developer"
export GITHUB_USER="janedeveloper"

alias vim='nvim'
```

## Template Detection

mimic automatically treats files as templates when:

1. **File extension** - Ends with `.tmpl` or `.hbs`
2. **Explicit flag** - `template = true` in dotfile declaration

```toml
[[dotfiles]]
source = "dotfiles/zshrc.tmpl"
target = "~/.zshrc"

[[dotfiles]]
source = "dotfiles/gitconfig.tmpl"
target = "~/.gitconfig"
template = true
```

## Usage

### Preview template rendering

See what the rendered output looks like:

```bash
cd examples/templates
mimic render dotfiles/gitconfig.tmpl
```

### Preview changes

See what symlinks would be created:

```bash
mimic diff
```

### Apply configuration

Render templates and create symlinks:

```bash
mimic apply
```

This will:
1. Render templates with variable substitution
2. Create symlinks:
   - `~/.gitconfig` → rendered gitconfig
   - `~/.zshrc` → rendered zshrc
   - `~/.ssh/config` → rendered ssh-config
3. Install missing packages

### Verify rendering

Check that templates rendered correctly:

```bash
cat ~/.gitconfig
cat ~/.zshrc
```

You should see your actual values, not `{{ variable }}` placeholders.

## Customization

To adapt this for your use:

1. **Edit variables** in `mimic.toml`:
   ```toml
   [variables]
   name = "Your Name"
   email = "your@email.com"
   editor = "vim"
   github_username = "yourusername"
   ```

2. **Add more variables** as needed:
   ```toml
   [variables]
   work_email = "you@company.com"
   timezone = "America/New_York"
   ```

3. **Use in templates**:
   ```
   export TZ="{{ timezone }}"
   ```

4. **Create new templates** - Any file can be templated:
   ```toml
   [[dotfiles]]
   source = "dotfiles/npmrc.tmpl"
   target = "~/.npmrc"
   ```

## Common Template Patterns

### Conditional sections

Use different configs for different machines:

```bash
{{#if (eq os "macos")}}
export PATH="/opt/homebrew/bin:$PATH"
{{/if}}

{{#if (eq os "linux")}}
export PATH="/home/linuxbrew/.linuxbrew/bin:$PATH"
{{/if}}
```

### Multiple email addresses

Work vs personal:

```toml
[variables]
personal_email = "me@personal.com"
work_email = "me@company.com"
```

```
[user]
    email = {{ work_email }}

[includeIf "gitdir:~/personal/"]
    path = ~/.gitconfig-personal

# In ~/.gitconfig-personal:
[user]
    email = {{ personal_email }}
```

## Next Steps

After exploring templates, check out:

- **examples/organized/** - Organize dotfiles by application
- **examples/multi-host/** - Different configs per machine using template variables
- **examples/complete/** - Templates combined with secrets and hooks
