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

    /// Show detailed preview information for a command (used by fzf preview)
    Preview(PreviewArgs),

    /// Command template system for reusable command patterns
    Template(TemplateArgs),

    /// Show version information
    Version,
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

    /// Use fzf for interactive selection (outputs selected command to stdout)
    #[arg(long)]
    pub fzf: bool,

    /// Allow selecting multiple commands with fzf (implies --fzf)
    #[arg(long)]
    pub multi_select: bool,

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

    /// Use fzf for interactive selection (outputs selected command to stdout)
    #[arg(long)]
    pub fzf: bool,

    /// Allow selecting multiple commands with fzf (implies --fzf)
    #[arg(long)]
    pub multi_select: bool,
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

    /// Use fzf for interactive selection (outputs selected command to stdout)
    #[arg(long)]
    pub fzf: bool,

    /// Allow selecting multiple commands with fzf (implies --fzf)
    #[arg(long)]
    pub multi_select: bool,
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

    /// Use fzf for interactive selection (outputs selected command to stdout)
    #[arg(long)]
    pub fzf: bool,

    /// Allow selecting multiple commands with fzf (implies --fzf)
    #[arg(long)]
    pub multi_select: bool,
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

    /// Use fzf for interactive selection (outputs selected command to stdout)
    #[arg(long)]
    pub fzf: bool,

    /// Allow selecting multiple commands with fzf (implies --fzf)
    #[arg(long)]
    pub multi_select: bool,
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

    /// Use fzf for interactive selection (outputs selected command to stdout)
    #[arg(long)]
    pub fzf: bool,

    /// Allow selecting multiple commands with fzf (implies --fzf)
    #[arg(long)]
    pub multi_select: bool,
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
    /// Show database schema information
    Schema,
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

#[derive(Parser, Debug)]
pub struct PreviewArgs {
    /// Command to preview
    pub command: String,
}

#[derive(Parser, Debug)]
pub struct TemplateArgs {
    /// Template name to execute (if not provided, lists all templates)
    pub name: Option<String>,

    /// Variable assignments in the format key=value
    #[arg(short, long)]
    pub var: Vec<String>,

    /// List all available templates
    #[arg(long)]
    pub list: bool,

    /// Create or update a template
    #[arg(long)]
    pub create: Option<String>,

    /// Delete a template
    #[arg(long)]
    pub delete: Option<String>,

    /// Use fzf for interactive template selection
    #[arg(long)]
    pub fzf: bool,

    /// Allow selecting multiple templates with fzf (implies --fzf)
    #[arg(long)]
    pub multi_select: bool,
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
        Commands::Preview(args) => cmd_preview(cfg, args),
        Commands::Template(args) => cmd_template(cfg, args),
        Commands::Version => {
            println!("sdbh {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
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

    #[serde(default)]
    fzf: FzfConfig,
}

#[derive(Debug, Default, serde::Deserialize)]
struct FzfConfig {
    /// Height of fzf window (e.g., "50%", "20")
    height: Option<String>,

    /// Layout style ("default", "reverse")
    layout: Option<String>,

    /// Border style ("rounded", "sharp", "bold", "double", "block", "thinblock")
    border: Option<String>,

    /// Color scheme (fzf color string)
    color: Option<String>,

    /// Color for header text
    color_header: Option<String>,

    /// Color for pointer
    color_pointer: Option<String>,

    /// Color for marker
    color_marker: Option<String>,

    /// Preview window settings (e.g., "right:50%")
    preview_window: Option<String>,

    /// Custom preview command
    preview_command: Option<String>,

    /// Key bindings (array of strings)
    #[serde(default)]
    bind: Vec<String>,

    /// Custom fzf binary path
    binary_path: Option<String>,
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

fn load_fzf_config() -> FzfConfig {
    load_config_file().map(|cfg| cfg.fzf).unwrap_or_default()
}

fn build_fzf_command(base_cmd: &mut std::process::Command, fzf_config: &FzfConfig) {
    // Apply configuration options to the fzf command

    // Layout and appearance
    if let Some(height) = &fzf_config.height {
        base_cmd.arg("--height").arg(height);
    }
    if let Some(layout) = &fzf_config.layout {
        base_cmd.arg("--layout").arg(layout);
    }
    if let Some(border) = &fzf_config.border {
        base_cmd.arg("--border").arg(border);
    }

    // Colors
    if let Some(color) = &fzf_config.color {
        base_cmd.arg("--color").arg(color);
    }
    if let Some(color_header) = &fzf_config.color_header {
        base_cmd
            .arg("--color")
            .arg(format!("header:{}", color_header));
    }
    if let Some(color_pointer) = &fzf_config.color_pointer {
        base_cmd
            .arg("--color")
            .arg(format!("pointer:{}", color_pointer));
    }
    if let Some(color_marker) = &fzf_config.color_marker {
        base_cmd
            .arg("--color")
            .arg(format!("marker:{}", color_marker));
    }

    // Preview settings
    if let Some(preview_window) = &fzf_config.preview_window {
        base_cmd.arg("--preview-window").arg(preview_window);
    }
    if let Some(preview_command) = &fzf_config.preview_command {
        base_cmd.arg("--preview").arg(preview_command);
    }

    // Key bindings
    for bind in &fzf_config.bind {
        base_cmd.arg("--bind").arg(bind);
    }

    // Always enable ANSI colors (can be overridden by config)
    if !fzf_config
        .color
        .as_ref()
        .is_some_and(|c| c.contains("ansi"))
    {
        base_cmd.arg("--ansi");
    }

    // Suppress stderr by default (can be overridden by config)
    if !fzf_config.bind.iter().any(|b| b.contains("stderr")) {
        base_cmd.stderr(std::process::Stdio::null());
    }
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
    // Check if multi_select was requested but not fzf
    if args.multi_select && !args.fzf {
        anyhow::bail!("--multi-select requires --fzf flag");
    }

    if args.fzf {
        return cmd_summary_fzf(cfg, args);
    }

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
    if args.fzf {
        return cmd_list_fzf(cfg, args);
    }

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
    if args.fzf {
        return cmd_search_fzf(cfg, args);
    }

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
    match args.command {
        StatsCommand::Top(a) => {
            // Check if multi_select was requested but not fzf
            if a.multi_select && !a.fzf {
                anyhow::bail!("--multi-select requires --fzf flag");
            }
            if a.fzf {
                return cmd_stats_top_fzf(cfg, a);
            }
            let conn = open_db(&cfg)?;
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
            // Check if multi_select was requested but not fzf
            if a.multi_select && !a.fzf {
                anyhow::bail!("--multi-select requires --fzf flag");
            }
            if a.fzf {
                return cmd_stats_by_pwd_fzf(cfg, a);
            }
            let conn = open_db(&cfg)?;
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
            // Check if multi_select was requested but not fzf
            if a.multi_select && !a.fzf {
                anyhow::bail!("--multi-select requires --fzf flag");
            }
            if a.fzf {
                return cmd_stats_daily_fzf(cfg, a);
            }
            let conn = open_db(&cfg)?;
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
        DbCommand::Schema => cmd_db_schema(cfg),
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

fn cmd_db_schema(cfg: DbConfig) -> Result<()> {
    let conn = open_db(&cfg)?;

    println!("Database Schema:");
    println!("================");

    // Tables
    println!("\nTables:");
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
    let tables = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for table in tables {
        let table_name = table?;
        println!("  {}", table_name);

        // Show table schema
        let mut schema_stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))?;
        let columns = schema_stmt.query_map([], |r| {
            let name: String = r.get(1)?;
            let type_: String = r.get(2)?;
            let notnull: i64 = r.get(3)?;
            let pk: i64 = r.get(5)?;
            Ok((name, type_, notnull, pk))
        })?;

        for column in columns {
            let (name, type_, notnull, pk) = column?;
            let mut flags = Vec::new();
            if pk == 1 {
                flags.push("PRIMARY KEY");
            }
            if notnull == 1 {
                flags.push("NOT NULL");
            }
            let flags_str = if flags.is_empty() {
                String::new()
            } else {
                format!(" ({})", flags.join(", "))
            };
            println!("    {} {}{}", name, type_, flags_str);
        }
    }

    // Indexes
    println!("\nIndexes:");
    let mut stmt = conn.prepare(
        "SELECT name, tbl_name, sql FROM sqlite_master WHERE type='index' AND sql IS NOT NULL ORDER BY name"
    )?;
    let indexes = stmt.query_map([], |r| {
        let name: String = r.get(0)?;
        let table: String = r.get(1)?;
        let sql: String = r.get(2)?;
        Ok((name, table, sql))
    })?;

    for index in indexes {
        let (name, table, sql) = index?;
        println!("  {} on {}: {}", name, table, sql);
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum CommandType {
    Git,
    Docker,
    Kubectl,
    Make,
    Cargo,
    Npm,
    Yarn,
    Python,
    Go,
    Navigation,
    System,
    Generic,
}

impl CommandType {
    fn detect(cmd: &str) -> Self {
        let cmd_lower = cmd.to_lowercase();
        let first_word = cmd_lower.split_whitespace().next().unwrap_or("");

        match first_word {
            "git" => CommandType::Git,
            "docker" => CommandType::Docker,
            "kubectl" | "kubectx" | "kubens" => CommandType::Kubectl,
            "make" => CommandType::Make,
            "cargo" => CommandType::Cargo,
            "npm" => CommandType::Npm,
            "yarn" => CommandType::Yarn,
            "python" | "python3" | "pip" | "pip3" => CommandType::Python,
            "go" | "gofmt" | "goimports" => CommandType::Go,
            "cd" | "ls" | "pwd" | "find" | "grep" | "mkdir" | "rm" | "cp" | "mv" => {
                CommandType::Navigation
            }
            "ps" | "top" | "htop" | "df" | "du" | "free" | "uptime" | "whoami" | "id" | "uname" => {
                CommandType::System
            }
            _ => CommandType::Generic,
        }
    }
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

fn cmd_preview(cfg: DbConfig, args: PreviewArgs) -> Result<()> {
    let conn = open_db(&cfg)?;

    // Get command statistics
    let mut stmt = conn.prepare(
        "SELECT
            COUNT(*) as total_uses,
            MAX(epoch) as last_used_epoch,
            MIN(epoch) as first_used_epoch,
            COUNT(DISTINCT pwd) as unique_dirs,
            GROUP_CONCAT(DISTINCT pwd) as dirs
         FROM history
         WHERE cmd = ?1",
    )?;

    let mut rows = stmt.query([args.command.as_str()])?;
    if let Some(row) = rows.next()? {
        // Handle NULL values from aggregate functions
        let total_uses: i64 = row.get(0).unwrap_or(0);
        let last_used_epoch: Option<i64> = row.get(1).ok();
        let first_used_epoch: Option<i64> = row.get(2).ok();
        let unique_dirs: i64 = row.get(3).unwrap_or(0);
        let dirs: Option<String> = row.get(4).ok();

        // If no uses, show not found message
        if total_uses == 0 {
            println!("Command '{}' not found in history", args.command);
            return Ok(());
        }

        // Detect terminal width for responsive design
        let term_width = get_terminal_width().unwrap_or(80);

        // Format timestamps
        let last_used = last_used_epoch
            .map(format_relative_time)
            .unwrap_or_else(|| "Never".to_string());
        let first_used = first_used_epoch
            .map(format_relative_time)
            .unwrap_or_else(|| "Never".to_string());

        // Detect command type for context-aware preview
        let cmd_type = CommandType::detect(&args.command);

        // Phase 3: Professional Layout with Organized Sections
        println!(
            "🔍 Command Analysis: {}",
            truncate_for_display(&args.command, term_width - 25)
        );
        println!("{}", "━".repeat(term_width.min(80)));

        // 📊 Usage Statistics Section
        println!("📊 Usage Statistics");
        println!("  Total uses: {}", total_uses);
        println!("  First used: {}", first_used);
        println!("  Last used: {}", last_used);
        println!("  Directories: {}", unique_dirs);

        // ℹ️ Context Information Section
        if let Some(context) = get_command_context(&args.command, cmd_type) {
            println!("\nℹ️  Context: {}", context);
        }

        // 📁 Directory Usage Section
        if let Some(dirs) = dirs {
            let dir_list: Vec<&str> = dirs.split(',').collect();
            if !dir_list.is_empty() {
                println!("\n📁 Directory Usage:");
                let max_dirs = if term_width > 120 { 8 } else { 5 };
                for dir in dir_list.iter().take(max_dirs) {
                    println!("  • {}", truncate_for_display(dir, term_width - 6));
                }
                if dir_list.len() > max_dirs {
                    println!("  … and {} more", dir_list.len() - max_dirs);
                }
            }
        }

        // 🕒 Recent Activity Section
        println!("\n🕒 Recent Activity (Last 5 executions):");
        let mut recent_stmt = conn.prepare(
            "SELECT id, epoch, pwd, cmd
             FROM history
             WHERE cmd = ?1
             ORDER BY epoch DESC
             LIMIT 5",
        )?;
        let mut recent_rows = recent_stmt.query([args.command.as_str()])?;
        let mut count = 0;
        while let Some(recent_row) = recent_rows.next()? {
            count += 1;
            let _id: i64 = recent_row.get(0)?;
            let epoch: i64 = recent_row.get(1)?;
            let pwd: String = recent_row.get(2)?;
            let full_cmd: String = recent_row.get(3)?;

            // Enhanced relative time display
            let relative_time = format_relative_time(epoch);

            // Highlight command variations with better formatting
            let base_cmd = args.command.as_str();
            let (cmd_display, variation_indicator) = if full_cmd == base_cmd {
                (full_cmd.clone(), "")
            } else if full_cmd.starts_with(&(base_cmd.to_string() + " ")) {
                // Show the arguments that differ
                let args_part = &full_cmd[base_cmd.len()..];
                (format!("{}{}", base_cmd, args_part), "→")
            } else {
                (full_cmd.clone(), "≠")
            };

            // Responsive truncation based on terminal width
            let time_width = 12;
            let variation_width = if variation_indicator.is_empty() { 0 } else { 2 };
            let remaining_width = term_width.saturating_sub(time_width + variation_width + 8); // padding
            let cmd_width = (remaining_width * 60) / 100; // 60% for command
            let pwd_width = remaining_width - cmd_width;

            let short_cmd = truncate_for_display(&cmd_display, cmd_width);
            let short_pwd = truncate_for_display(&pwd, pwd_width);

            if variation_indicator.is_empty() {
                println!(
                    "  {}. {:<8} | {:<width1$} | {}",
                    count,
                    relative_time,
                    short_cmd,
                    short_pwd,
                    width1 = cmd_width
                );
            } else {
                println!(
                    "  {}. {:<8} {} {:<width1$} | {}",
                    count,
                    relative_time,
                    variation_indicator,
                    short_cmd,
                    short_pwd,
                    width1 = cmd_width
                );
            }
        }

        // 🔗 Related Commands Section
        show_related_commands(&conn, &args.command, cmd_type)?;
    } else {
        println!("Command '{}' not found in history", args.command);
    }

    Ok(())
}

fn format_timestamp(epoch: i64) -> String {
    // Simple timestamp formatting - could be enhanced
    format!("{}", epoch)
}

fn format_relative_time(epoch: i64) -> String {
    use time::OffsetDateTime;

    let now = OffsetDateTime::now_utc();
    let now_epoch = now.unix_timestamp();

    let diff_secs = now_epoch - epoch;

    if diff_secs < 0 {
        return "in the future".to_string();
    }

    let diff_mins = diff_secs / 60;
    let diff_hours = diff_mins / 60;
    let diff_days = diff_hours / 24;

    match diff_secs {
        0..=59 => format!("{}s ago", diff_secs),
        60..=3599 => format!("{}m ago", diff_mins),
        3600..=86399 => format!("{}h ago", diff_hours),
        86400..=604799 => format!("{}d ago", diff_days),
        _ => {
            // For older timestamps, show the actual date
            if let Ok(dt) = OffsetDateTime::from_unix_timestamp(epoch) {
                dt.format(time::macros::format_description!("[year]-[month]-[day]"))
                    .unwrap_or_else(|_| format_timestamp(epoch))
            } else {
                format_timestamp(epoch)
            }
        }
    }
}

#[allow(dead_code)]
fn format_command_type(cmd_type: CommandType) -> &'static str {
    match cmd_type {
        CommandType::Git => "🔧 Git",
        CommandType::Docker => "🐳 Docker",
        CommandType::Kubectl => "☸️  Kubernetes",
        CommandType::Make => "🔨 Make",
        CommandType::Cargo => "📦 Cargo",
        CommandType::Npm => "📦 NPM",
        CommandType::Yarn => "🧶 Yarn",
        CommandType::Python => "🐍 Python",
        CommandType::Go => "🐹 Go",
        CommandType::Navigation => "📂 Navigation",
        CommandType::System => "⚙️  System",
        CommandType::Generic => "💻 Generic",
    }
}

#[allow(dead_code)]
fn show_command_type_info(
    conn: &rusqlite::Connection,
    cmd: &str,
    cmd_type: CommandType,
) -> Result<()> {
    match cmd_type {
        CommandType::Git => show_git_info(conn, cmd),
        CommandType::Docker => show_docker_info(conn, cmd),
        CommandType::Kubectl => show_kubectl_info(conn, cmd),
        CommandType::Cargo => show_cargo_info(conn, cmd),
        CommandType::Npm => show_npm_info(conn, cmd),
        CommandType::Make => show_make_info(conn, cmd),
        _ => Ok(()), // No special info for other types
    }
}

fn show_git_info(_conn: &rusqlite::Connection, cmd: &str) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.len() >= 2 {
        let subcommand = parts[1];
        match subcommand {
            "status" => println!("ℹ️  Shows working directory status and changes"),
            "log" => println!("ℹ️  Shows commit history"),
            "diff" => println!("ℹ️  Shows changes between commits/working directory"),
            "branch" => println!("ℹ️  Manages branches"),
            "checkout" | "switch" => println!("ℹ️  Switches branches or restores files"),
            "commit" => println!("ℹ️  Records changes to repository"),
            "push" => println!("ℹ️  Uploads local commits to remote"),
            "pull" => println!("ℹ️  Downloads and integrates remote changes"),
            "clone" => println!("ℹ️  Creates local copy of remote repository"),
            "add" => println!("ℹ️  Stages files for commit"),
            "reset" => println!("ℹ️  Undoes commits or unstages files"),
            "merge" => println!("ℹ️  Joins development histories"),
            "rebase" => println!("ℹ️  Reapplies commits on new base"),
            _ => println!("ℹ️  Git version control operation"),
        }
    }

    Ok(())
}

fn show_docker_info(_conn: &rusqlite::Connection, cmd: &str) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.len() >= 2 {
        let subcommand = parts[1];
        match subcommand {
            "run" => println!("ℹ️  Creates and starts new container"),
            "build" => println!("ℹ️  Builds image from Dockerfile"),
            "ps" => println!("ℹ️  Lists running containers"),
            "images" => println!("ℹ️  Lists local images"),
            "exec" => println!("ℹ️  Runs command in running container"),
            "logs" => println!("ℹ️  Shows container logs"),
            "stop" => println!("ℹ️  Stops running container"),
            "rm" => println!("ℹ️  Removes stopped container"),
            "rmi" => println!("ℹ️  Removes local image"),
            "pull" => println!("ℹ️  Downloads image from registry"),
            "push" => println!("ℹ️  Uploads image to registry"),
            _ => println!("ℹ️  Docker container management"),
        }
    }

    Ok(())
}

fn show_kubectl_info(_conn: &rusqlite::Connection, cmd: &str) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.len() >= 2 {
        let subcommand = parts[1];
        match subcommand {
            "get" => println!("ℹ️  Displays resources"),
            "describe" => println!("ℹ️  Shows detailed resource information"),
            "logs" => println!("ℹ️  Shows container logs"),
            "exec" => println!("ℹ️  Executes command in container"),
            "apply" => println!("ℹ️  Applies configuration changes"),
            "delete" => println!("ℹ️  Removes resources"),
            "create" => println!("ℹ️  Creates resources"),
            "scale" => println!("ℹ️  Changes number of replicas"),
            "rollout" => println!("ℹ️  Manages resource rollouts"),
            "port-forward" => println!("ℹ️  Forwards local port to pod"),
            _ => println!("ℹ️  Kubernetes cluster management"),
        }
    }

    Ok(())
}

fn show_cargo_info(_conn: &rusqlite::Connection, cmd: &str) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.len() >= 2 {
        let subcommand = parts[1];
        match subcommand {
            "build" => println!("ℹ️  Compiles the current package"),
            "run" => println!("ℹ️  Builds and runs the current package"),
            "test" => println!("ℹ️  Runs package tests"),
            "check" => println!("ℹ️  Checks code without building"),
            "doc" => println!("ℹ️  Builds documentation"),
            "fmt" => println!("ℹ️  Formats code"),
            "clippy" => println!("ℹ️  Runs linter"),
            "update" => println!("ℹ️  Updates dependencies"),
            "add" => println!("ℹ️  Adds dependency"),
            "remove" => println!("ℹ️  Removes dependency"),
            _ => println!("ℹ️  Rust package management"),
        }
    }

    Ok(())
}

fn show_npm_info(_conn: &rusqlite::Connection, cmd: &str) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.len() >= 2 {
        let subcommand = parts[1];
        match subcommand {
            "install" => println!("ℹ️  Installs package dependencies"),
            "start" => println!("ℹ️  Starts the application"),
            "run" => println!("ℹ️  Runs package scripts"),
            "test" => println!("ℹ️  Runs test suite"),
            "build" => println!("ℹ️  Builds the application"),
            "dev" => println!("ℹ️  Starts development server"),
            "lint" => println!("ℹ️  Runs code linter"),
            "format" => println!("ℹ️  Formats code"),
            _ => println!("ℹ️  Node.js package management"),
        }
    }

    Ok(())
}

fn show_make_info(_conn: &rusqlite::Connection, cmd: &str) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.len() >= 2 {
        let target = parts[1];
        match target {
            "all" | "build" => println!("ℹ️  Builds the entire project"),
            "clean" => println!("ℹ️  Removes build artifacts"),
            "install" => println!("ℹ️  Installs project files"),
            "test" => println!("ℹ️  Runs test suite"),
            "check" => println!("ℹ️  Performs code checks"),
            "doc" | "docs" => println!("ℹ️  Generates documentation"),
            "fmt" | "format" => println!("ℹ️  Formats source code"),
            "lint" => println!("ℹ️  Runs code linter"),
            _ => println!("ℹ️  Runs make target: {}", target),
        }
    } else {
        println!("ℹ️  Runs default make target");
    }

    Ok(())
}

fn show_related_commands(
    conn: &rusqlite::Connection,
    base_cmd: &str,
    cmd_type: CommandType,
) -> Result<()> {
    let mut suggestions = Vec::new();

    // 1. Semantic similarity: Find commands with related purposes
    let semantic_suggestions = find_semantic_related_commands(base_cmd, cmd_type);
    suggestions.extend(semantic_suggestions);

    // 2. Same tool variations: Commands starting with same tool (current behavior)
    let tool_suggestions = find_tool_related_commands(conn, base_cmd)?;
    suggestions.extend(tool_suggestions);

    // 3. Workflow patterns: Commands commonly used in same sessions
    let workflow_suggestions = find_workflow_related_commands(conn, base_cmd)?;
    suggestions.extend(workflow_suggestions);

    // 4. Directory-based: Commands used in same directories
    let directory_suggestions = find_directory_related_commands(conn, base_cmd)?;
    suggestions.extend(directory_suggestions);

    // Remove duplicates and the base command itself
    let mut unique_suggestions: Vec<String> = suggestions
        .into_iter()
        .filter(|cmd| cmd != base_cmd)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Sort by relevance (semantic first, then tool, workflow, directory)
    // For now, just limit to 5 most relevant
    unique_suggestions.truncate(5);

    if !unique_suggestions.is_empty() {
        println!("\n🔗 Related Commands");
        for cmd in unique_suggestions.iter() {
            // Truncate long commands for display
            let display_cmd = if cmd.len() > 60 {
                format!("{}...", &cmd[..57])
            } else {
                cmd.clone()
            };
            println!("  {}", display_cmd);
        }
    }

    Ok(())
}

fn find_semantic_related_commands(base_cmd: &str, cmd_type: CommandType) -> Vec<String> {
    let mut suggestions = Vec::new();

    match cmd_type {
        CommandType::Git => {
            // Git workflow patterns
            if base_cmd.contains("commit") {
                suggestions.extend(vec![
                    "git status".to_string(),
                    "git log --oneline".to_string(),
                    "git push".to_string(),
                ]);
            } else if base_cmd.contains("push") {
                suggestions.extend(vec![
                    "git status".to_string(),
                    "git log --oneline -5".to_string(),
                    "git pull".to_string(),
                ]);
            } else if base_cmd.contains("pull") || base_cmd.contains("fetch") {
                suggestions.extend(vec![
                    "git status".to_string(),
                    "git log --oneline -5".to_string(),
                    "git merge".to_string(),
                ]);
            } else if base_cmd.contains("branch") {
                suggestions.extend(vec![
                    "git checkout".to_string(),
                    "git branch -a".to_string(),
                ]);
            } else if base_cmd.contains("checkout") || base_cmd.contains("switch") {
                suggestions.extend(vec!["git status".to_string(), "git branch".to_string()]);
            }
        }
        CommandType::Docker => {
            if base_cmd.contains("build") {
                suggestions.extend(vec![
                    "docker images".to_string(),
                    "docker run".to_string(),
                    "docker ps -a".to_string(),
                ]);
            } else if base_cmd.contains("run") {
                suggestions.extend(vec![
                    "docker ps".to_string(),
                    "docker logs".to_string(),
                    "docker stop".to_string(),
                ]);
            } else if base_cmd.contains("ps") {
                suggestions.extend(vec!["docker logs".to_string(), "docker exec".to_string()]);
            }
        }
        CommandType::Cargo => {
            if base_cmd.contains("build") {
                suggestions.extend(vec![
                    "cargo run".to_string(),
                    "cargo test".to_string(),
                    "cargo check".to_string(),
                ]);
            } else if base_cmd.contains("test") {
                suggestions.extend(vec!["cargo build".to_string(), "cargo run".to_string()]);
            } else if base_cmd.contains("run") {
                suggestions.extend(vec!["cargo build".to_string(), "cargo test".to_string()]);
            }
        }
        CommandType::Npm => {
            if base_cmd.contains("install") {
                suggestions.extend(vec![
                    "npm start".to_string(),
                    "npm run build".to_string(),
                    "npm test".to_string(),
                ]);
            } else if base_cmd.contains("start") {
                suggestions.extend(vec!["npm run build".to_string(), "npm test".to_string()]);
            }
        }
        CommandType::Make => {
            suggestions.extend(vec![
                "make clean".to_string(),
                "make install".to_string(),
                "make test".to_string(),
            ]);
        }
        _ => {}
    }

    suggestions
}

fn find_tool_related_commands(conn: &rusqlite::Connection, base_cmd: &str) -> Result<Vec<String>> {
    let first_word = base_cmd.split_whitespace().next().unwrap_or("");

    // Query for other commands that start with the same tool, ordered by most recent usage
    let sql = r#"
        SELECT cmd, MAX(epoch) as latest_epoch
        FROM history
        WHERE cmd LIKE ?1 || ' %'
          AND cmd != ?2
        GROUP BY cmd
        ORDER BY latest_epoch DESC
        LIMIT 3
    "#;

    let mut stmt = conn.prepare(sql)?;
    let like_pattern = format!("{} %", escape_like(first_word));
    let mut rows = stmt.query([&like_pattern, base_cmd])?;

    let mut suggestions = Vec::new();
    while let Some(row) = rows.next()? {
        let cmd: String = row.get(0)?;
        suggestions.push(cmd);
    }

    Ok(suggestions)
}

fn find_workflow_related_commands(
    conn: &rusqlite::Connection,
    base_cmd: &str,
) -> Result<Vec<String>> {
    // Find commands that are commonly used in the same sessions as the base command
    let sql = r#"
        SELECT h2.cmd, COUNT(*) as co_occurrences, MAX(h2.epoch) as latest_epoch
        FROM history h1
        JOIN history h2 ON h1.salt = h2.salt AND h1.ppid = h2.ppid
        WHERE h1.cmd = ?1
          AND h2.cmd != ?1
          AND ABS(h1.epoch - h2.epoch) < 3600  -- Within 1 hour
        GROUP BY h2.cmd
        ORDER BY co_occurrences DESC, latest_epoch DESC
        LIMIT 2
    "#;

    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([base_cmd])?;

    let mut suggestions = Vec::new();
    while let Some(row) = rows.next()? {
        let cmd: String = row.get(0)?;
        suggestions.push(cmd);
    }

    Ok(suggestions)
}

fn find_directory_related_commands(
    conn: &rusqlite::Connection,
    base_cmd: &str,
) -> Result<Vec<String>> {
    // Find commands used in the same directories as the base command
    let sql = r#"
        SELECT h2.cmd, COUNT(*) as shared_dirs, MAX(h2.epoch) as latest_epoch
        FROM history h1
        JOIN history h2 ON h1.pwd = h2.pwd
        WHERE h1.cmd = ?1
          AND h2.cmd != ?1
        GROUP BY h2.cmd
        ORDER BY shared_dirs DESC, latest_epoch DESC
        LIMIT 2
    "#;

    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([base_cmd])?;

    let mut suggestions = Vec::new();
    while let Some(row) = rows.next()? {
        let cmd: String = row.get(0)?;
        suggestions.push(cmd);
    }

    Ok(suggestions)
}

// Phase 3: Helper functions for responsive design and enhanced display

fn get_terminal_width() -> Option<usize> {
    terminal_size::terminal_size().map(|(terminal_size::Width(w), _)| w as usize)
}

fn truncate_for_display(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        text.to_string()
    } else if max_width <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &text[..max_width.saturating_sub(3)])
    }
}

fn get_command_context(cmd: &str, cmd_type: CommandType) -> Option<String> {
    match cmd_type {
        CommandType::Git => {
            if cmd.contains("status") {
                Some("Shows working directory status and changes".to_string())
            } else if cmd.contains("commit") {
                Some("Records changes to repository".to_string())
            } else if cmd.contains("push") {
                Some("Uploads local commits to remote".to_string())
            } else if cmd.contains("pull") {
                Some("Downloads and integrates remote changes".to_string())
            } else {
                Some("Git version control operation".to_string())
            }
        }
        CommandType::Docker => {
            if cmd.contains("build") {
                Some("Builds image from Dockerfile".to_string())
            } else if cmd.contains("run") {
                Some("Creates and starts new container".to_string())
            } else if cmd.contains("ps") {
                Some("Lists running containers".to_string())
            } else {
                Some("Docker container management".to_string())
            }
        }
        CommandType::Cargo => {
            if cmd.contains("build") {
                Some("Compiles the current package".to_string())
            } else if cmd.contains("test") {
                Some("Runs package tests".to_string())
            } else if cmd.contains("run") {
                Some("Builds and runs the current package".to_string())
            } else {
                Some("Rust package management".to_string())
            }
        }
        CommandType::Npm => {
            if cmd.contains("install") {
                Some("Installs package dependencies".to_string())
            } else if cmd.contains("start") {
                Some("Starts the application".to_string())
            } else if cmd.contains("test") {
                Some("Runs test suite".to_string())
            } else {
                Some("Node.js package management".to_string())
            }
        }
        CommandType::Make => {
            if cmd.contains("clean") {
                Some("Removes build artifacts".to_string())
            } else if cmd.contains("test") {
                Some("Runs test suite".to_string())
            } else if cmd.contains("install") {
                Some("Installs project files".to_string())
            } else {
                Some("Builds project targets".to_string())
            }
        }
        _ => None,
    }
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

fn cmd_list_fzf(cfg: DbConfig, args: ListArgs) -> Result<()> {
    // Load fzf configuration
    let fzf_config = load_fzf_config();

    // Check if fzf is available
    let fzf_binary = fzf_config.binary_path.as_deref().unwrap_or("fzf");
    if which(fzf_binary).is_none() {
        anyhow::bail!(
            "fzf is not installed or not found in PATH. Please install fzf to use --fzf flag."
        );
    }

    let conn = open_db(&cfg)?;
    let (sql, bind) = build_list_sql(&args)?;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;

    // Collect items for fzf in a compact format
    let mut fzf_input = String::new();
    while let Some(r) = rows.next()? {
        let dt: String = r.get(1)?;
        let pwd: String = r.get(2)?;
        let cmd: String = r.get(3)?;

        // Format: "cmd  (timestamp) [pwd]"
        // We put cmd first so it's the primary search target
        fzf_input.push_str(&format!("{}  ({}) [{}]\n", cmd, dt, pwd));
    }

    if fzf_input.is_empty() {
        return Ok(()); // No results to select from
    }

    // Run fzf with configuration
    let mut fzf_cmd = std::process::Command::new(fzf_binary);
    build_fzf_command(&mut fzf_cmd, &fzf_config);

    // Override defaults with our specific settings
    fzf_cmd.arg("--preview").arg("sdbh preview --command {{}}");

    // Enable multi-select if requested
    if args.multi_select {
        fzf_cmd.arg("--multi");
    } else {
        fzf_cmd.arg("--no-multi");
    }

    fzf_cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());

    let mut fzf_process = fzf_cmd.spawn()?;

    // Write input to fzf's stdin
    if let Some(mut stdin) = fzf_process.stdin.take() {
        std::io::Write::write_all(&mut stdin, fzf_input.as_bytes())?;
        drop(stdin); // Close stdin to signal EOF
    }

    // Wait for fzf to complete and get output
    let output = fzf_process.wait_with_output()?;

    if !output.status.success() {
        // User cancelled selection (Ctrl+C) or fzf failed
        return Ok(());
    }

    // Extract the selected command(s)
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected_lines: Vec<&str> = selected.lines().collect();

    if selected_lines.is_empty() {
        return Ok(());
    }

    // Process each selected line
    for line in selected_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Extract command from the fzf format: "cmd  (timestamp) [pwd]"
        if let Some(cmd_end) = line.find("  (") {
            let cmd = &line[..cmd_end];
            println!("{}", cmd);
        }
    }

    Ok(())
}

fn cmd_search_fzf(cfg: DbConfig, args: SearchArgs) -> Result<()> {
    // Load fzf configuration
    let fzf_config = load_fzf_config();

    // Check if fzf is available
    let fzf_binary = fzf_config.binary_path.as_deref().unwrap_or("fzf");
    if which(fzf_binary).is_none() {
        anyhow::bail!(
            "fzf is not installed or not found in PATH. Please install fzf to use --fzf flag."
        );
    }

    let conn = open_db(&cfg)?;
    let (sql, bind) = build_search_sql(&args)?;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;

    // Collect items for fzf in a compact format
    let mut fzf_input = String::new();
    while let Some(r) = rows.next()? {
        let dt: String = r.get(1)?;
        let pwd: String = r.get(2)?;
        let cmd: String = r.get(3)?;

        // Format: "cmd  (timestamp) [pwd]"
        // We put cmd first so it's the primary search target
        fzf_input.push_str(&format!("{}  ({}) [{}]\n", cmd, dt, pwd));
    }

    if fzf_input.is_empty() {
        return Ok(()); // No results to select from
    }

    // Run fzf with configuration
    let mut fzf_cmd = std::process::Command::new(fzf_binary);
    build_fzf_command(&mut fzf_cmd, &fzf_config);

    // Override defaults with our specific settings
    fzf_cmd.arg("--preview").arg("sdbh preview --command {{}}");

    // Enable multi-select if requested
    if args.multi_select {
        fzf_cmd.arg("--multi");
    } else {
        fzf_cmd.arg("--no-multi");
    }

    fzf_cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());

    let mut fzf_process = fzf_cmd.spawn()?;

    // Write input to fzf's stdin
    if let Some(mut stdin) = fzf_process.stdin.take() {
        std::io::Write::write_all(&mut stdin, fzf_input.as_bytes())?;
        drop(stdin); // Close stdin to signal EOF
    }

    // Wait for fzf to complete and get output
    let output = fzf_process.wait_with_output()?;

    if !output.status.success() {
        // User cancelled selection (Ctrl+C) or fzf failed
        return Ok(());
    }

    // Extract the selected command(s)
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected_lines: Vec<&str> = selected.lines().collect();

    if selected_lines.is_empty() {
        return Ok(());
    }

    // Process each selected line
    for line in selected_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Extract command from the fzf format: "cmd  (timestamp) [pwd]"
        if let Some(cmd_end) = line.find("  (") {
            let cmd = &line[..cmd_end];
            println!("{}", cmd);
        }
    }

    Ok(())
}

fn cmd_summary_fzf(cfg: DbConfig, args: SummaryArgs) -> Result<()> {
    // Check if multi_select was requested but not fzf
    if args.multi_select && !args.fzf {
        anyhow::bail!("--multi-select requires --fzf flag");
    }

    // Load fzf configuration
    let fzf_config = load_fzf_config();

    // Check if fzf is available
    let fzf_binary = fzf_config.binary_path.as_deref().unwrap_or("fzf");
    if which(fzf_binary).is_none() {
        anyhow::bail!(
            "fzf is not installed or not found in PATH. Please install fzf to use --fzf flag."
        );
    }

    let conn = open_db(&cfg)?;
    let (sql, bind) = build_summary_sql(&args)?;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;

    // Collect items for fzf in a compact format
    let mut fzf_input = String::new();
    while let Some(r) = rows.next()? {
        let _id_max: i64 = r.get(0)?;
        let dt: String = r.get(1)?;
        let count: i64 = r.get(2)?;
        let cmd: String = r.get(3)?;
        let pwd_part = if args.pwd {
            if let Ok(pwd) = r.get::<_, String>(4) {
                format!(" [{}]", pwd)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Format: "cmd  (count uses, last: timestamp) [pwd]"
        fzf_input.push_str(&format!(
            "{}{}  ({} uses, last: {})\n",
            cmd, pwd_part, count, dt
        ));
    }

    if fzf_input.is_empty() {
        return Ok(()); // No results to select from
    }

    // Run fzf with configuration
    let mut fzf_cmd = std::process::Command::new(fzf_binary);
    build_fzf_command(&mut fzf_cmd, &fzf_config);

    // Override defaults with our specific settings
    fzf_cmd.arg("--preview").arg("sdbh preview --command {{}}");

    // Enable multi-select if requested
    if args.multi_select {
        fzf_cmd.arg("--multi");
    } else {
        fzf_cmd.arg("--no-multi");
    }

    fzf_cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());

    let mut fzf_process = fzf_cmd.spawn()?;

    // Write input to fzf's stdin
    if let Some(mut stdin) = fzf_process.stdin.take() {
        std::io::Write::write_all(&mut stdin, fzf_input.as_bytes())?;
        drop(stdin); // Close stdin to signal EOF
    }

    // Wait for fzf to complete and get output
    let output = fzf_process.wait_with_output()?;

    if !output.status.success() {
        // User cancelled selection (Ctrl+C) or fzf failed
        return Ok(());
    }

    // Extract the selected command(s)
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected_lines: Vec<&str> = selected.lines().collect();

    if selected_lines.is_empty() {
        return Ok(());
    }

    // Process each selected line
    for line in selected_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Extract command from the fzf format: "cmd [pwd]  (count uses, last: timestamp)"
        if let Some(cmd_end) = line.find("  (") {
            let cmd_part = &line[..cmd_end];
            // Remove pwd part if present: "cmd [pwd]" -> "cmd"
            let cmd = if let Some(bracket_start) = cmd_part.find(" [") {
                cmd_part[..bracket_start].trim()
            } else {
                cmd_part.trim()
            };
            println!("{}", cmd);
        }
    }

    Ok(())
}

fn cmd_stats_top_fzf(cfg: DbConfig, args: StatsTopArgs) -> Result<()> {
    // Check if multi_select was requested but not fzf
    if args.multi_select && !args.fzf {
        anyhow::bail!("--multi-select requires --fzf flag");
    }

    // Load fzf configuration
    let fzf_config = load_fzf_config();

    // Check if fzf is available
    let fzf_binary = fzf_config.binary_path.as_deref().unwrap_or("fzf");
    if which(fzf_binary).is_none() {
        anyhow::bail!(
            "fzf is not installed or not found in PATH. Please install fzf to use --fzf flag."
        );
    }

    let conn = open_db(&cfg)?;
    let (sql, bind) = build_stats_top_sql(&args)?;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;

    // Collect items for fzf in a compact format
    let mut fzf_input = String::new();
    while let Some(r) = rows.next()? {
        let cnt: i64 = r.get(0)?;
        let cmd: String = r.get(1)?;

        // Format: "cmd  (count uses)"
        fzf_input.push_str(&format!("{}  ({} uses)\n", cmd, cnt));
    }

    if fzf_input.is_empty() {
        return Ok(()); // No results to select from
    }

    // Run fzf with configuration
    let mut fzf_cmd = std::process::Command::new(fzf_binary);
    build_fzf_command(&mut fzf_cmd, &fzf_config);

    // Override defaults with our specific settings
    fzf_cmd.arg("--preview").arg("sdbh preview --command {{}}");

    // Enable multi-select if requested
    if args.multi_select {
        fzf_cmd.arg("--multi");
    } else {
        fzf_cmd.arg("--no-multi");
    }

    fzf_cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());

    let mut fzf_process = fzf_cmd.spawn()?;

    // Write input to fzf's stdin
    if let Some(mut stdin) = fzf_process.stdin.take() {
        std::io::Write::write_all(&mut stdin, fzf_input.as_bytes())?;
        drop(stdin); // Close stdin to signal EOF
    }

    // Wait for fzf to complete and get output
    let output = fzf_process.wait_with_output()?;

    if !output.status.success() {
        // User cancelled selection (Ctrl+C) or fzf failed
        return Ok(());
    }

    // Extract the selected command(s)
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected_lines: Vec<&str> = selected.lines().collect();

    if selected_lines.is_empty() {
        return Ok(());
    }

    // Process each selected line
    for line in selected_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Extract command from the fzf format: "cmd  (count uses)"
        if let Some(cmd_end) = line.find("  (") {
            let cmd = &line[..cmd_end];
            println!("{}", cmd);
        }
    }

    Ok(())
}

fn cmd_stats_by_pwd_fzf(cfg: DbConfig, args: StatsByPwdArgs) -> Result<()> {
    // Check if multi_select was requested but not fzf
    if args.multi_select && !args.fzf {
        anyhow::bail!("--multi-select requires --fzf flag");
    }

    // Load fzf configuration
    let fzf_config = load_fzf_config();

    // Check if fzf is available
    let fzf_binary = fzf_config.binary_path.as_deref().unwrap_or("fzf");
    if which(fzf_binary).is_none() {
        anyhow::bail!(
            "fzf is not installed or not found in PATH. Please install fzf to use --fzf flag."
        );
    }

    let conn = open_db(&cfg)?;
    let (sql, bind) = build_stats_by_pwd_sql(&args)?;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;

    // Collect items for fzf in a compact format
    let mut fzf_input = String::new();
    while let Some(r) = rows.next()? {
        let cnt: i64 = r.get(0)?;
        let pwd: String = r.get(1)?;
        let cmd: String = r.get(2)?;

        // Format: "cmd  [pwd]  (count uses)"
        fzf_input.push_str(&format!("{}  [{}]  ({} uses)\n", cmd, pwd, cnt));
    }

    if fzf_input.is_empty() {
        return Ok(()); // No results to select from
    }

    // Run fzf with configuration
    let mut fzf_cmd = std::process::Command::new(fzf_binary);
    build_fzf_command(&mut fzf_cmd, &fzf_config);

    // Override defaults with our specific settings
    fzf_cmd.arg("--preview").arg("sdbh preview --command {{}}");

    // Enable multi-select if requested
    if args.multi_select {
        fzf_cmd.arg("--multi");
    } else {
        fzf_cmd.arg("--no-multi");
    }

    fzf_cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());

    let mut fzf_process = fzf_cmd.spawn()?;

    // Write input to fzf's stdin
    if let Some(mut stdin) = fzf_process.stdin.take() {
        std::io::Write::write_all(&mut stdin, fzf_input.as_bytes())?;
        drop(stdin); // Close stdin to signal EOF
    }

    // Wait for fzf to complete and get output
    let output = fzf_process.wait_with_output()?;

    if !output.status.success() {
        // User cancelled selection (Ctrl+C) or fzf failed
        return Ok(());
    }

    // Extract the selected command(s)
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected_lines: Vec<&str> = selected.lines().collect();

    if selected_lines.is_empty() {
        return Ok(());
    }

    // Process each selected line
    for line in selected_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Extract command from the fzf format: "cmd  [pwd]  (count uses)"
        if let Some(cmd_end) = line.find("  [") {
            let cmd = &line[..cmd_end];
            println!("{}", cmd);
        }
    }

    Ok(())
}

fn cmd_stats_daily_fzf(cfg: DbConfig, args: StatsDailyArgs) -> Result<()> {
    // Check if multi_select was requested but not fzf
    if args.multi_select && !args.fzf {
        anyhow::bail!("--multi-select requires --fzf flag");
    }

    // Load fzf configuration
    let fzf_config = load_fzf_config();

    // Check if fzf is available
    let fzf_binary = fzf_config.binary_path.as_deref().unwrap_or("fzf");
    if which(fzf_binary).is_none() {
        anyhow::bail!(
            "fzf is not installed or not found in PATH. Please install fzf to use --fzf flag."
        );
    }

    let conn = open_db(&cfg)?;
    let (sql, bind) = build_stats_daily_sql(&args)?;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(bind.iter()))?;

    // Collect items for fzf in a compact format
    let mut fzf_input = String::new();
    while let Some(r) = rows.next()? {
        let day: String = r.get(0)?;
        let cnt: i64 = r.get(1)?;

        // Format: "day  (count commands)"
        fzf_input.push_str(&format!("{}  ({} commands)\n", day, cnt));
    }

    if fzf_input.is_empty() {
        return Ok(()); // No results to select from
    }

    // Run fzf with configuration
    let mut fzf_cmd = std::process::Command::new(fzf_binary);
    build_fzf_command(&mut fzf_cmd, &fzf_config);

    // For daily stats, we can't preview individual commands since we only have dates
    // So we'll skip the preview for this one

    // Enable multi-select if requested
    if args.multi_select {
        fzf_cmd.arg("--multi");
    } else {
        fzf_cmd.arg("--no-multi");
    }

    fzf_cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());

    let mut fzf_process = fzf_cmd.spawn()?;

    // Write input to fzf's stdin
    if let Some(mut stdin) = fzf_process.stdin.take() {
        std::io::Write::write_all(&mut stdin, fzf_input.as_bytes())?;
        drop(stdin); // Close stdin to signal EOF
    }

    // Wait for fzf to complete and get output
    let output = fzf_process.wait_with_output()?;

    if !output.status.success() {
        // User cancelled selection (Ctrl+C) or fzf failed
        return Ok(());
    }

    // Extract the selected command(s)
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected_lines: Vec<&str> = selected.lines().collect();

    if selected_lines.is_empty() {
        return Ok(());
    }

    // Process each selected line
    for line in selected_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Extract day from the fzf format: "day  (count commands)"
        if let Some(day_end) = line.find("  (") {
            let day = &line[..day_end];
            println!("{}", day);
        }
    }

    Ok(())
}

fn cmd_template(_cfg: DbConfig, args: TemplateArgs) -> Result<()> {
    let engine = crate::template::TemplateEngine::new()?;

    if args.list {
        // List all templates
        let templates = engine.list_templates()?;
        if templates.is_empty() {
            println!("No templates found. Create one with: sdbh template --create <name>");
            return Ok(());
        }

        println!("Available Templates:");
        println!("===================");
        for template in templates {
            println!(
                "• {} - {}",
                template.name,
                template.description.as_deref().unwrap_or("No description")
            );
            if let Some(category) = &template.category {
                println!("  Category: {}", category);
            }
            println!("  Variables: {}", template.variables.len());
            println!();
        }
        return Ok(());
    }

    if let Some(name) = &args.create {
        // Create a new template interactively
        return create_template_interactive(&engine, name);
    }

    if let Some(name) = &args.delete {
        // Delete a template
        engine.delete_template(name)?;
        println!("Deleted template: {}", name);
        return Ok(());
    }

    // Execute a template
    if let Some(template_name) = &args.name {
        let template = engine.load_template(template_name)?;

        // Parse variable assignments from command line
        let mut provided_vars = std::collections::HashMap::new();
        for var_assignment in &args.var {
            if let Some((key, value)) = var_assignment.split_once('=') {
                provided_vars.insert(key.to_string(), value.to_string());
            } else {
                anyhow::bail!(
                    "Invalid variable assignment: {}. Use format: key=value",
                    var_assignment
                );
            }
        }

        // Resolve and execute the template with interactive prompting if needed
        let resolved = engine.resolve_template_interactive(&template, &provided_vars)?;
        println!("{}", resolved.resolved_command);
    } else if args.fzf {
        // fzf integration for template selection
        println!("fzf template selection will be available in v0.13.0");
        return Ok(());
    } else {
        // No specific action, show help
        println!("Command Templates System");
        println!("========================");
        println!();
        println!("Usage:");
        println!("  sdbh template --list                    # List all templates");
        println!("  sdbh template --create <name>           # Create a new template");
        println!("  sdbh template --delete <name>           # Delete a template");
        println!("  sdbh template <name>                    # Execute a template");
        println!("  sdbh template <name> --var key=value    # Execute with variables");
        println!();
        println!(
            "Templates are stored in: {}",
            engine.templates_dir().display()
        );
    }

    Ok(())
}

/// Create a template interactively
fn create_template_interactive(engine: &crate::template::TemplateEngine, name: &str) -> Result<()> {
    println!("Creating template: {}", name);
    println!("Enter template information interactively:");
    println!();

    // Get template name (use provided name as default)
    let name = dialoguer::Input::<String>::new()
        .with_prompt("Template name")
        .default(name.to_string())
        .interact_text()?;

    // Get description
    let description = dialoguer::Input::<String>::new()
        .with_prompt("Description (optional)")
        .allow_empty(true)
        .interact_text()?;

    // Get command template
    let command = dialoguer::Input::<String>::new()
        .with_prompt("Command template (use {variable} for placeholders)")
        .interact_text()?;

    // Get category (optional)
    let category = dialoguer::Input::<String>::new()
        .with_prompt("Category (optional, e.g., git, docker)")
        .allow_empty(true)
        .interact_text()?;
    let category = if category.trim().is_empty() {
        None
    } else {
        Some(category.trim().to_string())
    };

    // Extract variables from command
    let extracted_vars = crate::template::extract_variables(&command)?;
    let mut variables = Vec::new();

    if extracted_vars.is_empty() {
        println!("No variables found in command template.");
    } else {
        println!("Found variables in command: {}", extracted_vars.join(", "));
        println!("Configure each variable:");
        println!();

        for var_name in extracted_vars {
            // Get variable description
            let var_desc = dialoguer::Input::<String>::new()
                .with_prompt(format!("Description for '{}' (optional)", var_name))
                .allow_empty(true)
                .interact_text()?;

            // Check if variable is required
            let required = dialoguer::Confirm::new()
                .with_prompt(format!("Is '{}' required?", var_name))
                .default(true)
                .interact()?;

            // Get default value if not required
            let default = if !required {
                let default_val = dialoguer::Input::<String>::new()
                    .with_prompt(format!("Default value for '{}' (optional)", var_name))
                    .allow_empty(true)
                    .interact_text()?;
                if default_val.trim().is_empty() {
                    None
                } else {
                    Some(default_val.trim().to_string())
                }
            } else {
                None
            };

            variables.push(crate::domain::Variable {
                name: var_name,
                description: if var_desc.trim().is_empty() {
                    None
                } else {
                    Some(var_desc.trim().to_string())
                },
                required,
                default,
            });
        }
    }

    // Create the template
    let template = crate::domain::Template {
        id: name.clone(),
        name,
        description: if description.trim().is_empty() {
            None
        } else {
            Some(description.trim().to_string())
        },
        command,
        category,
        variables,
        defaults: std::collections::HashMap::new(), // Individual defaults are in variables
    };

    // Validate and save
    engine.save_template(&template)?;
    println!("Template '{}' created successfully!", template.name);

    Ok(())
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
            fzf: false,
            multi_select: false,
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
            fzf: false,
            multi_select: false,
        };
        let (_sql, bind) = build_summary_sql(&args).unwrap();
        assert_eq!(bind.last().unwrap(), "5");
    }

    #[test]
    fn build_stats_top_sql_basic() {
        let args = StatsTopArgs {
            days: 30,
            limit: 50,
            all: false,
            session: false,
            fzf: false,
            multi_select: false,
        };
        let (sql, bind) = build_stats_top_sql(&args).unwrap();
        assert!(sql.contains("GROUP BY cmd"));
        assert!(sql.contains("ORDER BY cnt DESC"));
        assert!(bind.len() > 0);
    }

    #[test]
    fn build_stats_by_pwd_sql_basic() {
        let args = StatsByPwdArgs {
            days: 30,
            limit: 50,
            all: false,
            session: false,
            fzf: false,
            multi_select: false,
        };
        let (sql, bind) = build_stats_by_pwd_sql(&args).unwrap();
        assert!(sql.contains("GROUP BY pwd, cmd"));
        assert!(sql.contains("ORDER BY cnt DESC"));
        assert!(bind.len() > 0);
    }

    #[test]
    fn build_stats_daily_sql_basic() {
        let args = StatsDailyArgs {
            days: 30,
            all: false,
            session: false,
            fzf: false,
            multi_select: false,
        };
        let (sql, bind) = build_stats_daily_sql(&args).unwrap();
        assert!(sql.contains("GROUP BY day"));
        assert!(sql.contains("ORDER BY day ASC"));
        assert!(bind.len() > 0);
    }
}
