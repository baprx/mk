use anyhow::{Context, Result};
use colored::*;
use etcetera::BaseStrategy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use toml_edit::{DocumentMut, Item, Table};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ContextConfig {
    #[serde(default)]
    pub disable_context_check: bool,
    #[serde(default)]
    pub mappings: HashMap<String, HashMap<String, String>>,
}

impl ContextConfig {
    /// Get the expected context for a given repo and environment
    pub fn get_mapping(&self, repo_id: &str, environment: &str) -> Option<String> {
        self.mappings
            .get(repo_id)
            .and_then(|envs| envs.get(environment))
            .cloned()
    }

    /// Set a context mapping for a repo and environment
    pub fn set_mapping(&mut self, repo_id: &str, environment: &str, context: &str) {
        self.mappings
            .entry(repo_id.to_string())
            .or_default()
            .insert(environment.to_string(), context.to_string());
    }
}

/// Main entry point for context validation
pub fn validate_context(project_path: &str, environment: &str, verbose: bool) -> Result<()> {
    // Check if feature is disabled in user config
    if is_context_check_disabled()? {
        if verbose {
            eprintln!("{} Context validation disabled in config", "INFO:".cyan());
        }
        return Ok(());
    }

    // Get git repo identifier
    let repo_id = match get_git_repo_identifier(project_path) {
        Ok(id) => id,
        Err(_) => {
            if verbose {
                eprintln!(
                    "{} Not a git repository, skipping context validation",
                    "INFO:".cyan()
                );
            }
            return Ok(());
        }
    };

    // Get current kubectl context
    let current_context = get_current_kube_context()
        .context("Failed to get current kubectl context. Is kubectl installed and configured?")?;

    // Find and load context config (repo config takes precedence)
    let (config_path, mut context_config) = load_context_mappings(project_path)?;

    // Check if mapping exists
    match context_config.get_mapping(&repo_id, environment) {
        Some(expected_context) => {
            // Mapping exists - validate
            if current_context != expected_context {
                anyhow::bail!(
                    "Kubernetes context mismatch!\n\
                     Repository: {}\n\
                     Environment: {}\n\
                     Expected context: {}\n\
                     Current context: {}\n\n\
                     Please switch to the correct context with:\n\
                     kubectl config use-context {}\n\n\
                     Or update the mapping if the context has changed.",
                    repo_id,
                    environment,
                    expected_context.cyan(),
                    current_context.red(),
                    expected_context.cyan()
                );
            }
            if verbose {
                eprintln!(
                    "{} Context validated: {}",
                    "✓".green(),
                    current_context.cyan()
                );
            }
        }
        None => {
            // No mapping - prompt to save
            prompt_save_context(
                &repo_id,
                environment,
                &current_context,
                &config_path,
                &mut context_config,
            )?;
        }
    }

    Ok(())
}

/// Check if context validation is disabled in user config
fn is_context_check_disabled() -> Result<bool> {
    let user_config = crate::config::Config::load()?;
    Ok(user_config.context.disable_context_check)
}

/// Get git repository identifier (normalized remote URL)
fn get_git_repo_identifier(project_path: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(project_path)
        .output()
        .context("Failed to get git remote URL")?;

    if !output.status.success() {
        anyhow::bail!("Not a git repository or no 'origin' remote configured");
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    normalize_git_url(&url)
}

/// Normalize git URL to a consistent format
/// Examples:
///   git@github.com:user/repo.git -> github.com/user/repo
///   https://github.com/user/repo.git -> github.com/user/repo
///   ssh://git@gitlab.com/user/repo.git -> gitlab.com/user/repo
fn normalize_git_url(url: &str) -> Result<String> {
    let url = url.trim();

    // Handle SSH format: git@github.com:user/repo.git
    if let Some(ssh_part) = url.strip_prefix("git@") {
        let normalized = ssh_part.replace(':', "/");
        let normalized = normalized.strip_suffix(".git").unwrap_or(&normalized);
        return Ok(normalized.to_string());
    }

    // Handle HTTPS format: https://github.com/user/repo.git
    if let Some(https_part) = url.strip_prefix("https://") {
        let normalized = https_part.strip_suffix(".git").unwrap_or(https_part);
        return Ok(normalized.to_string());
    }

    // Handle SSH URL format: ssh://git@gitlab.com/user/repo.git
    if let Some(ssh_url) = url.strip_prefix("ssh://git@") {
        let normalized = ssh_url.strip_suffix(".git").unwrap_or(ssh_url);
        return Ok(normalized.to_string());
    }

    // Handle http format: http://github.com/user/repo.git
    if let Some(http_part) = url.strip_prefix("http://") {
        let normalized = http_part.strip_suffix(".git").unwrap_or(http_part);
        return Ok(normalized.to_string());
    }

    // If no recognized format, return as-is
    Ok(url.to_string())
}

/// Get current kubectl context
fn get_current_kube_context() -> Result<String> {
    let output = Command::new("kubectl")
        .args(["config", "current-context"])
        .output()
        .context("Failed to execute kubectl command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("kubectl command failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Find context config file (repo config takes precedence over user config)
/// Returns the path to use for reading/writing and the loaded config
fn load_context_mappings(project_path: &str) -> Result<(PathBuf, ContextConfig)> {
    // Check for repo config first
    let repo_config_path = Path::new(project_path).join(".mk").join("contexts.toml");
    if repo_config_path.exists() {
        let content =
            fs::read_to_string(&repo_config_path).context("Failed to read repo context config")?;
        let config: ContextConfig =
            toml::from_str(&content).context("Failed to parse repo context config")?;
        return Ok((repo_config_path, config));
    }

    // Fall back to user config
    let user_config = crate::config::Config::load()?;
    let user_config_path = get_user_config_path()?;

    Ok((user_config_path, user_config.context.clone()))
}

/// Get the path to user config file
fn get_user_config_path() -> Result<PathBuf> {
    let strategy = etcetera::base_strategy::choose_base_strategy()?;
    let config_dir = strategy.config_dir().join("mk");
    Ok(config_dir.join("config.toml"))
}

/// Prompt user to save context mapping
fn prompt_save_context(
    repo_id: &str,
    environment: &str,
    current_context: &str,
    config_path: &Path,
    context_config: &mut ContextConfig,
) -> Result<()> {
    eprintln!(
        "\n{} No Kubernetes context configured for:",
        "WARNING:".yellow()
    );
    eprintln!("  Repository: {}", repo_id.cyan());
    eprintln!("  Environment: {}", environment.cyan());
    eprintln!("\n  Current kubectl context: {}", current_context.cyan());
    eprint!("\nContinue and save this context for future use? [Y/n]: ");
    io::stderr().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    let response = response.trim().to_lowercase();
    if response.is_empty() || response == "y" || response == "yes" {
        // Update context config
        context_config.set_mapping(repo_id, environment, current_context);

        // Save to appropriate config file
        save_context_config(config_path, context_config)?;

        eprintln!(
            "{} Context mapping saved to {}",
            "SUCCESS:".green(),
            config_path.display()
        );
        Ok(())
    } else {
        anyhow::bail!(
            "Cannot proceed without a configured Kubernetes context for this repository and environment.\n\
             Either:\n\
             - Switch to the correct kubectl context and run this command again\n\
             - Or answer 'Y' to save the current context mapping\n\
             - Or disable context validation in the user config (not recommended)"
        );
    }
}

/// Save context config to file
fn save_context_config(config_path: &Path, context_config: &ContextConfig) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // If this is the user config file, we need to merge with existing config
    if config_path.file_name() == Some(std::ffi::OsStr::new("config.toml")) {
        // Read existing config file as a TOML document to preserve formatting
        let content = fs::read_to_string(config_path).unwrap_or_else(|_| String::new());
        let mut doc = content
            .parse::<DocumentMut>()
            .context("Failed to parse existing config file")?;

        // Ensure [context] table exists
        if !doc.contains_key("context") {
            doc["context"] = Item::Table(Table::new());
        }

        // Update disable_context_check if needed
        if let Some(context_table) = doc["context"].as_table_mut() {
            context_table["disable_context_check"] =
                toml_edit::value(context_config.disable_context_check);

            // Update mappings - ensure [context.mappings] exists
            if !context_table.contains_key("mappings") {
                context_table["mappings"] = Item::Table(Table::new());
            }

            if let Some(mappings_table) = context_table["mappings"].as_table_mut() {
                // Update each repository's mappings
                for (repo_id, environments) in &context_config.mappings {
                    // Ensure repo table exists
                    if !mappings_table.contains_key(repo_id) {
                        mappings_table[repo_id] = Item::Table(Table::new());
                    }

                    if let Some(repo_table) = mappings_table[repo_id].as_table_mut() {
                        // Update each environment mapping
                        for (env, context) in environments {
                            repo_table[env] = toml_edit::value(context.as_str());
                        }
                    }
                }
            }
        }

        // Write back the document with preserved formatting
        fs::write(config_path, doc.to_string()).context("Failed to write config file")?;
    } else {
        // For repo config, write standalone context config with toml_edit
        let mut doc = DocumentMut::new();

        // Add disable_context_check
        doc["disable_context_check"] = toml_edit::value(context_config.disable_context_check);

        // Add mappings table
        let mut mappings_table = Table::new();
        for (repo_id, environments) in &context_config.mappings {
            let mut repo_table = Table::new();
            for (env, context) in environments {
                repo_table.insert(env, toml_edit::value(context.as_str()));
            }
            mappings_table.insert(repo_id, Item::Table(repo_table));
        }
        doc["mappings"] = Item::Table(mappings_table);

        fs::write(config_path, doc.to_string()).context("Failed to write context config file")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_git_url_ssh() {
        let url = "git@github.com:user/repo.git";
        let result = normalize_git_url(url).unwrap();
        assert_eq!(result, "github.com/user/repo");
    }

    #[test]
    fn test_normalize_git_url_https() {
        let url = "https://github.com/user/repo.git";
        let result = normalize_git_url(url).unwrap();
        assert_eq!(result, "github.com/user/repo");
    }

    #[test]
    fn test_normalize_git_url_ssh_protocol() {
        let url = "ssh://git@gitlab.com/user/repo.git";
        let result = normalize_git_url(url).unwrap();
        assert_eq!(result, "gitlab.com/user/repo");
    }

    #[test]
    fn test_normalize_git_url_without_git_suffix() {
        let url = "git@github.com:user/repo";
        let result = normalize_git_url(url).unwrap();
        assert_eq!(result, "github.com/user/repo");
    }

    #[test]
    fn test_normalize_git_url_http() {
        let url = "http://github.com/user/repo.git";
        let result = normalize_git_url(url).unwrap();
        assert_eq!(result, "github.com/user/repo");
    }

    #[test]
    fn test_context_config_get_mapping() {
        let mut config = ContextConfig::default();
        config.set_mapping("github.com/user/repo", "prod", "gke-prod");

        assert_eq!(
            config.get_mapping("github.com/user/repo", "prod"),
            Some("gke-prod".to_string())
        );
        assert_eq!(config.get_mapping("github.com/user/repo", "staging"), None);
        assert_eq!(config.get_mapping("other/repo", "prod"), None);
    }

    #[test]
    fn test_context_config_set_mapping() {
        let mut config = ContextConfig::default();
        config.set_mapping("repo1", "env1", "context1");
        config.set_mapping("repo1", "env2", "context2");
        config.set_mapping("repo2", "env1", "context3");

        assert_eq!(config.mappings.len(), 2);
        assert_eq!(config.mappings.get("repo1").unwrap().len(), 2);
        assert_eq!(config.mappings.get("repo2").unwrap().len(), 1);
    }
}
