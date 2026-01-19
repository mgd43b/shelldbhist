# fzf + Preview

`sdbh` integrates with [fzf](https://github.com/junegun/fzf) for interactive command selection.

## Requirements
- Install `fzf` (via your package manager)

## Basic usage

```bash
sdbh list --fzf
sdbh search "git" --fzf
sdbh summary --fzf
```

## What the preview pane shows

The preview pane shows **your personal usage patterns** from the database (command counts, recent executions, directories used, etc.).

It is not a manpage/web-derived explanation system.

## Context-aware subcommand recognition

When viewing **complete commands from your history**, `sdbh` recognizes some common tools + subcommands and adds small contextual hints.

Examples:
- Git: `git status`, `git log`, `git diff`, `git branch`
- Docker: `docker run`, `docker build`, `docker ps`, `docker exec`, `docker logs`
- Kubernetes: `kubectl get`, `kubectl describe`, `kubectl logs`, `kubectl apply`
- Cargo: `cargo build`, `cargo test`, `cargo check`, `cargo fmt`, `cargo clippy`
- NPM: `npm install`, `npm start`, `npm run`, `npm test`, `npm build`
- Make: `make clean`, `make test`, `make install`

If you preview a bare command like `kubectl` (no subcommand), you should expect little/no contextual hinting.

## Ctrl+R history replacement

Bash:

```bash
sdbh-fzf-history() {
  selected=$(sdbh list --all --fzf 2>/dev/null)
  [[ -n "$selected" ]] && READLINE_LINE="$selected" && READLINE_POINT=${#selected}
}
bind -x '"\C-r": sdbh-fzf-history'
```

Zsh:

```zsh
function sdbh-history-widget() {
  selected=$(sdbh list --all --fzf 2>/dev/null)
  [[ -n "$selected" ]] && LBUFFER="$selected"
  zle reset-prompt
}
zle -N sdbh-history-widget
bindkey '^R' sdbh-history-widget
```

## Advanced shell helpers

If you want shortcuts that immediately search a subset of commands:

```bash
sdbh-git() {
  local cmd
  cmd=$(sdbh search "git" --all --fzf 2>/dev/null)
  [[ -n "$cmd" ]] && eval "$cmd"
}
```
