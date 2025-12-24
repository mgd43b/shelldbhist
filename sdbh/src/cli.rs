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

    /// Search history by substring (case-insensitive). Supports time filtering via --since-epoch/--days.
    Search(SearchArgs),

    /// Export history as JSON Lines (one JSON object per line)
    Export(ExportArgs),

    /// Aggregate statistics
    Stats(StatsArgs),

    /// Import/merge another dbhist-compatible SQLite database
    Import(ImportArgs),

    /// Import from shell history files (bash/zsh)
    #[command(name = "import-history")]
    ImportHistory(ImportHistoryArgs),

    /// Diagnose shell integration / DB setup
    Doctor(DoctorArgs),

    /// Database operations
    Db(DbArgs),

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

    /// Disable default noisy-command filtering.
    /// Useful for debugging shell integration.
    #[arg(long)]
    pub no_filter: bool,
}

#[derive(Parser, Debug)]
pub struct SummaryArgs {
    /// Query substring (or prefix if --starts)
    pub query: Option<String>,

    #[arg(long, default_value_t = 100)]
    pub limit: u32,

    #[arg(long)]
    pub starts: bool,

    /// Show all entries (no limit)
    #[arg(long)]
    pub all: bool,

    /// Filter to current session only
    #[arg(long)]
    pub session: bool,

    #[arg(long)]
    pub pwd: bool,

    /// Override the working directory used by --here/--under (useful for tests)
    #[arg(long)]
    pub pwd_override: Option<String>,

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

    /// Show all entries (no limit)
    #[arg(long)]
    pub all: bool,

    /// Filter to current session only
    #[arg(long)]
    pub session: bool,

    /// Override the working directory used by --here/--under (useful for tests)
    #[arg(long)]
    pub pwd_override: Option<String>,

    #[arg(long, conflicts_with = "under")]
    pub here: bool,

    #[arg(long, conflicts_with = "here")]
    pub under: bool,
}

#[derive(Parser, Debug)]
pub struct SearchArgs {
    /// Query substring (case-insensitive)
    pub query: String,

    #[arg(long, default_value_t = 100)]
    pub limit: u32,

    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Show all entries (no limit)
    #[arg(long)]
    pub all: bool,

    /// Filter to current session only
    #[arg(long)]
    pub session: bool,

    /// Only include rows with epoch >= since_epoch.
    #[arg(long, conflicts_with = "days")]
    pub since_epoch: Option<i64>,

    /// Only include rows within the last N days.
    #[arg(long, conflicts_with = "since_epoch")]
    pub days: Option<u32>,

    /// Override the working directory used by --here/--under (useful for tests)
    #[arg(long)]
    pub pwd_override: Option<String>,

    #[arg(long, conflicts_with = "under")]
    pub here: bool,

    #[arg(long, conflicts_with = "here")]
    pub under: bool,
}

#[derive(Parser, Debug)]
pub struct ExportArgs {
    /// Show all entries (no limit)
    #[arg(long)]
    pub all: bool,

    /// Filter to current session only
    #[arg(long)]
    pub session: bool,
}

#[derive(Parser, Debug)]
pub struct StatsArgs {
    #[command(subcommand)]
    pub command: StatsCommand,
}

#[derive(Subcommand, Debug)]
pub enum StatsCommand {
    /// Top commands within the last N days
    Top(StatsTopArgs),

    /// Top commands grouped by pwd within the last N days
    ByPwd(StatsByPwdArgs),

    /// Command count per day within the last N days
    Daily(StatsDailyArgs),
}

#[derive(Parser, Debug)]
pub struct StatsTopArgs {
    #[arg(long, default_value_t = 30)]
    pub days: u32,

    #[arg(long, default_value_t = 50)]
    pub limit: u32,

    /// Show all entries (no limit)
    #[arg(long)]
    pub all: bool,

    /// Filter to current session only
    #[arg(long)]
    pub session: bool,
}

#[derive(Parser, Debug)]
pub struct StatsByPwdArgs {
    #[arg(long, default_value_t = 30)]
    pub days: u32,

    #[arg(long, default_value_t = 50)]
    pub limit: u32,

    /// Show all entries (no limit)
    #[arg(long)]
    pub all: bool,

    /// Filter to current session only
    #[arg(long)]
    pub session: bool,
}

#[derive(Parser, Debug)]
pub struct StatsDailyArgs {
    #[arg(long, default_value_t = 30)]
    pub days: u32,

    /// Show all entries (no limit)
    #[arg(long)]
    pub all: bool,

    /// Filter to current session only
    #[arg(long)]
    pub session: bool,
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
pub struct ImportHistoryArgs {
    /// Path to a bash history file (e.g. ~/.bash_history)
    #[arg(long, conflicts_with = "zsh")]
    pub bash: Option<PathBuf>,

    /// Path to a zsh history file (e.g. ~/.zsh_history)
    #[arg(long, conflicts_with = "bash")]
    pub zsh: Option<PathBuf>,

    /// PWD to store on imported entries (default: current directory)
    #[arg(long)]
    pub pwd: Option<String>,

    /// Salt to store on imported entries (default: 0)
    #[arg(long, default_value_t = 0)]
    pub salt: i64,

    /// PPID to store on imported entries (default: 0)
    #[arg(long, default_value_t = 0)]
    pub ppid: i64,
}

#[derive(Parser, Debug)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Subcommand, Debug)]
pub enum DbCommand {
    /// Check database health and statistics
    Health,
    /// Optimize database (rebuild indexes, vacuum)
    Optimize,
    /// Show database statistics
    Stats,
}

#[derive(Parser, Debug)]
pub struct DoctorArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Skip spawning subshells for deeper inspection.
    #[arg(long, conflicts_with = "spawn_only")]
    pub no_spawn: bool,

    /// Only use spawned subshell inspection.
    #[arg(long, conflicts_with = "no_spawn")]
    pub spawn_only: bool,
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
        Commands::Search(args) => cmd_search(cfg, args),
        Commands::Export(args) => cmd_export(cfg, args),
        Commands::Stats(args) => cmd_stats(cfg, args),
        Commands::Import(args) => cmd_import(cfg, args),
        Commands::ImportHistory(args) => cmd_import_history(cfg, args),
        Commands::Doctor(args) => cmd_doctor(cfg, args),
        Commands::Db(args) => cmd_db(cfg, args),
        Commands::Shell(args) => cmd_shell(args),
    }
}

fn cmd_log(cfg: DbConfig, args: LogArgs) -> Result<()> {
    if !args.no_filter {
        let filter = LogFilter::load_default();
        if filter.should_skip(&args.cmd) {
            return Ok(());
        }
    }

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

#[derive(Debug, Default, serde::Deserialize)]
struct LogConfig {
    #[serde(default)]
    ignore_exact: Vec<String>,

    #[serde(default)]
    ignore_prefix: Vec<String>,

    #[serde(default = "default_true")]
    use_builtin_ignores: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, serde::Deserialize)]
struct ConfigFile {
    #[serde(default)]
    log: LogConfig,
}

#[derive(Debug)]
struct LogFilter {
    use_builtin_ignores: bool,
    ignore_exact: Vec<String>,
    ignore_prefix: Vec<String>,
}

impl LogFilter {
    fn load_default() -> Self {
        let mut filter = Self {
            use_builtin_ignores: true,
            ignore_exact: vec![],
            ignore_prefix: vec![],
        };

        if let Some(cfg) = load_config_file() {
            filter.use_builtin_ignores = cfg.log.use_builtin_ignores;
            filter.ignore_exact = cfg.log.ignore_exact;
            filter.ignore_prefix = cfg.log.ignore_prefix;
        }

        filter
    }

    fn should_skip(&self, cmd: &str) -> bool {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return true;
        }

        if self.use_builtin_ignores && is_builtin_noisy_command(trimmed) {
            return true;
        }

        if self.ignore_exact.iter().any(|s| s.trim() == trimmed) {
            return true;
        }

        for prefix in &self.ignore_prefix {
            let p = prefix.as_str();
            if trimmed.starts_with(p) {
                return true;
            }
        }

        false
    }
}

fn config_path() -> Option<std::path::PathBuf> {
    // User-requested location: ~/.sdbh.toml
    let home = std::env::var_os("HOME").or_else(|| dirs::home_dir().map(|p| p.into_os_string()))?;
    let mut p = std::path::PathBuf::from(home);
    p.push(".sdbh.toml");
    Some(p)
}

fn load_config_file() -> Option<ConfigFile> {
    let path = config_path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    toml::from_str::<ConfigFile>(&text).ok()
}

fn is_builtin_noisy_command(cmd: &str) -> bool {
    // Built-in filter: keep conservative defaults.
    // Note: `cmd` is expected to be trimmed.

    // Exact ignores
    match cmd {
        "ls" | "pwd" | "history" | "clear" | "exit" => return true,
        _ => {}
    }

    // Prefix/word ignores
    // Treat as token prefix: "cd" or "cd <arg>"
    let starts_with_word = |w: &str| {
        cmd == w || cmd.starts_with(&format!("{} ", w)) || cmd.starts_with(&format!("{}\t", w))
    };

    if starts_with_word("cd") {
        return true;
    }

    // Avoid self-logging (sdbh commands)
    if starts_with_word("sdbh") {
        return true;
    }

    // Also treat `ls -la` etc as noisy.
    if starts_with_word("ls") {
        return true;
    }

    false
}

fn session_filter(session_only: bool) -> Option<(i64, i64)> {
    if session_only {
        // Filter to current session only
        let salt = std::env::var("SDBH_SALT").ok()?.parse::<i64>().ok()?;
        let ppid = std::env::var("SDBH_PPID").ok()?.parse::<i64>().ok()?;
        Some((salt, ppid))
    } else {
        // No session filtering (show all sessions)
        None
    }
}

fn location_filter(
    here: bool,
    under: bool,
    pwd_override: &Option<String>,
) -> Option<(String, bool)> {
    if !(here || under) {
        return None;
    }
    let pwd = pwd_override.clone().or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    })?;
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

    if let Some((salt, ppid)) = session_filter(args.session) {
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

    if let Some((pwd, under)) = location_filter(args.here, args.under, &args.pwd_override) {
        if under {
            sql.push_str("AND pwd LIKE ? ESCAPE '\\' ");
            // For an under-query, treat the override as a literal directory prefix.
            // The suffix '%' is a wildcard and must NOT be escaped.
            bind.push(format!("{}%", escape_like(&pwd)));
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
    let limit = if args.all { u32::MAX } else { args.limit };
    bind.push(limit.to_string());

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

    if let Some((salt, ppid)) = session_filter(args.session) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    if let Some(q) = &args.query {
        sql.push_str("AND cmd LIKE ? ESCAPE '\\' ");
        bind.push(escape_like(&format!("%{}%", q)));
    }

    if let Some((pwd, under)) = location_filter(args.here, args.under, &args.pwd_override) {
        if under {
            sql.push_str("AND pwd LIKE ? ESCAPE '\\' ");
            bind.push(format!("{}%", escape_like(&pwd)));
        } else {
            sql.push_str("AND pwd = ? ");
            bind.push(pwd);
        }
    }

    sql.push_str("ORDER BY epoch ASC, id ASC ");
    sql.push_str("LIMIT ? OFFSET ?");
    let limit = if args.all { u32::MAX } else { args.limit };
    bind.push(limit.to_string());
    bind.push(args.offset.to_string());

    Ok((sql, bind))
}

fn cmd_search(cfg: DbConfig, args: SearchArgs) -> Result<()> {
    let conn = open_db(&cfg)?;

    let (sql, bind) = build_search_sql(&args)?;
    // Debugging aid: enable with SDBH_DEBUG=1
    if std::env::var("SDBH_DEBUG").ok().as_deref() == Some("1") {
        eprintln!("sql: {sql}");
        eprintln!("bind: {:?}", bind);
    }

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

fn build_search_sql(args: &SearchArgs) -> Result<(String, Vec<String>)> {
    let mut bind: Vec<String> = vec![];
    let mut sql = String::from(
        "SELECT id, datetime(epoch, 'unixepoch', 'localtime') as dt, pwd, cmd, epoch FROM history WHERE 1=1 ",
    );

    // Optional time filtering
    if let Some(since) = args.since_epoch {
        sql.push_str("AND epoch >= ? ");
        bind.push(since.to_string());
    } else if let Some(days) = args.days {
        sql.push_str("AND epoch >= ? ");
        bind.push(days_cutoff_epoch(days).to_string());
    }

    // WORKAROUND: In some SQLite builds / PRAGMA settings, `COLLATE NOCASE` can behave
    // unexpectedly with LIKE. Instead we normalize both sides with lower(), which is
    // deterministic for ASCII (our common use case) and matches our tests.
    // Note: the query string is lowercased for binding below.

    if let Some((salt, ppid)) = session_filter(args.session) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    // Case-insensitive substring match.
    // Use a NOCASE collation on the command column rather than applying lower()
    // to avoid surprises with expression collation + LIKE in some SQLite builds.
    sql.push_str("AND cmd LIKE ? ESCAPE '\\' ");
    // Do NOT escape the surrounding wildcards; only escape user-provided text.
    bind.push(format!("%{}%", escape_like(&args.query)));

    if let Some((pwd, under)) = location_filter(args.here, args.under, &args.pwd_override) {
        if under {
            sql.push_str("AND pwd LIKE ? ESCAPE '\\' ");
            bind.push(format!("{}%", escape_like(&pwd)));
        } else {
            sql.push_str("AND pwd = ? ");
            bind.push(pwd);
        }
    }

    sql.push_str("ORDER BY epoch DESC, id DESC ");
    sql.push_str("LIMIT ?");
    let limit = if args.all { u32::MAX } else { args.limit };
    bind.push(limit.to_string());

    Ok((sql, bind))
}

fn cmd_export(cfg: DbConfig, args: ExportArgs) -> Result<()> {
    let conn = open_db(&cfg)?;

    let mut bind: Vec<String> = vec![];

    let mut sql =
        String::from("SELECT id, hist_id, cmd, epoch, ppid, pwd, salt FROM history WHERE 1=1 ");

    if let Some((salt, ppid)) = session_filter(args.session) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    sql.push_str("ORDER BY epoch ASC, id ASC");

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;

    while let Some(r) = rows.next()? {
        let id: i64 = r.get(0)?;
        let hist_id: Option<i64> = r.get(1)?;
        let cmd: String = r.get(2)?;
        let epoch: i64 = r.get(3)?;
        let ppid: i64 = r.get(4)?;
        let pwd: String = r.get(5)?;
        let salt: i64 = r.get(6)?;

        // JSONL without serde.
        // Keep fields simple and stable.
        let hist_id_json = match hist_id {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        };

        println!(
            "{{\"id\":{},\"hist_id\":{},\"epoch\":{},\"ppid\":{},\"pwd\":{},\"salt\":{},\"cmd\":{}}}",
            id,
            hist_id_json,
            epoch,
            ppid,
            json_string(&pwd),
            salt,
            json_string(&cmd)
        );
    }

    Ok(())
}

fn cmd_stats(cfg: DbConfig, args: StatsArgs) -> Result<()> {
    let conn = open_db(&cfg)?;

    match args.command {
        StatsCommand::Top(a) => {
            let (sql, bind) = build_stats_top_sql(&a)?;
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;
            while let Some(r) = rows.next()? {
                let cnt: i64 = r.get(0)?;
                let cmd: String = r.get(1)?;
                println!("{cnt:>6} | {cmd}");
            }
            Ok(())
        }
        StatsCommand::ByPwd(a) => {
            let (sql, bind) = build_stats_by_pwd_sql(&a)?;
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;
            while let Some(r) = rows.next()? {
                let cnt: i64 = r.get(0)?;
                let pwd: String = r.get(1)?;
                let cmd: String = r.get(2)?;
                println!("{cnt:>6} | {pwd} | {cmd}");
            }
            Ok(())
        }
        StatsCommand::Daily(a) => {
            let (sql, bind) = build_stats_daily_sql(&a)?;
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;
            while let Some(r) = rows.next()? {
                let day: String = r.get(0)?;
                let cnt: i64 = r.get(1)?;
                println!("{day} | {cnt:>6}");
            }
            Ok(())
        }
    }
}

fn days_cutoff_epoch(days: u32) -> i64 {
    let now = std::time::SystemTime::now();
    let now_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let secs = (days as i64) * 86400;
    now_epoch - secs
}

fn build_stats_top_sql(args: &StatsTopArgs) -> Result<(String, Vec<String>)> {
    let mut bind: Vec<String> = vec![];
    let mut sql = String::from("SELECT count(*) as cnt, cmd FROM history WHERE 1=1 ");

    if let Some((salt, ppid)) = session_filter(args.session) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    sql.push_str("AND epoch >= ? ");
    bind.push(days_cutoff_epoch(args.days).to_string());

    sql.push_str("GROUP BY cmd ORDER BY cnt DESC, max(epoch) DESC LIMIT ?");
    let limit = if args.all { u32::MAX } else { args.limit };
    bind.push(limit.to_string());

    Ok((sql, bind))
}

fn build_stats_by_pwd_sql(args: &StatsByPwdArgs) -> Result<(String, Vec<String>)> {
    let mut bind: Vec<String> = vec![];
    let mut sql = String::from("SELECT count(*) as cnt, pwd, cmd FROM history WHERE 1=1 ");

    if let Some((salt, ppid)) = session_filter(args.session) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    sql.push_str("AND epoch >= ? ");
    bind.push(days_cutoff_epoch(args.days).to_string());

    sql.push_str("GROUP BY pwd, cmd ORDER BY cnt DESC, max(epoch) DESC LIMIT ?");
    let limit = if args.all { u32::MAX } else { args.limit };
    bind.push(limit.to_string());

    Ok((sql, bind))
}

fn build_stats_daily_sql(args: &StatsDailyArgs) -> Result<(String, Vec<String>)> {
    let mut bind: Vec<String> = vec![];
    let mut sql = String::from(
        "SELECT date(epoch, 'unixepoch', 'localtime') as day, count(*) as cnt FROM history WHERE 1=1 ",
    );

    if let Some((salt, ppid)) = session_filter(args.session) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    sql.push_str("AND epoch >= ? ");
    bind.push(days_cutoff_epoch(args.days).to_string());

    sql.push_str("GROUP BY day ORDER BY day ASC");

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

fn cmd_import_history(cfg: DbConfig, args: ImportHistoryArgs) -> Result<()> {
    let mut conn = open_db(&cfg)?;
    ensure_hash_index(&conn)?;

    let pwd = args.pwd.clone().or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    });
    let pwd = pwd.unwrap_or_else(|| "/".to_string());

    let entries = if let Some(path) = args.bash.as_ref() {
        read_bash_history(path)?
    } else if let Some(path) = args.zsh.as_ref() {
        read_zsh_history(path)?
    } else {
        anyhow::bail!("one of --bash or --zsh is required");
    };

    // Assign synthetic sequential timestamps for entries that don't have an epoch.
    // For stable dedup on repeated imports, synthetic timestamps must be deterministic.
    // Use a fixed epoch base for missing timestamps (preserves ordering but not real time).
    let missing = entries.iter().filter(|e| e.epoch.is_none()).count() as i64;
    let mut next_synth_epoch = 1_000_000_000i64 - missing;

    let mut considered = 0u64;
    let mut inserted = 0u64;

    for e in entries {
        let epoch = match e.epoch {
            Some(v) => v,
            None => {
                next_synth_epoch += 1;
                next_synth_epoch
            }
        };

        let row = HistoryRow {
            hist_id: None,
            cmd: e.cmd,
            epoch,
            ppid: args.ppid,
            pwd: pwd.clone(),
            salt: args.salt,
        };
        considered += 1;

        // Dedup using history_hash
        let hash = crate::db::row_hash(&row);
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM history_hash WHERE hash=?1)",
            rusqlite::params![hash],
            |r| r.get::<_, i64>(0),
        )? == 1;

        if exists {
            continue;
        }

        // insert_history also populates history_hash.
        insert_history(&mut conn, &row)?;
        inserted += 1;
    }

    eprintln!("import-history: considered {considered}, inserted {inserted}");
    Ok(())
}

fn cmd_doctor(cfg: DbConfig, args: DoctorArgs) -> Result<()> {
    let mut checks: Vec<DoctorCheck> = vec![];

    // --- DB check ---
    let db_path = cfg.path.clone();
    let db_display = db_path.to_string_lossy().to_string();

    match open_db(&cfg) {
        Ok(mut conn) => {
            // Basic write check: create a temp table and rollback.
            let write_ok = (|| {
                let tx = conn.transaction()?;
                tx.execute_batch("CREATE TABLE IF NOT EXISTS __sdbh_doctor_tmp(id INTEGER);")?;
                tx.rollback()?;
                Ok::<(), rusqlite::Error>(())
            })()
            .is_ok();

            checks.push(DoctorCheck::ok("db.open", format!("opened {db_display}")));

            if write_ok {
                checks.push(DoctorCheck::ok(
                    "db.write",
                    "write transaction OK".to_string(),
                ));
            } else {
                checks.push(DoctorCheck::warn(
                    "db.write",
                    "db opened but write test failed".to_string(),
                ));
            }

            // Database integrity check
            let integrity_ok = conn
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .map(|result| result == "ok")
                .unwrap_or(false);

            if integrity_ok {
                checks.push(DoctorCheck::ok(
                    "db.integrity",
                    "Database integrity check passed".to_string(),
                ));
            } else {
                checks.push(DoctorCheck::fail(
                    "db.integrity",
                    "Database integrity check failed".to_string(),
                ));
            }

            // Database statistics and health
            let page_count: i64 = conn
                .query_row("PRAGMA page_count", [], |r| r.get(0))
                .unwrap_or(0);
            let freelist_count: i64 = conn
                .query_row("PRAGMA freelist_count", [], |r| r.get(0))
                .unwrap_or(0);
            let page_size: i64 = conn
                .query_row("PRAGMA page_size", [], |r| r.get(0))
                .unwrap_or(4096);
            let _row_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
                .unwrap_or(0);

            let db_size_mb = (page_count * page_size) as f64 / 1_000_000.0;
            let free_space_mb = (freelist_count * page_size) as f64 / 1_000_000.0;
            let fragmentation_ratio = if page_count > 0 {
                freelist_count as f64 / page_count as f64
            } else {
                0.0
            };

            // Size assessment
            if db_size_mb > 100.0 {
                checks.push(DoctorCheck::info(
                    "db.size",
                    format!("Large database ({:.1} MB)", db_size_mb),
                ));
            }

            // Fragmentation assessment
            if fragmentation_ratio > 0.2 {
                checks.push(DoctorCheck::warn(
                    "db.fragmentation",
                    format!(
                        "High fragmentation ({:.1}%, {:.1} MB free) - consider VACUUM",
                        fragmentation_ratio * 100.0,
                        free_space_mb
                    ),
                ));
            } else if fragmentation_ratio > 0.1 {
                checks.push(DoctorCheck::info(
                    "db.fragmentation",
                    format!(
                        "Moderate fragmentation ({:.1}%, {:.1} MB free)",
                        fragmentation_ratio * 100.0,
                        free_space_mb
                    ),
                ));
            }

            // VACUUM suggestion
            if free_space_mb > 10.0 {
                checks.push(DoctorCheck::info(
                    "db.optimize",
                    format!(
                        "{:.1} MB of free space available - VACUUM could reduce size",
                        free_space_mb
                    ),
                ));
            }

            // Check for missing indexes
            let mut missing_indexes = Vec::new();
            let indexes = [
                (
                    "idx_history_epoch",
                    "CREATE INDEX IF NOT EXISTS idx_history_epoch ON history(epoch)",
                ),
                (
                    "idx_history_session",
                    "CREATE INDEX IF NOT EXISTS idx_history_session ON history(salt, ppid)",
                ),
                (
                    "idx_history_pwd",
                    "CREATE INDEX IF NOT EXISTS idx_history_pwd ON history(pwd)",
                ),
                (
                    "idx_history_hash",
                    "CREATE INDEX IF NOT EXISTS idx_history_hash ON history_hash(hash)",
                ),
            ];

            for (name, _) in &indexes {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name=?1)",
                        [name],
                        |r| r.get(0),
                    )
                    .unwrap_or(false);
                if !exists {
                    missing_indexes.push(*name);
                }
            }

            if !missing_indexes.is_empty() {
                checks.push(DoctorCheck::warn(
                    "db.indexes",
                    format!(
                        "Missing performance indexes: {} (run 'sdbh db optimize')",
                        missing_indexes.join(", ")
                    ),
                ));
            } else {
                checks.push(DoctorCheck::ok(
                    "db.indexes",
                    "All performance indexes present".to_string(),
                ));
            }
        }
        Err(e) => {
            checks.push(DoctorCheck::fail(
                "db.open",
                format!("failed to open {db_display}: {e}"),
            ));
        }
    }

    // --- Env vars ---
    checks.extend(check_env_i64("SDBH_SALT"));
    checks.extend(check_env_i64("SDBH_PPID"));

    // --- Env-only shell detection ---
    if !args.spawn_only {
        if let Ok(pc) = std::env::var("PROMPT_COMMAND") {
            if pc.contains("__sdbh_prompt") {
                checks.push(DoctorCheck::ok(
                    "bash.hook.env",
                    "PROMPT_COMMAND contains __sdbh_prompt".to_string(),
                ));
            } else {
                checks.push(DoctorCheck::info(
                    "bash.hook.env",
                    "PROMPT_COMMAND does not contain __sdbh_prompt".to_string(),
                ));
            }
        } else {
            checks.push(DoctorCheck::info(
                "bash.hook.env",
                "PROMPT_COMMAND not set".to_string(),
            ));
        }
    }

    // --- Spawned shell inspection ---
    if !args.no_spawn {
        if let Some(bash) = which("bash") {
            match spawn_bash_inspect(&bash) {
                Ok(rep) => {
                    checks.push(DoctorCheck::info(
                        "bash.spawn",
                        format!("ok: {}", rep.summary()),
                    ));
                    if rep.prompt_command.contains("__sdbh_prompt") {
                        checks.push(DoctorCheck::ok(
                            "bash.hook.spawn",
                            "PROMPT_COMMAND contains __sdbh_prompt".to_string(),
                        ));
                    } else {
                        checks.push(DoctorCheck::info(
                            "bash.hook.spawn",
                            "PROMPT_COMMAND missing __sdbh_prompt".to_string(),
                        ));
                    }

                    if rep.trap_debug.contains("__sdbh_debug_trap") {
                        checks.push(DoctorCheck::ok(
                            "bash.intercept.spawn",
                            "DEBUG trap contains __sdbh_debug_trap".to_string(),
                        ));
                    } else {
                        checks.push(DoctorCheck::info(
                            "bash.intercept.spawn",
                            "DEBUG trap missing __sdbh_debug_trap".to_string(),
                        ));
                    }
                }
                Err(e) => checks.push(DoctorCheck::warn(
                    "bash.spawn",
                    format!("failed to inspect bash: {e}"),
                )),
            }
        } else {
            checks.push(DoctorCheck::info(
                "bash.spawn",
                "bash not found on PATH".to_string(),
            ));
        }

        if let Some(zsh) = which("zsh") {
            match spawn_zsh_inspect(&zsh) {
                Ok(rep) => {
                    checks.push(DoctorCheck::info(
                        "zsh.spawn",
                        format!("ok: {}", rep.summary()),
                    ));

                    if rep.precmd_functions.contains("sdbh_precmd") {
                        checks.push(DoctorCheck::ok(
                            "zsh.hook.spawn",
                            "precmd_functions contains sdbh_precmd".to_string(),
                        ));
                    } else {
                        checks.push(DoctorCheck::info(
                            "zsh.hook.spawn",
                            "precmd_functions missing sdbh_precmd".to_string(),
                        ));
                    }

                    if rep.preexec_functions.contains("sdbh_preexec") {
                        checks.push(DoctorCheck::ok(
                            "zsh.intercept.spawn",
                            "preexec_functions contains sdbh_preexec".to_string(),
                        ));
                    } else {
                        checks.push(DoctorCheck::info(
                            "zsh.intercept.spawn",
                            "preexec_functions missing sdbh_preexec".to_string(),
                        ));
                    }
                }
                Err(e) => checks.push(DoctorCheck::warn(
                    "zsh.spawn",
                    format!("failed to inspect zsh: {e}"),
                )),
            }
        } else {
            checks.push(DoctorCheck::info(
                "zsh.spawn",
                "zsh not found on PATH".to_string(),
            ));
        }
    }

    output_doctor(&checks, args.format);
    Ok(())
}

fn cmd_db(cfg: DbConfig, args: DbArgs) -> Result<()> {
    match args.command {
        DbCommand::Health => cmd_db_health(cfg),
        DbCommand::Optimize => cmd_db_optimize(cfg),
        DbCommand::Stats => cmd_db_stats(cfg),
    }
}

fn cmd_db_health(cfg: DbConfig) -> Result<()> {
    let conn = open_db(&cfg)?;

    // Database integrity check
    let integrity_ok = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
        .map(|result| result == "ok")
        .unwrap_or(false);

    if integrity_ok {
        println!("✓ Database integrity check passed");
    } else {
        println!("✗ Database integrity check failed");
    }

    // Get database statistics
    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let freelist_count: i64 = conn.query_row("PRAGMA freelist_count", [], |r| r.get(0))?;
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let row_count: i64 = conn.query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))?;

    let db_size_mb = (page_count * page_size) as f64 / 1_000_000.0;
    let free_space_mb = (freelist_count * page_size) as f64 / 1_000_000.0;
    let fragmentation_ratio = if page_count > 0 {
        freelist_count as f64 / page_count as f64
    } else {
        0.0
    };

    println!("Database Statistics:");
    println!("  Rows: {}", row_count);
    println!("  Size: {:.1} MB", db_size_mb);
    println!("  Free space: {:.1} MB", free_space_mb);
    println!("  Fragmentation: {:.1}%", fragmentation_ratio * 100.0);

    // Check for missing indexes
    let mut missing_indexes = Vec::new();
    let indexes = [
        (
            "idx_history_epoch",
            "CREATE INDEX IF NOT EXISTS idx_history_epoch ON history(epoch)",
        ),
        (
            "idx_history_session",
            "CREATE INDEX IF NOT EXISTS idx_history_session ON history(salt, ppid)",
        ),
        (
            "idx_history_pwd",
            "CREATE INDEX IF NOT EXISTS idx_history_pwd ON history(pwd)",
        ),
        (
            "idx_history_hash",
            "CREATE INDEX IF NOT EXISTS idx_history_hash ON history_hash(hash)",
        ),
    ];

    for (name, _sql) in &indexes {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name=?1)",
            [name],
            |r| r.get(0),
        )?;
        if !exists {
            missing_indexes.push(*name);
        }
    }

    if missing_indexes.is_empty() {
        println!("✓ All performance indexes present");
    } else {
        println!("⚠ Missing indexes (run 'sdbh db optimize' to create):");
        for index in &missing_indexes {
            println!("  - {}", index);
        }
    }

    // VACUUM suggestions
    if free_space_mb > 10.0 {
        println!(
            "💡 Consider running VACUUM ({} MB reclaimable)",
            free_space_mb
        );
    }

    Ok(())
}

fn cmd_db_optimize(cfg: DbConfig) -> Result<()> {
    let conn = open_db(&cfg)?;

    println!("Optimizing database...");

    // Ensure all indexes exist
    crate::db::ensure_indexes(&conn)?;
    println!("✓ Ensured all indexes exist");

    // Rebuild indexes (REINDEX)
    conn.execute_batch("REINDEX;")?;
    println!("✓ Reindexed database");

    // Vacuum to reclaim space
    conn.execute_batch("VACUUM;")?;
    println!("✓ Vacuumed database");

    println!("Database optimization complete!");
    Ok(())
}

fn cmd_db_stats(cfg: DbConfig) -> Result<()> {
    let conn = open_db(&cfg)?;

    // Basic statistics
    let row_count: i64 = conn.query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))?;
    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;

    let db_size_mb = (page_count * page_size) as f64 / 1_000_000.0;

    println!("Database Statistics:");
    println!("  Total rows: {}", row_count);
    println!("  Database size: {:.1} MB", db_size_mb);
    println!("  Page count: {}", page_count);
    println!("  Page size: {} bytes", page_size);

    // Index information
    println!("\nIndexes:");
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for row in rows {
        let name = row?;
        println!("  {}", name);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum DoctorStatus {
    Ok,
    Warn,
    Fail,
    Info,
}

#[derive(Debug, Clone)]
struct DoctorCheck {
    name: &'static str,
    status: DoctorStatus,
    detail: String,
}

impl DoctorCheck {
    fn ok(name: &'static str, detail: String) -> Self {
        Self {
            name,
            status: DoctorStatus::Ok,
            detail,
        }
    }

    fn warn(name: &'static str, detail: String) -> Self {
        Self {
            name,
            status: DoctorStatus::Warn,
            detail,
        }
    }

    fn fail(name: &'static str, detail: String) -> Self {
        Self {
            name,
            status: DoctorStatus::Fail,
            detail,
        }
    }

    fn info(name: &'static str, detail: String) -> Self {
        Self {
            name,
            status: DoctorStatus::Info,
            detail,
        }
    }
}

fn check_env_i64(key: &'static str) -> Vec<DoctorCheck> {
    match std::env::var(key) {
        Ok(v) => match v.parse::<i64>() {
            Ok(_) => vec![DoctorCheck::ok(key, format!("{key}={v}"))],
            Err(_) => vec![DoctorCheck::warn(
                key,
                format!("{key} is set but not an integer: {v}"),
            )],
        },
        Err(_) => vec![DoctorCheck::warn(key, format!("{key} is not set"))],
    }
}

fn status_str(s: DoctorStatus) -> &'static str {
    match s {
        DoctorStatus::Ok => "ok",
        DoctorStatus::Warn => "warn",
        DoctorStatus::Fail => "fail",
        DoctorStatus::Info => "info",
    }
}

fn output_doctor(checks: &[DoctorCheck], format: OutputFormat) {
    match format {
        OutputFormat::Table => {
            for c in checks {
                println!("{:18} | {:5} | {}", c.name, status_str(c.status), c.detail);
            }
        }
        OutputFormat::Json => {
            print!("[");
            let mut first = true;
            for c in checks {
                if !first {
                    print!(",");
                }
                first = false;
                print!(
                    "{{\"check\":{},\"status\":{},\"detail\":{}}}",
                    json_string(c.name),
                    json_string(status_str(c.status)),
                    json_string(&c.detail)
                );
            }
            println!("]");
        }
    }
}

fn which(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(bin);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

#[derive(Debug)]
struct BashInspect {
    prompt_command: String,
    trap_debug: String,
}

impl BashInspect {
    fn summary(&self) -> String {
        format!(
            "prompt_command_len={}, trap_debug_len={}",
            self.prompt_command.len(),
            self.trap_debug.len()
        )
    }
}

fn spawn_bash_inspect(bash: &std::path::Path) -> Result<BashInspect> {
    let out = std::process::Command::new(bash)
        .args([
            "-lc",
            "echo __SDBH_PROMPT_COMMAND__=$PROMPT_COMMAND; echo __SDBH_TRAP_DEBUG__=$(trap -p DEBUG)",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut prompt_command = String::new();
    let mut trap_debug = String::new();

    for line in stdout.lines() {
        if let Some(v) = line.strip_prefix("__SDBH_PROMPT_COMMAND__=") {
            prompt_command = v.to_string();
        }
        if let Some(v) = line.strip_prefix("__SDBH_TRAP_DEBUG__=") {
            trap_debug = v.to_string();
        }
    }

    Ok(BashInspect {
        prompt_command,
        trap_debug,
    })
}

#[derive(Debug)]
struct ZshInspect {
    precmd_functions: String,
    preexec_functions: String,
}

impl ZshInspect {
    fn summary(&self) -> String {
        format!(
            "precmd_len={}, preexec_len={}",
            self.precmd_functions.len(),
            self.preexec_functions.len()
        )
    }
}

fn spawn_zsh_inspect(zsh: &std::path::Path) -> Result<ZshInspect> {
    let out = std::process::Command::new(zsh)
        .args([
            "-lc",
            "echo __SDBH_PRECMD__=${precmd_functions[*]}; echo __SDBH_PREEXEC__=${preexec_functions[*]}",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut precmd_functions = String::new();
    let mut preexec_functions = String::new();

    for line in stdout.lines() {
        if let Some(v) = line.strip_prefix("__SDBH_PRECMD__=") {
            precmd_functions = v.to_string();
        }
        if let Some(v) = line.strip_prefix("__SDBH_PREEXEC__=") {
            preexec_functions = v.to_string();
        }
    }

    Ok(ZshInspect {
        precmd_functions,
        preexec_functions,
    })
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    epoch: Option<i64>,
    cmd: String,
}

fn read_bash_history(path: &std::path::Path) -> Result<Vec<HistoryEntry>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();

    // Bash history file is typically one command per line.
    // If timestamps are enabled, it uses lines like:
    //   #1700000000
    //   echo hi
    // We support both.
    let mut pending_epoch: Option<i64> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix('#')
            && let Ok(v) = rest.trim().parse::<i64>()
        {
            pending_epoch = Some(v);
            continue;
        }

        out.push(HistoryEntry {
            epoch: pending_epoch.take(),
            cmd: line.to_string(),
        });
    }

    Ok(out)
}

fn read_zsh_history(path: &std::path::Path) -> Result<Vec<HistoryEntry>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();

    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        // Extended history format:
        //   : 1700000000:0;cmd...
        if let Some(rest) = line.strip_prefix(": ")
            && let Some((epoch_part, cmd_part)) = rest.split_once(';')
        {
            // epoch_part = "1700000000:0" (duration after second colon)
            let epoch_str = epoch_part.split(':').next().unwrap_or("");
            if let Ok(epoch) = epoch_str.parse::<i64>() {
                out.push(HistoryEntry {
                    epoch: Some(epoch),
                    cmd: cmd_part.to_string(),
                });
                continue;
            }
        }

        // Fallback: treat as a raw command without a timestamp.
        out.push(HistoryEntry {
            epoch: None,
            cmd: line.to_string(),
        });
    }

    Ok(out)
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
  # history output sometimes contains multiple spaces between fields, so trim
  # spaces before splitting.
  local hist_id epoch cmd

  # trim leading spaces
  line="${line#${line%%[! ]*}}"

  hist_id="${line%% *}"
  line="${line#* }"

  # trim leading spaces again (in case there were multiple spaces)
  line="${line#${line%%[! ]*}}"

  epoch="${line%% *}"
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
    fn build_summary_sql_with_all_unlimited() {
        let args = SummaryArgs {
            query: None,
            limit: 5,
            starts: false,
            all: true,
            session: false,
            pwd: false,
            pwd_override: None,
            here: false,
            under: false,
            verbose: false,
        };
        let (_sql, bind) = build_summary_sql(&args).unwrap();
        // --all means unlimited, so limit should be u32::MAX
        assert_eq!(bind.last().unwrap(), &u32::MAX.to_string());
    }

    #[test]
    fn build_summary_sql_with_limit() {
        let args = SummaryArgs {
            query: None,
            limit: 5,
            starts: false,
            all: false,
            session: false,
            pwd: false,
            pwd_override: None,
            here: false,
            under: false,
            verbose: false,
        };
        let (_sql, bind) = build_summary_sql(&args).unwrap();
        assert_eq!(bind.last().unwrap(), "5");
    }
}
