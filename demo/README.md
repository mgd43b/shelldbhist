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
3. **VHS sets `SDBH_DB=/tmp/demo-sdbh.sqlite`** environment variable (invisible to viewers)
4. **sdbh automatically reads `SDBH_DB`** - no need for `--db` flags!
5. Commands in the demo use clean syntax: `sdbh search build --all`
6. VHS records the terminal session and outputs `demo.gif`

### Environment Variable Magic

The demo uses the `SDBH_DB` environment variable for database isolation:

- **VHS tape sets it invisibly**: `Env SDBH_DB "/tmp/demo-sdbh.sqlite"` in `demo.tape`
- **sdbh reads it automatically**: All commands use the demo database without extra flags
- **Result**: Clean command syntax perfect for demos and screenshots
- **Your personal history stays private**: The demo never touches `~/.sdbh.sqlite`

This same technique works for:
- Recording tutorials without exposing personal history
- Testing with isolated databases
- Running CI/CD pipelines with separate test databases
- Creating reproducible demo environments

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