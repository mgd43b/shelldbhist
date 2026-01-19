# Shell integration

`sdbh` supports two modes:

## Hook mode (recommended)
Logs the *last executed* command each time your prompt renders.

Bash:

```bash
eval "$(sdbh shell --bash)"
```

Zsh:

```bash
eval "$(sdbh shell --zsh)"
```

## Intercept mode (more invasive)
Logs commands *as they execute*.

Bash:

```bash
eval "$(sdbh shell --bash --intercept)"
```

Zsh:

```bash
eval "$(sdbh shell --zsh --intercept)"
```

## Troubleshooting

### Bash hook requirements
For bash hook mode, `HISTTIMEFORMAT="%s "` is required so `history 1` includes an epoch timestamp.

### Bash troubleshooting

```bash
type __sdbh_prompt
echo "$PROMPT_COMMAND"

# Reload after editing rc files
eval "$(sdbh shell --bash)"
```
