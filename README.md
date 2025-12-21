# sdbh

Shell DB History (`sdbh`) stores your shell command history in a local SQLite database.
It’s inspired by `dbhist.sh`, but implemented as a portable Rust CLI backed by SQLite.

## Features
- Local SQLite history database (`~/.sdbh.sqlite` by default)
- Fast search (substring), raw history listing, grouped summaries
- Stats (top commands, by-directory, daily buckets)
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
```

### Summary
Grouped-by-command output (count + last run):
```bash
sdbh summary git
sdbh summary --starts git
sdbh summary --pwd --under
```

### List
Raw history (latest first):
```bash
sdbh list --all --limit 50
sdbh list --all --format json
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
