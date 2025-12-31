# sdbh

[![CI](https://github.com/mgd43b/shelldbhist/actions/workflows/ci.yml/badge.svg)](https://github.com/mgd43b/shelldbhist/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/mgd43b/shelldbhist)](https://github.com/mgd43b/shelldbhist/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/mgd43b/shelldbhist/latest/total)](https://github.com/mgd43b/shelldbhist/releases/latest)
[![License](https://img.shields.io/github/license/mgd43b/shelldbhist)](https://github.com/mgd43b/shelldbhist/blob/main/LICENSE)
[![codecov](https://codecov.io/gh/mgd43b/shelldbhist/branch/main/graph/badge.svg)](https://codecov.io/gh/mgd43b/shelldbhist)

Shell DB History (`sdbh`) stores your shell command history in a local SQLite database.
It’s inspired by `dbhist.sh`, but implemented as a portable Rust CLI backed by SQLite.

## Features
- 🔍 **Interactive fuzzy search** with `--fzf` flag for intelligent command selection
- 📊 **Rich preview panes** showing command statistics and usage patterns
- 🎯 **Multi-select support** with `--multi-select` flag for batch operations
- ⚙️ **Full configuration system** via `~/.sdbh.toml` for colors, layout, and key bindings
- 🎨 **Ctrl+R replacement** for shell history search (transformative UX improvement)
- 📱 **Responsive terminal design** adapting to different terminal widths (80-200+ chars)
- 🔧 **Command Templates System** for reusable command patterns with variable substitution
- 💻 **Professional UI/UX** with organized information hierarchy and smart truncation
- Local SQLite history database (`~/.sdbh.sqlite` by default)
- Fast search (substring), raw history listing, grouped summaries
- Stats (top commands, by-directory, daily buckets)
- Database health monitoring and performance optimization
- Comprehensive test coverage (64.8% overall, 87 integration tests)
- Import/merge from existing `dbhist.sh` SQLite databases
- Import from shell history files (`.bash_history`, `.zsh_history`)

## Install

### Recommended: download a prebuilt binary
Download the right binary for your OS from the **latest GitHub Release**:

https://github.com/mgd43b/shelldbhist/releases/latest

(Assets include macOS/Linux/Windows builds.)

### Build from source
```bash
git clone https://github.com/mgd43b/shelldbhist.git
cd shelldbhist
cargo build --release

# optional
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

## Configuration (optional)
`sdbh` reads optional settings from `~/.sdbh.toml`.

### Logging Configuration
```toml
[log]
# Add extra ignores (exact command match)
ignore_exact = ["echo hello", "make test"]

# Add extra ignores (prefix match)
ignore_prefix = ["cd ", "sdbh "]

# If false, disables built-in ignores (like `ls`, `pwd`, etc.)
use_builtin_ignores = true
```

### fzf Configuration
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

### Command Templates System
`sdbh` includes a powerful Command Templates System for defining reusable command patterns with variable substitution. This feature is currently in infrastructure setup phase for v0.12.0.

#### Template Definition (Coming in v0.12.0)
Define reusable command patterns in `~/.sdbh.toml`:

```toml
[templates.git-commit]
description = "Git commit with conventional format"
command = "git add . && git commit -m '{type}: {message}'"
variables = ["type", "message"]
default_values = { type = "feat" }

[templates.docker-deploy]
description = "Deploy to Docker environment"
command = "docker build -t {image}:{tag} . && docker push {image}:{tag} && kubectl set image deployment/{deployment} app={image}:{tag}"
variables = ["image", "tag", "deployment"]
default_values = { tag = "latest" }
```

#### Usage (CLI structure ready for v0.12.0)
```bash
# List all available templates
sdbh template --list

# Execute a template with variable substitution
sdbh template git-commit --var type=feat --var message="add new feature"

# Interactive template selection with fzf
sdbh template --fzf

# Create or edit templates interactively
sdbh template --create git-workflow

# Delete a template
sdbh template --delete old-template
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
