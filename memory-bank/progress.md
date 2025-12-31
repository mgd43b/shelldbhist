# Progress: sdbh

## What works
- Rust crate `sdbh/` created and builds.
- Repo root is a Cargo workspace (`Cargo.toml`) with member `sdbh/`.
- **Database performance optimizations**: Automatic creation of 4 performance indexes for optimal query performance
- **Database health monitoring**: Comprehensive health checks via `doctor` command and dedicated `db` subcommands
- **New db subcommands**:
  - `sdbh db health` - database integrity, fragmentation, and optimization checks
  - `sdbh db optimize` - creates missing indexes and runs optimization
  - `sdbh db stats` - detailed database statistics and fragmentation analysis
- **Automatic performance optimization**: Enabled for all users by default on database open
- **Integration test coverage**: 56 tests covering all functionality including 17 new comprehensive error handling tests
- **Test coverage improvement**: CLI module from 53% to 60.6% coverage (+7.6% absolute improvement)
- **fzf integration**: Interactive fuzzy selection with `--fzf` flag for `list`, `search`, and `summary` commands
- **fzf preview pane**: Right-side preview showing command statistics when hovering in fzf
- **Multi-select fzf**: `--multi-select` flag allows selecting multiple commands with Tab key
- **Custom fzf configuration**: Comprehensive `~/.sdbh.toml` support for colors, layout, key bindings, and preview settings
- **Comprehensive documentation**: README.md updated with fzf integration examples, configuration guide, and shell functions

## CI / Releases
- CI workflow: `.github/workflows/ci.yml` runs fmt/clippy/test.

### cargo-dist binary releases
- cargo-dist workflow `.github/workflows/release.yml` runs on tag pushes and uploads artifacts to GitHub Releases.
- Verified successful end-to-end artifact publishing for **v0.3.0**.

### release-please automation
- release-please workflow exists: `.github/workflows/release-please.yml`.
- Config/manifest:
  - `release-please-config.json` is configured **path-based** for `sdbh/`.
  - `.release-please-manifest.json` tracks `sdbh` version.

### Drift prevention
- Added `Version Sync Guard` workflow: `.github/workflows/version-sync-guard.yml`.
  - Runs on tag pushes `vX.Y.Z…`.
  - Fails if tag version != `sdbh/Cargo.toml` version or != manifest `sdbh` version.

## Release flow (recommended)
1) Merge PRs into `main` using **Conventional Commit** titles.
2) release-please opens/updates a **Release PR**.
3) Merge the Release PR.
4) release-please creates `vX.Y.Z` tag + GitHub Release.
5) cargo-dist runs on the tag and uploads binaries.

## Known gotchas
- cargo-dist requires tag version to match `sdbh/Cargo.toml` version.
- Avoid manually pushing tags unless versions are confirmed to match.
