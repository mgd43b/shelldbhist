# sdbh

Shell DB History (`sdbh`) stores your shell command history in a local SQLite database and provides fast search, summaries, raw browsing, and import from existing `dbhist.sh` databases.

> Repo note: the Rust crate lives in `./sdbh`.

## Install

For now, build from source:

```bash
git clone https://github.com/mgd43b/shelldbhist.git
cd shelldbhist
cargo build --release

# optional
cp target/release/sdbh /usr/local/bin/sdbh
```

## Releases

This repo publishes binaries via **cargo-dist** on tag push.

### Automated releases (recommended): Release Please
We use **release-please** to automate version bumps + tagging.

Workflow:
1. Merge PRs into `main` (use **Conventional Commit** titles like `feat: ...`, `fix: ...`).
2. release-please opens/updates a **Release PR**.
3. Merge the Release PR.
4. release-please creates a tag `vX.Y.Z` + GitHub Release.
5. cargo-dist Release workflow runs on that tag and uploads binaries.

### Manual releases (fallback)
```bash
# 1) bump version in sdbh/Cargo.toml
# 2) commit it
# 3) tag the commit with the same version
git tag -a v0.1.5 -m "sdbh v0.1.5"

# 4) push the tag to trigger GitHub Actions release
git push origin v0.1.5
```

Artifacts will appear in the GitHub Release for that tag.

## Database

Default DB path: `~/.sdbh.sqlite`

Override per command:

```bash
sdbh --db /path/to/file.sqlite list --all
```

## Shell integration

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

## Usage

### Log (used by the shell integration)

```bash
sdbh log --cmd "echo hello" --epoch "$(date +%s)" --ppid $$ --pwd "$PWD" --salt 123

# Force logging even if the command is considered "noisy" by default
sdbh log --no-filter --cmd "ls" --epoch "$(date +%s)" --ppid $$ --pwd "$PWD" --salt 123
```

### Config (optional)
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

### Search
Substring search (case-insensitive):

```bash
sdbh search kubectl --all --limit 50
sdbh search "git status" --all --limit 20
sdbh search kubectl --all --format json --limit 10
```

### Export
Export history as JSON Lines (JSONL) to stdout:

```bash
sdbh export --all > sdbh.jsonl
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

Note: Some older/hand-edited dbhist databases may contain corrupted rows where numeric
columns contain TEXT. `sdbh import` will **skip** those rows and print how many it skipped.

### Import from shell history files
Import from plain shell history files into the SQLite DB (deduplicated via `history_hash`).

Bash:

```bash
sdbh import-history --bash ~/.bash_history --pwd "$PWD"
```

Zsh (extended history format):

```bash
sdbh import-history --zsh ~/.zsh_history --pwd "$PWD"
```

If a history file doesn’t include timestamps (common for bash), `sdbh` assigns **synthetic sequential timestamps** to preserve ordering.

### Doctor
Diagnose your setup (DB access, env vars, and shell integration).

```bash
sdbh doctor

# Only run env-based checks (don’t spawn shells)
sdbh doctor --no-spawn

# Only run spawned-shell checks
sdbh doctor --spawn-only

# JSON output (minimal, scripting-friendly)
sdbh doctor --format json
```

What it checks:
- DB path is openable/writable (based on `--db` override or default `~/.sdbh.sqlite`).
- `SDBH_SALT` and `SDBH_PPID` are present and integers.
- Shell integration detection:
  - env-only heuristics (e.g. bash `PROMPT_COMMAND` contains `__sdbh_prompt`)
  - spawned shell introspection:
    - bash: `trap -p DEBUG` and `PROMPT_COMMAND`
    - zsh: `precmd_functions` / `preexec_functions`

Note: spawned-shell detection is best-effort and depends on what your shell loads in non-interactive mode.

## Notes / Caveats
- For bash hook mode, `HISTTIMEFORMAT="%s "` is required so `history 1` includes an epoch timestamp.
- Intercept mode is more invasive; it can capture internal shell commands and may need additional filtering.

## Bash troubleshooting
- Confirm the function is defined:
  ```bash
  type __sdbh_prompt
  ```
- Confirm it’s wired into your prompt:
  ```bash
  echo "$PROMPT_COMMAND"
  ```
- If you updated your rc file, remember to reload it (or re-run):
  ```bash
  eval "$(sdbh shell --bash)"
  ```
