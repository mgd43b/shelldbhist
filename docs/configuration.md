# Configuration

`sdbh` can be configured via an optional `~/.sdbh.toml` file.

If the file does not exist, defaults apply.

## Configuration file location
- **Path**: `~/.sdbh.toml`
- **Format**: TOML

## Example configuration (with defaults)

```toml
[log]
ignore_exact = ["echo hello", "make test"]
ignore_prefix = ["cd ", "sdbh "]
use_builtin_ignores = true

[cleanup]
allow_list = [
    "curl *",
    "wget *",
    "git commit*",
]
size_threshold_small = 500
size_threshold_medium = 2048
size_threshold_large = 10240

[fzf]
height = "60%"
layout = "reverse"
border = "rounded"
color = "fg:#d0d0d0,bg:#121212,hl:#5f87af"
color_header = "fg:#87afaf"
color_pointer = "fg:#ff8700"
color_marker = "fg:#87ff00"
preview_window = "right:50%"
bind = [
    "ctrl-k:kill-line",
    "ctrl-j:accept",
]
binary_path = "/usr/local/bin/fzf"
```

## Sections

### `[log]`
Controls which commands are logged.

### `[cleanup]`
Controls garbage detection behavior.

### `[fzf]`
Controls fzf appearance and keybindings.
