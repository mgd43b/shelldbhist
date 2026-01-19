# Development

## Repo layout
- Rust crate lives in `./sdbh`

## Common commands
```bash
cd sdbh
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## Local run
```bash
cd sdbh
cargo run -- --help
```

## Testing with isolated databases

Use the `SDBH_DB` environment variable to test with isolated databases without affecting your personal history:

```bash
# Run with a temporary test database
SDBH_DB=/tmp/test-sdbh.sqlite cargo run -- list --all

# Test import functionality
SDBH_DB=/tmp/test-sdbh.sqlite cargo run -- import --from ~/.dbhist

# Test with a specific test database
SDBH_DB=./test-data/fixture.sqlite cargo run -- search test --all

# Clean up test database
rm /tmp/test-sdbh.sqlite
```

This is especially useful for:
- Testing new features without polluting your personal history
- Reproducing bugs with specific test data
- Running integration tests with known data sets
- Developing demo content (see `demo/` directory)

**Note:** The `--db` flag takes precedence over `SDBH_DB` if both are specified.