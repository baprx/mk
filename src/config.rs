use crate::techno::Technology;
use anyhow::Result;
use etcetera::BaseStrategy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub technology_priority: Vec<String>,
    #[serde(default)]
    pub bump: BumpConfig,
    #[serde(default)]
    pub context: crate::context::ContextConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BumpConfig {
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default)]
    pub oci_registries: HashMap<String, OciRegistryAuth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciRegistryAuth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

fn default_max_depth() -> usize {
    5
}

impl Default for BumpConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            oci_registries: HashMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from ~/.config/mk/config.toml
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            // No config file, return default (empty priority list)
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;

        Ok(config)
    }

    /// Get the path to the config file
    fn get_config_path() -> Result<PathBuf> {
        let strategy = etcetera::base_strategy::choose_base_strategy()?;
        let config_dir = strategy.config_dir().join("mk");

        Ok(config_dir.join("config.toml"))
    }

    /// Get the priority order for technologies
    /// Returns None if no priority is configured (should use interactive selection)
    pub fn get_technology_priority(&self) -> Option<Vec<Technology>> {
        if self.technology_priority.is_empty() {
            return None;
        }

        let mut priorities = Vec::new();
        for tech_str in &self.technology_priority {
            match tech_str.to_lowercase().as_str() {
                "terraform" => priorities.push(Technology::Terraform),
                "ansible" => priorities.push(Technology::Ansible),
                "helm" => priorities.push(Technology::Helm),
                "kustomize" => priorities.push(Technology::Kustomize),
                _ => {
                    // Ignore unknown technology names
                    eprintln!("Warning: Unknown technology '{}' in config", tech_str);
                }
            }
        }

        if priorities.is_empty() {
            None
        } else {
            Some(priorities)
        }
    }

    /// Initialize a config file with example content
    ///
    /// # Arguments
    /// * `path` - Optional custom path for the config file. If None, uses ~/.config/mk/config.toml
    /// * `force` - If true, overwrites existing config file
    pub fn init_config(path: Option<PathBuf>, force: bool) -> Result<PathBuf> {
        let config_path = if let Some(custom_path) = path {
            custom_path
        } else {
            Self::get_config_path()?
        };

        // Check if file exists and force is not set
        if config_path.exists() && !force {
            anyhow::bail!(
                "Config file already exists at {}. Use --force to overwrite.",
                config_path.display()
            );
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let example_content = r#"# mk configuration file
# This file allows you to customize mk's behavior

# Technology priority order when multiple technologies are detected in a project
# If not set or empty, mk will prompt interactively when multiple technologies are found
#
# Supported values: terraform, ansible, helm, kustomize
#
# Example: prioritize Terraform over other technologies
# technology_priority = ["terraform", "kustomize", "helm", "ansible"]
#
# Uncomment and customize the line below:
# technology_priority = []

# Bump command configuration
[bump]
# Maximum directory depth for recursive scanning (default: 5)
max_depth = 5

# OCI registry authentication for Helm charts
# Configure authentication tokens or commands for OCI registries
#
# You can specify either a static token or a command that returns a token
#
# Examples:
# [bump.oci_registries."ghcr.io"]
# token = "ghp_your_github_token_here"
#
# [bump.oci_registries."123456789.dkr.ecr.us-east-1.amazonaws.com"]
# command = "aws ecr get-login-password --region us-east-1"
#
# [bump.oci_registries."registry.gitlab.com"]
# token = "glpat-your_gitlab_token"

# Kubernetes context validation (Helm/Kustomize only)
[context]
# Disable context validation checks (default: false)
# When enabled (false), mk will verify that you're using the correct kubectl context
# before applying/diffing Helm or Kustomize changes
disable_context_check = false

# Context mappings: repository -> environment -> kubectl context
# These mappings are automatically created when you run commands
# You can also define them manually here
#
# Example:
# [context.mappings."github.com/user/infra"]
# prod = "gke_project_cluster-prod"
# staging = "gke_project_cluster-staging"
#
# Note: You can also create a .mk/contexts.toml file in your git repository
# to share context mappings with your team
"#;

        fs::write(&config_path, example_content)?;

        Ok(config_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.technology_priority.is_empty());
        assert!(config.get_technology_priority().is_none());
    }

    #[test]
    fn test_config_parse_priorities() {
        let config = Config {
            technology_priority: vec!["terraform".to_string(), "ansible".to_string()],
            bump: BumpConfig::default(),
            context: crate::context::ContextConfig::default(),
        };

        let priorities = config.get_technology_priority().unwrap();
        assert_eq!(priorities.len(), 2);
        assert_eq!(priorities[0], Technology::Terraform);
        assert_eq!(priorities[1], Technology::Ansible);
    }

    #[test]
    fn test_config_case_insensitive() {
        let config = Config {
            technology_priority: vec!["Terraform".to_string(), "ANSIBLE".to_string()],
            bump: BumpConfig::default(),
            context: crate::context::ContextConfig::default(),
        };

        let priorities = config.get_technology_priority().unwrap();
        assert_eq!(priorities.len(), 2);
        assert_eq!(priorities[0], Technology::Terraform);
        assert_eq!(priorities[1], Technology::Ansible);
    }

    #[test]
    fn test_config_ignores_unknown() {
        let config = Config {
            technology_priority: vec![
                "terraform".to_string(),
                "unknown".to_string(),
                "ansible".to_string(),
            ],
            bump: BumpConfig::default(),
            context: crate::context::ContextConfig::default(),
        };

        let priorities = config.get_technology_priority().unwrap();
        assert_eq!(priorities.len(), 2);
        assert_eq!(priorities[0], Technology::Terraform);
        assert_eq!(priorities[1], Technology::Ansible);
    }

    #[test]
    fn test_init_config_creates_file() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Initialize config
        let result = Config::init_config(Some(config_path.clone()), false);
        assert!(result.is_ok());

        // Verify file exists
        assert!(config_path.exists());

        // Verify file content
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("# mk configuration file"));
        assert!(content.contains("technology_priority"));
        assert!(content.contains("terraform"));
        assert!(content.contains("ansible"));
        assert!(content.contains("helm"));
        assert!(content.contains("kustomize"));
    }

    #[test]
    fn test_init_config_fails_without_force() {
        use tempfile::TempDir;

        // Create a temporary directory with an existing config
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("existing_config.toml");

        // Create the file first
        Config::init_config(Some(config_path.clone()), false).unwrap();

        // Try to create it again without force - should fail
        let result = Config::init_config(Some(config_path.clone()), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_init_config_overwrites_with_force() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory with an existing config
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("overwrite_config.toml");

        // Create initial file with custom content
        fs::write(&config_path, "old content").unwrap();

        // Overwrite with force
        let result = Config::init_config(Some(config_path.clone()), true);
        assert!(result.is_ok());

        // Verify new content
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("# mk configuration file"));
        assert!(!content.contains("old content"));
    }

    #[test]
    fn test_init_config_creates_parent_directories() {
        use tempfile::TempDir;

        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("dirs")
            .join("config.toml");

        // Initialize config with non-existent parent directories
        let result = Config::init_config(Some(nested_path.clone()), false);
        assert!(result.is_ok());

        // Verify file exists
        assert!(nested_path.exists());
    }
}
