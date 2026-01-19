use anyhow::Result;
use rusqlite::Connection;
use std::process::Command;
use tempfile::TempDir;

fn setup_test_db() -> Result<(TempDir, String)> {
    let tmp = tempfile::tempdir()?;
    let db_path = tmp.path().join("test.db").to_str().unwrap().to_string();
    
    // Initialize the database schema
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            hist_id INTEGER,
            cmd TEXT NOT NULL,
            epoch INTEGER NOT NULL,
            ppid INTEGER NOT NULL,
            pwd TEXT NOT NULL,
            salt INTEGER NOT NULL,
            hash TEXT
        );
        CREATE TABLE IF NOT EXISTS history_hash (
            hash TEXT PRIMARY KEY
        );"
    )?;
    
    Ok((tmp, db_path))
}

fn insert_test_entries(db_path: &str) -> Result<()> {
    // Insert directly into database to support special test data
    let conn = Connection::open(db_path)?;
    
    // Insert legitimate command
    conn.execute(
        "INSERT INTO history (cmd, epoch, ppid, pwd, salt) VALUES (?1, ?2, ?3, ?4, ?5)",
        ["echo hello", "1000000", "1", "/tmp", "1"],
    )?;
    
    // Insert command with null bytes (detectable as garbage)
    // Using escaped null bytes that are valid in SQLite TEXT
    let null_byte_cmd = format!("test{}command{}with{}nulls", '\0', '\0', '\0');
    conn.execute(
        "INSERT INTO history (cmd, epoch, ppid, pwd, salt) VALUES (?1, ?2, ?3, ?4, ?5)",
        [&null_byte_cmd, "1000001", "1", "/tmp", "1"],
    )?;
    
    // Insert large repetitive garbage
    let repetitive = "a".repeat(1000);
    conn.execute(
        "INSERT INTO history (cmd, epoch, ppid, pwd, salt) VALUES (?1, ?2, ?3, ?4, ?5)",
        [&repetitive, "1000002", "1", "/tmp", "1"],
    )?;
    
    Ok(())
}

#[test]
fn test_cleanup_scan_mode_basic() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;
    insert_test_entries(&db_path)?;

    let output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--scan"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should find garbage entries
    assert!(stdout.contains("Found"));
    assert!(stdout.contains("potential garbage entries"));
    
    // Should not contain legitimate command
    assert!(!stdout.contains("echo hello"));

    Ok(())
}

#[test]
fn test_cleanup_scan_mode_json_format() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;
    insert_test_entries(&db_path)?;

    let output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--scan", "--format", "json"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should be valid JSON array
    assert!(stdout.starts_with('['));
    assert!(stdout.trim().ends_with(']'));
    assert!(stdout.contains("\"id\":"));
    assert!(stdout.contains("\"score\":"));
    assert!(stdout.contains("\"level\":"));

    Ok(())
}

#[test]
fn test_cleanup_scan_mode_with_min_score() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;
    
    // Insert legitimate command
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "INSERT INTO history (cmd, epoch, ppid, pwd, salt) VALUES (?1, ?2, ?3, ?4, ?5)",
        ["echo hello", "1000000", "1", "/tmp", "1"],
    )?;
    
    // Insert binary content (ELF magic number) which scores e50 points
    let binary_cmd = format!("{}binary_data", "\x7fELF\x02\x01\x01\x00");
    conn.execute(
        "INSERT INTO history (cmd, epoch, ppid, pwd, salt) VALUES (?1, ?2, ?3, ?4, ?5)",
        [&binary_cmd, "1000001", "1", "/tmp", "1"],
    )?;
    drop(conn);

    let output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--scan", "--min-score", "50.0"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should only find high-confidence garbage (binary with ELF magic scores 50+)
    assert!(stdout.contains("potential garbage entries"), "Expected to find garbage with min-score 50.0, got: {}", stdout);

    Ok(())
}

#[test]
fn test_cleanup_scan_mode_empty_db() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;

    let output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--scan"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should report no garbage found
    assert!(stdout.contains("No garbage entries found"));

    Ok(())
}

#[test]
fn test_cleanup_scan_mode_only_legitimate() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;
    
    // Only insert legitimate commands
    Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "log"])
        .args(["--cmd", "git status"])
        .args(["--epoch", "1000000", "--ppid", "1", "--pwd", "/tmp", "--salt", "1"])
        .output()?;

    Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "log"])
        .args(["--cmd", "cargo build"])
        .args(["--epoch", "1000001", "--ppid", "1", "--pwd", "/tmp", "--salt", "1"])
        .output()?;

    let output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--scan"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should report no garbage found
    assert!(stdout.contains("No garbage entries found"));

    Ok(())
}

#[test]
fn test_cleanup_auto_mode_dry_run() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;
    insert_test_entries(&db_path)?;

    // First, verify entries exist
    let list_output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "list"])
        .output()?;
    let before_count = String::from_utf8_lossy(&list_output.stdout)
        .lines()
        .filter(|line| line.contains("|"))
        .count();

    // Run cleanup in auto mode but don't confirm (will exit without deletion)
    let output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--auto"])
        .output()?;

    // Should show preview but not delete without --yes
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("high-confidence entries identified") || 
            stdout.contains("No high-confidence garbage entries found"));

    // Verify entries still exist (count unchanged)
    let list_output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "list"])
        .output()?;
    let after_count = String::from_utf8_lossy(&list_output.stdout)
        .lines()
        .filter(|line| line.contains("|"))
        .count();

    assert_eq!(before_count, after_count, "Entries should not be deleted without confirmation");

    Ok(())
}

#[test]
fn test_cleanup_auto_mode_with_yes_flag() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;
    insert_test_entries(&db_path)?;

    // Get initial count
    let list_output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "list"])
        .output()?;
    let before_count = String::from_utf8_lossy(&list_output.stdout)
        .lines()
        .filter(|line| line.contains("|"))
        .count();

    // Run cleanup in auto mode with --yes
    let output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--auto", "-y"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should report deletion or no high-confidence garbage
    assert!(stdout.contains("Successfully deleted") || 
            stdout.contains("No high-confidence garbage entries found"));

    // Verify legitimate command still exists
    let list_output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "list"])
        .output()?;
    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_stdout.contains("echo hello"), "Legitimate command should remain");

    // Count should be reduced if garbage was found
    let after_count = String::from_utf8_lossy(&list_output.stdout)
        .lines()
        .filter(|line| line.contains("|"))
        .count();
    
    assert!(after_count <= before_count, "Count should not increase");

    Ok(())
}

#[test]
fn test_cleanup_conflicting_modes() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;

    // Try to use both scan and auto modes
    let output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--scan", "--auto"])
        .output()?;

    // Should fail due to conflicting arguments
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("conflicts with") || stderr.contains("cannot be used"));

    Ok(())
}

#[test]
fn test_cleanup_preserves_legitimate_commands() -> Result<()> {
    let (_tmp, db_path) = setup_test_db()?;
    
    // Insert various legitimate commands (use direct DB insertion to avoid CLI quote issues)
    let conn = Connection::open(&db_path)?;
    
    let legitimate_commands = vec![
        "git commit -m initial",
        "cargo test --all",
        "SELECT * FROM users WHERE id=1",
        "curl -X POST https://api.example.com/data",
    ];

    for (i, cmd) in legitimate_commands.iter().enumerate() {
        let epoch = format!("{}", 1000000 + i);
        conn.execute(
            "INSERT INTO history (cmd, epoch, ppid, pwd, salt) VALUES (?1, ?2, ?3, ?4, ?5)",
            [cmd, epoch.as_str(), "1", "/tmp", "1"],
        )?;
    }
    drop(conn);

    // Run cleanup
    Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "cleanup", "--auto", "-y"])
        .output()?;

    // Verify all legitimate commands still exist
    let list_output = Command::new(env!("CARGO_BIN_EXE_sdbh"))
        .args(["--db", &db_path, "list"])
        .output()?;
    let stdout = String::from_utf8_lossy(&list_output.stdout);

    for cmd in legitimate_commands {
        assert!(stdout.contains(cmd), "Legitimate command '{}' should be preserved. List output:\n{}", cmd, stdout);
    }

    Ok(())
}