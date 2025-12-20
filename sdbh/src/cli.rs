use crate::db::{ensure_hash_index, import_from_db, insert_history, open_db};
use crate::domain::{DbConfig, HistoryRow};
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "sdbh", version, about = "Shell DB History (sdbh)")]
pub struct Cli {
    /// Path to SQLite database
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Insert one history row (intended for shell integration)
    Log(LogArgs),

    /// Grouped-by-command summary (last seen + count)
    Summary(SummaryArgs),

    /// Raw chronological history
    List(ListArgs),

    /// Import/merge another dbhist-compatible SQLite database
    Import(ImportArgs),

    /// Print shell integration snippets
    Shell(ShellArgs),
}

#[derive(Parser, Debug)]
pub struct LogArgs {
    #[arg(long)]
    pub cmd: String,

    #[arg(long)]
    pub epoch: i64,

    #[arg(long)]
    pub ppid: i64,

    #[arg(long)]
    pub pwd: String,

    #[arg(long)]
    pub salt: i64,

    #[arg(long)]
    pub hist_id: Option<i64>,
}

#[derive(Parser, Debug)]
pub struct SummaryArgs {
    /// Query substring (or prefix if --starts)
    pub query: Option<String>,

    #[arg(long, default_value_t = 100)]
    pub limit: u32,

    #[arg(long)]
    pub starts: bool,

    #[arg(long)]
    pub all: bool,

    #[arg(long)]
    pub pwd: bool,

    #[arg(long, conflicts_with = "under")]
    pub here: bool,

    #[arg(long, conflicts_with = "here")]
    pub under: bool,

    #[arg(long)]
    pub verbose: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum OutputFormat {
    Table,
    Json,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

#[derive(Parser, Debug)]
pub struct ListArgs {
    /// Query substring
    pub query: Option<String>,

    #[arg(long, default_value_t = 100)]
    pub limit: u32,

    #[arg(long, default_value_t = 0)]
    pub offset: u32,

    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    #[arg(long)]
    pub all: bool,

    #[arg(long, conflicts_with = "under")]
    pub here: bool,

    #[arg(long, conflicts_with = "here")]
    pub under: bool,
}

#[derive(Parser, Debug)]
pub struct ImportArgs {
    /// Source SQLite path (dbhist compatible). Can be provided multiple times.
    #[arg(long = "from")]
    pub from_paths: Vec<PathBuf>,

    /// Destination db path (defaults to ~/.sdbh.sqlite)
    #[arg(long = "to")]
    pub to: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct ShellArgs {
    /// Print bash integration
    #[arg(long)]
    pub bash: bool,

    /// Print zsh integration
    #[arg(long)]
    pub zsh: bool,

    /// Print intercept-style integration (more invasive)
    #[arg(long)]
    pub intercept: bool,
}

pub fn run(cli: Cli) -> Result<()> {
    let db_path = cli.db.unwrap_or_else(DbConfig::default_path);
    let cfg = DbConfig { path: db_path };

    match cli.command {
        Commands::Log(args) => cmd_log(cfg, args),
        Commands::Summary(args) => cmd_summary(cfg, args),
        Commands::List(args) => cmd_list(cfg, args),
        Commands::Import(args) => cmd_import(cfg, args),
        Commands::Shell(args) => cmd_shell(args),
    }
}

fn cmd_log(cfg: DbConfig, args: LogArgs) -> Result<()> {
    let mut conn = open_db(&cfg)?;
    ensure_hash_index(&conn)?;

    let row = HistoryRow {
        hist_id: args.hist_id,
        cmd: args.cmd,
        epoch: args.epoch,
        ppid: args.ppid,
        pwd: args.pwd,
        salt: args.salt,
    };

    insert_history(&mut conn, &row)?;
    Ok(())
}

fn session_filter(all: bool) -> Option<(i64, i64)> {
    if all {
        None
    } else {
        // In hook mode, salt and ppid come from the current shell integration.
        // For querying "current session" we read env vars set by the hook.
        let salt = std::env::var("SDBH_SALT").ok()?.parse::<i64>().ok()?;
        let ppid = std::env::var("SDBH_PPID").ok()?.parse::<i64>().ok()?;
        Some((salt, ppid))
    }
}

fn location_filter(here: bool, under: bool) -> Option<(String, bool)> {
    if !(here || under) {
        return None;
    }
    let pwd = std::env::current_dir().ok()?.to_string_lossy().to_string();
    Some((pwd, under))
}

fn cmd_summary(cfg: DbConfig, args: SummaryArgs) -> Result<()> {
    let conn = open_db(&cfg)?;

    let (sql, bind) = build_summary_sql(&args)?;
    if args.verbose {
        eprintln!("db: {}", cfg.path.display());
        eprintln!("sql: {}", sql);
    }

    let mut stmt = conn.prepare(&sql)?;

    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;
    while let Some(r) = rows.next()? {
        let id_max: i64 = r.get(0)?;
        let dt: String = r.get(1)?;
        let count: i64 = r.get(2)?;
        let cmd: String = r.get(3)?;
        if args.pwd {
            let pwd: String = r.get(4)?;
            println!(
                "{id:>6} | {dt} | {count:>6} | {pwd} > {cmd}",
                id = id_max,
                dt = dt,
                count = count,
                pwd = pwd,
                cmd = cmd
            );
        } else {
            println!(
                "{id:>6} | {dt} | {count:>6} | {cmd}",
                id = id_max,
                dt = dt,
                count = count,
                cmd = cmd
            );
        }
    }

    Ok(())
}

fn build_summary_sql(args: &SummaryArgs) -> Result<(String, Vec<String>)> {
    let mut bind: Vec<String> = vec![];

    let mut select = String::from(
        "SELECT max(id) as mid, datetime(max(epoch), 'unixepoch', 'localtime') as dt, count(*) as cnt, cmd",
    );
    if args.pwd {
        select.push_str(", pwd");
    }

    let mut sql = format!("{select} FROM history WHERE 1=1 ");

    if let Some((salt, ppid)) = session_filter(args.all) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    if let Some(q) = &args.query {
        let like = if args.starts {
            format!("{}%", q)
        } else {
            format!("%{}%", q)
        };
        sql.push_str("AND cmd LIKE ? ESCAPE '\\' ");
        bind.push(escape_like(&like));
    }

    if let Some((pwd, under)) = location_filter(args.here, args.under) {
        if under {
            sql.push_str("AND pwd LIKE ? ESCAPE '\\' ");
            bind.push(escape_like(&format!("{}%", pwd)));
        } else {
            sql.push_str("AND pwd = ? ");
            bind.push(pwd);
        }
    }

    sql.push_str("GROUP BY cmd ");
    if args.pwd {
        sql.push_str(", pwd ");
    }

    sql.push_str("ORDER BY max(id) DESC ");
    sql.push_str("LIMIT ?");
    bind.push(args.limit.to_string());

    Ok((sql, bind))
}

fn cmd_list(cfg: DbConfig, args: ListArgs) -> Result<()> {
    let conn = open_db(&cfg)?;
    let (sql, bind) = build_list_sql(&args)?;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;

    match args.format {
        OutputFormat::Table => {
            while let Some(r) = rows.next()? {
                let id: i64 = r.get(0)?;
                let dt: String = r.get(1)?;
                let pwd: String = r.get(2)?;
                let cmd: String = r.get(3)?;
                println!("{id:>6} | {dt} | {pwd} | {cmd}");
            }
        }
        OutputFormat::Json => {
            // Minimal JSON without serde_json dependency for now.
            // (We can add serde_json later.)
            print!("[");
            let mut first = true;
            while let Some(r) = rows.next()? {
                let id: i64 = r.get(0)?;
                let epoch: i64 = r.get(4)?;
                let pwd: String = r.get(2)?;
                let cmd: String = r.get(3)?;

                if !first {
                    print!(",");
                }
                first = false;
                print!(
                    "{{\"id\":{},\"epoch\":{},\"pwd\":{},\"cmd\":{}}}",
                    id,
                    epoch,
                    json_string(&pwd),
                    json_string(&cmd)
                );
            }
            println!("]");
        }
    }

    Ok(())
}

fn build_list_sql(args: &ListArgs) -> Result<(String, Vec<String>)> {
    let mut bind: Vec<String> = vec![];
    let mut sql = String::from(
        "SELECT id, datetime(epoch, 'unixepoch', 'localtime') as dt, pwd, cmd, epoch FROM history WHERE 1=1 ",
    );

    if let Some((salt, ppid)) = session_filter(args.all) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    if let Some(q) = &args.query {
        sql.push_str("AND cmd LIKE ? ESCAPE '\\' ");
        bind.push(escape_like(&format!("%{}%", q)));
    }

    if let Some((pwd, under)) = location_filter(args.here, args.under) {
        if under {
            sql.push_str("AND pwd LIKE ? ESCAPE '\\' ");
            bind.push(escape_like(&format!("{}%", pwd)));
        } else {
            sql.push_str("AND pwd = ? ");
            bind.push(pwd);
        }
    }

    sql.push_str("ORDER BY epoch DESC, id DESC ");
    sql.push_str("LIMIT ? OFFSET ?");
    bind.push(args.limit.to_string());
    bind.push(args.offset.to_string());

    Ok((sql, bind))
}

fn cmd_import(mut cfg: DbConfig, args: ImportArgs) -> Result<()> {
    if let Some(to) = args.to {
        cfg.path = to;
    }

    let mut conn = open_db(&cfg)?;
    ensure_hash_index(&conn)?;

    if args.from_paths.is_empty() {
        anyhow::bail!("--from must be specified at least once");
    }

    let mut total_considered = 0u64;
    let mut total_inserted = 0u64;

    for p in &args.from_paths {
        let (considered, inserted) = import_from_db(&mut conn, p)?;
        eprintln!(
            "imported from {}: considered {}, inserted {}",
            p.display(),
            considered,
            inserted
        );
        total_considered += considered;
        total_inserted += inserted;
    }

    eprintln!(
        "total: considered {}, inserted {}",
        total_considered, total_inserted
    );

    Ok(())
}

fn cmd_shell(args: ShellArgs) -> Result<()> {
    // Default: print both if neither specified
    let want_bash = args.bash || !args.zsh;
    let want_zsh = args.zsh || !args.bash;

    if args.intercept {
        if want_bash {
            println!("{}", bash_intercept_snippet());
        }
        if want_zsh {
            println!("{}", zsh_intercept_snippet());
        }
        return Ok(());
    }

    if want_bash {
        println!("{}", bash_hook_snippet());
    }
    if want_zsh {
        println!("{}", zsh_hook_snippet());
    }

    Ok(())
}

fn bash_hook_snippet() -> String {
    r#"# sdbh bash hook mode
# Add to ~/.bashrc (and ensure HISTTIMEFORMAT="%s ")

export SDBH_SALT=${RANDOM}
export SDBH_PPID=$PPID

__sdbh_prompt() {
  [[ -n "${COMP_LINE}" ]] && return

  local line
  line="$(history 1)"

  # Parse: <hist_id> <epoch> <cmd...>
  local hist_id epoch cmd
  hist_id="${line%% *}";
  line="${line#* }"
  epoch="${line%% *}";
  cmd="${line#* }"

  [[ -z "${cmd}" ]] && return
  [[ ! "${epoch}" =~ ^[0-9]+$ ]] && return

  sdbh log --hist-id "${hist_id}" --epoch "${epoch}" --ppid "${PPID}" --pwd "${PWD}" --salt "${SDBH_SALT}" --cmd "${cmd}" 2>/dev/null || true
}

if ! [[ "${PROMPT_COMMAND}" =~ __sdbh_prompt ]]; then
  PROMPT_COMMAND="__sdbh_prompt${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
fi
"#
    .to_string()
}

fn zsh_hook_snippet() -> String {
    r#"# sdbh zsh hook mode
# Add to ~/.zshrc

export SDBH_SALT=$RANDOM
export SDBH_PPID=$$

sdbh_precmd() {
  local cmd epoch
  cmd="$(fc -ln -1)"
  epoch="$(date +%s)"
  [[ -z "${cmd}" ]] && return
  sdbh log --epoch "${epoch}" --ppid "$$" --pwd "${PWD}" --salt "${SDBH_SALT}" --cmd "${cmd}" 2>/dev/null || true
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd sdbh_precmd
"#
    .to_string()
}

fn bash_intercept_snippet() -> String {
    r#"# sdbh bash intercept mode (more invasive)
# Uses DEBUG trap to log each command before it runs.
# Add to ~/.bashrc

export SDBH_SALT=${RANDOM}
export SDBH_PPID=$PPID

__sdbh_debug_trap() {
  # Avoid recursion
  [[ -n "${__SDBH_IN_TRAP}" ]] && return
  __SDBH_IN_TRAP=1

  local cmd epoch
  cmd="${BASH_COMMAND}"
  epoch="$(date +%s)"

  # Filter out the trap itself / empty
  [[ -z "${cmd}" ]] && __SDBH_IN_TRAP= && return
  [[ "${cmd}" == sdbh* ]] && __SDBH_IN_TRAP= && return

  sdbh log --epoch "${epoch}" --ppid "${PPID}" --pwd "${PWD}" --salt "${SDBH_SALT}" --cmd "${cmd}" 2>/dev/null || true
  __SDBH_IN_TRAP=
}

trap '__sdbh_debug_trap' DEBUG
"#
    .to_string()
}

fn zsh_intercept_snippet() -> String {
    r#"# sdbh zsh intercept mode (more invasive)
# Uses preexec to log each command before it runs.
# Add to ~/.zshrc

export SDBH_SALT=$RANDOM
export SDBH_PPID=$$

function sdbh_preexec() {
  local cmd="$1"
  local epoch="$(date +%s)"
  [[ -z "${cmd}" ]] && return
  [[ "${cmd}" == sdbh* ]] && return
  sdbh log --epoch "${epoch}" --ppid "$$" --pwd "${PWD}" --salt "${SDBH_SALT}" --cmd "${cmd}" 2>/dev/null || true
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec sdbh_preexec
"#
    .to_string()
}

fn escape_like(s: &str) -> String {
    // Escape LIKE wildcards and backslash itself
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_like_escapes_wildcards() {
        assert_eq!(escape_like("a%b_c\\d"), "a\\%b\\_c\\\\d");
    }

    #[test]
    fn build_summary_sql_adds_limit_bind() {
        let args = SummaryArgs {
            query: None,
            limit: 5,
            starts: false,
            all: true,
            pwd: false,
            here: false,
            under: false,
            verbose: false,
        };
        let (_sql, bind) = build_summary_sql(&args).unwrap();
        assert_eq!(bind.last().unwrap(), "5");
    }
}
