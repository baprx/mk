use anyhow::Result;
use colored::*;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::Action;
use crate::executor::execute_command;
use crate::executor::execute_command_output;

/// Check if helm dependencies need updating and update if needed
pub fn helm_deps_update(
    project_path: &str,
    environment: &str,
    verbose: bool,
    force: bool,
    silent: bool,
) -> Result<()> {
    let chart_yaml = Path::new(project_path).join("Chart.yaml");
    if !chart_yaml.exists() {
        return Ok(());
    }

    let chart_lock = Path::new(project_path).join("Chart.lock");
    let charts_dir = Path::new(project_path).join("charts");

    // Determine if dependencies need updating
    let needs_update = force
        || !chart_lock.exists()
        || !charts_dir.exists()
        || chart_dependencies_outdated(project_path, verbose)?;

    if needs_update {
        if !silent {
            eprintln!("{} Helm dependencies need updating", "INFO:".cyan());
        }

        // Authenticate to helm registries if needed
        if let Ok(registries) = extract_helm_registries(project_path) {
            for registry in registries {
                if verbose {
                    eprintln!(
                        "{} Authenticating to Helm registry {}",
                        "INFO:".cyan(),
                        registry
                    );
                }
                let auth_cmd = format!(
                    "gcloud auth print-access-token | helm registry login -u oauth2accesstoken --password-stdin https://{}",
                    registry
                );
                let _ = execute_command_output(&auth_cmd, project_path, false);
            }
        }

        let deps_cmd = format!("helmfile deps -e {}", environment);
        if verbose {
            // Stream output when verbose
            execute_command(&deps_cmd, project_path, verbose)?;
        } else {
            // Suppress output when not verbose, but still propagate errors
            execute_command_output(&deps_cmd, project_path, false)?;
        }
    }

    Ok(())
}

/// Check if Chart.yaml dependencies are outdated compared to Chart.lock
fn chart_dependencies_outdated(project_path: &str, verbose: bool) -> Result<bool> {
    use yaml_rust2::YamlLoader;

    let chart_yaml_path = Path::new(project_path).join("Chart.yaml");
    let chart_lock_path = Path::new(project_path).join("Chart.lock");
    let charts_dir = Path::new(project_path).join("charts");

    // Parse Chart.yaml dependencies
    let chart_content = fs::read_to_string(&chart_yaml_path)?;
    let chart_docs = YamlLoader::load_from_str(&chart_content)?;

    let chart_deps = if let Some(doc) = chart_docs.first() {
        if let Some(dependencies) = doc["dependencies"].as_vec() {
            dependencies
                .iter()
                .filter_map(|dep| {
                    let name = dep["name"].as_str()?;
                    let version = dep["version"].as_str()?;
                    Some((name.to_string(), version.to_string()))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // If no dependencies defined, nothing to check
    if chart_deps.is_empty() {
        return Ok(false);
    }

    // If Chart.lock doesn't exist, return false since the outer check handles this
    if !chart_lock_path.exists() {
        return Ok(false);
    }

    // Parse Chart.lock dependencies
    let lock_content = fs::read_to_string(&chart_lock_path)?;
    let lock_docs = YamlLoader::load_from_str(&lock_content)?;

    let lock_deps = if let Some(doc) = lock_docs.first() {
        if let Some(dependencies) = doc["dependencies"].as_vec() {
            dependencies
                .iter()
                .filter_map(|dep| {
                    let name = dep["name"].as_str()?;
                    let version = dep["version"].as_str()?;
                    Some((name.to_string(), version.to_string()))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Compare versions
    for (chart_name, chart_version) in &chart_deps {
        // Check if this dependency exists in Chart.lock with the same version
        let lock_match = lock_deps.iter().any(|(lock_name, lock_version)| {
            lock_name == chart_name && lock_version == chart_version
        });

        if !lock_match {
            if verbose {
                eprintln!(
                    "{} Dependency mismatch: {} requires version {} but Chart.lock has different/missing version",
                    "INFO:".cyan(),
                    chart_name,
                    chart_version
                );
            }
            return Ok(true);
        }

        // Also verify the actual .tgz file exists in charts/
        let chart_file = charts_dir.join(format!("{}-{}.tgz", chart_name, chart_version));
        if !chart_file.exists() {
            if verbose {
                eprintln!(
                    "{} Chart file missing: {}",
                    "INFO:".cyan(),
                    chart_file.display()
                );
            }
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn get_command(
    action: &Action,
    project_path: &str,
    environment: &str,
    options: &[String],
    verbose: bool,
    silent: bool,
) -> Result<Option<String>> {
    // Auto-update helm dependencies if needed (except for Deps action which handles it explicitly)
    if !matches!(action, Action::Deps | Action::Duplicate { .. }) {
        helm_deps_update(project_path, environment, verbose, false, silent)?;
    }

    let options_str = options.join(" ");
    let opts = if options_str.is_empty() {
        String::new()
    } else {
        format!(" {}", options_str)
    };

    let cmd = match action {
        Action::Apply => {
            format!("helmfile sync -e {} --skip-deps{}", environment, opts)
        }
        Action::Check | Action::Diff => {
            format!("helmfile diff -e {} --skip-deps{}", environment, opts)
        }
        Action::Template => {
            format!("helmfile template -e {} --skip-deps{}", environment, opts)
        }
        Action::Delete | Action::Destroy | Action::Uninstall => {
            format!("helmfile destroy -e {} --skip-deps{}", environment, opts)
        }
        Action::Deps => {
            helm_deps_update(project_path, environment, verbose, true, false)?;
            return Ok(None);
        }
        Action::Duplicate { target_env } => {
            // Perform the duplication using native Rust
            duplicate_helm_env(project_path, environment, target_env)?;
            return Ok(None);
        }
        _ => {
            anyhow::bail!("Action {:?} not implemented for helm", action);
        }
    };

    Ok(Some(cmd))
}

/// Extract helm registries from Chart.yaml dependencies
fn extract_helm_registries(project_path: &str) -> Result<HashSet<String>> {
    use yaml_rust2::YamlLoader;

    let chart_yaml_path = Path::new(project_path).join("Chart.yaml");
    let content = fs::read_to_string(chart_yaml_path)?;

    let docs = YamlLoader::load_from_str(&content)?;
    let mut registries = HashSet::new();

    if let Some(doc) = docs.first() {
        if let Some(dependencies) = doc["dependencies"].as_vec() {
            for dep in dependencies {
                if let Some(repo) = dep["repository"].as_str() {
                    if repo.contains("docker.pkg.dev") {
                        // Extract registry from oci://registry/path format
                        if let Some(registry) = repo.strip_prefix("oci://") {
                            if let Some(slash_pos) = registry.find('/') {
                                registries.insert(registry[..slash_pos].to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(registries)
}

/// Duplicate helm environment configuration
fn duplicate_helm_env(project_path: &str, source_env: &str, target_env: &str) -> Result<()> {
    let values_dir = Path::new(project_path).join("values");
    let source_dir = values_dir.join(source_env);
    let target_dir = values_dir.join(target_env);

    // Create target directory
    fs::create_dir_all(&target_dir)?;

    // Copy values.yaml
    let source_file = source_dir.join("values.yaml");
    let target_file = target_dir.join("values.yaml");

    if source_file.exists() {
        let content = fs::read_to_string(&source_file)?;
        // Replace source environment name with target environment name
        let updated_content = content.replace(source_env, target_env);
        fs::write(&target_file, updated_content)?;
    }

    // Walk through the target directory and replace environment names in all values.yaml files
    for entry in ignore::WalkBuilder::new(&target_dir)
        .build()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_some_and(|ft| ft.is_file()) && entry.file_name() == "values.yaml" {
            let content = fs::read_to_string(entry.path())?;
            let updated_content = content.replace(source_env, target_env);
            fs::write(entry.path(), updated_content)?;
        }
    }

    Ok(())
}
