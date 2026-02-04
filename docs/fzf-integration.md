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

### Bash 3.2 (macOS default) - Immediate Execution

**Note:** Bash 3.2 (the default on macOS) cannot modify the readline buffer from `bind -x`, so selected commands execute immediately. This is a limitation of bash 3.2, not sdbh.

```bash
sdbh-fzf-history() {
  local selected
  selected=$(sdbh list --all --fzf 2>/dev/null)
  if [[ -n "$selected" ]]; then
    history -s "$selected"  # Add to history for up-arrow access
    echo "¶ $selected"       # Show what's executing
    eval "$selected"         # Execute immediately
  fi
}
bind -x '"\C-r": sdbh-fzf-history'
```

**UX Flow:**
1. Press **Ctrl+R** ’ fzf opens with your history
2. Type to **search**, use arrows to navigate
3. Press **Enter** ’ command shows with `¶` prefix and executes immediately
4. Command is in your bash history (up-arrow works)
5. Command is logged to sdbh database via your `__sdbh_prompt` hook

**Trade-off:** You cannot edit the command before execution. If you need to edit, press Ctrl+C in fzf and type the command manually.

### Bash 4.0+ - Edit Before Execute

If you're using bash 4.0 or later, you can use this version that places the command on your readline buffer for editing:

```bash
sdbh-fzf-history() {
  selected=$(sdbh list --all --fzf 2>/dev/null)
  [[ -n "$selected" ]] && READLINE_LINE="$selected" && READLINE_POINT=${#selected}
}
bind -x '"\C-r": sdbh-fzf-history'
```

**Note:** This does NOT work on bash 3.2 (macOS default). Use the immediate execution version above instead.

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
