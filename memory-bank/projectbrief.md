# Project Brief: sdbh

## Summary
`sdbh` is an open-source, portable command history database tool inspired by `dbhist.sh`. It records shell commands into a local SQLite database and provides fast search, grouped summaries, raw history browsing, and import/merge from existing `dbhist` SQLite databases.

## Goals
- Provide a cross-platform Rust CLI called **`sdbh`**.
- Support **bash** and **zsh**.
- Provide two integration modes:
  1) **Prompt hook** integration (bash `PROMPT_COMMAND`, zsh `precmd`) calling `sdbh log ...`
  2) **More invasive widget/plugin** integration that intercepts command execution (shell function/widget wrappers)
- Store data in SQLite at default path: **`~/.sdbh.sqlite`**.
- Import/merge existing `dbhist` SQLite data with **hash-based de-duplication**.
- Ship with tests + documentation and be released via GitHub.

## Non-goals (initial)
- Remote sync / cloud storage
- Collecting sensitive telemetry
- Full-shell replacement or terminal emulator integration

## Success Criteria
- `sdbh` builds on macOS and Linux in CI.
- Shell integration scripts log commands reliably for bash and zsh.
- `sdbh import` merges existing `~/.dbhist` without creating duplicates.
- Test suite covers core logic and SQLite interactions.
- README documents install + setup + import + examples.
