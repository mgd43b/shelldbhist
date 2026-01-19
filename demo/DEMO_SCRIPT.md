# Demo Recording Script

This script helps you create a sanitized demo GIF that doesn't contain personal commands.

## Prerequisites

1. Clean database for demo:
```bash
# Create a temporary demo database
export SDBH_DB=/tmp/demo-sdbh.sqlite

# Or use the --db flag with each command
```

2. Add sample commands to database:
```bash
# Add example commands to show off features

# Git commands
sdbh --db /tmp/demo-sdbh.sqlite log "git status" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "git log --oneline -10" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "git diff HEAD~1" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "git branch -a" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "git checkout main" "$PWD" 0

# Cargo/Rust commands
sdbh --db /tmp/demo-sdbh.sqlite log "cargo build --release" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "cargo test" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "cargo fmt" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "cargo clippy" "$PWD" 0

# npm/Node commands
sdbh --db /tmp/demo-sdbh.sqlite log "npm install" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "npm test" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "npm run build" "$PWD" 0

# Make/build commands
sdbh --db /tmp/demo-sdbh.sqlite log "make all" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "make test" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "make clean" "$PWD" 0

# Search/find commands
sdbh --db /tmp/demo-sdbh.sqlite log "grep -r 'TODO' src/" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "find . -name '*.rs' -type f" "$PWD" 0

# API/curl commands
sdbh --db /tmp/demo-sdbh.sqlite log "curl -X GET https://api.example.com/health" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "curl -H 'Accept: application/json' https://api.example.com/users" "$PWD" 0

# Python commands
sdbh --db /tmp/demo-sdbh.sqlite log "python -m pytest tests/" "$PWD" 0
sdbh --db /tmp/demo-sdbh.sqlite log "pip install -r requirements.txt" "$PWD" 0
```

## VHS Tape Script

Update `demo.tape` to use the demo database:

```tape
Output demo.gif

Set FontSize 16
Set Width 1400
Set Height 800
Set Theme "Catppuccin Mocha"
Set TypingSpeed 50ms

Type "# sdbh - Shell Database History"
Enter
Sleep 1s
Type "# SQLite-backed command history with fuzzy search"
Enter
Sleep 2s

Type "clear"
Enter
Sleep 500ms

# Search example
Type "# Search for git commands"
Enter
Sleep 1s
Type "sdbh --db /tmp/demo-sdbh.sqlite search git --all"
Enter
Sleep 3s

Type "clear"
Enter
Sleep 500ms

# Summary example
Type "# View command summaries"
Enter
Sleep 1s
Type "sdbh --db /tmp/demo-sdbh.sqlite summary cargo --all"
Enter
Sleep 3s

Type "clear"
Enter
Sleep 500ms

# Stats example
Type "# Check your top commands"
Enter
Sleep 1s
Type "sdbh --db /tmp/demo-sdbh.sqlite stats top --all --limit 5"
Enter
Sleep 3s

Type "clear"
Enter
Sleep 500ms

# List example
Type "# Browse your command history"
Enter
Sleep 1s
Type "sdbh --db /tmp/demo-sdbh.sqlite list --all --limit 10"
Enter
Sleep 3s

Type "clear"
Enter
Sleep 500ms

# Final message
Type "# Install: brew install mgd43b/taps/sdbh"
Enter
Sleep 1s
Type "# GitHub: github.com/mgd43b/shelldbhist"
Enter
Sleep 3s
```

## Recording Steps

1. **Prepare demo database:**
   ```bash
   rm -f /tmp/demo-sdbh.sqlite
   # Run the sample commands above to populate
   ```

2. **Record with VHS:**
   ```bash
   vhs demo.tape
   ```

3. **Verify output:**
   ```bash
   open demo.gif
   # Check that no personal commands appear
   # Check file size (should be reasonable, <5MB)
   ```

4. **Add to README:**
   ```bash
   # Edit README.md to add demo.gif after project description
   git add demo.gif README.md
   git commit -m "feat: add sanitized demo GIF showcasing sdbh features"
   git push origin main
   ```

## Tips for Better Demos

- **Keep it short**: 20-30 seconds max
- **Show key features**: search, summary, stats, list
- **Use clean commands**: Generic examples everyone understands
- **Good contrast**: Light text on dark background or vice versa
- **Readable font size**: 14-16pt minimum
- **Clear terminal**: No cluttered prompts or distractions

## What to Show

Focus on these killer features:
1. **Search** - Fast command lookup
2. **Summary** - Grouped by command with counts
3. **Stats** - Top commands analytics
4. **fzf integration** - Mention but hard to demo in GIF
5. **Easy install** - Homebrew one-liner

## Common Issues

**Issue**: VHS shows my actual history
**Fix**: Use `--db /tmp/demo-sdbh.sqlite` flag on all commands

**Issue**: Demo is too long/large
**Fix**: Reduce Sleep times, trim example count

**Issue**: Can't see commands clearly
**Fix**: Increase FontSize in demo.tape

**Issue**: Personal info still visible
**Fix**: Check $SDBH_DB environment variable, clear terminal history