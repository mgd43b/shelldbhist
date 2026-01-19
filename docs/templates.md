# Command templates

`sdbh template` supports reusable command patterns with variable substitution.

Templates are stored in `~/.sdbh/templates/` as TOML files.

## Usage

```bash
sdbh template --list
sdbh template my-template --var key=value
sdbh template --create my-template
sdbh template --delete my-template
```

## Example template

```toml
id = "git-commit"
name = "Git Commit"
description = "Git commit with conventional format"
command = "git add . && git commit -m '{type}: {message}'"

[[variables]]
name = "type"
description = "Commit type (feat, fix, docs, etc.)"
required = true

[[variables]]
name = "message"
description = "Commit message"
required = true
```
