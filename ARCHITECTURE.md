# Architecture

This document explains the design and implementation of mimic, a declarative dotfile management system.

## System Overview

mimic follows a declarative configuration model: users specify their desired state in a TOML configuration file, and mimic makes the system match that state.

```
┌─────────────┐
│ mimic.toml  │  User declares desired state
│ (Config)    │
└──────┬──────┘
       │
       v
┌──────────────────────────────────────────┐
│          CLI Commands                     │
│  apply │ diff │ status │ undo             │
└──────┬───────────────────────────────────┘
       │
       v
┌─────────────────────────────────────────┐
│         Core Engines                     │
│                                          │
│  ┌──────────┐  ┌──────────┐            │
│  │  Diff    │  │  State   │            │
│  │  Engine  │  │  Manager │            │
│  └──────────┘  └──────────┘            │
│                                          │
│  ┌──────────┐  ┌──────────┐            │
│  │  Linker  │  │ Template │            │
│  │  Engine  │  │  Engine  │            │
│  └──────────┘  └──────────┘            │
│                                          │
│  ┌──────────┐                           │
│  │Installer │                           │
│  │ Engine   │                           │
│  └──────────┘                           │
└─────────────────────────────────────────┘
       │
       v
┌──────────────────────────────────────────┐
│         System State                      │
│                                           │
│  ~/.zshrc -> dotfiles/zshrc              │
│  ~/.vimrc -> dotfiles/vimrc              │
│  brew packages installed                  │
└──────────────────────────────────────────┘
       │
       v
┌──────────────────┐
│  state.toml      │  Tracks what mimic manages
└──────────────────┘
```

## Module Descriptions

### Spinner (`src/spinner.rs`)

**Purpose:** Visual feedback for long-running operations with automatic CI detection.

**Responsibilities:**
- Display ASCII spinners during operations
- Show operation timing on completion
- Detect CI environments and disable spinners automatically
- Support concurrent operations via MultiProgress
- Provide consistent visual feedback across all commands

**Key types:**
```rust
pub struct Spinner {
    pb: Option<ProgressBar>,
    start_time: Instant,
}

pub struct SpinnerManager {
    multi: MultiProgress,
}
```

**Key functions:**
```rust
impl Spinner {
    pub fn new(message: String) -> Self
    pub fn set_message(&self, message: String)
    pub fn finish_with_message(&self, message: String)
    pub fn finish_with_error(&self, message: String)
    pub fn finish_and_clear(&self)
}

impl SpinnerManager {
    pub fn new() -> Self
    pub fn add_spinner(&self, message: String) -> Spinner
}
```

**Design decisions:**
- CI detection via `std::env::var("CI")` (standard CI environment variable)
- Optional `ProgressBar` pattern: `pb: Option<ProgressBar>` allows no-op in CI
- Timing tracked from construction via `start_time: Instant`
- Template: `{spinner:.green} {msg}` with 100ms tick interval
- MultiProgress enables concurrent operations without visual conflicts
- Auto-timing display: "took X.XXs" appended to completion messages

### Secrets Scanner (`src/secrets_scan.rs`)

**Purpose:** Detect secrets (API keys, tokens, credentials) in files before git operations.

**Responsibilities:**
- Scan files for common secret patterns (GitHub PAT, AWS keys, JWT tokens, etc.)
- Use ripsecrets library for pattern matching
- Support `.secretsignore` file format
- Return structured results with file paths, line numbers, and matched patterns
- Local-only processing (no data sent off machine)

**Key types:**
```rust
pub struct SecretMatch {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub pattern: String,
    pub matched_text: String,
}
```

**Key functions:**
```rust
pub fn scan_for_secrets(paths: &[PathBuf]) -> Result<Vec<SecretMatch>>
```

**Design decisions:**
- Uses `ripsecrets` crate (built-in patterns for common secret formats)
- Strict ignore mode via `.secretsignore` file
- Returns structured results for UI display (not raw text)
- Separate from `secrets.rs` (keychain storage) for clear separation of concerns
- All scanning is local (privacy-focused design)

### Config (`src/config.rs`)

**Purpose:** Parse and validate TOML configuration files.

**Responsibilities:**
- Load TOML files from disk
- Deserialize into strongly-typed Rust structs
- Validate configuration structure
- Provide access to variables, dotfiles, and packages

**Key types:**
```rust
pub struct Config {
    pub variables: HashMap<String, String>,
    pub dotfiles: Vec<Dotfile>,
    pub packages: Packages,
}

pub struct Dotfile {
    pub source: String,
    pub target: String,
}

pub struct Packages {
    pub homebrew: Vec<HomebrewPackage>,
}
```

### State (`src/state.rs`)

**Purpose:** Track what mimic has applied to the system.

**Responsibilities:**
- Persist state to disk as TOML
- Atomic writes to prevent corruption
- Track symlinks created (with backup paths for undo)
- Track packages installed
- Timestamp last apply operation

**Key types:**
```rust
pub struct State {
    pub dotfiles: Vec<DotfileState>,
    pub packages: Vec<PackageState>,
    pub applied_at: DateTime<Utc>,
}

pub struct DotfileState {
    pub source: String,
    pub target: String,
    pub backup_path: Option<String>,
}
```

**Design decisions:**
- Uses atomic writes (write to `.tmp`, then rename) to prevent corruption
- Stores absolute paths after expansion (not raw `~` paths)
- `backup_path` enables undo to restore original files

### Template Engine (`src/template.rs`)

**Purpose:** Substitute variables in configuration strings.

**Responsibilities:**
- Integrate Handlebars template engine
- Provide system variables (hostname, username, os, arch)
- Merge user-defined variables from config
- Strict mode: error on undefined variables

**Key functions:**
```rust
pub fn render_template(template: &str, config: &Config) -> Result<String>
```

**System variables:**
- `{{ hostname }}` - via `whoami::hostname()`
- `{{ username }}` - via `whoami::username()`
- `{{ os }}` - via `std::env::consts::OS`
- `{{ arch }}` - via `std::env::consts::ARCH`

**Design decisions:**
- Strict mode prevents silent failures from typos
- User variables override system variables
- Supports template file contents with Handlebars (`.tmpl` and `.hbs` files)

### Linker Engine (`src/linker.rs`)

**Purpose:** Create symlinks with conflict resolution.

**Responsibilities:**
- Expand paths (tilde, environment variables)
- Create Unix symlinks
- Detect conflicts (target already exists)
- Interactive conflict resolution (skip, overwrite, backup)
- Apply-to-all strategy for batch operations
- Update state with created symlinks

**Key functions:**
```rust
pub fn create_symlink(
    source: &Path,
    target: &Path,
    state: &mut State,
) -> Result<()>

pub fn create_symlink_with_resolution(
    source: &Path,
    target: &Path,
    state: &mut State,
    apply_to_all: &mut Option<ApplyToAllChoice>,
) -> Result<()>
```

**Conflict resolution flow:**
1. Check if target exists
2. If apply-to-all is set, use stored choice
3. Otherwise, prompt user: [s]kip, [o]verwrite, [b]ackup, [a]pply to all
4. Execute chosen action
5. Create symlink
6. Update state

**Backup naming:** `{filename}.backup.{YYYYMMDD_HHMMSS}`

### Installer Engine (`src/installer.rs`)

**Purpose:** Manage package installation via Homebrew.

**Responsibilities:**
- Detect if Homebrew is installed
- List installed packages
- Install packages idempotently
- Update state with installed packages

**Key functions:**
```rust
impl HomebrewManager {
    pub fn new() -> Self
    pub fn list_installed() -> Result<Vec<String>>
    pub fn is_installed(name: &str) -> Result<bool>
    pub fn install(
        name: &str,
        package_type: &str,
        state: &mut State,
    ) -> Result<()>
}
```

**Design decisions:**
- Install-only, never uninstall (safe by design)
- Idempotent: checks if already installed before running `brew install`
- If package already installed but not in state, adds to state without reinstalling
- Uses `std::process::Command` to shell out to `brew`

### Diff Engine (`src/diff.rs`)

**Purpose:** Compare desired state (config) against actual system state.

**Responsibilities:**
- Check if dotfile symlinks exist and point to correct sources
- Check if packages are installed
- Classify changes: Add, Modify, AlreadyCorrect
- Format changes with colored output

**Key functions:**
```rust
pub fn diff(&self, config: &Config) -> Result<Vec<Change>>

pub enum Change {
    Add { resource_type: ResourceType, description: String },
    Modify { resource_type: ResourceType, description: String, reason: String },
    AlreadyCorrect { description: String },
}
```

**Algorithm:**
- For dotfiles: check symlink existence, read target, canonicalize paths, compare
- For packages: call `HomebrewManager::is_installed()`
- Returns all changes (including AlreadyCorrect for comprehensive view)

**Design decisions:**
- Additive-only diff (no Remove variant in MVP)
- Canonical path comparison handles relative symlinks correctly
- Pretty formatting with `colored` crate for terminal output

### CLI (`src/cli.rs`)

**Purpose:** Command-line interface and orchestration.

**Responsibilities:**
- Parse command-line arguments with Clap
- Implement command workflows (apply, diff, status, undo)
- Error handling and user feedback
- Progress reporting with colored output

**Commands:**

#### `apply`
1. Load config
2. Run diff engine
3. Show preview
4. Prompt for confirmation (unless `--yes` or `--dry-run`)
5. Apply dotfiles with conflict resolution
6. Install packages
7. Save state

#### `diff`
1. Load config
2. Run diff engine
3. Print changes with colored output
4. Show summary (X to add, Y to modify)

#### `status`
1. Load state file
2. Check each dotfile symlink
3. Check each package installation
4. Report drift (missing, wrong target, etc.)
5. Exit with code 1 if drift detected

#### `undo`
1. Load state file
2. Remove all symlinks
3. Restore backups if they exist
4. Clear state file
5. Report summary

**Global flags:**
- `--config <PATH>` - Config file location
- `--state <PATH>` - State file location
- `--yes, -y` - Skip prompts, auto-backup
- `--dry-run, -n` - Preview only
- `--verbose, -v` - Detailed output

### Error Handling (`src/error.rs`)

**Purpose:** Strongly-typed error handling with user-friendly output.

**Design:**
- Uses `thiserror` for custom error types
- Uses `anyhow` for CLI-level error context
- Module-specific error types (LinkError, InstallError, etc.)
- Errors propagate up to CLI layer for user-friendly messages

**Error Display Function:**
```rust
pub fn display_error(error: &anyhow::Error) {
    eprintln!("{} {}", "Error:".red().bold(), error);
    let mut current = error.source();
    if current.is_some() {
        eprintln!();
        eprintln!("{}", "Caused by:".bright_black());
    }
    while let Some(source) = current {
        eprintln!("  {} {}", "→".bright_black(), source);
        current = source.source();
    }
}
```

**Error Message Pattern:**
- Main error: What went wrong
- "Caused by:" chain: Technical details showing full error source chain
- "To fix:" sections: Actionable steps for common issues
- BrokenPipe handling: Silent exit(0) when piping to `head` or closed pager

## Data Flow: Apply Command

This shows how data flows through the system during `mimic apply`:

```
1. User runs: mimic apply

2. CLI parses args, finds config file
   └─> Config::from_file("mimic.toml")
       └─> Parse TOML, validate structure

3. DiffEngine::diff(&config)
   ├─> For each dotfile:
   │   └─> Check symlink state
   ├─> For each package:
   │   └─> Call HomebrewManager::is_installed()
   └─> Return Vec<Change>

4. CLI displays changes, prompts user

5. For each dotfile:
   └─> create_symlink_with_resolution()
       ├─> shellexpand::full() on paths
       ├─> Check target exists?
       ├─> If conflict:
       │   └─> resolve_conflict() (interactive prompt)
       ├─> Create symlink
       └─> state.add_dotfile()

6. For each package:
   └─> HomebrewManager::install()
       ├─> is_installed() check
       ├─> If not installed:
       │   └─> Command::new("brew").arg("install")...
       └─> state.add_package()

7. State::save(&state_path)
   ├─> Serialize to TOML
   ├─> Write to state.toml.tmp
   ├─> sync_all()
   └─> rename to state.toml (atomic)

8. CLI prints success message
```

## Key Design Decisions

### 1. Declarative Model

**Decision:** Users declare desired state, mimic handles transitions.

**Rationale:**
- Simpler mental model than imperative commands
- Reproducible: same config always produces same result
- Self-documenting: config file is source of truth

### 2. Additive-Only Package Management

**Decision:** mimic installs declared packages but never uninstalls.

**Rationale:**
- Uninstalling is risky (may break system or other apps)
- Explicit > implicit: user should manually remove packages
- State file tracks "mimic installed this" without claiming ownership

### 3. Atomic State Writes

**Decision:** Write state to `.tmp` file, then rename.

**Rationale:**
- Prevents corruption if process crashes during write
- Filesystem rename is atomic on Unix
- State file is critical for undo operations

### 4. Backup on Conflict

**Decision:** Default conflict resolution creates timestamped backups.

**Rationale:**
- Never lose data
- Timestamped names allow multiple backups without collision
- Enables undo to restore original state

### 5. Symlinks Over Copies

**Decision:** Create symlinks to dotfiles, don't copy content.

**Rationale:**
- Changes to dotfile repo immediately reflected in home directory
- Git tracks dotfile changes (can't track copied files)
- Standard approach used by chezmoi, stow, yadm, etc.

### 6. State-Based Drift Detection

**Decision:** `status` command compares state file against system, not config.

**Rationale:**
- State represents last applied configuration
- Config may have been edited since last apply
- Drift detection answers: "Did system diverge from last apply?"

### 7. Interactive Conflict Resolution

**Decision:** Prompt user for each conflict, offer apply-to-all.

**Rationale:**
- Safe by default: user reviews each conflict
- Apply-to-all for efficiency when user trusts operation
- `--yes` flag for automation (always backups)

## Testing Strategy

### Unit Tests
- Each module has `module_test.rs` in `tests/`
- Test individual functions in isolation
- Use `tempfile::TempDir` for filesystem tests
- Mock external commands where needed

### Integration Tests
- `tests/integration/end_to_end_test.rs`
- Test full workflows: first-time setup, conflicts, drift, undo
- Use `assert_cmd` to run compiled binary
- Verify symlinks, state files, backups created correctly

### Test Coverage
- 89+ tests across all modules
- TDD approach: write tests first (RED), implement (GREEN)
- All tests pass before merge

## Dependencies

### Core Dependencies
- `serde` + `toml` - Configuration parsing
- `anyhow` + `thiserror` - Error handling
- `clap` - CLI argument parsing
- `handlebars` - Template engine
- `chrono` - Timestamps
- `shellexpand` - Path expansion
- `dialoguer` - Interactive prompts
- `colored` - Terminal colors
- `directories` - XDG base directory
- `whoami` - System information

### Dev Dependencies
- `tempfile` - Temporary directories for tests
- `assert_cmd` - CLI integration tests
- `predicates` - Assertion helpers
- `regex` - Pattern matching in tests
- `glob` - File pattern matching

## Performance Characteristics

- **Config parsing:** O(n) where n = lines in config file
- **Diff computation:** O(d + p) where d = dotfiles, p = packages
- **Apply operation:** O(d + p), sequential (safe operations)
- **State I/O:** Small files, typically < 10KB

Bottleneck: Homebrew package checks (external command execution).

## Security Considerations

- Path traversal: `shellexpand` handles `~` safely
- Symlink attacks: Validates source exists before creating symlink
- State file: Stored in `~/.config/mimic/` (user-only access)
- Backups: Created in same directory as target (preserves permissions)
- No elevated privileges required (except for Homebrew, which handles its own sudo)
