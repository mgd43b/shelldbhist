use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::Connection;
use tempfile::TempDir;

fn conn(path: &std::path::Path) -> Connection {
    Connection::open(path).unwrap()
}

fn parse_bash_history_hook_fields(line: &str) -> Option<(String, String, String)> {
    // Mirrors the bash snippet logic used in `sdbh shell --bash`.
    // Trim leading spaces, then split by spaces, tolerating multiple spaces.
    let line = line.trim_start_matches(' ');
    let (hist_id, rest) = line.split_once(' ')?;
    let rest = rest.trim_start_matches(' ');
    let (epoch, cmd) = rest.split_once(' ')?;
    Some((hist_id.to_string(), epoch.to_string(), cmd.to_string()))
}

#[test]
fn bash_history_parsing_tolerates_multiple_spaces() {
    let (hist_id, epoch, cmd) =
        parse_bash_history_hook_fields("23070  1766267288 history 1").unwrap();
    assert_eq!(hist_id, "23070");
    assert_eq!(epoch, "1766267288");
    assert_eq!(cmd, "history 1");
}

#[test]
fn log_inserts_row_and_list_shows_it() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "echo hello",
            "--epoch",
            "1700000000",
            "--ppid",
            "123",
            "--pwd",
            "/tmp",
            "--salt",
            "42",
            "--hist-id",
            "7",
        ])
        .assert()
        .success();

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--limit",
            "10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("echo hello"));
}

#[test]
fn import_dedups_by_hash() {
    let tmp = TempDir::new().unwrap();
    let src_db = tmp.path().join("src.sqlite");
    let dst_db = tmp.path().join("dst.sqlite");

    // Create a dbhist-compatible src DB
    {
        let c = conn(&src_db);
        c.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;

            CREATE TABLE history (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              hist_id INTEGER,
              cmd TEXT,
              epoch INTEGER,
              ppid INTEGER,
              pwd TEXT,
              salt INTEGER
            );
            "#,
        )
        .unwrap();

        c.execute(
            "INSERT INTO history(hist_id, cmd, epoch, ppid, pwd, salt) VALUES (?,?,?,?,?,?)",
            (1i64, "echo hi", 1700000000i64, 10i64, "/tmp", 99i64),
        )
        .unwrap();
    }

    // Ensure src connection is fully closed before import.
    drop(conn(&src_db));

    // Import twice; second should insert 0
    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            dst_db.to_string_lossy().as_ref(),
            "import",
            "--from",
            src_db.to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("inserted 1"));

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            dst_db.to_string_lossy().as_ref(),
            "import",
            "--from",
            src_db.to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("inserted 0"));
}

#[test]
fn summary_groups_and_counts() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    // Insert same command twice
    for epoch in [1700000000i64, 1700000001i64] {
        Command::cargo_bin("sdbh")
            .unwrap()
            .args([
                "--db",
                db.to_string_lossy().as_ref(),
                "log",
                "--cmd",
                "git status",
                "--epoch",
                &epoch.to_string(),
                "--ppid",
                "123",
                "--pwd",
                "/tmp",
                "--salt",
                "42",
            ])
            .assert()
            .success();
    }

    // Insert a different command once
    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "ls",
            "--epoch",
            "1700000002",
            "--ppid",
            "123",
            "--pwd",
            "/tmp",
            "--salt",
            "42",
        ])
        .assert()
        .success();

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "summary",
            "--all",
            "--limit",
            "50",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("git status"))
        .stdout(predicate::str::contains("     2 |"));
}

#[test]
fn list_under_filters_by_pwd_prefix_and_escapes_wildcards() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    // Two similar prefixes, one contains SQL wildcard chars
    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "echo a",
            "--epoch",
            "1700000000",
            "--ppid",
            "123",
            "--pwd",
            "/tmp/proj_%",
            "--salt",
            "42",
        ])
        .assert()
        .success();

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "echo b",
            "--epoch",
            "1700000001",
            "--ppid",
            "123",
            "--pwd",
            "/tmp/proj_x",
            "--salt",
            "42",
        ])
        .assert()
        .success();

    // Use the new --pwd-override to make this test deterministic
    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--under",
            "--pwd-override",
            "/tmp/proj_%",
            "--limit",
            "50",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("echo a"))
        .stdout(predicate::str::contains("echo b").not());
}

#[test]
fn import_skips_corrupted_rows_with_text_in_numeric_columns() {
    let tmp = TempDir::new().unwrap();
    let src_db = tmp.path().join("src.sqlite");
    let dst_db = tmp.path().join("dst.sqlite");

    // Source DB with one good row and one corrupted row.
    {
        let c = conn(&src_db);
        c.execute_batch(
            r#"
            CREATE TABLE history (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              hist_id INTEGER,
              cmd TEXT,
              epoch INTEGER,
              ppid INTEGER,
              pwd TEXT,
              salt INTEGER
            );
            "#,
        )
        .unwrap();

        // Good row
        c.execute(
            "INSERT INTO history(hist_id, cmd, epoch, ppid, pwd, salt) VALUES (?,?,?,?,?,?)",
            (1i64, "echo good", 1700000000i64, 10i64, "/tmp", 99i64),
        )
        .unwrap();

        // Corrupted row: epoch column contains text
        c.execute(
            "INSERT INTO history(hist_id, cmd, epoch, ppid, pwd, salt) VALUES (?,?,?,?,?,?)",
            (
                "  970* 1571608128 ssh ubnt@192.168.2.1 ",
                "bad",
                "",
                10i64,
                "/tmp",
                99i64,
            ),
        )
        .unwrap();
    }

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            dst_db.to_string_lossy().as_ref(),
            "import",
            "--from",
            src_db.to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("skipped 1 corrupted"));

    // Destination should contain the good row
    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            dst_db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--limit",
            "10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("echo good"))
        .stdout(predicate::str::contains("bad").not());
}

#[test]
fn import_errors_when_history_table_missing() {
    let tmp = TempDir::new().unwrap();
    let src_db = tmp.path().join("src.sqlite");
    let dst_db = tmp.path().join("dst.sqlite");

    // Source DB without history table
    {
        let c = conn(&src_db);
        c.execute_batch("CREATE TABLE not_history(id INTEGER);")
            .unwrap();
    }

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            dst_db.to_string_lossy().as_ref(),
            "import",
            "--from",
            src_db.to_string_lossy().as_ref(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not have a history table"));
}

#[test]
fn json_output_is_valid_shape() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "printf 'a'",
            "--epoch",
            "1700000000",
            "--ppid",
            "123",
            "--pwd",
            "/tmp",
            "--salt",
            "42",
        ])
        .assert()
        .success();

    Command::cargo_bin("sdbh")
        .unwrap()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--format",
            "json",
            "--limit",
            "10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["))
        .stdout(predicate::str::contains("\"cmd\""));
}
