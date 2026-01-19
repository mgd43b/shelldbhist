/// Confidence level for garbage detection
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum ConfidenceLevel {
    Low = 1,
    Moderate = 2,
    High = 3,
}

/// A candidate command that might be garbage
#[derive(Debug, Clone)]
pub struct GarbageCandidate {
    pub id: i64,
    pub cmd: String,
    pub epoch: i64,
    pub pwd: String,
    pub size_bytes: usize,
    pub confidence_score: f32,
    pub confidence_level: ConfidenceLevel,
    pub reasons: Vec<String>,
}

use crate::config::CleanupConfig;

/// Analyze a command to determine if it's likely garbage
/// Returns (confidence_score, reasons)
/// 
/// If a config is provided, commands matching the allow-list will return (0.0, vec![])
/// and custom size thresholds will be used.
pub fn analyze_command_for_garbage(cmd: &str) -> (f32, Vec<String>) {
    analyze_command_for_garbage_with_config(cmd, &CleanupConfig::default())
}

/// Analyze a command to determine if it's likely garbage with configuration
/// Returns (confidence_score, reasons)
pub fn analyze_command_for_garbage_with_config(
    cmd: &str,
    config: &CleanupConfig,
) -> (f32, Vec<String>) {
    // Check allow-list first - exempt commands get 0 score
    if config.is_allowed(cmd) {
        return (0.0, vec![]);
    }

    let mut score: f32 = 0.0;
    let mut reasons = Vec::new();
    let size = cmd.len();

    // Check for binary content (high confidence indicators)
    if contains_binary_magic_numbers(cmd) {
        score += 50.0;
        reasons.push("Binary file magic number detected".to_string());
    }

    if contains_null_bytes(cmd) {
        score += 40.0;
        reasons.push("Null bytes detected (binary content)".to_string());
    }

    let non_printable_ratio = calculate_non_printable_ratio(cmd);
    if non_printable_ratio > 0.2 {
        score += 35.0;
        reasons.push(format!(
            "High non-printable character ratio ({:.1}%)",
            non_printable_ratio * 100.0
        ));
    }

    // Pattern-based detection (check before size scoring to inform decisions)
    let has_repetition = has_repeated_patterns(cmd);
    
    // Size-based scoring (conservative) - use configurable thresholds
    if size > config.size_threshold_large {
        // >large threshold (default 10KB) - but be lenient with legitimate patterns
        if has_legitimate_patterns(cmd) {
            score += 15.0;  // Reduced score for large legitimate commands
            reasons.push(format!("Very large command ({}KB)", size / 1024));
        } else {
            score += 30.0;
            reasons.push(format!("Very large command ({}KB)", size / 1024));
        }
    } else if size >= config.size_threshold_medium {
        // medium-large range (default 2-10KB)
        // Only score if no legitimate patterns detected
        if !has_legitimate_patterns(cmd) {
            score += 15.0;
            reasons.push(format!("Large command ({}KB)", size / 1024));
        }
    } else if size > config.size_threshold_small {
        // small-medium range (default 500-2KB)
        // Only mention, don't score heavily
        if !has_legitimate_patterns(cmd) {
            score += 5.0;
            reasons.push(format!("Medium-sized command ({} bytes)", size));
        }
    }

    // Add repetition score after size checks
    if has_repetition {
        score += 25.0;
        reasons.push("Repetitive content pattern".to_string());
    }

    if has_excessive_newlines(cmd) && !has_heredoc_or_script(cmd) {
        score += 15.0;
        let newline_count = cmd.matches('\n').count();
        reasons.push(format!("Excessive newlines ({}) without shell syntax", newline_count));
    }

    // Clamp score to 0-100 range
    score = score.min(100.0).max(0.0);

    (score, reasons)
}

/// Determine confidence level from score
pub fn score_to_confidence_level(score: f32) -> ConfidenceLevel {
    if score >= 60.0 {
        ConfidenceLevel::High
    } else if score >= 30.0 {
        ConfidenceLevel::Moderate
    } else {
        ConfidenceLevel::Low
    }
}

/// Check for binary file magic numbers
fn contains_binary_magic_numbers(cmd: &str) -> bool {
    let bytes = cmd.as_bytes();
    if bytes.len() < 4 {
        return false;
    }

    // ELF executable
    if bytes.starts_with(b"\x7fELF") {
        return true;
    }

    // PNG image
    if bytes.starts_with(b"\x89PNG") {
        return true;
    }

    // PDF document
    if bytes.starts_with(b"%PDF") {
        return true;
    }

    // JPEG image
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        return true;
    }

    // ZIP/JAR/etc
    if bytes.starts_with(b"PK\x03\x04") {
        return true;
    }

    // GIF image
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return true;
    }

    false
}

/// Check for null bytes (strong indicator of binary content)
fn contains_null_bytes(cmd: &str) -> bool {
    cmd.contains('\0')
}

/// Calculate ratio of non-printable characters
fn calculate_non_printable_ratio(cmd: &str) -> f32 {
    if cmd.is_empty() {
        return 0.0;
    }

    let non_printable_count = cmd
        .chars()
        .filter(|c| {
            // Consider printable: ASCII printable + common whitespace + Unicode
            let ch = *c as u32;
            !(ch >= 32 && ch <= 126) // ASCII printable range
                && !matches!(ch, 9 | 10 | 13) // tab, newline, carriage return
                && ch < 128 // Exclude extended Unicode (might be legitimate)
        })
        .count();

    non_printable_count as f32 / cmd.len() as f32
}

/// Check if command has legitimate patterns (to avoid false positives)
fn has_legitimate_patterns(cmd: &str) -> bool {
    // Heredoc pattern
    if cmd.contains("<<") && (cmd.contains("EOF") || cmd.contains("END")) {
        return true;
    }

    // JSON structure (balanced braces)
    let open_braces = cmd.matches('{').count();
    let close_braces = cmd.matches('}').count();
    if open_braces > 2 && open_braces == close_braces {
        // Likely JSON
        if cmd.contains("\"") && cmd.contains(":") {
            return true;
        }
    }

    // SQL query keywords
    if cmd.to_uppercase().contains("SELECT")
        || cmd.to_uppercase().contains("INSERT")
        || cmd.to_uppercase().contains("UPDATE")
        || cmd.to_uppercase().contains("CREATE TABLE")
    {
        return true;
    }

    // curl with data (common for API calls with large payloads)
    if cmd.starts_with("curl") && (cmd.contains("-d ") || cmd.contains("--data")) {
        return true;
    }
    
    // curl with @ file reference
    if cmd.starts_with("curl") && cmd.contains("-d @") {
        return true;
    }

    // Python/Ruby/etc multi-line scripts
    if cmd.contains("def ") || cmd.contains("class ") || cmd.contains("import ") {
        return true;
    }

    false
}

/// Check for repetitive patterns (like accidental paste of repeated chars)
fn has_repeated_patterns(cmd: &str) -> bool {
    if cmd.len() < 100 {
        return false;
    }

    // Check for runs of the same character
    let mut max_run = 0;
    let mut current_run = 1;
    let chars: Vec<char> = cmd.chars().collect();

    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] {
            current_run += 1;
            max_run = max_run.max(current_run);
        } else {
            current_run = 1;
        }
    }

    // If >30% of the command is a single repeated character
    if max_run > cmd.len() * 30 / 100 {
        return true;
    }

    // Check for repeated short patterns (like "abcabcabcabc...")
    // Try multiple pattern lengths
    for pattern_len in 2..=10 {
        if cmd.len() < pattern_len * 11 {
            continue;
        }

        // Sample from start of string
        let pattern = &cmd[0..pattern_len];
        let mut match_count = 0;

        // Count how many times the pattern appears consecutively from the start
        for i in (0..cmd.len()).step_by(pattern_len) {
            if i + pattern_len > cmd.len() {
                break;
            }
            if &cmd[i..i + pattern_len] == pattern {
                match_count += 1;
            } else {
                break; // Stop on first non-match
            }
        }

        // If pattern repeats >10 times from the start
        if match_count > 10 {
            return true;
        }
    }

    false
}

/// Check for excessive newlines without proper shell syntax
fn has_excessive_newlines(cmd: &str) -> bool {
    let newline_count = cmd.matches('\n').count();
    newline_count > 50
}

/// Check if command contains heredoc or script syntax
fn has_heredoc_or_script(cmd: &str) -> bool {
    // Heredoc
    if cmd.contains("<<EOF") || cmd.contains("<<END") || cmd.contains("<<-") {
        return true;
    }

    // Script shebang
    if cmd.starts_with("#!") {
        return true;
    }

    // Common scripting keywords
    if cmd.contains("#!/bin/bash")
        || cmd.contains("#!/bin/sh")
        || cmd.contains("#!/usr/bin/env")
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_list_exact_match() {
        let config = CleanupConfig {
            allow_list: vec!["curl".to_string()],
            ..Default::default()
        };
        let (score, reasons) = analyze_command_for_garbage_with_config("curl", &config);
        assert_eq!(score, 0.0);
        assert_eq!(reasons.len(), 0);
    }

    #[test]
    fn test_allow_list_wildcard_match() {
        let config = CleanupConfig {
            allow_list: vec!["curl *".to_string()],
            ..Default::default()
        };
        let cmd = "curl https://example.com -d '{\"data\": \"test\"}'";
        let (score, reasons) = analyze_command_for_garbage_with_config(cmd, &config);
        assert_eq!(score, 0.0);
        assert_eq!(reasons.len(), 0);
    }

    #[test]
    fn test_allow_list_no_match() {
        let config = CleanupConfig {
            allow_list: vec!["curl *".to_string()],
            ..Default::default()
        };
        let cmd = "\x7fELF binary content";
        let (score, _reasons) = analyze_command_for_garbage_with_config(cmd, &config);
        assert!(score > 0.0, "Non-allowed command should still be scored");
    }

    #[test]
    fn test_custom_size_thresholds() {
        let config = CleanupConfig {
            allow_list: vec![],
            size_threshold_small: 100,
            size_threshold_medium: 200,
            size_threshold_large: 300,
        };

        // Test small threshold (101 bytes should be scored)
        let cmd_small = "x".repeat(101);
        let (score_small, reasons_small) = analyze_command_for_garbage_with_config(&cmd_small, &config);
        assert!(score_small > 0.0, "Command exceeding small threshold should be scored");
        assert!(
            reasons_small.iter().any(|r| r.contains("Medium-sized")),
            "Should mention medium size"
        );

        // Test medium threshold (201 bytes)
        let cmd_medium = "y".repeat(201);
        let (score_medium, reasons_medium) = analyze_command_for_garbage_with_config(&cmd_medium, &config);
        assert!(score_medium > score_small, "Medium command should score higher");
        assert!(
            reasons_medium.iter().any(|r| r.contains("Large command")),
            "Should mention large size"
        );

        // Test large threshold (301 bytes)
        let cmd_large = "z".repeat(301);
        let (score_large, reasons_large) = analyze_command_for_garbage_with_config(&cmd_large, &config);
        assert!(score_large > score_medium, "Large command should score higher");
        assert!(
            reasons_large.iter().any(|r| r.contains("Very large command")),
            "Should mention very large size"
        );
    }

    #[test]
    fn test_default_config_backward_compatibility() {
        // Calling without config should use defaults
        let cmd = "x".repeat(501);
        let (score_new, reasons_new) = analyze_command_for_garbage(&cmd);
        let (score_config, reasons_config) = 
            analyze_command_for_garbage_with_config(&cmd, &CleanupConfig::default());
        
        assert_eq!(score_new, score_config, "Default function should match default config");
        assert_eq!(reasons_new, reasons_config);
    }

    #[test]
    fn test_binary_content_elf() {
        let cmd = "\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let (score, reasons) = analyze_command_for_garbage(cmd);
        assert!(score >= 60.0, "ELF binary should have high confidence");
        assert!(reasons.iter().any(|r| r.contains("Binary file magic")));
    }

    #[test]
    fn test_binary_content_png() {
        // PNG magic number as bytes, then convert to string (will have invalid UTF-8)
        let bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR...";
        let cmd = String::from_utf8_lossy(bytes);
        let (score, reasons) = analyze_command_for_garbage(&cmd);
        // PNG detection might not work after lossy conversion, but null bytes should be detected
        assert!(score >= 40.0, "PNG binary should have moderate-high confidence (score: {})", score);
        assert!(reasons.iter().any(|r| r.contains("Null bytes") || r.contains("Binary file magic")));
    }

    #[test]
    fn test_binary_content_pdf() {
        let cmd = "%PDF-1.4\n%âãÏÓ...";
        let (score, reasons) = analyze_command_for_garbage(cmd);
        // PDF starts with %PDF which is detected
        assert!(score >= 50.0, "PDF binary should have high confidence");
        assert!(reasons.iter().any(|r| r.contains("Binary file magic")));
    }

    #[test]
    fn test_null_bytes() {
        let cmd = "some command\0with null\0bytes";
        let (score, reasons) = analyze_command_for_garbage(cmd);
        assert!(score >= 40.0, "Null bytes should indicate garbage");
        assert!(reasons.iter().any(|r| r.contains("Null bytes")));
    }

    #[test]
    fn test_large_valid_sql_query() {
        let sql = format!(
            "SELECT * FROM users WHERE id IN ({}) ORDER BY created_at DESC",
            (1..=500).map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
        );
        let (score, _reasons) = analyze_command_for_garbage(&sql);
        assert!(
            score < 30.0,
            "Large SQL query should not be high confidence garbage (score: {})",
            score
        );
    }

    #[test]
    fn test_curl_with_large_json() {
        let json_data = format!(
            r#"{{"users": [{}]}}"#,
            (1..=200)
                .map(|i| format!(r#"{{"id": {}, "name": "User {}", "email": "user{}@example.com"}}"#, i, i, i))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let cmd = format!("curl -X POST https://api.example.com/users -H 'Content-Type: application/json' -d '{}'", json_data);
        let (score, _reasons) = analyze_command_for_garbage(&cmd);
        // Legitimate curl with large data gets moderate score (30) but not high confidence
        assert!(
            score < 60.0,
            "curl with large JSON should not be high confidence (score: {})",
            score
        );
    }

    #[test]
    fn test_heredoc() {
        let cmd = r#"cat <<EOF > file.txt
Line 1
Line 2
Line 3
EOF"#;
        let (score, _reasons) = analyze_command_for_garbage(cmd);
        assert!(
            score < 30.0,
            "Heredoc should not be flagged (score: {})",
            score
        );
    }

    #[test]
    fn test_multiline_python_script() {
        let cmd = r#"python3 << 'EOF'
import sys
import json

def process_data(data):
    return [x * 2 for x in data]

if __name__ == '__main__':
    data = [1, 2, 3, 4, 5]
    result = process_data(data)
    print(json.dumps(result))
EOF"#;
        let (score, _reasons) = analyze_command_for_garbage(cmd);
        assert!(
            score < 30.0,
            "Python script should not be flagged (score: {})",
            score
        );
    }

    #[test]
    fn test_repeated_characters() {
        let cmd = "a".repeat(5000);
        let (score, reasons) = analyze_command_for_garbage(&cmd);
        assert!(score >= 30.0, "Repeated characters should be flagged");
        assert!(reasons.iter().any(|r| r.contains("Repetitive content")));
    }

    #[test]
    fn test_repeated_pattern() {
        let cmd = "abc".repeat(500);
        let (score, reasons) = analyze_command_for_garbage(&cmd);
        assert!(score >= 30.0, "Repeated pattern should be flagged");
        assert!(reasons.iter().any(|r| r.contains("Repetitive content")));
    }

    #[test]
    fn test_excessive_newlines_no_syntax() {
        let cmd = "\n".repeat(100);
        let (score, reasons) = analyze_command_for_garbage(&cmd);
        assert!(score >= 15.0, "Excessive newlines should be flagged");
        assert!(reasons.iter().any(|r| r.contains("Excessive newlines")));
    }

    #[test]
    fn test_size_threshold_500_bytes() {
        // Exactly 500 bytes
        let cmd = "a".repeat(500);
        let (score, _reasons) = analyze_command_for_garbage(&cmd);
        // Should have low score since it's at boundary
        assert!(score < 30.0, "500 bytes should be low confidence");
    }

    #[test]
    fn test_size_threshold_2kb() {
        // Exactly 2048 bytes without legitimate patterns
        let cmd = "x".repeat(2048);
        let (score, reasons) = analyze_command_for_garbage(&cmd);
        assert!(score >= 30.0, "2KB garbage should be moderate confidence (score: {})", score);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("Large command") || r.contains("Repetitive")),
            "Reasons should mention size or repetition: {:?}",
            reasons
        );
    }

    #[test]
    fn test_size_threshold_10kb() {
        // Exactly 10240 bytes - one more than the threshold
        let cmd = "y".repeat(10241);
        let (score, reasons) = analyze_command_for_garbage(&cmd);
        assert!(score >= 30.0, "10KB should have decent confidence (score: {})", score);
        assert!(
            reasons.iter().any(|r| r.contains("Very large command")),
            "Should have 'Very large command' reason: {:?}",
            reasons
        );
    }

    #[test]
    fn test_legitimate_large_curl_15kb() {
        // 15KB curl command with valid JSON
        let large_json = format!(
            r#"{{"data": "{}"}}"#,
            "x".repeat(15000)
        );
        let cmd = format!("curl -X POST https://api.example.com/upload -d '{}'", large_json);
        let (score, _reasons) = analyze_command_for_garbage(&cmd);
        // Large legitimate curl gets moderate score (size + maybe repetition), but not high
        assert!(
            score < 60.0,
            "15KB curl command should not be high confidence (score: {})",
            score
        );
    }

    #[test]
    fn test_normal_command() {
        let cmd = "ls -la /home/user/documents";
        let (score, _reasons) = analyze_command_for_garbage(cmd);
        assert!(score < 10.0, "Normal command should have very low score");
    }

    #[test]
    fn test_confidence_level_mapping() {
        assert_eq!(score_to_confidence_level(80.0), ConfidenceLevel::High);
        assert_eq!(score_to_confidence_level(60.0), ConfidenceLevel::High);
        assert_eq!(score_to_confidence_level(50.0), ConfidenceLevel::Moderate);
        assert_eq!(score_to_confidence_level(30.0), ConfidenceLevel::Moderate);
        assert_eq!(score_to_confidence_level(20.0), ConfidenceLevel::Low);
        assert_eq!(score_to_confidence_level(0.0), ConfidenceLevel::Low);
    }

    #[test]
    fn test_edge_case_empty_command() {
        let cmd = "";
        let (score, _reasons) = analyze_command_for_garbage(cmd);
        assert_eq!(score, 0.0, "Empty command should have zero score");
    }

    #[test]
    fn test_edge_case_whitespace_only() {
        let cmd = "   \t\n  ";
        let (score, _reasons) = analyze_command_for_garbage(cmd);
        assert!(score < 10.0, "Whitespace only should have very low score");
    }

    #[test]
    fn test_high_non_printable_ratio() {
        // Mix of binary and text
        let mut cmd = String::from("command ");
        for _ in 0..100 {
            cmd.push('\x01');
            cmd.push('\x02');
            cmd.push('\x03');
        }
        let (score, reasons) = analyze_command_for_garbage(&cmd);
        assert!(score >= 30.0, "High non-printable ratio should be flagged");
        assert!(reasons.iter().any(|r| r.contains("non-printable")));
    }
}