use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::Connection;
use tempfile::TempDir;

fn conn(path: &std::path::Path) -> Connection {
    Connection::open(path).unwrap()
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
