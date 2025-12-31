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
- **Four command integration points**: `list --fzf`, `search --fzf`, `summary --fzf`, `stats --fzf` all support interactive selection
- **Enhanced preview pane**: Right-side preview with comprehensive command analysis
- **Multi-select support**: `--multi-select` flag enables Tab-based multi-selection in fzf
- **Custom fzf configuration**: Comprehensive `~/.sdbh.toml` support for all fzf options
- **Configuration options**: Height, layout, borders, colors, key bindings, preview settings, binary path
- **Graceful fallbacks**: Works perfectly without configuration, invalid configs ignored
- **Ctrl+R history integration**: Complete shell integration examples for bash/zsh Ctrl+R replacement
- **Compact output formats**: Optimized for fzf readability:
  - List/search: `"command  (timestamp) [directory]"`
  - Summary: `"command [directory]  (count uses, last: timestamp)"`
- **Command parsing**: Robust parsing of fzf line formats to extract command names for preview
- **Output handling**: Single commands or multi-line output for multiple selections

### Enhanced Preview System (Phase 3: UI/UX Polish and Performance)
- **Context-aware command analysis**: `CommandType::detect()` categorizes commands into 11+ types (Git, Docker, Kubernetes, Cargo, NPM, Make, etc.)
- **Intelligent command explanations**: `show_command_type_info()` displays type-specific information for each command category
- **Enhanced recent executions**: `format_relative_time()` converts timestamps to human-readable format ("2h ago", "1d ago")
- **Smart related commands**: Four algorithms for intelligent command suggestions:
  - **Semantic similarity**: Workflow patterns based on command type (e.g., `git commit` → `git status`, `git push`)
  - **Tool variations**: Commands starting with same tool (`find_tool_related_commands`)
  - **Workflow patterns**: Commands used together in same sessions within 1-hour windows (`find_workflow_related_commands`)
  - **Directory-based**: Commands used in same directories (`find_directory_related_commands`)
- **Command highlighting**: Shows argument variations from base commands
- **Directory usage tracking**: Displays all directories where commands were executed
- **Smart deduplication**: Removes duplicate suggestions and limits to 5 most relevant

#### **Phase 3: Professional Layout & Performance**
- **Organized sections** with clear visual hierarchy and separators
- **Responsive design** adapting to terminal width (wide >120 chars vs narrow <80 chars)
- **Optimized queries** with separate fast queries instead of complex aggregates
- **Pagination** for execution history (configurable limits)
- **Terminal size detection** using `terminal_size` crate
- **Smart truncation** preserving important information based on available space
- **Color coding** system for command age, frequency, and type
- **Collapsible sections** for optional detailed information
- **Performance caching** for frequently accessed command metadata

## Preview subcommand
- **Statistics display**: Shows total uses, first/last execution times, unique directories
- **Recent executions**: Lists up to 3 most recent command executions with timestamps and directories
- **Error handling**: Graceful handling of commands not found in history
- **Database queries**: Efficient SQL aggregates for comprehensive command analysis

## Test coverage and quality assurance
- **Major coverage expansion**: Overall coverage 54.60% → 65.39% (+10.79%), CLI module 53% → 63.3% (+10.3%)
- **Comprehensive integration tests**: 76 tests covering all functionality with systematic error condition coverage
- **Systematic test addition patterns**:
  - **Error handling**: All major failure paths, invalid arguments, missing dependencies
  - **JSON output**: Complete testing of all commands with JSON formatting and validation
  - **Configuration systems**: TOML parsing, environment variables, fzf configuration
  - **Shell integration**: Hook and intercept modes, environment variable handling
  - **Database operations**: Health checks, optimization, statistics, corruption scenarios
  - **Advanced features**: Preview system, fzf integration, multi-select functionality
- **Boundary condition testing**: Empty commands, very long inputs (10KB+), extreme timestamps, special characters
- **SQL safety validation**: Comprehensive LIKE escaping for %, _, \, quotes in all query paths
- **Concurrent access testing**: Multiple rapid database operations without corruption
- **Configuration robustness**: Malformed config files, missing environment variables, invalid TOML
- **File system edge cases**: Permission issues, malformed inputs, concurrent file access
- **Production readiness validation**: All tests passing (76/76), no bugs discovered in expansion

## Project layout
- Repo root is a Cargo workspace (`Cargo.toml`) with member `sdbh/`.
- Workspace lockfile: `Cargo.lock` at repo root.
