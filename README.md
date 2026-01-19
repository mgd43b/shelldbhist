# sdbh

[![CI](https://github.com/mgd43b/shelldbhist/actions/workflows/ci.yml/badge.svg)](https://github.com/mgd43b/shelldbhist/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/mgd43b/shelldbhist)](https://github.com/mgd43b/shelldbhist/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/mgd43b/shelldbhist/latest/total)](https://github.com/mgd43b/shelldbhist/releases/latest)
[![License](https://img.shields.io/github/license/mgd43b/shelldbhist)](https://github.com/mgd43b/shelldbhist/blob/main/LICENSE)
[![codecov](https://codecov.io/gh/mgd43b/shelldbhist/branch/main/graph/badge.svg)](https://codecov.io/gh/mgd43b/shelldbhist)

Shell DB History (`sdbh`) stores your shell command history in a local SQLite database.
It's inspired by `dbhist.sh`, but implemented as a portable Rust CLI backed by SQLite.

![sdbh demo](demo.gif)

## Features
- 🔍 **Interactive fuzzy search** with `--fzf` flag for intelligent command selection
- 📊 **Rich preview panes** showing command statistics and usage patterns
- 🎯 **Multi-select support** with `--multi-select` flag for batch operations
- ⚙️ **Full configuration system** via `~/.sdbh.toml` for colors, layout, and key bindings
- 🎨 **Ctrl+R replacement** for shell history search (transformative UX improvement)
- 📱 **Responsive terminal design** adapting to different terminal widths (80-200+ chars)
- 🔧 **Command Templates System** for reusable command patterns with variable substitution
- 💻 **Professional UI/UX** with organized information hierarchy and smart truncation
- 🧹 **Garbage detection** (Phase 1) for identifying likely garbage entries with conservative heuristics
- Local SQLite history database (`~/.sdbh.sqlite` by default)
- Fast search (substring), raw history listing, grouped summaries
- Stats (top commands, by-directory, daily buckets)
- Database health monitoring and performance optimization
- Comprehensive test coverage with 87 integration tests
- Import/merge from existing `dbhist.sh` SQLite databases
- Import from shell history files (`.bash_history`, `.zsh_history`)

## Install

### Option 1 (recommended): Homebrew

```bash
brew tap mgd43b/taps
brew install sdbh
```

### Option 2: Download a prebuilt binary
Download the right binary for your OS from the **latest GitHub Release**:

https://github.com/mgd43b/shelldbhist/releases/latest

(Assets include macOS/Linux/Windows builds.)

### Option 3: Build from source

```bash
git clone https://github.com/mgd43b/shelldbhist.git
cd shelldbhist

# build the binary
cargo build --release

# optional: install somewhere on your PATH
cp target/release/sdbh /usr/local/bin/sdbh
```

## Quickstart

### 1) Enable shell integration (recommended)
Bash:
```bash
eval "$(sdbh shell --bash)"
```

Zsh:
```bash
eval "$(sdbh shell --zsh)"
```

### 2) Replace Ctrl+R with intelligent fuzzy search (optional but recommended)
**Bash** (~/.bashrc):
```bash
sdbh-fzf-history() {
  selected=$(sdbh list --all --fzf 2>/dev/null)
  [[ -n "$selected" ]] && READLINE_LINE="$selected" && READLINE_POINT=${#selected}
}
bind -x '"\C-r": sdbh-fzf-history'
```

**Zsh** (~/.zshrc):
```zsh
function sdbh-history-widget() {
  selected=$(sdbh list --all --fzf 2>/dev/null)
  [[ -n "$selected" ]] && LBUFFER="$selected"
  zle reset-prompt
}
zle -N sdbh-history-widget
bindkey '^R' sdbh-history-widget
```

### 3) Try it
```bash
# Basic commands
sdbh search kubectl --all --limit 20
sdbh summary git
sdbh list --all --limit 20

# Try the new Ctrl+R fuzzy search!
# Press Ctrl+R in your terminal - you'll get intelligent fuzzy search instead of basic history
```

## Database
Default DB path: `~/.sdbh.sqlite`

Override per command:
```bash
sdbh --db /path/to/file.sqlite list --all
```

## Configuration

`sdbh` supports comprehensive customization through the optional `~/.sdbh.toml` configuration file. All settings have sensible defaults and the tool works perfectly without any configuration.

### Configuration File Location
- **Path**: `~/.sdbh.toml`
- **Format**: TOML
- **Optional**: All settings have defaults; file doesn't need to exist

### Complete Configuration Reference

Here's a complete example showing all available configuration options with their defaults:

```toml
# ============================================
# Logging Configuration
# ============================================
[log]
# Commands to never log (exact match)
ignore_exact = ["echo hello", "make test"]

# Command prefixes to never log
ignore_prefix = ["cd ", "sdbh "]

# Use built-in ignore list (ls, pwd, exit, etc.)
use_builtin_ignores = true  # default: true

# ============================================
# Cleanup/Garbage Detection Configuration
# ============================================
[cleanup]
# Commands/patterns exempt from garbage detection
# Supports glob patterns: * (any sequence), ? (single char)
allow_list = [
    "curl *",           # Exempt all curl commands
    "wget *",           # Exempt all wget commands
    "git commit*",      # Exempt git commits
]

# Size thresholds for garbage detection (bytes)
# Must be in ascending order: small < medium < large
size_threshold_small = 500     # default: 500 bytes
size_threshold_medium = 2048   # default: 2KB
size_threshold_large = 10240   # default: 10KB

# ============================================
# fzf Integration Configuration
# ============================================
[fzf]
# Window height
height = "60%"                 # default: "60%"

# Layout style
layout = "reverse"             # default: "reverse" | options: "default", "reverse"

# Border style
border = "rounded"             # default: "rounded"
                              # options: "rounded", "sharp", "bold", "double", "block", "thinblock"

# Color scheme (fzf color string format)
color = "fg:#d0d0d0,bg:#121212,hl:#5f87af"

# Individual color settings
color_header = "fg:#87afaf"    # Header text color
color_pointer = "fg:#ff8700"   # Selection pointer color
color_marker = "fg:#87ff00"    # Multi-select marker color

# Preview window layout
preview_window = "right:50%"   # default: "right:50%"
                              # options: "right:50%", "top:40%", "left:30%", etc.

# Custom key bindings (array of fzf bind strings)
bind = [
    "ctrl-k:kill-line",        # Custom keybindings
    "ctrl-j:accept"
]

# Custom fzf binary path (optional)
binary_path = "/usr/local/bin/fzf"  # default: searches $PATH
```

### Configuration Sections

#### 1. Logging Configuration (`[log]`)

Control which commands are logged to your history database:

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `ignore_exact` | Array of strings | `[]` | Commands to never log (exact match) |
| `ignore_prefix` | Array of strings | `[]` | Command prefixes to never log |
| `use_builtin_ignores` | Boolean | `true` | Use built-in ignore list (ls, pwd, exit, etc.) |

**Example:**
```toml
[log]
ignore_exact = ["echo hello", "make test"]
ignore_prefix = ["cd ", "sdbh "]
use_builtin_ignores = true
```

#### 2. Cleanup Configuration (`[cleanup]`)

Customize garbage detection behavior:

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `allow_list` | Array of strings | `[]` | Commands/patterns exempt from garbage detection |
| `size_threshold_small` | Integer | `500` | Threshold for "medium-sized" classification (bytes) |
| `size_threshold_medium` | Integer | `2048` | Threshold for "large" classification (bytes) |
| `size_threshold_large` | Integer | `10240` | Threshold for "very large" classification (bytes) |

**Allow-list Pattern Matching:**
- `"curl *"` - Matches all curl commands (glob pattern)
- `"git commit*"` - Matches git commits with any message
- `"ls -?"` - Matches `ls -l`, `ls -a`, etc. (single character wildcard)
- `"exact command"` - Exact match only (no wildcards)

**Important:** Thresholds must be in ascending order: `small < medium < large`

**Example:**
```toml
[cleanup]
allow_list = ["curl *", "wget *", "git commit*"]
size_threshold_small = 1000
size_threshold_medium = 5000
size_threshold_large = 20000
```

#### 3. fzf Configuration (`[fzf]`)
Customize your fzf experience with the `[fzf]` section:

```toml
[fzf]
# Layout and appearance
height = "60%"                    # Window height ("50%", "20", etc.)
layout = "reverse"                # Layout style ("default", "reverse")
border = "rounded"                # Border style ("rounded", "sharp", "bold", "double", "block", "thinblock")

# Color scheme (fzf color string)
color = "fg:#d0d0d0,bg:#121212,hl:#5f87af"
color_header = "fg:#87afaf"      # Header text color
color_pointer = "fg:#ff8700"     # Pointer color
color_marker = "fg:#87ff00"      # Marker color

# Preview settings
preview_window = "right:50%"      # Preview window layout ("right:50%", "top:40%", etc.)

# Key bindings (array of fzf bind strings)
bind = [
    "ctrl-k:kill-line",           # Custom key bindings
    "ctrl-j:accept"
]

# Custom fzf binary path (optional)
binary_path = "/usr/local/bin/fzf"
```

**Example full configuration:**
```toml
[log]
ignore_exact = ["echo hello", "make test"]
use_builtin_ignores = true

[fzf]
height = "70%"
layout = "reverse"
border = "rounded"
color = "fg:#ebdbb2,bg:#282828,hl:#fabd2f,fg+:#ebdbb2,bg+:#3c3836,hl+:#fabd2f"
color_header = "fg:#83a598"
color_pointer = "fg:#fb4934"
color_marker = "fg:#b8bb26"
preview_window = "right:60%"
bind = ["ctrl-k:kill-line", "ctrl-j:accept", "alt-enter:print-query"]
```

## Shell integration modes
`sdbh` supports two modes:

### Hook mode (recommended)
Logs the *last executed* command each time your prompt renders.

Bash:
```bash
sdbh shell --bash
```

Zsh:
```bash
sdbh shell --zsh
```

### Intercept mode (more invasive)
Logs commands *as they execute*.

Bash (DEBUG trap):
```bash
sdbh shell --bash --intercept
```

Zsh (preexec hook):
```bash
sdbh shell --zsh --intercept
```

## Common commands

### Search
Substring search (case-insensitive):
```bash
sdbh search kubectl --all --limit 50
sdbh search "git status" --all --limit 20
sdbh search kubectl --all --format json --limit 10

# time filtering
sdbh search kubectl --all --days 30
sdbh search kubectl --all --since-epoch 1700000000

# Interactive fuzzy selection
sdbh search kubectl --fzf
```

### Summary
Grouped-by-command output (count + last run):
```bash
sdbh summary git
sdbh summary --starts git
sdbh summary --pwd --under

# Interactive fuzzy selection from command summaries
sdbh summary --fzf
```

### List
Raw history (latest first):
```bash
sdbh list --all --limit 50
sdbh list --all --format json

# Interactive fuzzy selection
sdbh list --fzf
```

### Stats
Quick aggregates:
```bash
# top commands in last N days
sdbh stats top --all --days 30 --limit 20

# top commands per directory
sdbh stats by-pwd --all --days 30 --limit 20

# commands per day (localtime buckets)
sdbh stats daily --all --days 30
```

#### Interactive Stats Selection
```bash
# Select from top commands interactively
sdbh stats top --fzf

# Select from commands by directory
sdbh stats by-pwd --fzf

# Multi-select from daily command counts
sdbh stats daily --fzf --multi-select
```

### Import
Import/merge an existing `dbhist.sh` database (hash de-dup):
```bash
sdbh import --from ~/.dbhist
```

Multiple sources:
```bash
sdbh import --from ~/.dbhist --from /path/other.db
```

### Import from shell history files
Bash:
```bash
sdbh import-history --bash ~/.bash_history --pwd "$PWD"
```

Zsh (extended history format):
```bash
sdbh import-history --zsh ~/.zsh_history --pwd "$PWD"
```

If a history file doesn’t include timestamps (common for bash), `sdbh` assigns synthetic sequential timestamps to preserve ordering.

### Doctor
Diagnose your setup (DB access, env vars, and shell integration):
```bash
sdbh doctor
sdbh doctor --no-spawn
sdbh doctor --format json
```

### Cleanup Command - Garbage Detection
`sdbh` includes a conservative garbage detection system to identify likely garbage entries in your command history (accidental pastes, binary content, executed files, etc.) while avoiding false positives on legitimate multi-line commands.

**All Phases Complete ✅**

#### Detection Features
- **Binary content detection**: Recognizes ELF executables, PNG/JPEG/GIF images, PDF documents, ZIP archives
- **Pattern analysis**: Identifies null bytes, excessive non-printable characters, repetitive patterns
- **Size-based scoring**: Flags commands >500 bytes, >2KB, >10KB with appropriate confidence levels
- **Conservative approach**: Recognizes legitimate patterns (SQL queries, JSON, curl with data, heredocs, Python/Ruby scripts)
- **Confidence levels**: High (≥60), Moderate (30-59), Low (<30) for safe decision-making
- **Comprehensive testing**: 56 tests covering all scenarios (20 detection + 6 database + 9 integration + 21 configuration)

#### Cleanup Modes

**Scan Mode** - Preview garbage candidates without deleting:
```bash
# Scan for all potential garbage entries
sdbh cleanup --scan

# Only show high-confidence garbage (score ≥ 60)
sdbh cleanup --scan --min-score 60.0

# Output as JSON for scripting
sdbh cleanup --scan --format json

# Only show entries with score ≥ 30 (moderate and high)
sdbh cleanup --scan --min-score 30.0
```

**Interactive Mode** - Review and selectively delete:
```bash
# Interactively review each candidate
sdbh cleanup --interactive

# Filter to high-confidence entries only
sdbh cleanup --interactive --min-score 60.0
```

The interactive mode presents each garbage candidate with:
- Full command text
- Timestamp and directory
- Confidence score and level
- Detailed reasons for detection
- Yes/No prompt for deletion

**Auto Mode** - Automatically delete high-confidence garbage:
```bash
# Preview what would be deleted (requires confirmation)
sdbh cleanup --auto

# Automatically delete without confirmation (use with caution!)
sdbh cleanup --auto --yes

# Auto-delete with higher threshold
sdbh cleanup --auto --min-score 70.0 --yes
```

⚠️ **Auto mode only deletes High confidence entries (≥60)** for safety. Lower-scored items require interactive review.

#### Configuration

The cleanup system respects your `~/.sdbh.toml` configuration:

```toml
[cleanup]
# Exempt specific commands or patterns from garbage detection
allow_list = [
    "curl *",           # Never flag curl commands
    "wget *",           # Never flag wget commands
    "git commit*",      # Exempt git commits
]

# Customize size thresholds (in bytes)
size_threshold_small = 500      # Default: 500 bytes
size_threshold_medium = 2048    # Default: 2KB
size_threshold_large = 10240    # Default: 10KB
```

#### Example Output

**Scan mode:**
```bash
$ sdbh cleanup --scan
Found 3 potential garbage entries:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    42 | 2026-01-19 00:15:32 | High (50) | ▒ELFbinary_data...
       | Binary file magic number detected
────────────────────────────────────────────────────────────────────────────────
    45 | 2026-01-19 01:22:11 | Moderate (40) | command�with�null�bytes
       | Null bytes detected (binary content)
────────────────────────────────────────────────────────────────────────────────
    48 | 2026-01-19 02:30:45 | Moderate (30) | aaaaaaaaaaaaa... (1000 chars)
       | Repetitive content pattern
       | Large command (1KB)
────────────────────────────────────────────────────────────────────────────────
```

**JSON output:**
```bash
$ sdbh cleanup --scan --format json | jq
[
  {
    "id": 42,
    "epoch": 1737266132,
    "pwd": "/tmp",
    "cmd": "▒ELFbinary_data",
    "score": 50.0,
    "level": "High",
    "reasons": ["Binary file magic number detected"]
  }
]
```

#### Safety Features

The garbage detection system is designed to be **extremely conservative**:

- ✅ **Never flags legitimate commands**: SQL queries, JSON, curl with data, heredocs, shell scripts
- ✅ **Requires high confidence for auto-deletion**: Only entries scored ≥60 (High confidence)
- ✅ **Configurable allow-list**: Exempt your most-used command patterns
- ✅ **Multiple review modes**: Scan, interactive, or auto deletion
- ✅ **Detailed reasoning**: See exactly why each entry was flagged
- ✅ **Preview before deletion**: Auto mode requires confirmation unless `--yes` is used

Multi-line SQL queries, curl commands with large JSON payloads, heredocs, and shell scripts are all recognized as legitimate and scored appropriately.

### Command Templates System
`sdbh` includes a powerful Command Templates System for defining reusable command patterns with variable substitution. Templates are stored as TOML files in `~/.sdbh/templates/` and support variable substitution with defaults and validation.

#### Creating Templates
Templates are TOML files in `~/.sdbh/templates/`. Create a template file like `~/.sdbh/templates/git-commit.toml`:

```toml
id = "git-commit"
name = "Git Commit"
description = "Git commit with conventional format"
command = "git add . && git commit -m '{type}: {message}'"

[[variables]]
name = "type"
description = "Commit type (feat, fix, docs, etc.)"
required = true

[[variables]]
name = "message"
description = "Commit message"
required = true

[[variables]]
name = "scope"
description = "Optional scope"
required = false
default = ""
```

#### Template Usage
```bash
# List all available templates
sdbh template --list

# Execute a template with variable substitution
sdbh template git-commit --var type=feat --var message="add new feature"

# Execute with defaults (interactive prompts for missing required variables)
sdbh template git-commit --var message="fix bug"

# Delete a template
sdbh template --delete git-commit

# Interactive template creation (requires terminal)
sdbh template --create my-template
```

#### Template Variables
- **Required variables**: Must be provided via `--var` or will prompt interactively
- **Optional variables**: Can use `default` values or be left empty
- **Variable substitution**: Use `{variable_name}` in command templates
- **Validation**: Variable names must be alphanumeric with underscores

#### Example Templates
**Docker Build & Deploy:**
```toml
id = "docker-deploy"
name = "Docker Deploy"
description = "Build and deploy Docker image"
command = "docker build -t {image}:{tag} . && docker push {image}:{tag} && kubectl set image deployment/{deployment} app={image}:{tag}"

[[variables]]
name = "image"
description = "Docker image name"
required = true

[[variables]]
name = "tag"
description = "Image tag"
required = false
default = "latest"

[[variables]]
name = "deployment"
description = "Kubernetes deployment name"
required = true
```

**API Testing:**
```toml
id = "api-test"
name = "API Test"
description = "Test API endpoint"
command = "curl -X {method} '{base_url}/api/v{version}/{endpoint}' -H 'Authorization: Bearer {token}'"

[[variables]]
name = "method"
description = "HTTP method"
required = false
default = "GET"

[[variables]]
name = "base_url"
description = "API base URL"
required = true

[[variables]]
name = "version"
description = "API version"
required = false
default = "1"

[[variables]]
name = "endpoint"
description = "API endpoint"
required = true

[[variables]]
name = "token"
description = "Auth token"
required = true
```

## Interactive Fuzzy Selection

`sdbh` integrates with [fzf](https://github.com/junegun/fzf) for interactive command selection. The killer feature is **replacing your shell's Ctrl+R history search** with sdbh's intelligent fuzzy search across your entire command history.

### Enhanced Preview System

`sdbh` provides a rich, context-aware preview system that transforms command selection from basic text matching into intelligent analysis:

#### Command Analysis Preview
When browsing commands with `--fzf`, the right-side preview pane shows detailed command intelligence with responsive design that adapts to terminal width:

```bash
# Example preview for "git status"
🔍 Command Analysis: git status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 Usage Statistics
  Total uses: 45
  First used: 3 weeks ago
  Last used: 2h ago
  Directories: 3

ℹ️  Context: Shows working directory status and changes

📁 Directory Usage:
  • /home/user/project
  • /tmp/build
  • /var/www

🕒 Recent Activity (Last 5 executions):
  1. 2h ago   | git status          | /home/user/project
  2. 1d ago   | git status --porcelain | /home/user/project
  3. 3d ago   | git status          | /tmp/build
```

#### Context-Aware Intelligence
The preview system recognizes command types and provides specific information:

- **🔧 Git**: Explains what `status`, `log`, `diff`, `branch`, etc. do
- **🐳 Docker**: Describes `run`, `build`, `ps`, `exec`, `logs` functionality
- **☸️ Kubernetes**: Explains `get`, `describe`, `logs`, `apply` operations
- **📦 Cargo**: Details `build`, `test`, `check`, `fmt`, `clippy` purposes
- **📦 NPM**: Describes `install`, `start`, `run`, `test`, `build` workflows
- **🔨 Make**: Explains build targets and common operations

#### Manual Preview Inspection
You can also manually inspect any command's detailed analysis:

```bash
# Get full analysis for any command in your history
sdbh preview "git status"
sdbh preview "docker build ."
sdbh preview "kubectl get pods"
```

### Requirements
- Install [fzf](https://github.com/junegun/fzf) (available via most package managers)

### ⚡ Power User Feature: Ctrl+R History Replacement

**Transform your shell experience** by replacing the basic Ctrl+R search with sdbh's advanced fuzzy search:

- **Before**: Basic substring matching in current session only
- **After**: Intelligent fuzzy search across your entire command history with preview pane

**One-time setup** (add to your `~/.bashrc` or `~/.zshrc`):

**Bash:**
```bash
# Replace Ctrl+R with sdbh fuzzy search
sdbh-fzf-history() {
  selected=$(sdbh list --all --fzf 2>/dev/null)
  [[ -n "$selected" ]] && READLINE_LINE="$selected" && READLINE_POINT=${#selected}
}
bind -x '"\C-r": sdbh-fzf-history'
```

**Zsh:**
```zsh
function sdbh-history-widget() {
  selected=$(sdbh list --all --fzf 2>/dev/null)
  [[ -n "$selected" ]] && LBUFFER="$selected"
  zle reset-prompt
}
zle -N sdbh-history-widget
bindkey '^R' sdbh-history-widget
```

Now **Ctrl+R** gives you:
- Fuzzy search across ALL your commands (not just current session)
- Rich preview pane showing command usage statistics
- Customizable colors and layout via `~/.sdbh.toml`
- Multi-select capability for batch operations

### Basic Usage

**Command History Selection:**
```bash
# Browse recent commands interactively
sdbh list --fzf

# Search and select from matching commands
sdbh search "git" --fzf

# Select from command summaries
sdbh summary --fzf
```

**Output Format:**
When you select a command in fzf, it prints the command to stdout, ready for execution:
```bash
$ sdbh search kubectl --fzf
kubectl get pods -n kube-system
```

### Advanced Shell Integration

Add these functions to your `~/.bashrc` or `~/.zshrc` for enhanced fzf integration:

**Bash/Zsh: Enhanced History Search (Ctrl+R replacement)**
Replace your shell's default history search with sdbh's powerful fuzzy search:

**Bash (~/.bashrc):**
```bash
# sdbh-powered history search - replaces Ctrl+R
sdbh-fzf-history() {
  local selected
  # Use sdbh's fuzzy search instead of basic shell history
  selected=$(sdbh list --all --fzf 2>/dev/null)
  if [[ -n "$selected" ]]; then
    # Insert selected command into current line
    READLINE_LINE="$selected"
    READLINE_POINT=${#selected}
  fi
}

# Bind to Ctrl+R (replaces default reverse-search-history)
bind -x '"\C-r": sdbh-fzf-history'
```

**Zsh (~/.zshrc):**
```zsh
# sdbh-powered history search widget - replaces Ctrl+R
function sdbh-history-widget() {
  local selected
  # Launch sdbh fuzzy search
  selected=$(sdbh list --all --fzf 2>/dev/null)
  if [[ -n "$selected" ]]; then
    # Insert into command line buffer
    LBUFFER="$selected"
  fi
  # Reset prompt display
  zle reset-prompt
}

# Register the widget
zle -N sdbh-history-widget

# Bind to Ctrl+R (replaces default history-incremental-search-backward)
bindkey '^R' sdbh-history-widget
```

**Bash/Zsh: Command Templates**
```bash
# Search for git commands
sdbh-git() {
  local cmd
  cmd=$(sdbh search "git" --all --fzf 2>/dev/null)
  if [[ -n "$cmd" ]]; then
    echo "Executing: $cmd"
    eval "$cmd"
  fi
}

# Search for docker commands
sdbh-docker() {
  local cmd
  cmd=$(sdbh search "docker" --all --fzf 2>/dev/null)
  if [[ -n "$cmd" ]]; then
    echo "Executing: $cmd"
    eval "$cmd"
  fi
}

# Interactive summary selection
sdbh-summary() {
  local cmd
  cmd=$(sdbh summary --all --fzf 2>/dev/null)
  if [[ -n "$cmd" ]]; then
    echo "Executing: $cmd"
    eval "$cmd"
  fi
}
```

**Zsh: Custom Widgets**
```bash
# Zsh widget for sdbh history
sdbh-history-widget() {
  local selected
  selected=$(sdbh list --all --fzf 2>/dev/null)
  if [[ -n "$selected" ]]; then
    LBUFFER="$selected"
  fi
  zle reset-prompt
}
zle -N sdbh-history-widget
bindkey '^R' sdbh-history-widget
```

## Troubleshooting

### Bash hook requirements
For bash hook mode, `HISTTIMEFORMAT="%s "` is required so `history 1` includes an epoch timestamp.

### Bash troubleshooting
- Confirm the function is defined:
  ```bash
  type __sdbh_prompt
  ```
- Confirm it’s wired into your prompt:
  ```bash
  echo "$PROMPT_COMMAND"
  ```
- If you updated your rc file, remember to reload it:
  ```bash
  eval "$(sdbh shell --bash)"
  ```

## Development

### Pre-commit Quality Checks
This project uses a git pre-commit hook to enforce code quality standards:

- **Formatting**: `cargo fmt --check` ensures code follows Rust formatting standards
- **Linting**: `cargo clippy -- -D warnings` treats all clippy warnings as errors

The hook automatically runs on every commit and will prevent commits that don't meet these standards.

To set up the hook for your local development:
```bash
# The hook is already configured in .git/hooks/pre-commit
# Make sure it's executable (should be by default)
chmod +x .git/hooks/pre-commit
```

To bypass the hook for special cases (not recommended):
```bash
git commit --no-verify -m "your commit message"
```

## Project documentation
- Release process: `docs/releasing.md`
- Development notes: `docs/development.md`