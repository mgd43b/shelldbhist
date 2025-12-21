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

    /// Search history by substring (case-insensitive)
    Search(SearchArgs),

    /// Export history as JSON Lines (one JSON object per line)
    Export(ExportArgs),

    /// Aggregate statistics
    Stats(StatsArgs),

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

    #[arg(long)]
    pub all: bool,

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

    #[arg(long)]
    pub all: bool,

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

    #[arg(long)]
    pub all: bool,

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
    #[arg(long)]
    pub all: bool,
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

    #[arg(long)]
    pub all: bool,
}

#[derive(Parser, Debug)]
pub struct StatsByPwdArgs {
    #[arg(long, default_value_t = 30)]
    pub days: u32,

    #[arg(long, default_value_t = 50)]
    pub limit: u32,

    #[arg(long)]
    pub all: bool,
}

#[derive(Parser, Debug)]
pub struct StatsDailyArgs {
    #[arg(long, default_value_t = 30)]
    pub days: u32,

    #[arg(long)]
    pub all: bool,
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
        Commands::Search(args) => cmd_search(cfg, args),
        Commands::Export(args) => cmd_export(cfg, args),
        Commands::Stats(args) => cmd_stats(cfg, args),
        Commands::Import(args) => cmd_import(cfg, args),
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
    sql.push_str("LIMIT ? OFFSET ?");
    bind.push(args.limit.to_string());
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

    // WORKAROUND: In some SQLite builds / PRAGMA settings, `COLLATE NOCASE` can behave
    // unexpectedly with LIKE. Instead we normalize both sides with lower(), which is
    // deterministic for ASCII (our common use case) and matches our tests.
    // Note: the query string is lowercased for binding below.

    if let Some((salt, ppid)) = session_filter(args.all) {
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
    bind.push(args.limit.to_string());

    Ok((sql, bind))
}

fn cmd_export(cfg: DbConfig, args: ExportArgs) -> Result<()> {
    let conn = open_db(&cfg)?;

    let mut bind: Vec<String> = vec![];

    let mut sql =
        String::from("SELECT id, hist_id, cmd, epoch, ppid, pwd, salt FROM history WHERE 1=1 ");

    if let Some((salt, ppid)) = session_filter(args.all) {
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

    if let Some((salt, ppid)) = session_filter(args.all) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    sql.push_str("AND epoch >= ? ");
    bind.push(days_cutoff_epoch(args.days).to_string());

    sql.push_str("GROUP BY cmd ORDER BY cnt DESC, max(epoch) DESC LIMIT ?");
    bind.push(args.limit.to_string());

    Ok((sql, bind))
}

fn build_stats_by_pwd_sql(args: &StatsByPwdArgs) -> Result<(String, Vec<String>)> {
    let mut bind: Vec<String> = vec![];
    let mut sql = String::from("SELECT count(*) as cnt, pwd, cmd FROM history WHERE 1=1 ");

    if let Some((salt, ppid)) = session_filter(args.all) {
        sql.push_str("AND salt=? AND ppid=? ");
        bind.push(salt.to_string());
        bind.push(ppid.to_string());
    }

    sql.push_str("AND epoch >= ? ");
    bind.push(days_cutoff_epoch(args.days).to_string());

    sql.push_str("GROUP BY pwd, cmd ORDER BY cnt DESC, max(epoch) DESC LIMIT ?");
    bind.push(args.limit.to_string());

    Ok((sql, bind))
}

fn build_stats_daily_sql(args: &StatsDailyArgs) -> Result<(String, Vec<String>)> {
    let mut bind: Vec<String> = vec![];
    let mut sql = String::from(
        "SELECT date(epoch, 'unixepoch', 'localtime') as day, count(*) as cnt FROM history WHERE 1=1 ",
    );

    if let Some((salt, ppid)) = session_filter(args.all) {
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
    fn build_summary_sql_adds_limit_bind() {
        let args = SummaryArgs {
            query: None,
            limit: 5,
            starts: false,
            all: true,
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
