use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration for garbage detection and cleanup
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CleanupConfig {
    /// Commands/patterns to never consider as garbage
    /// Supports simple glob patterns (*, ?)
    #[serde(default)]
    pub allow_list: Vec<String>,

    /// Size threshold for "medium-sized" classification (default: 500 bytes)
    #[serde(default = "default_size_threshold_small")]
    pub size_threshold_small: usize,

    /// Size threshold for "large" classification (default: 2048 bytes)
    #[serde(default = "default_size_threshold_medium")]
    pub size_threshold_medium: usize,

    /// Size threshold for "very large" classification (default: 10240 bytes)
    #[serde(default = "default_size_threshold_large")]
    pub size_threshold_large: usize,
}

fn default_size_threshold_small() -> usize {
    500
}

fn default_size_threshold_medium() -> usize {
    2048
}

fn default_size_threshold_large() -> usize {
    10240
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            allow_list: Vec::new(),
            size_threshold_small: default_size_threshold_small(),
            size_threshold_medium: default_size_threshold_medium(),
            size_threshold_large: default_size_threshold_large(),
        }
    }
}

impl CleanupConfig {
    /// Check if a command matches the allow-list
    pub fn is_allowed(&self, cmd: &str) -> bool {
        for pattern in &self.allow_list {
            if matches_pattern(cmd, pattern) {
                return true;
            }
        }
        false
    }

    /// Validate that thresholds are in ascending order
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        if self.size_threshold_small >= self.size_threshold_medium {
            anyhow::bail!(
                "size_threshold_small ({}) must be less than size_threshold_medium ({})",
                self.size_threshold_small,
                self.size_threshold_medium
            );
        }
        if self.size_threshold_medium >= self.size_threshold_large {
            anyhow::bail!(
                "size_threshold_medium ({}) must be less than size_threshold_large ({})",
                self.size_threshold_medium,
                self.size_threshold_large
            );
        }
        Ok(())
    }
}

/// Application configuration (root config file structure)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub cleanup: CleanupConfig,
}

impl AppConfig {
    /// Load configuration from ~/.sdbh.toml
    #[allow(dead_code)]
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: AppConfig = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // Validate cleanup config
        config
            .cleanup
            .validate()
            .with_context(|| "Invalid cleanup configuration")?;

        Ok(config)
    }

    /// Save configuration to ~/.sdbh.toml
    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        // Validate before saving
        self.cleanup
            .validate()
            .with_context(|| "Invalid cleanup configuration")?;

        let path = Self::config_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Get the path to the config file (~/.sdbh.toml)
    #[allow(dead_code)]
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".sdbh.toml")
    }
}

/// Simple pattern matching supporting * and ?
///
/// * matches any sequence of characters
///
/// ? matches any single character
fn matches_pattern(text: &str, pattern: &str) -> bool {
    // Simple recursive implementation
    matches_pattern_impl(text, pattern)
}

fn matches_pattern_impl(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    matches_recursive(&text_chars, &pattern_chars, 0, 0)
}

fn matches_recursive(text: &[char], pattern: &[char], t_idx: usize, p_idx: usize) -> bool {
    // Base cases
    if p_idx == pattern.len() {
        return t_idx == text.len();
    }

    if t_idx == text.len() {
        // Check if remaining pattern is all stars
        return pattern[p_idx..].iter().all(|&c| c == '*');
    }

    match pattern[p_idx] {
        '*' => {
            // Try matching zero or more characters
            // First try matching zero characters (skip the *)
            if matches_recursive(text, pattern, t_idx, p_idx + 1) {
                return true;
            }
            // Then try matching one or more characters
            matches_recursive(text, pattern, t_idx + 1, p_idx)
        }
        '?' => {
            // Match any single character
            matches_recursive(text, pattern, t_idx + 1, p_idx + 1)
        }
        c => {
            // Exact character match required
            if text[t_idx] == c {
                matches_recursive(text, pattern, t_idx + 1, p_idx + 1)
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_config_defaults() {
        let config = CleanupConfig::default();
        assert_eq!(config.allow_list.len(), 0);
        assert_eq!(config.size_threshold_small, 500);
        assert_eq!(config.size_threshold_medium, 2048);
        assert_eq!(config.size_threshold_large, 10240);
    }

    #[test]
    fn test_cleanup_config_validation_valid() {
        let config = CleanupConfig {
            allow_list: vec![],
            size_threshold_small: 100,
            size_threshold_medium: 500,
            size_threshold_large: 1000,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_cleanup_config_validation_small_not_less_than_medium() {
        let config = CleanupConfig {
            allow_list: vec![],
            size_threshold_small: 2048,
            size_threshold_medium: 2048,
            size_threshold_large: 10240,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cleanup_config_validation_medium_not_less_than_large() {
        let config = CleanupConfig {
            allow_list: vec![],
            size_threshold_small: 500,
            size_threshold_medium: 10240,
            size_threshold_large: 10240,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cleanup_config_is_allowed_exact_match() {
        let config = CleanupConfig {
            allow_list: vec!["curl".to_string(), "wget".to_string()],
            ..Default::default()
        };
        assert!(config.is_allowed("curl"));
        assert!(config.is_allowed("wget"));
        assert!(!config.is_allowed("ls"));
    }

    #[test]
    fn test_cleanup_config_is_allowed_wildcard() {
        let config = CleanupConfig {
            allow_list: vec!["curl *".to_string(), "git commit*".to_string()],
            ..Default::default()
        };
        assert!(config.is_allowed("curl https://example.com"));
        assert!(config.is_allowed("curl -X POST https://api.example.com"));
        assert!(config.is_allowed("git commit -m 'test'"));
        assert!(config.is_allowed("git commit"));
        assert!(!config.is_allowed("git push"));
        assert!(!config.is_allowed("wget https://example.com"));
    }

    #[test]
    fn test_cleanup_config_is_allowed_question_mark() {
        let config = CleanupConfig {
            allow_list: vec!["ls -?".to_string()],
            ..Default::default()
        };
        assert!(config.is_allowed("ls -l"));
        assert!(config.is_allowed("ls -a"));
        assert!(!config.is_allowed("ls -la"));
        assert!(!config.is_allowed("ls"));
    }

    #[test]
    fn test_pattern_matching_exact() {
        assert!(matches_pattern("hello", "hello"));
        assert!(!matches_pattern("hello", "world"));
    }

    #[test]
    fn test_pattern_matching_star() {
        assert!(matches_pattern("hello world", "hello*"));
        assert!(matches_pattern("hello world", "*world"));
        assert!(matches_pattern("hello world", "*"));
        assert!(matches_pattern("hello world", "hello*world"));
        assert!(matches_pattern("", "*"));
        assert!(!matches_pattern("hello", "world*"));
    }

    #[test]
    fn test_pattern_matching_question() {
        assert!(matches_pattern("hello", "h?llo"));
        assert!(matches_pattern("hallo", "h?llo"));
        assert!(!matches_pattern("hello", "h??llo"));
        assert!(!matches_pattern("hllo", "h?llo"));
    }

    #[test]
    fn test_pattern_matching_combined() {
        assert!(matches_pattern(
            "curl -X POST https://api.example.com",
            "curl *"
        ));
        assert!(matches_pattern("git commit -m 'test'", "git commit*"));
        assert!(matches_pattern("ls -la", "ls -??"));
        assert!(matches_pattern("mv file.txt backup.txt", "mv *.txt *.txt"));
    }

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfig {
            cleanup: CleanupConfig {
                allow_list: vec!["curl *".to_string(), "wget *".to_string()],
                size_threshold_small: 1000,
                size_threshold_medium: 5000,
                size_threshold_large: 20000,
            },
        };

        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(deserialized.cleanup.allow_list, config.cleanup.allow_list);
        assert_eq!(
            deserialized.cleanup.size_threshold_small,
            config.cleanup.size_threshold_small
        );
        assert_eq!(
            deserialized.cleanup.size_threshold_medium,
            config.cleanup.size_threshold_medium
        );
        assert_eq!(
            deserialized.cleanup.size_threshold_large,
            config.cleanup.size_threshold_large
        );
    }

    #[test]
    fn test_app_config_defaults_in_toml() {
        // Empty TOML should deserialize to defaults
        let toml_str = "";
        let config: AppConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.cleanup.allow_list.len(), 0);
        assert_eq!(config.cleanup.size_threshold_small, 500);
        assert_eq!(config.cleanup.size_threshold_medium, 2048);
        assert_eq!(config.cleanup.size_threshold_large, 10240);
    }

    #[test]
    fn test_app_config_partial_cleanup_section() {
        // Only specify some cleanup fields, rest should use defaults
        let toml_str = r#"
[cleanup]
allow_list = ["curl *", "wget *"]
size_threshold_large = 50000
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.cleanup.allow_list.len(), 2);
        assert_eq!(config.cleanup.size_threshold_small, 500); // default
        assert_eq!(config.cleanup.size_threshold_medium, 2048); // default
        assert_eq!(config.cleanup.size_threshold_large, 50000); // custom
    }

    #[test]
    fn test_app_config_save_and_load() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join(".sdbh.toml");

        // Create a config with custom values
        let original_config = AppConfig {
            cleanup: CleanupConfig {
                allow_list: vec!["test *".to_string()],
                size_threshold_small: 1000,
                size_threshold_medium: 5000,
                size_threshold_large: 20000,
            },
        };

        // Save to temp file
        let contents = toml::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, contents).unwrap();

        // Load from temp file
        let loaded_contents = fs::read_to_string(&config_path).unwrap();
        let loaded_config: AppConfig = toml::from_str(&loaded_contents).unwrap();

        assert_eq!(
            loaded_config.cleanup.allow_list,
            original_config.cleanup.allow_list
        );
        assert_eq!(
            loaded_config.cleanup.size_threshold_small,
            original_config.cleanup.size_threshold_small
        );
    }
}
