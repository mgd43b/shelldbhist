# Demo Recording Files

This directory contains everything needed to create a professional demo GIF for sdbh.

## Quick Start

```bash
# Run the setup script (creates demo database and optionally records GIF)
./demo/create-demo.sh

# Or manually:
./demo/create-demo.sh    # Create demo database
vhs demo/demo.tape       # Record the GIF
```

## Files

- **`create-demo.sh`** - Populates `/tmp/demo-sdbh.sqlite` with 21 sample commands and optionally records the demo
- **`demo.tape`** - VHS tape script that records the demo GIF
- **`DEMO_RECORDING_GUIDE.md`** - Alternative recording methods (asciinema, screenshots, etc.)
- **`DEMO_SCRIPT.md`** - Original demo script and documentation

## How It Works

1. **`create-demo.sh`** creates a clean demo database at `/tmp/demo-sdbh.sqlite`
2. Adds sample commands (git, cargo, npm, make, curl, python, etc.)
3. Sets `SDBH_DB=/tmp/demo-sdbh.sqlite` environment variable
4. Commands in the demo use clean syntax without `--db` flags
5. VHS records the terminal session and outputs `demo.gif`

## Requirements

- **sdbh** installed and in PATH
- **VHS** for recording: `brew install vhs`

## Output

- Demo database: `/tmp/demo-sdbh.sqlite` (temporary, safe to delete)
- Demo GIF: `demo.gif` in project root (ignored by git, can be regenerated)

## Tips

- The demo database is separate from your personal `~/.sdbh.sqlite`
- Safe to run multiple times (cleans up and recreates)
- Edit `demo.tape` to customize the recording
- Adjust timing with `Sleep` commands in the tape script