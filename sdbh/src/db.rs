use crate::domain::{DbConfig, HistoryRow};
use anyhow::{Context, Result};
use rusqlite::{Connection, params, types::Value};
use sha2::{Digest, Sha256};

pub fn open_db(cfg: &DbConfig) -> Result<Connection> {
    let conn = Connection::open(&cfg.path)
        .with_context(|| format!("opening sqlite db at {}", cfg.path.display()))?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS history (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          hist_id INTEGER,
          cmd TEXT,
          epoch INTEGER,
          ppid INTEGER,
          pwd TEXT,
          salt INTEGER
        );

        CREATE TABLE IF NOT EXISTS meta (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS history_hash (
          hash TEXT PRIMARY KEY,
          history_id INTEGER
        );
        "#,
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO meta(key,value) VALUES('schema_version','1')",
        [],
    )?;

    Ok(())
}

pub fn insert_history(conn: &mut Connection, row: &HistoryRow) -> Result<i64> {
    let tx = conn.transaction()?;
    tx.execute(
        r#"
        INSERT INTO history(hist_id, cmd, epoch, ppid, pwd, salt)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![row.hist_id, row.cmd, row.epoch, row.ppid, row.pwd, row.salt],
    )?;

    let id = tx.last_insert_rowid();
    let hash = row_hash(row);

    tx.execute(
        "INSERT OR IGNORE INTO history_hash(hash, history_id) VALUES (?1, ?2)",
        params![hash, id],
    )?;

    tx.commit()?;
    Ok(id)
}

pub fn row_hash(row: &HistoryRow) -> String {
    // Stable: field separator is '\n'. Keep it simple & deterministic.
    let mut hasher = Sha256::new();
    hasher.update(row.epoch.to_string());
    hasher.update("\n");
    hasher.update(row.ppid.to_string());
    hasher.update("\n");
    hasher.update(row.salt.to_string());
    hasher.update("\n");
    hasher.update(row.hist_id.map(|v| v.to_string()).unwrap_or_default());
    hasher.update("\n");
    hasher.update(&row.pwd);
    hasher.update("\n");
    hasher.update(&row.cmd);
    format!("{:x}", hasher.finalize())
}

pub fn ensure_indexes(conn: &Connection) -> Result<()> {
    // Performance indexes for common query patterns
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_history_epoch ON history(epoch);
        CREATE INDEX IF NOT EXISTS idx_history_session ON history(salt, ppid);
        CREATE INDEX IF NOT EXISTS idx_history_pwd ON history(pwd);
        CREATE INDEX IF NOT EXISTS idx_history_hash ON history_hash(hash);
        "#,
    )?;
    Ok(())
}

// Keep the old function for backward compatibility
pub fn ensure_hash_index(conn: &Connection) -> Result<()> {
    ensure_indexes(conn)
}

pub fn import_from_db(conn: &mut Connection, from_path: &std::path::Path) -> Result<(u64, u64)> {
    // Returns (considered, inserted)

    // ATTACH is convenient but can trigger locking edge cases on some platforms
    // and temp dir configurations. Instead, open the source DB as a separate
    // connection and stream rows into destination.

    let src = Connection::open(from_path)
        .with_context(|| format!("opening source db {}", from_path.display()))?;

    conn.execute_batch("BEGIN")?;

    // Ensure src.history exists; if not, fail with clearer message
    let src_has_history: bool = src.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='history')",
        [],
        |r| r.get::<_, i64>(0),
    )? == 1;
    if !src_has_history {
        anyhow::bail!(
            "source db {} does not have a history table",
            from_path.display()
        );
    }

    let mut considered: u64 = 0;
    let mut inserted: u64 = 0;
    let mut skipped_bad: u64 = 0;

    {
        let mut stmt = src.prepare(
            r#"
            SELECT hist_id, cmd, epoch, ppid, pwd, salt
            FROM history
            ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Value>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Value>(2)?,
                r.get::<_, Value>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Value>(5)?,
            ))
        })?;

        for row in rows {
            let (hist_id_v, cmd, epoch_v, ppid_v, pwd, salt_v) = row?;
            considered += 1;

            let hist_id = value_to_i64(&hist_id_v);
            let epoch = match value_to_i64(&epoch_v) {
                Some(v) => v,
                None => {
                    skipped_bad += 1;
                    continue;
                }
            };
            let ppid = match value_to_i64(&ppid_v) {
                Some(v) => v,
                None => {
                    skipped_bad += 1;
                    continue;
                }
            };
            let salt = match value_to_i64(&salt_v) {
                Some(v) => v,
                None => {
                    skipped_bad += 1;
                    continue;
                }
            };

            let row = HistoryRow {
                hist_id,
                cmd,
                epoch,
                ppid,
                pwd,
                salt,
            };

            let hash = row_hash(&row);

            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM history_hash WHERE hash=?1)",
                params![hash],
                |r| r.get::<_, i64>(0),
            )? == 1;

            if exists {
                continue;
            }

            conn.execute(
                r#"
                INSERT INTO history(hist_id, cmd, epoch, ppid, pwd, salt)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![row.hist_id, row.cmd, row.epoch, row.ppid, row.pwd, row.salt],
            )?;
            let id = conn.last_insert_rowid();
            conn.execute(
                "INSERT OR IGNORE INTO history_hash(hash, history_id) VALUES (?1, ?2)",
                params![hash, id],
            )?;
            inserted += 1;
        }
    }

    conn.execute_batch("COMMIT")?;

    if skipped_bad > 0 {
        eprintln!(
            "import skipped {} corrupted row(s) (non-integer hist_id/epoch/ppid/salt)",
            skipped_bad
        );
    }

    Ok((considered, inserted))
}

fn value_to_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Null => None,
        Value::Integer(i) => Some(*i),
        Value::Real(f) => {
            // Try to coerce if it's actually an integer.
            if f.fract() == 0.0 {
                Some(*f as i64)
            } else {
                None
            }
        }
        Value::Text(t) => {
            let s = t.trim().to_string();
            if s.is_empty() {
                return None;
            }
            // Some corrupted values look like: "  970* 1571608128 ssh ..."
            // Extract the first integer token.
            // Prefer first integer-like token; if none, try the second token.
            // This helps with cases like: "970* 1571608128 ssh ..." where epoch is token 2.
            let mut it = s.split_whitespace();
            let t1 = it.next().unwrap_or("");
            let t2 = it.next().unwrap_or("");

            let parse_token = |tok: &str| tok.trim_end_matches('*').parse::<i64>().ok();
            parse_token(t1).or_else(|| parse_token(t2))
        }
        Value::Blob(_) => None,
    }
}
