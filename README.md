# sdbh

Shell DB History (`sdbh`) stores your shell command history in a local SQLite database.
It’s inspired by `dbhist.sh`, but implemented as a portable Rust CLI backed by SQLite.

## Features
- Local SQLite history database (`~/.sdbh.sqlite` by default)
- Fast search (substring), raw history listing, grouped summaries
- **Interactive fuzzy selection** with fzf integration (`--fzf` flags)
- Stats (top commands, by-directory, daily buckets)
- Database health monitoring and performance optimization
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

### 2) Try it
```bash
sdbh search kubectl --all --limit 20
sdbh summary git
sdbh list --all --limit 20
```

## Database
Default DB path: `~/.sdbh.sqlite`

Override per command:
```bash
sdbh --db /path/to/file.sqlite list --all
```

## Configuration (optional)
`sdbh` reads optional settings from `~/.sdbh.toml`.

Example:
```toml
[log]
# Add extra ignores (exact command match)
ignore_exact = ["echo hello", "make test"]

# Add extra ignores (prefix match)
ignore_prefix = ["cd ", "sdbh "]

# If false, disables built-in ignores (like `ls`, `pwd`, etc.)
use_builtin_ignores = true
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

## Interactive Fuzzy Selection

`sdbh` integrates with [fzf](https://github.com/junegun/fzf) for interactive command selection. Add the `--fzf` flag to any search, list, or summary command to launch an interactive fuzzy finder.

### Requirements
- Install [fzf](https://github.com/junegun/fzf) (available via most package managers)

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
```bash
# sdbh-powered history search
sdbh-fzf-history() {
  local selected
  selected=$(sdbh list --all --fzf 2>/dev/null)
  if [[ -n "$selected" ]]; then
    READLINE_LINE="$selected"
    READLINE_POINT=${#selected}
  fi
}

# Bind to Ctrl+R in bash
bind -x '"\C-r": sdbh-fzf-history'

# Bind to Ctrl+R in zsh
bindkey '^R' sdbh-fzf-history
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

## Project documentation
- Release process: `docs/releasing.md`
- Development notes: `docs/development.md`
