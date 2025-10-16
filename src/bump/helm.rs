use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::Path;
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

use super::registry;
use super::{Dependency, DependencyType};

/// Scan Helm Chart.yaml for chart dependencies
pub fn scan_helm_charts(
    project_path: &str,
    verbose: bool,
    include_prereleases: bool,
) -> Result<Vec<Dependency>> {
    // Load config to get OCI registry authentication
    let config = crate::config::Config::load().unwrap_or_default();

    let mut dependencies = Vec::new();

    let chart_yaml_path = Path::new(project_path).join("Chart.yaml");
    if !chart_yaml_path.exists() {
        return Ok(dependencies);
    }

    if verbose {
        eprintln!("  Scanning: Chart.yaml");
    }

    let content = fs::read_to_string(&chart_yaml_path).context("Failed to read Chart.yaml")?;

    if verbose {
        eprintln!("  Chart.yaml content length: {} bytes", content.len());
    }

    let docs = YamlLoader::load_from_str(&content).context("Failed to parse Chart.yaml")?;

    if verbose {
        eprintln!("  Loaded {} YAML documents", docs.len());
    }
    let doc = docs
        .first()
        .ok_or_else(|| anyhow::anyhow!("Empty Chart.yaml"))?;

    if verbose {
        eprintln!("  Parsed YAML document successfully");
    }

    // Check for dependencies - use proper hash access
    let deps_yaml = if let Some(hash) = doc.as_hash() {
        hash.get(&Yaml::String("dependencies".to_string()))
    } else {
        None
    };

    if verbose {
        eprintln!("  Dependencies field found: {}", deps_yaml.is_some());
    }

    if let Some(deps) = deps_yaml.and_then(|d| d.as_vec()) {
        if verbose {
            eprintln!("  Found {} dependencies", deps.len());
        }
        for dep in deps.iter() {
            let name = dep["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Dependency missing name"))?;
            let version = dep["version"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Dependency missing version"))?;
            let repository = dep["repository"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Dependency missing repository"))?;

            if verbose {
                eprintln!(
                    "  Found chart: {} from {}, current: {}",
                    name, repository, version
                );
            }

            // Skip local file:// dependencies - they don't need remote fetching
            if repository.starts_with("file://") {
                if verbose {
                    eprintln!("  Skipping local file dependency: {}", repository);
                }
                // Add as dependency with current version (no update available)
                let line_number = content
                    .lines()
                    .enumerate()
                    .find(|(_, line)| line.contains(&format!("name: {}", name)))
                    .map(|(i, _)| i + 1)
                    .unwrap_or(1);

                dependencies.push(Dependency {
                    name: name.to_string(),
                    current_version: version.to_string(),
                    latest_version: version.to_string(), // Same as current for local
                    latest_app_version: None,            // No appVersion for local files
                    file_path: chart_yaml_path.to_string_lossy().to_string(),
                    line_number,
                    dep_type: DependencyType::HelmChart {
                        repository: repository.to_string(),
                    },
                });
                continue;
            }

            // Fetch latest version - handle both OCI and HTTP registries
            let fetch_result = if repository.starts_with("oci://") {
                if verbose {
                    eprintln!("  Fetching from OCI registry: {}", repository);
                }
                // OCI registries only return version, no appVersion available
                registry::fetch_helm_chart_version_oci(
                    repository,
                    name,
                    &config,
                    verbose,
                    include_prereleases,
                )
                .map(|version| (version, None))
            } else {
                // HTTP registries return (version, appVersion)
                registry::fetch_helm_chart_version(repository, name, verbose, include_prereleases)
            };

            // Find line number (approximate)
            let line_number = content
                .lines()
                .enumerate()
                .find(|(_, line)| line.contains(&format!("name: {}", name)))
                .map(|(i, _)| i + 1)
                .unwrap_or(1);

            match fetch_result {
                Ok((latest_version, latest_app_version)) => {
                    dependencies.push(Dependency {
                        name: name.to_string(),
                        current_version: version.to_string(),
                        latest_version,
                        latest_app_version,
                        file_path: chart_yaml_path.to_string_lossy().to_string(),
                        line_number,
                        dep_type: DependencyType::HelmChart {
                            repository: repository.to_string(),
                        },
                    });
                }
                Err(e) => {
                    // Only log errors in verbose mode to avoid cluttering output
                    if verbose {
                        eprintln!(
                            "  {} Failed to fetch version for {}: {}",
                            "✗".bright_red(),
                            name.bright_cyan(),
                            e.to_string().yellow()
                        );
                    }

                    // Add dependency with ERROR marker so it can be filtered out later
                    dependencies.push(Dependency {
                        name: name.to_string(),
                        current_version: version.to_string(),
                        latest_version: format!("ERROR: {}", e),
                        latest_app_version: None,
                        file_path: chart_yaml_path.to_string_lossy().to_string(),
                        line_number,
                        dep_type: DependencyType::HelmChart {
                            repository: repository.to_string(),
                        },
                    });
                }
            }
        }
    }

    Ok(dependencies)
}

/// Update a Helm chart version in Chart.yaml
/// Also updates the Chart.yaml's version and appVersion fields if they match the old dependency version
pub fn update_helm_chart(
    project_path: &str,
    chart_name: &str,
    old_version: &str,
    new_version: &str,
    new_app_version: Option<&str>,
) -> Result<()> {
    let chart_yaml_path = Path::new(project_path).join("Chart.yaml");
    let content = fs::read_to_string(&chart_yaml_path).context("Failed to read Chart.yaml")?;

    let mut docs = YamlLoader::load_from_str(&content).context("Failed to parse Chart.yaml")?;
    let doc = docs
        .first_mut()
        .ok_or_else(|| anyhow::anyhow!("Empty Chart.yaml"))?;

    if let Some(hash) = doc.as_mut_hash() {
        // Update the version in dependencies
        if let Some(Yaml::Array(ref mut deps)) =
            hash.get_mut(&Yaml::String("dependencies".to_string()))
        {
            for dep in deps.iter_mut() {
                if let Some(dep_hash) = dep.as_mut_hash() {
                    if let Some(Yaml::String(name)) =
                        dep_hash.get(&Yaml::String("name".to_string()))
                    {
                        if name == chart_name {
                            dep_hash.insert(
                                Yaml::String("version".to_string()),
                                Yaml::String(new_version.to_string()),
                            );
                        }
                    }
                }
            }
        }

        // Check if Chart.yaml's version field matches the old dependency version
        // If so, update it to the new version
        if let Some(Yaml::String(chart_version)) = hash.get(&Yaml::String("version".to_string())) {
            let chart_version_clean = chart_version.trim_start_matches('v');
            let old_version_clean = old_version.trim_start_matches('v');

            if chart_version == old_version
                || chart_version_clean == old_version
                || chart_version == old_version_clean
                || chart_version_clean == old_version_clean
            {
                hash.insert(
                    Yaml::String("version".to_string()),
                    Yaml::String(new_version.to_string()),
                );
            }
        }

        // Check if Chart.yaml's appVersion field matches the old dependency version
        // If so, and we have a new appVersion from the registry, update it
        if let Some(new_app_ver) = new_app_version {
            if let Some(Yaml::String(chart_app_version)) =
                hash.get(&Yaml::String("appVersion".to_string()))
            {
                let chart_app_version_clean = chart_app_version.trim_start_matches('v');
                let old_version_clean = old_version.trim_start_matches('v');

                if chart_app_version == old_version
                    || chart_app_version_clean == old_version
                    || chart_app_version == old_version_clean
                    || chart_app_version_clean == old_version_clean
                {
                    hash.insert(
                        Yaml::String("appVersion".to_string()),
                        Yaml::String(new_app_ver.to_string()),
                    );
                }
            }
        }
    }

    // Serialize back to YAML
    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(doc).context("Failed to serialize YAML")?;
    }

    fs::write(&chart_yaml_path, out_str + "\n").context("Failed to write Chart.yaml")?;

    Ok(())
}
