use crate::domain::{DbConfig, HistoryRow};
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
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

pub fn ensure_hash_index(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_history_epoch ON history(epoch);")?;
    Ok(())
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

    {
        let mut stmt = src.prepare(
            r#"
            SELECT hist_id, cmd, epoch, ppid, pwd, salt
            FROM history
            ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map([], |r| {
            Ok(HistoryRow {
                hist_id: r.get(0)?,
                cmd: r.get(1)?,
                epoch: r.get(2)?,
                ppid: r.get(3)?,
                pwd: r.get(4)?,
                salt: r.get(5)?,
            })
        })?;

        for row in rows {
            let row = row?;
            considered += 1;
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

    Ok((considered, inserted))
}
