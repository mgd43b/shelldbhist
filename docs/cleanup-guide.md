# Cleanup / Garbage detection

`sdbh cleanup` identifies likely garbage entries in your history (accidental pastes, binary content, etc.) with conservative heuristics.

## Modes

### Scan mode

```bash
sdbh cleanup --scan
sdbh cleanup --scan --min-score 60.0
sdbh cleanup --scan --format json
```

### Interactive mode

```bash
sdbh cleanup --interactive
sdbh cleanup --interactive --min-score 60.0
```

### Auto mode

```bash
sdbh cleanup --auto
sdbh cleanup --auto --yes
sdbh cleanup --auto --min-score 70.0 --yes
```

Auto mode deletes **high-confidence** entries only.

## Configuration
See [Configuration](configuration.md) for the `[cleanup]` section.
