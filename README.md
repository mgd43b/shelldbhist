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

```bash
# create an annotated tag that matches sdbh/Cargo.toml version
git tag -a v0.1.0 -m "sdbh v0.1.0"

# push the tag to trigger GitHub Actions release
git push origin v0.1.0
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
