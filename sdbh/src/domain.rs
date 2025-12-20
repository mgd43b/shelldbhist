use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub hist_id: Option<i64>,
    pub cmd: String,
    pub epoch: i64,
    pub ppid: i64,
    pub pwd: String,
    pub salt: i64,
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub path: PathBuf,
}

impl DbConfig {
    pub fn default_path() -> PathBuf {
        // Simple portable default (matches product decision)
        let home = std::env::var_os("HOME").unwrap_or_default();
        PathBuf::from(home).join(".sdbh.sqlite")
    }
}
