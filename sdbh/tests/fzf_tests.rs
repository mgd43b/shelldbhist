use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

/// Test that preview command accepts positional argument (correct usage)
#[test]
fn test_preview_with_positional_arg() {
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path();

    // Initialize database with a test command
    Command::cargo_bin("sdbh")
        .unwrap()
        .args(["--db", db_path.to_str().unwrap()])
        .args([
            "log",
            "--cmd",
            "git status",
            "--epoch",
            "1700000000",
            "--ppid",
            "1234",
            "--pwd",
            "/tmp",
            "--salt",
            "42",
        ])
        .assert()
        .success();

    // Preview with positional argument should work
    Command::cargo_bin("sdbh")
        .unwrap()
        .args(["--db", db_path.to_str().unwrap()])
        .args(["preview", "git status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Command Analysis"));
}

/// Test that preview command rejects --command flag (incorrect usage that causes the bug)
#[test]
fn test_preview_rejects_command_flag() {
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path();

    // This should fail with "unexpected argument '--command'"
    Command::cargo_bin("sdbh")
        .unwrap()
        .args(["--db", db_path.to_str().unwrap()])
        .args(["preview", "--command", "git status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

/// Test extracting command from fzf list format: "cmd  (timestamp) [pwd]"
#[test]
fn test_extract_command_from_fzf_list_format() {
    let input = "git status  (2026-01-19 08:30:15) [/home/user/project]";
    let expected = "git status";

    // This will test the helper function once we add it
    let extracted = extract_command_from_fzf_line(input);
    assert_eq!(extracted, expected);
}

/// Test extracting command from fzf summary format: "cmd [pwd]  (count uses, last: timestamp)"
#[test]
fn test_extract_command_from_fzf_summary_format() {
    let input = "git commit [/home/user/project]  (42 uses, last: 2026-01-19 08:30:15)";
    let expected = "git commit";

    let extracted = extract_command_from_fzf_line(input);
    assert_eq!(extracted, expected);
}

/// Test extracting command from fzf stats format: "cmd  (count uses)"
#[test]
fn test_extract_command_from_fzf_stats_format() {
    let input = "cargo build  (156 uses)";
    let expected = "cargo build";

    let extracted = extract_command_from_fzf_line(input);
    assert_eq!(extracted, expected);
}

/// Test extracting command with complex arguments
#[test]
fn test_extract_command_with_complex_args() {
    let input = r#"git commit -m "fix: update tests"  (2026-01-19 08:30:15) [/home/user]"#;
    let expected = r#"git commit -m "fix: update tests""#;

    let extracted = extract_command_from_fzf_line(input);
    assert_eq!(extracted, expected);
}

/// Helper function to extract command from fzf formatted line
/// This matches the different fzf output formats used in sdbh
fn extract_command_from_fzf_line(line: &str) -> String {
    let line = line.trim();

    // Try to find the first occurrence of "  (" which separates command from metadata
    if let Some(pos) = line.find("  (") {
        // Extract everything before "  ("
        let cmd_part = &line[..pos];

        // If there's a " [" (pwd in summary format), remove it
        if let Some(bracket_pos) = cmd_part.find(" [") {
            cmd_part[..bracket_pos].trim().to_string()
        } else {
            cmd_part.trim().to_string()
        }
    } else {
        // Fallback: return the whole line if no metadata markers found
        line.to_string()
    }
}

/// Integration test: Verify fzf fails gracefully when fzf is not installed
#[test]
fn test_fzf_requires_fzf_binary() {
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path();

    // Add a test command
    Command::cargo_bin("sdbh")
        .unwrap()
        .args(["--db", db_path.to_str().unwrap()])
        .args([
            "log",
            "--cmd",
            "echo test",
            "--epoch",
            "1700000000",
            "--ppid",
            "1234",
            "--pwd",
            "/tmp",
            "--salt",
            "42",
        ])
        .assert()
        .success();

    // If fzf is not in PATH, should fail with clear error message
    // Note: This test may pass if fzf IS installed, which is fine
    // The important thing is it doesn't crash with the preview bug
    let result = Command::cargo_bin("sdbh")
        .unwrap()
        .args(["--db", db_path.to_str().unwrap()])
        .args(["list", "--fzf"])
        .env("PATH", "/nonexistent") // Ensure fzf won't be found
        .assert();

    // Should either succeed (if fzf somehow found) or fail with clear message
    if !result.get_output().status.success() {
        let stderr = String::from_utf8_lossy(&result.get_output().stderr);
        assert!(
            stderr.contains("fzf") || stderr.contains("not found"),
            "Expected error about fzf, got: {}",
            stderr
        );
    }
}
