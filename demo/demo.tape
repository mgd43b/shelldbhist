# VHS documentation: https://github.com/charmbracelet/vhs
#
# Output demo.gif showing sdbh features
#
Output ../demo.gif

# Settings
Set FontSize 16
Set Width 1400
Set Height 800
Set Theme "Catppuccin Mocha"
Set TypingSpeed 50ms
Set PlaybackSpeed 1.0

# Set demo database environment variable (VHS command)
Env SDBH_DB "/tmp/demo-sdbh.sqlite"

# Title
Type "# sdbh - Shell Database History"
Enter
Sleep 1s
Type "# SQLite-backed shell history with fuzzy search"
Enter
Sleep 2s

Type "clear"
Enter
Sleep 500ms

# First, show some example commands being logged
Type "# Let's add some commands to our history database"
Enter
Sleep 1s
Type 'echo "Working with kubectl"'
Enter
Sleep 500ms
Type 'echo "Building with cargo"'
Enter
Sleep 500ms
Type 'echo "Managing with docker"'
Enter
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# Show search functionality
Type "# Search for commands containing 'build'"
Enter
Sleep 1s
Type "sdbh search build --all --limit 5"
Enter
Sleep 2s

Type "clear"
Enter
Sleep 500ms

# Show summary functionality
Type "# View command summaries grouped by usage"
Enter
Sleep 1s
Type "sdbh summary cargo --all"
Enter
Sleep 2s

Type "clear"
Enter
Sleep 500ms

# Show stats
Type "# Check your top commands"
Enter
Sleep 1s
Type "sdbh stats top --all --limit 5"
Enter
Sleep 2s

Type "clear"
Enter
Sleep 500ms

# Show the killer feature - list all
Type "# The killer feature: Interactive fuzzy search with Ctrl+R"
Enter
Sleep 1s
Type "# Replaces your default history search with fuzzy search"
Enter
Sleep 2s

Type "clear"
Enter
Sleep 500ms

# Show list preview
Type "# Browse your command history"
Enter
Sleep 1s
Type "sdbh list --all --limit 10"
Enter
Sleep 3s

Type "clear"
Enter
Sleep 500ms

# Final message
Type "# Install with: brew install mgd43b/taps/sdbh"
Enter
Sleep 1s
Type "# Learn more: github.com/mgd43b/shelldbhist"
Enter
Sleep 3s