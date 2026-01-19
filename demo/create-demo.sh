#!/bin/bash
# create-demo.sh - Populate demo database and optionally record demo GIF

set -e

DEMO_DB="/tmp/demo-sdbh.sqlite"
PWD_PATH="$PWD"

echo "🎬 Creating sdbh demo..."
echo ""

# Clean up any existing demo database
if [ -f "$DEMO_DB" ]; then
    echo "📝 Removing existing demo database..."
    rm -f "$DEMO_DB"
fi

echo "📊 Populating demo database with sample commands..."
echo ""

export SDBH_DB=/tmp/demo-sdbh.sqlite

# Git commands
echo "  → Adding git commands..."
sdbh log --cmd "git status" --pwd "$PWD_PATH" --epoch 0 --ppid $$ --salt 0
sdbh log --cmd "git log --oneline -10" --pwd "$PWD_PATH" --epoch 1 --ppid $$ --salt 0
sdbh log --cmd "git diff HEAD~1" --pwd "$PWD_PATH" --epoch 2 --ppid $$ --salt 0
sdbh log --cmd "git branch -a" --pwd "$PWD_PATH" --epoch 3 --ppid $$ --salt 0
sdbh log --cmd "git checkout main" --pwd "$PWD_PATH" --epoch 4 --ppid $$ --salt 0

# Cargo/Rust commands
echo "  → Adding cargo commands..."
sdbh log --cmd "cargo build --release" --pwd "$PWD_PATH" --epoch 5 --ppid $$ --salt 0
sdbh log --cmd "cargo test" --pwd "$PWD_PATH" --epoch 6 --ppid $$ --salt 0
sdbh log --cmd "cargo fmt" --pwd "$PWD_PATH" --epoch 7 --ppid $$ --salt 0
sdbh log --cmd "cargo clippy" --pwd "$PWD_PATH" --epoch 8 --ppid $$ --salt 0

# npm/Node commands
echo "  → Adding npm commands..."
sdbh log --cmd "npm install" --pwd "$PWD_PATH" --epoch 9 --ppid $$ --salt 0
sdbh log --cmd "npm test" --pwd "$PWD_PATH" --epoch 10 --ppid $$ --salt 0
sdbh log --cmd "npm run build" --pwd "$PWD_PATH" --epoch 11 --ppid $$ --salt 0

# Make/build commands
echo "  → Adding make commands..."
sdbh log --cmd "make all" --pwd "$PWD_PATH" --epoch 12 --ppid $$ --salt 0
sdbh log --cmd "make test" --pwd "$PWD_PATH" --epoch 13 --ppid $$ --salt 0
sdbh log --cmd "make clean" --pwd "$PWD_PATH" --epoch 14 --ppid $$ --salt 0

# Search/find commands
echo "  → Adding search commands..."
sdbh log --cmd "grep -r 'TODO' src/" --pwd "$PWD_PATH" --epoch 15 --ppid $$ --salt 0
sdbh log --cmd "find . -name '*.rs' -type f" --pwd "$PWD_PATH" --epoch 16 --ppid $$ --salt 0

# API/curl commands
echo "  → Adding curl commands..."
sdbh log --cmd "curl -X GET https://api.example.com/health" --pwd "$PWD_PATH" --epoch 17 --ppid $$ --salt 0
sdbh log --cmd "curl -H 'Accept: application/json' https://api.example.com/users" --pwd "$PWD_PATH" --epoch 18 --ppid $$ --salt 0

# Python commands
echo "  → Adding python commands..."
sdbh log --cmd "python -m pytest tests/" --pwd "$PWD_PATH" --epoch 19 --ppid $$ --salt 0
sdbh log --cmd "pip install -r requirements.txt" --pwd "$PWD_PATH" --epoch 20 --ppid $$ --salt 0

echo ""
echo "✅ Demo database created at: $DEMO_DB"
echo "📊 Total commands: 21"
echo ""

# Check if VHS is available
if command -v vhs >/dev/null 2>&1; then
    echo "🎥 VHS found! Do you want to record the demo now? (y/n)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo "🎬 Recording demo with VHS..."
        SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        cd "$SCRIPT_DIR" && vhs demo.tape
        echo ""
        if [ -f "../demo.gif" ]; then
            echo "✅ Demo GIF created: demo.gif"
            echo "📏 Size: $(du -h ../demo.gif | cut -f1)"
            echo ""
            echo "🔍 Opening demo for review..."
            open ../demo.gif 2>/dev/null || xdg-open ../demo.gif 2>/dev/null || echo "   (Could not open automatically - please check demo.gif)"
        else
            echo "❌ Demo GIF was not created. Check demo/demo.tape for errors."
        fi
    else
        echo "ℹ️  Skipping demo recording."
        echo "   Run 'cd demo && vhs demo.tape' when ready to record."
    fi
else
    echo "ℹ️  VHS not found. Install with: brew install vhs"
    echo "   Then run: cd demo && vhs demo.tape"
fi

echo ""
echo "🧪 Test the demo database:"
echo "   sdbh --db $DEMO_DB search git --all"
echo "   sdbh --db $DEMO_DB summary cargo --all"
echo "   sdbh --db $DEMO_DB stats top --all --limit 5"
echo ""