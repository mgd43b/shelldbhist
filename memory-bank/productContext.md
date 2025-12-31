# Product Context: sdbh

## Why this exists
Shell history files are hard to search and lose context across sessions. `sdbh` persists command history in a structured local database so users can query what they ran, when, and where.

## Problems solved
- Fast search across a long history
- Grouped “what do I run most / when did I last run this?” summaries
- Raw chronological history browsing
- Reliable storage independent of shell history truncation
- Migration path from existing `dbhist.sh` SQLite databases

## UX principles
- Local-first: data stays on the machine
- Safe defaults: writes are robust; import de-dups
- Minimal friction: drop-in shell setup
- Portable: one binary per platform via GitHub Releases
