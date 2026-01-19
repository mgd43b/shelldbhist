# Usage / Commands

This page contains the **user-facing command reference** for `sdbh`.

> Note: `sdbh` operates on *your* history stored in a local SQLite database. Output and previews reflect your recorded usage.

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

# interactive fuzzy selection
sdbh search kubectl --fzf
```

### Summary
Grouped-by-command output (count + last run):

```bash
sdbh summary git
sdbh summary --starts git
sdbh summary --pwd --under

# interactive fuzzy selection from command summaries
sdbh summary --fzf
```

### List
Raw history (latest first):

```bash
sdbh list --all --limit 50
sdbh list --all --format json

# interactive fuzzy selection
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
sdbh stats top --fzf
sdbh stats by-pwd --fzf
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

### Preview
Show detailed preview information for a command (used by fzf preview):

```bash
sdbh preview "git status"
```
