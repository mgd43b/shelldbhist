# System Patterns: sdbh

## High-level architecture
- `sdbh` is a Rust CLI.
- SQLite storage (via `rusqlite`).
- Two shell integration approaches:
  1) **Hook mode**: bash `PROMPT_COMMAND` / zsh `precmd` collects last command + context and runs `sdbh log ...`.
  2) **Intercept mode** (more invasive): shell widget/plugin wraps command execution to capture richer context (e.g., preexec-like) then calls `sdbh log`.

## Data storage
- Default DB: `~/.sdbh.sqlite` (override via `--db`).
- Base table compatible with dbhist:
  `history(id, hist_id, cmd, epoch, ppid, pwd, salt)`
- Additional tables for robustness:
  - `meta(key,value)` for tool/schema version
  - `history_hash(hash TEXT UNIQUE, history_id INTEGER)` to support hash-based de-dup

## Import / merge
- Hash-based de-duplication across rows via deterministic `history_hash`.
- Import sources:
  - `sdbh import`: merges dbhist-compatible SQLite DBs
    - best-effort: skips corrupted/mixed-type rows instead of failing.
  - `sdbh import-history`: imports from **bash** / **zsh** history files
    - bash: supports `#<epoch>` timestamp lines, otherwise uses synthetic sequential timestamps to preserve ordering.
    - zsh: parses extended history `: <epoch>:<duration>;<cmd>`.

## Query patterns
- `summary`: GROUP BY cmd (+ optional pwd)
- `list`: chronological view ordered by epoch ASC, id ASC (oldest entries first)
- `--here` / `--under` location filtering is implemented in SQL.
  - For testability and scripting, `summary` and `list` support `--pwd-override <path>`.

## Database management and optimization
- **Automatic index creation**: 4 performance indexes created automatically on database open:
  - `idx_history_session` on (salt, ppid) - optimizes session filtering queries
  - `idx_history_pwd` on pwd - optimizes directory-based filtering
  - `idx_history_hash` on hash - optimizes deduplication queries
  - `idx_history_epoch` on epoch - optimizes time-based filtering
- **Database health monitoring**: Comprehensive checks integrated into `doctor` command:
  - Database integrity verification via `PRAGMA integrity_check`
  - Fragmentation analysis with VACUUM recommendations
  - Missing index detection and creation suggestions
  - Size and space usage statistics
- **Dedicated db subcommands**:
  - `sdbh db health` - runs comprehensive health checks and reports issues
  - `sdbh db optimize` - creates missing indexes and optimizes database structure
  - `sdbh db stats` - provides detailed statistics on database size, fragmentation, and usage

## Session filtering
- Default: no session filtering (shows all entries from all sessions)
- `--session`: filter to current session only (requires SDBH_SALT and SDBH_PPID environment variables)
- Session filtering available on: list, summary, search, export, stats commands

## Limits and --all flag
- `--all`: removes limits (unlimited results) for commands that have limits: list, summary, search, stats top, stats by-pwd
- For export and stats daily (no limits), `--all` has no effect

## Configuration
- Optional config at `~/.sdbh.toml`.
  - Currently used for configurable log noise filtering.

## CI / Releases
- GitHub Actions CI workflow (`.github/workflows/ci.yml`): runs `cargo fmt`, `cargo clippy`, `cargo test`.

### Release automation patterns (important)
#### release-please (Release PR + tag)
- Workflow: `.github/workflows/release-please.yml`
- Config: `release-please-config.json`
- Manifest: `.release-please-manifest.json`
- **Path-based package**: the crate lives in `sdbh/`, so release-please must be configured with:
  - package name/key: `sdbh`
  - `path: "sdbh"`
- Merging the Release PR is the normal way tags `vX.Y.Z` get created.

#### cargo-dist (build + upload artifacts)
- Workflow: `.github/workflows/release.yml`
- Trigger: tag push matching a semver-looking pattern.
- Constraint: tag version must match `sdbh/Cargo.toml` `package.version`.

#### Version Sync Guard (drift prevention)
- Workflow: `.github/workflows/version-sync-guard.yml`
- Trigger: tag push `vX.Y.Z…`
- Fails if tag version != `sdbh/Cargo.toml` version or != `.release-please-manifest.json["sdbh"]`.

## Interactive Fuzzy Selection (fzf integration)
- **fzf detection and execution**: Commands check for fzf availability and fail gracefully if not installed
- **Three command integration points**: `list --fzf`, `search --fzf`, `summary --fzf` all support interactive selection
- **Preview pane**: Right-side preview (50% width) shows command statistics when hovering
- **Multi-select support**: `--multi-select` flag enables Tab-based multi-selection in fzf
- **Custom fzf configuration**: Comprehensive `~/.sdbh.toml` support for all fzf options
- **Configuration options**: Height, layout, borders, colors, key bindings, preview settings, binary path
- **Graceful fallbacks**: Works perfectly without configuration, invalid configs ignored
- **Compact output formats**: Optimized for fzf readability:
  - List/search: `"command  (timestamp) [directory]"`
  - Summary: `"command [directory]  (count uses, last: timestamp)"`
- **Command parsing**: Robust parsing of fzf line formats to extract command names for preview
- **Output handling**: Single commands or multi-line output for multiple selections

## Preview subcommand
- **Statistics display**: Shows total uses, first/last execution times, unique directories
- **Recent executions**: Lists up to 3 most recent command executions with timestamps and directories
- **Error handling**: Graceful handling of commands not found in history
- **Database queries**: Efficient SQL aggregates for comprehensive command analysis

## Test coverage and quality assurance
- **Comprehensive integration tests**: 53 tests covering all functionality with focus on error conditions
- **Error handling coverage**: Extensive testing of invalid inputs, missing files, database corruption
- **Boundary condition testing**: Empty commands, very long inputs (10KB+), extreme timestamps
- **SQL safety validation**: Special character handling with proper LIKE escaping
- **Concurrent access testing**: Multiple rapid database operations without corruption
- **Configuration robustness**: Malformed config files and missing environment variables
- **File system edge cases**: Permission issues and malformed input handling
- **Coverage metrics**: CLI module at 60.6% coverage with ongoing improvement focus

## Project layout
- Repo root is a Cargo workspace (`Cargo.toml`) with member `sdbh/`.
- Workspace lockfile: `Cargo.lock` at repo root.
