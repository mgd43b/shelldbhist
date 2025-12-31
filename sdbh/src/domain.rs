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
    pub description: Option<String>,
    /// The command template with {variable} placeholders
    pub command: String,
    /// Category for organization (git, docker, kubernetes, etc.)
    pub category: Option<String>,
    /// List of variables that can be used in the command
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
