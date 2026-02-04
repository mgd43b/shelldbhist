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

**Important:** The Ctrl+R integration requires adding a shell function to your shell configuration file. This is **separate** from the shell logging integration (see [shell-integration.md](shell-integration.md)).

### How it works

1. Press **Ctrl+R** in your terminal
2. The **fzf** interface opens with your command history
3. Type to **fuzzy search** through commands
4. Use **arrow keys** or keep typing to select the command you want
5. Press **Enter** to select
6. The command **appears on your command line** (ready for you to edit or execute)
7. Press **Enter again** to execute the command

The selected command is placed on your readline buffer - it does not execute automatically. This allows you to review and edit it before running.

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

## Troubleshooting

### Ctrl+R closes fzf but command doesn't appear on command line

**Symptom:** When you press Enter in the fzf interface, fzf closes but nothing appears on your command line.

**Cause:** The shell function (`sdbh-fzf-history` or `sdbh-history-widget`) is missing or not loaded.

**Solution:**

1. **Verify the function exists** in your shell config:
   - Bash: Check `~/.bashrc` or `~/.bash_profile` (macOS uses `.bash_profile` by default)
   - Zsh: Check `~/.zshrc`

2. **Add the function** if missing (see "Ctrl+R history replacement" section above)

3. **Reload your shell config:**
   ```bash
   # Bash
   source ~/.bashrc  # or ~/.bash_profile on macOS
   
   # Zsh
   source ~/.zshrc
   ```

4. **Verify the binding:**
   ```bash
   # Bash
   bind -P | grep 'sdbh'
   
   # Zsh
   bindkey | grep sdbh
   ```

5. **Test the command directly:**
   ```bash
   sdbh list --all --fzf
   ```
   This should open fzf. When you select a command and press Enter, it should print the command to stdout.

### fzf not found

**Symptom:** Error message about fzf not being found.

**Solution:** Install fzf via your package manager:
```bash
# macOS
brew install fzf

# Ubuntu/Debian
sudo apt install fzf

# Fedora
sudo dnf install fzf
```

### Commands appear on command line but don't execute

This is **expected behavior**. The Ctrl+R integration places the selected command on your command line for review/editing. Press Enter a second time to execute it.

If you want immediate execution, use a custom function with `eval` (see "Advanced shell helpers" above).
