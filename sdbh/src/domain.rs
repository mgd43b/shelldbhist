use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub hist_id: Option<i64>,
    pub cmd: String,
    pub epoch: i64,
    pub ppid: i64,
    pub pwd: String,
    pub salt: i64,
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub path: PathBuf,
}

impl DbConfig {
    pub fn default_path() -> PathBuf {
        // Simple portable default (matches product decision)
        let home = std::env::var_os("HOME").unwrap_or_default();
        PathBuf::from(home).join(".sdbh.sqlite")
    }
}

// Command Templates System domain models

/// A command template with variable substitution
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Template {
    /// Unique template identifier (filename without extension)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this template does
    #[serde(default)]
    pub description: Option<String>,
    /// The command template with {variable} placeholders
    pub command: String,
    /// Category for organization (git, docker, kubernetes, etc.)
    #[serde(default)]
    pub category: Option<String>,
    /// List of variables that can be used in the command
    #[serde(default)]
    pub variables: Vec<Variable>,
    /// Default values for variables
    #[serde(default)]
    pub defaults: HashMap<String, String>,
}

/// A variable definition within a template
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Variable {
    /// Variable name (used in {name} placeholders)
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Whether this variable is required (default: true)
    #[serde(default = "default_true")]
    pub required: bool,
    /// Default value if not provided
    pub default: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Template with resolved variables, ready for execution
#[derive(Debug, Clone)]
pub struct ResolvedTemplate {
    #[allow(dead_code)]
    pub template: Template,
    pub resolved_command: String,
    #[allow(dead_code)]
    pub variables_used: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_db_config_default_path() {
        // Test with HOME set
        unsafe { env::set_var("HOME", "/home/testuser") };
        let path = DbConfig::default_path();
        assert_eq!(path, PathBuf::from("/home/testuser/.sdbh.sqlite"));

        // Test with HOME unset (should use empty string)
        unsafe { env::remove_var("HOME") };
        let path = DbConfig::default_path();
        assert_eq!(path, PathBuf::from(".sdbh.sqlite"));
    }

    #[test]
    fn test_template_serialization() {
        let mut defaults = HashMap::new();
        defaults.insert("env".to_string(), "dev".to_string());

        let template = Template {
            id: "test-template".to_string(),
            name: "Test Template".to_string(),
            description: Some("A test template".to_string()),
            command: "echo {message}".to_string(),
            category: Some("test".to_string()),
            variables: vec![Variable {
                name: "message".to_string(),
                description: Some("Message to echo".to_string()),
                required: true,
                default: Some("hello".to_string()),
            }],
            defaults,
        };

        // Test TOML serialization
        let toml = toml::to_string(&template).unwrap();

        // Test TOML deserialization
        let deserialized: Template = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.id, template.id);
        assert_eq!(deserialized.name, template.name);
        assert_eq!(deserialized.description, template.description);
        assert_eq!(deserialized.command, template.command);
        assert_eq!(deserialized.category, template.category);
        assert_eq!(deserialized.variables.len(), template.variables.len());
        assert_eq!(deserialized.defaults.get("env").unwrap(), "dev");
    }

    #[test]
    fn test_variable_serialization() {
        let variable = Variable {
            name: "test_var".to_string(),
            description: Some("A test variable".to_string()),
            required: false,
            default: Some("default_value".to_string()),
        };

        // Test TOML serialization
        let toml = toml::to_string(&variable).unwrap();

        // Test TOML deserialization
        let deserialized: Variable = toml::from_str(&toml).unwrap();

        assert_eq!(deserialized.name, variable.name);
        assert_eq!(deserialized.description, variable.description);
        assert_eq!(deserialized.required, variable.required);
        assert_eq!(deserialized.default, variable.default);
    }

    #[test]
    fn test_variable_default_required() {
        // Test that required defaults to true
        let toml_str = r#"
            name = "test_var"
            description = "A test variable"
        "#;

        let variable: Variable = toml::from_str(toml_str).unwrap();
        assert_eq!(variable.required, true); // Should default to true
    }

    #[test]
    fn test_template_defaults_empty() {
        // Test that defaults field defaults to empty HashMap
        let toml_str = r#"
            id = "test"
            name = "Test"
            command = "echo hello"
            variables = []
        "#;

        let template: Template = toml::from_str(toml_str).unwrap();
        assert!(template.defaults.is_empty());
    }

    #[test]
    fn test_history_row_debug() {
        let row = HistoryRow {
            hist_id: Some(123),
            cmd: "ls -la".to_string(),
            epoch: 1640995200,
            ppid: 456,
            pwd: "/home/user".to_string(),
            salt: 789,
        };

        // Test Debug formatting (implicitly tested by assert)
        assert_eq!(format!("{:?}", row).len() > 0, true);
    }
}
