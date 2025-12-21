use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::Connection;
use tempfile::TempDir;

fn sdbh_cmd() -> Command {
    // Prefer the new macro API to avoid cargo build-dir issues.
    // (See deprecation notes in assert_cmd.)
    let exe = assert_cmd::cargo::cargo_bin!("sdbh");
    Command::new(exe)
}

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

    sdbh_cmd()
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

    sdbh_cmd()
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
    sdbh_cmd()
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

    sdbh_cmd()
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
        sdbh_cmd()
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
    sdbh_cmd()
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

    sdbh_cmd()
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
    sdbh_cmd()
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

    sdbh_cmd()
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
    sdbh_cmd()
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

    sdbh_cmd()
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
    sdbh_cmd()
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

    sdbh_cmd()
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

    sdbh_cmd()
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

    sdbh_cmd()
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

#[test]
fn search_finds_substring_case_insensitive_and_respects_limit() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    for (cmd, epoch) in [
        ("kubectl get pods", "1700000000"),
        ("KUBECTL describe pod", "1700000001"),
        ("git status", "1700000002"),
    ] {
        sdbh_cmd()
            .args([
                "--db",
                db.to_string_lossy().as_ref(),
                "log",
                "--cmd",
                cmd,
                "--epoch",
                epoch,
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

    // Sanity check: list should show at least one kubectl row
    sdbh_cmd()
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
        .stdout(predicate::str::contains("kubectl").or(predicate::str::contains("KUBECTL")));

    // Should match both kubectl commands regardless of case, but only return 1 due to limit.
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "search",
            "kubectl",
            "--all",
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("kubectl").or(predicate::str::contains("KUBECTL")))
        .stdout(predicate::str::contains("git status").not());
}

#[test]
fn search_supports_since_epoch_filter() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.sqlite");

    // Insert 2 rows: one old, one new.
    let old_epoch = 1_000_000_000i64;
    let new_epoch = 1_000_000_000i64 + 10_000;

    sdbh_cmd()
        .args([
            "--db",
            db_path.to_str().unwrap(),
            "log",
            "--cmd",
            "foo old",
            "--epoch",
            &old_epoch.to_string(),
            "--ppid",
            "1",
            "--pwd",
            "/tmp",
            "--salt",
            "1",
            "--no-filter",
        ])
        .assert()
        .success();

    sdbh_cmd()
        .args([
            "--db",
            db_path.to_str().unwrap(),
            "log",
            "--cmd",
            "foo new",
            "--epoch",
            &new_epoch.to_string(),
            "--ppid",
            "1",
            "--pwd",
            "/tmp",
            "--salt",
            "1",
            "--no-filter",
        ])
        .assert()
        .success();

    // Cutoff excludes old, includes new.
    let cutoff = old_epoch + 1;

    let out = sdbh_cmd()
        .args([
            "--db",
            db_path.to_str().unwrap(),
            "search",
            "foo",
            "--all",
            "--since-epoch",
            &cutoff.to_string(),
            "--limit",
            "50",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("foo new"));
    assert!(!stdout.contains("foo old"));
}

#[test]
fn search_json_output_is_valid_shape() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "kubectl get pods",
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

    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "search",
            "kubectl",
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

#[test]
fn export_outputs_jsonl_to_stdout() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "echo hi",
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

    // One JSON object per line. Keep assertions minimal to avoid ordering concerns.
    sdbh_cmd()
        .args(["--db", db.to_string_lossy().as_ref(), "export", "--all"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"cmd\":\"echo hi\"").and(predicate::str::contains("\n")),
        );
}

#[test]
fn search_escapes_like_wildcards_in_query() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    // Should match literally on "%" and "_" characters.
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "echo 100% done",
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

    // Without escaping, this would match too broadly. We want literal "%".
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "search",
            "100%",
            "--all",
            "--limit",
            "10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("100% done"));
}

#[test]
fn stats_top_shows_most_common_commands() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    // 2x git status
    for epoch in [1700000000i64, 1700000001i64] {
        sdbh_cmd()
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

    // 1x ls
    sdbh_cmd()
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

    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "stats",
            "top",
            "--all",
            "--days",
            "9999",
            "--limit",
            "10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("git status"))
        .stdout(predicate::str::contains("     2"));
}

#[test]
fn stats_by_pwd_groups_by_directory() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    // Same cmd in two different pwds
    for (pwd, epoch) in [("/tmp/a", "1700000000"), ("/tmp/b", "1700000001")] {
        sdbh_cmd()
            .args([
                "--db",
                db.to_string_lossy().as_ref(),
                "log",
                "--cmd",
                "make test",
                "--epoch",
                epoch,
                "--ppid",
                "123",
                "--pwd",
                pwd,
                "--salt",
                "42",
            ])
            .assert()
            .success();
    }

    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "stats",
            "by-pwd",
            "--all",
            "--days",
            "9999",
            "--limit",
            "10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("/tmp/a"))
        .stdout(predicate::str::contains("/tmp/b"))
        .stdout(predicate::str::contains("make test"));
}

#[test]
fn stats_daily_outputs_day_buckets_in_localtime() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    // Two commands on different epochs (not asserting exact date string, just that we get 2 lines).
    for epoch in [1700000000i64, 1700086400i64] {
        sdbh_cmd()
            .args([
                "--db",
                db.to_string_lossy().as_ref(),
                "log",
                "--cmd",
                "echo x",
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

    let out = sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "stats",
            "daily",
            "--all",
            "--days",
            "9999",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2);
}

#[test]
fn log_skips_noisy_commands_by_default() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "ls",
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

    sdbh_cmd()
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
        .stdout(predicate::str::contains("| ls").not());
}

#[test]
fn log_no_filter_allows_logging_noisy_commands() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--no-filter",
            "--cmd",
            "ls",
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

    sdbh_cmd()
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
        .stdout(predicate::str::contains("| ls"));
}

#[test]
fn log_respects_config_ignore_exact_in_home_sdbh_toml() {
    let tmp = TempDir::new().unwrap();

    // Fake HOME so sdbh reads config from tmp.
    let home = tmp.path();
    std::fs::write(
        home.join(".sdbh.toml"),
        r#"[log]
ignore_exact = ["echo hello"]
"#,
    )
    .unwrap();

    let db = home.join("test.sqlite");

    // This would normally be logged, but config says to ignore it.
    sdbh_cmd()
        .env("HOME", home)
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
        ])
        .assert()
        .success();

    sdbh_cmd()
        .env("HOME", home)
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
        .stdout(predicate::str::contains("echo hello").not());
}

#[test]
fn log_respects_config_use_builtin_ignores_false() {
    let tmp = TempDir::new().unwrap();

    let home = tmp.path();
    std::fs::write(
        home.join(".sdbh.toml"),
        r#"[log]
use_builtin_ignores = false
"#,
    )
    .unwrap();

    let db = home.join("test.sqlite");

    // Built-in ignores would skip `ls`, but with use_builtin_ignores=false it should be logged.
    sdbh_cmd()
        .env("HOME", home)
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--cmd",
            "ls",
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

    sdbh_cmd()
        .env("HOME", home)
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
        .stdout(predicate::str::contains("| ls"));
}

#[test]
fn log_no_filter_overrides_config() {
    let tmp = TempDir::new().unwrap();

    let home = tmp.path();
    std::fs::write(
        home.join(".sdbh.toml"),
        r#"[log]
ignore_exact = ["ls"]
"#,
    )
    .unwrap();

    let db = home.join("test.sqlite");

    sdbh_cmd()
        .env("HOME", home)
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "log",
            "--no-filter",
            "--cmd",
            "ls",
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

    sdbh_cmd()
        .env("HOME", home)
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
        .stdout(predicate::str::contains("| ls"));
}

#[test]
fn import_history_bash_assigns_synthetic_timestamps_and_dedups() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path();

    let db = home.join("test.sqlite");
    let hist = home.join("bash_history");

    // No timestamps in bash history; importer should create synthetic epochs.
    std::fs::write(&hist, "echo one\necho two\n").unwrap();

    // Import twice; second should insert 0 due to dedup.
    sdbh_cmd()
        .env("HOME", home)
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "import-history",
            "--bash",
            hist.to_string_lossy().as_ref(),
            "--pwd",
            "/tmp",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("inserted 2"));

    sdbh_cmd()
        .env("HOME", home)
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "import-history",
            "--bash",
            hist.to_string_lossy().as_ref(),
            "--pwd",
            "/tmp",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("inserted 0"));

    // Should have both commands present.
    let out = sdbh_cmd()
        .env("HOME", home)
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
        .get_output()
        .stdout
        .clone();

    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("echo one"));
    assert!(out.contains("echo two"));
}

#[test]
fn import_history_zsh_parses_extended_history_format() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path();

    let db = home.join("test.sqlite");
    let hist = home.join("zsh_history");

    // Zsh extended history line format: ": <epoch>:<duration>;<command>"
    std::fs::write(&hist, ": 1700000000:0;echo zsh\n").unwrap();

    sdbh_cmd()
        .env("HOME", home)
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "import-history",
            "--zsh",
            hist.to_string_lossy().as_ref(),
            "--pwd",
            "/tmp",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("inserted 1"));

    sdbh_cmd()
        .env("HOME", home)
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
        .stdout(predicate::str::contains("echo zsh"));
}

#[test]
fn doctor_reports_missing_env_vars_when_not_set() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    sdbh_cmd()
        .env_remove("SDBH_SALT")
        .env_remove("SDBH_PPID")
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "doctor",
            "--no-spawn",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("SDBH_SALT").and(predicate::str::contains("is not set")))
        .stdout(predicate::str::contains("SDBH_PPID").and(predicate::str::contains("is not set")));
}

#[test]
fn doctor_detects_hook_via_prompt_command_env() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.sqlite");

    sdbh_cmd()
        .env("PROMPT_COMMAND", "__sdbh_prompt")
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "doctor",
            "--no-spawn",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("bash.hook.env")
                .and(predicate::str::contains("contains __sdbh_prompt")),
        );
}
