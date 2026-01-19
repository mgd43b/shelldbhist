use crate::cleanup::{
    GarbageCandidate, analyze_command_for_garbage_with_config, score_to_confidence_level,
};
use crate::config::CleanupConfig;
use crate::domain::{DbConfig, HistoryRow};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params, types::Value};
use sha2::{Digest, Sha256};

pub fn open_db(cfg: &DbConfig) -> Result<Connection> {
    let conn = Connection::open(&cfg.path)
        .with_context(|| format!("opening sqlite db at {}", cfg.path.display()))?;
    init_schema(&conn)?;
    ensure_indexes(&conn)?;
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

/// Delete history entries by their IDs and return the list of actually deleted IDs
pub fn delete_history_by_ids(conn: &mut Connection, ids: &[i64]) -> Result<Vec<i64>> {
    let mut deleted_ids = Vec::new();

    let tx = conn.transaction()?;

    for id in ids {
        // First, get the hash for this row (for cleanup)
        let hash_opt: Option<String> = tx
            .query_row(
                "SELECT hash FROM history_hash WHERE history_id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;

        // Delete from history table
        let affected = tx.execute("DELETE FROM history WHERE id = ?1", params![id])?;

        if affected > 0 {
            deleted_ids.push(*id);

            // Clean up history_hash table
            if let Some(hash) = hash_opt {
                tx.execute("DELETE FROM history_hash WHERE hash = ?1", params![hash])?;
            }
        }
    }

    tx.commit()?;
    Ok(deleted_ids)
}

/// Preview which entries would be deleted (returns rows with their IDs)
pub fn preview_delete(conn: &Connection, ids: &[i64]) -> Result<Vec<(i64, HistoryRow)>> {
    let mut rows = Vec::new();

    if ids.is_empty() {
        return Ok(rows);
    }

    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, hist_id, cmd, epoch, ppid, pwd, salt FROM history WHERE id IN ({}) ORDER BY id ASC",
        placeholders
    );

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> =
        ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

    let mut query_rows = stmt.query(params.as_slice())?;
    while let Some(r) = query_rows.next()? {
        let id: i64 = r.get(0)?;
        let row = HistoryRow {
            hist_id: r.get(1)?,
            cmd: r.get(2)?,
            epoch: r.get(3)?,
            ppid: r.get(4)?,
            pwd: r.get(5)?,
            salt: r.get(6)?,
        };
        rows.push((id, row));
    }

    Ok(rows)
}

/// Scan database for potential garbage entries
/// Returns a list of candidates with their garbage analysis scores
pub fn scan_garbage_candidates(
    conn: &Connection,
    min_score: Option<f32>,
) -> Result<Vec<GarbageCandidate>> {
    scan_garbage_candidates_with_config(conn, min_score, &CleanupConfig::default())
}

/// Scan database for potential garbage entries with custom configuration
/// Returns a list of candidates with their garbage analysis scores
pub fn scan_garbage_candidates_with_config(
    conn: &Connection,
    min_score: Option<f32>,
    config: &CleanupConfig,
) -> Result<Vec<GarbageCandidate>> {
    let mut candidates = Vec::new();

    // Query all history entries
    let mut stmt = conn.prepare(
        r#"
        SELECT id, cmd, epoch, pwd
        FROM history
        ORDER BY id ASC
        "#,
    )?;

    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let cmd: String = row.get(1)?;
        let epoch: i64 = row.get(2)?;
        let pwd: String = row.get(3)?;

        // Analyze command for garbage with config
        let (confidence_score, reasons) = analyze_command_for_garbage_with_config(&cmd, config);

        // Apply minimum score filter if specified
        if let Some(min) = min_score
            && confidence_score < min
        {
            continue;
        }

        // Only include if score is above 0 (has some indication of garbage)
        if confidence_score > 0.0 {
            let confidence_level = score_to_confidence_level(confidence_score);

            candidates.push(GarbageCandidate {
                id,
                cmd,
                epoch,
                pwd,
                size_bytes: row.get::<_, String>(1)?.len(),
                confidence_score,
                confidence_level,
                reasons,
            });
        }
    }

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scan_garbage_finds_binary_content() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        let cfg = DbConfig {
            path: db_path.clone(),
        };
        let mut conn = open_db(&cfg).unwrap();

        // Insert normal command
        let normal_row = HistoryRow {
            hist_id: Some(1),
            cmd: "echo hello".to_string(),
            epoch: 1700000000,
            ppid: 123,
            pwd: "/tmp".to_string(),
            salt: 42,
        };
        insert_history(&mut conn, &normal_row).unwrap();

        // Insert binary command
        let binary_cmd = format!("{}garbage", "\x7fELF\x02\x01\x01\x00");
        let binary_row = HistoryRow {
            hist_id: Some(2),
            cmd: binary_cmd.clone(),
            epoch: 1700000001,
            ppid: 123,
            pwd: "/tmp".to_string(),
            salt: 42,
        };
        insert_history(&mut conn, &binary_row).unwrap();

        // Scan for garbage
        let candidates = scan_garbage_candidates(&conn, None).unwrap();

        assert!(!candidates.is_empty(), "Expected at least 1 candidate");

        let binary_candidate = candidates
            .iter()
            .find(|c| c.cmd.contains("ELF"))
            .expect("Should find binary");
        assert!(binary_candidate.confidence_score >= 50.0);
        assert!(
            binary_candidate
                .reasons
                .iter()
                .any(|r| r.contains("Binary file magic"))
        );
    }

    #[test]
    fn test_scan_garbage_respects_min_score() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        let cfg = DbConfig {
            path: db_path.clone(),
        };
        let mut conn = open_db(&cfg).unwrap();

        // Insert commands with different scores
        let normal_row = HistoryRow {
            hist_id: Some(1),
            cmd: "echo hello".to_string(),
            epoch: 1700000000,
            ppid: 123,
            pwd: "/tmp".to_string(),
            salt: 42,
        };
        insert_history(&mut conn, &normal_row).unwrap();

        let repetitive_row = HistoryRow {
            hist_id: Some(2),
            cmd: "abc".repeat(500),
            epoch: 1700000001,
            ppid: 123,
            pwd: "/tmp".to_string(),
            salt: 42,
        };
        insert_history(&mut conn, &repetitive_row).unwrap();

        let binary_row = HistoryRow {
            hist_id: Some(3),
            cmd: format!("{}data", "\x7fELF"),
            epoch: 1700000002,
            ppid: 123,
            pwd: "/tmp".to_string(),
            salt: 42,
        };
        insert_history(&mut conn, &binary_row).unwrap();

        // Test different thresholds
        let all = scan_garbage_candidates(&conn, None).unwrap();
        assert!(all.len() >= 2);

        // Binary magic alone gives 50 points, which is moderate confidence
        // Test with 50.0 threshold to catch high-scoring items
        let high_conf = scan_garbage_candidates(&conn, Some(50.0)).unwrap();
        assert!(
            !high_conf.is_empty(),
            "Expected at least 1 high confidence candidate"
        );
        assert!(high_conf.iter().all(|c| c.confidence_score >= 50.0));

        let moderate_conf = scan_garbage_candidates(&conn, Some(30.0)).unwrap();
        assert!(moderate_conf.len() >= 2);
        assert!(moderate_conf.iter().all(|c| c.confidence_score >= 30.0));
    }

    #[test]
    fn test_scan_garbage_returns_complete_metadata() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        let cfg = DbConfig {
            path: db_path.clone(),
        };
        let mut conn = open_db(&cfg).unwrap();

        let binary_cmd = "\x7fELFbinary";
        let row = HistoryRow {
            hist_id: Some(1),
            cmd: binary_cmd.to_string(),
            epoch: 1700000000,
            ppid: 123,
            pwd: "/home/test".to_string(),
            salt: 42,
        };
        insert_history(&mut conn, &row).unwrap();

        let candidates = scan_garbage_candidates(&conn, None).unwrap();
        assert!(!candidates.is_empty());

        let candidate = &candidates[0];
        assert!(candidate.id > 0);
        assert_eq!(candidate.cmd, binary_cmd);
        assert_eq!(candidate.epoch, 1700000000);
        assert_eq!(candidate.pwd, "/home/test");
        assert_eq!(candidate.size_bytes, binary_cmd.len());
        assert!(candidate.confidence_score >= 50.0);
        assert!(!candidate.reasons.is_empty());
    }

    #[test]
    fn test_scan_garbage_empty_database() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        let cfg = DbConfig { path: db_path };
        let conn = open_db(&cfg).unwrap();

        let candidates = scan_garbage_candidates(&conn, None).unwrap();
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_scan_garbage_skips_legitimate_commands() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        let cfg = DbConfig {
            path: db_path.clone(),
        };
        let mut conn = open_db(&cfg).unwrap();

        let legitimate = [
            "git status",
            "ls -la /home/user",
            "SELECT * FROM users WHERE id > 100",
            "curl -X POST https://api.example.com -d '{\"key\":\"value\"}'",
        ];

        for (i, cmd) in legitimate.iter().enumerate() {
            let row = HistoryRow {
                hist_id: Some(i as i64 + 1),
                cmd: cmd.to_string(),
                epoch: 1700000000 + i as i64,
                ppid: 123,
                pwd: "/tmp".to_string(),
                salt: 42,
            };
            insert_history(&mut conn, &row).unwrap();
        }

        let candidates = scan_garbage_candidates(&conn, Some(30.0)).unwrap();
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_scan_garbage_identifies_various_types() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        let cfg = DbConfig {
            path: db_path.clone(),
        };
        let mut conn = open_db(&cfg).unwrap();

        // Create strings that need to live long enough
        let large_cmd = "x".repeat(11000);
        let repetitive_cmd = "abc".repeat(600);

        let garbage = [
            "\x7fELF\x02\x01\x01binary",
            "command\0with\0nulls",
            large_cmd.as_str(),
            repetitive_cmd.as_str(),
        ];

        for (i, cmd) in garbage.iter().enumerate() {
            let row = HistoryRow {
                hist_id: Some(i as i64 + 1),
                cmd: cmd.to_string(),
                epoch: 1700000000 + i as i64,
                ppid: 123,
                pwd: "/tmp".to_string(),
                salt: 42,
            };
            insert_history(&mut conn, &row).unwrap();
        }

        let candidates = scan_garbage_candidates(&conn, Some(30.0)).unwrap();
        assert!(candidates.len() >= 4);

        let has_binary = candidates
            .iter()
            .any(|c| c.reasons.iter().any(|r| r.contains("Binary")));
        let has_null = candidates
            .iter()
            .any(|c| c.reasons.iter().any(|r| r.contains("Null bytes")));
        let has_large = candidates
            .iter()
            .any(|c| c.reasons.iter().any(|r| r.contains("Very large")));
        let has_repetitive = candidates
            .iter()
            .any(|c| c.reasons.iter().any(|r| r.contains("Repetitive")));

        assert!(has_binary);
        assert!(has_null);
        assert!(has_large);
        assert!(has_repetitive);
    }
}
