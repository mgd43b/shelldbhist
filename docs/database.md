# Database

Default DB path: `~/.sdbh.sqlite`

## Override database location

### Environment variable

```bash
export SDBH_DB=/tmp/demo-sdbh.sqlite
sdbh list --all

SDBH_DB=/tmp/demo-sdbh.sqlite sdbh search build --all
```

### Command-line flag

```bash
sdbh --db /path/to/file.sqlite list --all
```

Priority order: `--db` flag > `SDBH_DB` environment variable > `~/.sdbh.sqlite`.

## Database operations

```bash
sdbh db health
sdbh db optimize
sdbh db stats
sdbh db schema
```
