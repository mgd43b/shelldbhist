use crate::domain::{ResolvedTemplate, Template};
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Template parsing and management engine
#[derive(Debug)]
pub struct TemplateEngine {
    templates_dir: PathBuf,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .context("Could not determine home directory")?;

        let templates_dir = PathBuf::from(home).join(".sdbh").join("templates");

        // Ensure templates directory exists
        fs::create_dir_all(&templates_dir).with_context(|| {
            format!(
                "Failed to create templates directory: {}",
                templates_dir.display()
            )
        })?;

        Ok(Self { templates_dir })
    }

    /// Get the templates directory path
    pub fn templates_dir(&self) -> &Path {
        &self.templates_dir
    }

    /// Load a template from a TOML file
    pub fn load_template(&self, template_id: &str) -> Result<Template> {
        let template_path = self.templates_dir.join(format!("{}.toml", template_id));

        if !template_path.exists() {
            anyhow::bail!(
                "Template '{}' not found at {}",
                template_id,
                template_path.display()
            );
        }

        let content = fs::read_to_string(&template_path).with_context(|| {
            format!("Failed to read template file: {}", template_path.display())
        })?;

        let mut template: Template = toml::from_str(&content).with_context(|| {
            format!("Failed to parse template TOML: {}", template_path.display())
        })?;

        // Set the ID from filename if not specified in TOML
        if template.id.is_empty() {
            template.id = template_id.to_string();
        }

        // Validate the template
        self.validate_template(&template)?;

        Ok(template)
    }

    /// List all available templates
    pub fn list_templates(&self) -> Result<Vec<Template>> {
        let mut templates = Vec::new();

        for entry in fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("toml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                match self.load_template(stem) {
                    Ok(template) => templates.push(template),
                    Err(e) => eprintln!("Warning: Failed to load template {}: {}", stem, e),
                }
            }
        }

        Ok(templates)
    }

    /// Save a template to disk
    pub fn save_template(&self, template: &Template) -> Result<()> {
        self.validate_template(template)?;

        let template_path = self.templates_dir.join(format!("{}.toml", template.id));
        let content =
            toml::to_string_pretty(template).context("Failed to serialize template to TOML")?;

        fs::write(&template_path, content).with_context(|| {
            format!("Failed to write template file: {}", template_path.display())
        })?;

        Ok(())
    }

    /// Delete a template
    pub fn delete_template(&self, template_id: &str) -> Result<()> {
        let template_path = self.templates_dir.join(format!("{}.toml", template_id));

        if !template_path.exists() {
            anyhow::bail!("Template '{}' not found", template_id);
        }

        fs::remove_file(&template_path).with_context(|| {
            format!(
                "Failed to delete template file: {}",
                template_path.display()
            )
        })?;

        Ok(())
    }

    /// Validate a template
    pub fn validate_template(&self, template: &Template) -> Result<()> {
        if template.id.is_empty() {
            anyhow::bail!("Template ID cannot be empty");
        }

        if template.name.is_empty() {
            anyhow::bail!("Template name cannot be empty");
        }

        if template.command.is_empty() {
            anyhow::bail!("Template command cannot be empty");
        }

        // Check for valid variable names
        for var in &template.variables {
            if var.name.is_empty() {
                anyhow::bail!("Variable name cannot be empty");
            }

            if !is_valid_variable_name(&var.name) {
                anyhow::bail!(
                    "Invalid variable name '{}': must be alphanumeric with underscores",
                    var.name
                );
            }
        }

        // Extract variables from command and ensure they're defined
        let command_vars = extract_variables(&template.command)?;
        let defined_vars: std::collections::HashSet<_> =
            template.variables.iter().map(|v| v.name.clone()).collect();

        for var in command_vars {
            if !defined_vars.contains(&var) {
                anyhow::bail!(
                    "Variable '{}' used in command but not defined in variables list",
                    var
                );
            }
        }

        Ok(())
    }

    /// Resolve a template with provided variables
    #[allow(dead_code)]
    pub fn resolve_template(
        &self,
        template: &Template,
        provided_vars: &HashMap<String, String>,
    ) -> Result<ResolvedTemplate> {
        let mut resolved_vars = HashMap::new();

        // Start with defaults
        for (key, value) in &template.defaults {
            resolved_vars.insert(key.clone(), value.clone());
        }

        // Override with provided variables
        for (key, value) in provided_vars {
            resolved_vars.insert(key.clone(), value.clone());
        }

        // Check for missing required variables
        for var in &template.variables {
            if var.required && !resolved_vars.contains_key(&var.name) {
                if let Some(default) = &var.default {
                    resolved_vars.insert(var.name.clone(), default.clone());
                } else {
                    anyhow::bail!(
                        "Required variable '{}' not provided and no default available",
                        var.name
                    );
                }
            }
        }

        // Perform variable substitution
        let resolved_command = substitute_variables(&template.command, &resolved_vars)?;

        Ok(ResolvedTemplate {
            template: template.clone(),
            resolved_command,
            variables_used: resolved_vars,
        })
    }

    /// Resolve a template with interactive prompting for missing variables
    pub fn resolve_template_interactive(
        &self,
        template: &Template,
        provided_vars: &HashMap<String, String>,
    ) -> Result<ResolvedTemplate> {
        let mut resolved_vars = HashMap::new();

        // Start with defaults
        for (key, value) in &template.defaults {
            resolved_vars.insert(key.clone(), value.clone());
        }

        // Override with provided variables
        for (key, value) in provided_vars {
            resolved_vars.insert(key.clone(), value.clone());
        }

        // Apply defaults for variables that don't have values yet
        for var in &template.variables {
            if !resolved_vars.contains_key(&var.name) {
                if let Some(default) = &var.default {
                    resolved_vars.insert(var.name.clone(), default.clone());
                }
            }
        }

        // Collect missing required variables that need prompting
        let mut missing_vars = Vec::new();
        for var in &template.variables {
            if var.required && !resolved_vars.contains_key(&var.name) {
                missing_vars.push(var.clone());
            }
        }

        // Prompt for missing variables interactively
        if !missing_vars.is_empty() {
            println!(
                "Template '{}' requires the following variables:",
                template.name
            );
            println!();

            for var in &missing_vars {
                let prompt_text = if let Some(desc) = &var.description {
                    format!("{} ({})", var.name, desc)
                } else {
                    var.name.clone()
                };

                let default_value = var.default.as_deref().unwrap_or("");

                let value = if !default_value.is_empty() {
                    dialoguer::Input::<String>::new()
                        .with_prompt(&prompt_text)
                        .default(default_value.to_string())
                        .interact_text()?
                } else {
                    dialoguer::Input::<String>::new()
                        .with_prompt(&prompt_text)
                        .interact_text()?
                };

                resolved_vars.insert(var.name.clone(), value);
            }
            println!();
        }

        // Perform variable substitution
        let resolved_command = substitute_variables(&template.command, &resolved_vars)?;

        Ok(ResolvedTemplate {
            template: template.clone(),
            resolved_command,
            variables_used: resolved_vars,
        })
    }
}

/// Extract variable names from a command string
pub fn extract_variables(command: &str) -> Result<Vec<String>> {
    let re = Regex::new(r"\{([^}]+)\}").context("Failed to create variable extraction regex")?;

    let mut variables = Vec::new();
    for cap in re.captures_iter(command) {
        if let Some(var_name) = cap.get(1) {
            variables.push(var_name.as_str().to_string());
        }
    }

    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    variables.retain(|v| seen.insert(v.clone()));

    Ok(variables)
}

/// Substitute variables in a command string
pub fn substitute_variables(command: &str, variables: &HashMap<String, String>) -> Result<String> {
    let mut result = command.to_string();

    for (var_name, var_value) in variables {
        let pattern = format!("{{{}}}", var_name);
        result = result.replace(&pattern, var_value);
    }

    // Check for unsubstituted variables
    if let Some(pos) = result.find('{')
        && let Some(end_pos) = result[pos..].find('}')
    {
        let var_name = &result[pos + 1..pos + end_pos];
        anyhow::bail!("Variable '{}' not provided", var_name);
    }

    Ok(result)
}

/// Check if a variable name is valid
fn is_valid_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
        && name.chars().next().unwrap().is_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    // Helper function to create a temporary template engine for testing
    fn create_test_engine() -> (TemplateEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let templates_dir = temp_dir.path().join("templates");
        std::fs::create_dir_all(&templates_dir).unwrap();

        // Create a mock TemplateEngine with the temp directory
        let engine = TemplateEngine {
            templates_dir: templates_dir.clone(),
        };

        (engine, temp_dir)
    }

    // Helper function to create a sample template
    fn create_sample_template() -> Template {
        Template {
            id: "test-template".to_string(),
            name: "Test Template".to_string(),
            description: Some("A test template".to_string()),
            command: "echo {message} from {user}".to_string(),
            category: Some("test".to_string()),
            variables: vec![
                crate::domain::Variable {
                    name: "message".to_string(),
                    description: Some("The message to echo".to_string()),
                    required: true,
                    default: Some("hello".to_string()),
                },
                crate::domain::Variable {
                    name: "user".to_string(),
                    description: Some("The user name".to_string()),
                    required: true,
                    default: None,
                },
            ],
            defaults: HashMap::new(),
        }
    }

    #[test]
    fn test_extract_variables() {
        assert_eq!(
            extract_variables("git commit -m '{message}'").unwrap(),
            vec!["message"]
        );
        assert_eq!(
            extract_variables("docker build -t {image}:{tag} .").unwrap(),
            vec!["image", "tag"]
        );
        assert_eq!(
            extract_variables("echo {var1} {var2} {var1}").unwrap(),
            vec!["var1", "var2"]
        );
        assert_eq!(
            extract_variables("no variables here").unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_substitute_variables() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());
        vars.insert("cmd".to_string(), "echo".to_string());

        assert_eq!(
            substitute_variables("Hello {name}", &vars).unwrap(),
            "Hello world"
        );
        assert_eq!(
            substitute_variables("{cmd} {name}", &vars).unwrap(),
            "echo world"
        );
    }

    #[test]
    fn test_substitute_variables_missing() {
        let vars = HashMap::new();
        assert!(substitute_variables("Hello {name}", &vars).is_err());
    }

    #[test]
    fn test_is_valid_variable_name() {
        assert!(is_valid_variable_name("valid_name"));
        assert!(is_valid_variable_name("name123"));
        assert!(is_valid_variable_name("a"));
        assert!(!is_valid_variable_name(""));
        assert!(!is_valid_variable_name("123invalid"));
        assert!(!is_valid_variable_name("invalid-name"));
        assert!(!is_valid_variable_name("invalid name"));
    }

    #[test]
    fn test_template_engine_new() {
        // Test with HOME set
        unsafe { env::set_var("HOME", "/tmp") };
        let result = TemplateEngine::new();
        assert!(result.is_ok());

        let engine = result.unwrap();
        assert!(engine.templates_dir().ends_with(".sdbh/templates"));
    }

    #[test]
    fn test_template_engine_new_no_home() {
        // Test without HOME or USERPROFILE
        unsafe {
            env::remove_var("HOME");
            env::remove_var("USERPROFILE");
        }

        let result = TemplateEngine::new();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Could not determine home directory")
        );
    }

    #[test]
    fn test_validate_template_valid() {
        let (engine, _temp) = create_test_engine();
        let template = create_sample_template();

        let result = engine.validate_template(&template);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_template_empty_id() {
        let (engine, _temp) = create_test_engine();
        let mut template = create_sample_template();
        template.id = "".to_string();

        let result = engine.validate_template(&template);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Template ID cannot be empty")
        );
    }

    #[test]
    fn test_validate_template_empty_name() {
        let (engine, _temp) = create_test_engine();
        let mut template = create_sample_template();
        template.name = "".to_string();

        let result = engine.validate_template(&template);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Template name cannot be empty")
        );
    }

    #[test]
    fn test_validate_template_empty_command() {
        let (engine, _temp) = create_test_engine();
        let mut template = create_sample_template();
        template.command = "".to_string();

        let result = engine.validate_template(&template);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Template command cannot be empty")
        );
    }

    #[test]
    fn test_validate_template_invalid_variable_name() {
        let (engine, _temp) = create_test_engine();
        let mut template = create_sample_template();
        template.variables[0].name = "invalid-name".to_string();

        let result = engine.validate_template(&template);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid variable name")
        );
    }

    #[test]
    fn test_validate_template_undefined_variable() {
        let (engine, _temp) = create_test_engine();
        let mut template = create_sample_template();
        template.command = "echo {undefined_var}".to_string();

        let result = engine.validate_template(&template);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("used in command but not defined")
        );
    }

    #[test]
    fn test_save_and_load_template() {
        let (engine, _temp) = create_test_engine();
        let template = create_sample_template();

        // Save template
        let save_result = engine.save_template(&template);
        assert!(save_result.is_ok());

        // Load template
        let load_result = engine.load_template("test-template");
        assert!(load_result.is_ok());

        let loaded = load_result.unwrap();
        assert_eq!(loaded.id, template.id);
        assert_eq!(loaded.name, template.name);
        assert_eq!(loaded.command, template.command);
        assert_eq!(loaded.variables.len(), template.variables.len());
    }

    #[test]
    fn test_load_template_not_found() {
        let (engine, _temp) = create_test_engine();

        let result = engine.load_template("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_list_templates() {
        let (engine, _temp) = create_test_engine();
        let template1 = create_sample_template();

        let mut template2 = create_sample_template();
        template2.id = "template2".to_string();
        template2.name = "Template 2".to_string();

        // Save templates
        engine.save_template(&template1).unwrap();
        engine.save_template(&template2).unwrap();

        // List templates
        let result = engine.list_templates();
        assert!(result.is_ok());

        let templates = result.unwrap();
        assert_eq!(templates.len(), 2);

        // Check that both templates are present
        let ids: Vec<String> = templates.iter().map(|t| t.id.clone()).collect();
        assert!(ids.contains(&"test-template".to_string()));
        assert!(ids.contains(&"template2".to_string()));
    }

    #[test]
    fn test_delete_template() {
        let (engine, _temp) = create_test_engine();
        let template = create_sample_template();

        // Save template
        engine.save_template(&template).unwrap();

        // Verify it exists
        assert!(engine.load_template("test-template").is_ok());

        // Delete template
        let delete_result = engine.delete_template("test-template");
        assert!(delete_result.is_ok());

        // Verify it's gone
        assert!(engine.load_template("test-template").is_err());
    }

    #[test]
    fn test_delete_template_not_found() {
        let (engine, _temp) = create_test_engine();

        let result = engine.delete_template("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_template_with_defaults() {
        let (engine, _temp) = create_test_engine();
        let template = create_sample_template();

        let mut provided_vars = HashMap::new();
        provided_vars.insert("user".to_string(), "alice".to_string());
        // message should use default "hello"

        let result = engine.resolve_template(&template, &provided_vars);
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert_eq!(resolved.resolved_command, "echo hello from alice");
        assert_eq!(resolved.variables_used.get("message").unwrap(), "hello");
        assert_eq!(resolved.variables_used.get("user").unwrap(), "alice");
    }

    #[test]
    fn test_resolve_template_missing_required() {
        let (engine, _temp) = create_test_engine();
        let template = create_sample_template();

        let provided_vars = HashMap::new(); // Missing required "user"

        let result = engine.resolve_template(&template, &provided_vars);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Required variable 'user' not provided")
        );
    }

    #[test]
    fn test_extract_variables_complex() {
        // Test various edge cases
        assert_eq!(
            extract_variables("{var} {var} {other}").unwrap(),
            vec!["var", "other"]
        );

        assert_eq!(
            extract_variables("cmd {var1} --flag={var2}").unwrap(),
            vec!["var1", "var2"]
        );

        assert_eq!(
            extract_variables("no braces here").unwrap(),
            Vec::<String>::new()
        );

        assert_eq!(extract_variables("{single}").unwrap(), vec!["single"]);
    }

    #[test]
    fn test_substitute_variables_edge_cases() {
        let mut vars = HashMap::new();
        vars.insert("empty".to_string(), "".to_string());
        vars.insert("spaces".to_string(), "hello world".to_string());
        vars.insert("special".to_string(), "chars/with-dashes".to_string());

        assert_eq!(
            substitute_variables("{empty} test", &vars).unwrap(),
            " test"
        );

        assert_eq!(
            substitute_variables("'{spaces}'", &vars).unwrap(),
            "'hello world'"
        );

        assert_eq!(
            substitute_variables("cmd {special}", &vars).unwrap(),
            "cmd chars/with-dashes"
        );
    }
}