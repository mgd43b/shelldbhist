use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::Connection;
use tempfile::TempDir;

fn sdbh_cmd() -> Command {
    let exe = assert_cmd::cargo::cargo_bin!("sdbh");
    Command::new(exe)
}

fn conn(path: &std::path::Path) -> Connection {
    Connection::open(path).unwrap()
}

fn setup_test_db(tmp: &TempDir) -> std::path::PathBuf {
    let db = tmp.path().join("test.sqlite");

    // Insert 10 test entries
    for i in 1..=10 {
        sdbh_cmd()
            .args([
                "--db",
                db.to_string_lossy().as_ref(),
                "log",
                "--cmd",
                &format!("echo test{}", i),
                "--epoch",
                &format!("17000000{:02}", i),
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

    db
}

#[test]
fn delete_single_id() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Delete ID 5 with --yes to skip confirmation
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "5",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted 1 entries"));

    // Verify entry 5 is gone but others remain
    let output = sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--limit",
            "20",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("echo test5"));
    assert!(stdout.contains("echo test1"));
    assert!(stdout.contains("echo test10"));
}

#[test]
fn delete_range_of_ids() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Delete range 3-6
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "3-6",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted 4 entries"));

    // Verify entries 3-6 are gone
    let output = sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--limit",
            "20",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("echo test3"));
    assert!(!stdout.contains("echo test4"));
    assert!(!stdout.contains("echo test5"));
    assert!(!stdout.contains("echo test6"));
    assert!(stdout.contains("echo test1"));
    assert!(stdout.contains("echo test2"));
    assert!(stdout.contains("echo test7"));
}

#[test]
fn delete_range_with_double_dots() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Delete range 2..5 (inclusive)
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "2..5",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted 4 entries"));
}

#[test]
fn delete_comma_separated_ids() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Delete IDs 2, 5, 8
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "2,5,8",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted 3 entries"));

    let output = sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--limit",
            "20",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("echo test2"));
    assert!(!stdout.contains("echo test5"));
    assert!(!stdout.contains("echo test8"));
    assert!(stdout.contains("echo test1"));
    assert!(stdout.contains("echo test3"));
}

#[test]
fn delete_mixed_format() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Delete mixed: 1, 3-5, 7
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "1,3-5,7",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted 5 entries"));

    let output = sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--limit",
            "20",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Use newline to avoid false positives with test10 containing test1
    assert!(!stdout.contains("echo test1\n"));
    assert!(!stdout.contains("echo test3"));
    assert!(!stdout.contains("echo test4"));
    assert!(!stdout.contains("echo test5"));
    assert!(!stdout.contains("echo test7"));
    assert!(stdout.contains("echo test2"));
    assert!(stdout.contains("echo test6"));
}

#[test]
fn delete_nonexistent_ids() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Try to delete IDs that don't exist (100-105)
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "100-105",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No matching entries found"));
}

#[test]
fn delete_partial_match() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Mix of existing and non-existing IDs
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "5,100,200",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted 1 entries"));
}

#[test]
fn delete_dry_run_mode() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Dry run should show what would be deleted but not delete
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "1-5",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Entries to be deleted (5 total)"))
        .stdout(predicate::str::contains("DRY RUN: No entries were deleted"));

    // Verify nothing was actually deleted
    let output = sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "list",
            "--all",
            "--limit",
            "20",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("echo test1"));
    assert!(stdout.contains("echo test5"));
}

#[test]
fn delete_with_yes_flag_skips_confirmation() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // --yes should delete without prompting
    let output = sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "3",
            "--yes",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should not contain confirmation prompt
    assert!(!stdout.contains("Are you sure"));
    assert!(stdout.contains("Successfully deleted 1 entries"));
}

#[test]
fn delete_invalid_range_start_greater_than_end() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Invalid range: 5-2
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "5-2",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid range"));
}

#[test]
fn delete_invalid_format_non_numeric() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Invalid: non-numeric
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "abc",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid ID"));
}

#[test]
fn delete_invalid_format_multiple_dashes() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Invalid: 1-2-3
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "1-2-3",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid range end"));
}

#[test]
fn delete_cleans_up_history_hash() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Get hash count before deletion
    let c = conn(&db);
    let hash_count_before: i64 = c
        .query_row("SELECT COUNT(*) FROM history_hash", [], |r| r.get(0))
        .unwrap();

    // Delete an entry
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "5",
            "--yes",
        ])
        .assert()
        .success();

    // Verify hash table was cleaned up
    let hash_count_after: i64 = c
        .query_row("SELECT COUNT(*) FROM history_hash", [], |r| r.get(0))
        .unwrap();

    assert_eq!(hash_count_before - 1, hash_count_after);
}

#[test]
fn delete_json_output_format() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Delete with JSON output
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "2,4",
            "--format",
            "json",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("["))
        .stdout(predicate::str::contains("\"id\""))
        .stdout(predicate::str::contains("\"cmd\""));
}

#[test]
fn delete_with_spaces_in_id_spec() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Should handle spaces: " 1 , 3 - 5 , 7 "
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            " 1 , 3 - 5 , 7 ",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted 5 entries"));
}

#[test]
fn delete_duplicate_ids_in_spec() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Duplicates should be handled: 5,5,5
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "5,5,5",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted 1 entries"));
}

#[test]
fn delete_empty_id_spec() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Empty spec should fail
    sdbh_cmd()
        .args(["--db", db.to_string_lossy().as_ref(), "delete", "", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No valid IDs provided"));
}

#[test]
fn delete_shows_preview_table() {
    let tmp = TempDir::new().unwrap();
    let db = setup_test_db(&tmp);

    // Preview should show entries in table format
    sdbh_cmd()
        .args([
            "--db",
            db.to_string_lossy().as_ref(),
            "delete",
            "1-3",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Entries to be deleted (3 total)"))
        .stdout(predicate::str::contains("echo test1"))
        .stdout(predicate::str::contains("echo test2"))
        .stdout(predicate::str::contains("echo test3"));
}
