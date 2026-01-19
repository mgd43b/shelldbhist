# Demo Recording Guide

This guide helps you create a professional demo GIF for the README.

## Quick Option: Use asciinema + agg

### 1. Install tools
```bash
brew install asciinema
cargo install --locked agg
```

### 2. Record the demo
```bash
# Start recording
asciinema rec demo.cast

# Now perform these actions in the recording:
# - Type: echo "Demo of sdbh features"
# - Show: sdbh search kubectl --all --limit 5
# - Show: sdbh summary git --all
# - Show: sdbh stats top --all --limit 5
# - Show: sdbh list --all --limit 10
# - Type: echo "Install: brew install mgd43b/taps/sdbh"
# - Exit: Press Ctrl+D to stop recording
```

### 3. Convert to GIF
```bash
agg demo.cast demo.gif --speed 1.5
```

### 4. Optimize GIF size
```bash
# Install gifsicle if needed
brew install gifsicle

# Optimize
gifsicle -O3 --colors 256 demo.gif -o demo-optimized.gif
mv demo-optimized.gif demo.gif
```

## Alternative: Simple Screenshot Recording

If GIF recording is problematic, you can use macOS screenshot tool:

1. Open Terminal
2. Press Cmd+Shift+5
3. Select "Record Selected Portion"
4. Record a 30-second demo
5. Convert to GIF using online tools or:
   ```bash
   ffmpeg -i demo.mov -vf "fps=15,scale=1200:-1:flags=lanczos" demo.gif
   ```

## Demo Script

Here's a suggested script for your demo:

```bash
# Clear terminal first
clear

# 1. Introduction
echo "# sdbh - Shell Database History"
echo "# SQLite-backed shell history with fuzzy search"
sleep 2

# 2. Search example
clear
echo "# Search for commands"
sdbh search git --all --limit 5
sleep 3

# 3. Summary example
clear
echo "# View command summaries"
sdbh summary --all | head -10
sleep 3

# 4. Stats example
clear
echo "# Check your top commands"
sdbh stats top --all --limit 5
sleep 3

# 5. List example
clear
echo "# Browse your history"
sdbh list --all --limit 10
sleep 3

# 6. Closing
clear
echo "# Install: brew install mgd43b/taps/sdbh"
echo "# GitHub: github.com/mgd43b/shelldbhist"
sleep 3
```

## What to Show

Focus on these killer features:
1. **Search** - Fast substring search across history
2. **Summary** - Grouped commands with usage counts
3. **Stats** - Top commands and analytics
4. **fzf integration** - Mention Ctrl+R replacement
5. **Installation** - Simple brew install

## Demo Best Practices

- Keep it under 30 seconds
- Use a clean terminal with good contrast
- Show real commands (not just echo)
- Highlight the Ctrl+R replacement feature
- End with installation instructions
- Use a reasonable terminal size (1200x600 or similar)
- Good font size (14-16pt)

## Alternative Tools

If asciinema doesn't work:
- **termtosvg** - Creates SVG animations
- **ttygif** - Another GIF creator
- **peek** - Linux screen recorder
- **licecap** - Cross-platform GIF recorder
- **ScreenToGif** - Windows GIF recorder

## Where to Place the GIF

Add to README.md right after the project description:
```markdown
# sdbh

[![CI](https://github.com/mgd43b/shelldbhist/actions/workflows/ci.yml/badge.svg)]...

Shell DB History (`sdbh`) stores your shell command history in a local SQLite database.

![sdbh demo](demo.gif)

## Features
...
```