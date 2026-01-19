# sdbh

[![CI](https://github.com/mgd43b/shelldbhist/actions/workflows/ci.yml/badge.svg)](https://github.com/mgd43b/shelldbhist/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/mgd43b/shelldbhist)](https://github.com/mgd43b/shelldbhist/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/mgd43b/shelldbhist/latest/total)](https://github.com/mgd43b/shelldbhist/releases/latest)
[![License](https://img.shields.io/github/license/mgd43b/shelldbhist)](https://github.com/mgd43b/shelldbhist/blob/main/LICENSE)
[![codecov](https://codecov.io/gh/mgd43b/shelldbhist/branch/main/graph/badge.svg)](https://codecov.io/gh/mgd43b/shelldbhist)

Shell DB History (`sdbh`) stores your shell command history in a local SQLite database.
It's inspired by `dbhist.sh`, but implemented as a portable Rust CLI backed by SQLite.

> Note: `sdbh` currently targets macOS and Linux (Unix-like shells). Windows is not supported.

## Features
- 🔍 **Interactive fuzzy search** with `--fzf` flag for intelligent command selection
- 📊 **Rich preview panes** showing command statistics and usage patterns
- 🎯 **Multi-select support** with `--multi-select` flag for batch operations
- ⚙️ **Full configuration system** via `~/.sdbh.toml` for colors, layout, and key bindings
- 🎨 **Ctrl+R replacement** for shell history search (transformative UX improvement)
- 📱 **Responsive terminal design** adapting to different terminal widths (80-200+ chars)
- 🔧 **Command Templates System** for reusable command patterns with variable substitution
- 💻 **Organized command preview** with information hierarchy and smart truncation
- 🧹 **Garbage detection** (Phase 1) for identifying likely garbage entries with conservative heuristics
- Local SQLite history database (`~/.sdbh.sqlite` by default)
- Fast search (substring), raw history listing, grouped summaries
- Stats (top commands, by-directory, daily buckets)
- Database health monitoring and performance optimization
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

(Assets include macOS/Linux builds.)

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

### Override database location

You can override the database location in two ways:

**1. Environment variable (recommended for demos/testing):**
```bash
# Set for current session
export SDBH_DB=/tmp/demo-sdbh.sqlite
sdbh list --all

# Or set for a single command
SDBH_DB=/tmp/demo-sdbh.sqlite sdbh search build --all
```

**2. Command-line flag:**
```bash
sdbh --db /path/to/file.sqlite list --all
```

The `SDBH_DB` environment variable is particularly useful for:
- Recording clean demos without exposing personal history
- Testing with isolated databases
- Running multiple sdbh instances with separate histories
- CI/CD pipelines requiring separate test databases

**Priority order:** `--db` flag > `SDBH_DB` environment variable > `~/.sdbh.sqlite` (default)

## Documentation

The README is intentionally kept as a quickstart. Full docs live in `docs/`:

- [Commands / usage](docs/usage.md)
- [fzf + preview](docs/fzf-integration.md)
- [Shell integration](docs/shell-integration.md)
- [Configuration](docs/configuration.md)
- [Database](docs/database.md)
- [Cleanup / garbage detection](docs/cleanup-guide.md)
- [Command templates](docs/templates.md)

## Common commands

See the full command reference in [docs/usage.md](docs/usage.md).

Quick examples:

```bash
sdbh search kubectl --all --limit 20
sdbh summary git
sdbh list --all --limit 20
```

## Commands overview

| Command | Description |
|---|---|
| `sdbh log` | Insert one history row (used by shell integration). |
| `sdbh list` | List raw history entries. |
| `sdbh search` | Search history by substring (supports time filtering). |
| `sdbh summary` | Group history by command (count + last seen). |
| `sdbh stats` | Aggregate statistics (top/by-pwd/daily). |
| `sdbh preview` | Show usage-oriented preview for a command (used by fzf preview). |
| `sdbh export` | Export history as JSON Lines. |
| `sdbh import` | Import/merge another dbhist-compatible SQLite database. |
| `sdbh import-history` | Import from shell history files (bash/zsh). |
| `sdbh db` | Database operations (health/optimize/stats/schema). |
| `sdbh cleanup` | Scan/review/delete likely garbage/noisy entries. |
| `sdbh delete` | Delete history entries by ID (permanent). |
| `sdbh template` | Manage/execute command templates. |
| `sdbh shell` | Print shell integration snippets. |
| `sdbh doctor` | Diagnose shell integration / DB setup. |
| `sdbh version` | Print version information. |

## Development

Maintainer documentation:
- Development notes: [docs/development.md](docs/development.md)
- Release process: [docs/releasing.md](docs/releasing.md)