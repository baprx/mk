pub mod helm;
pub mod registry;
pub mod terraform;

use anyhow::{Context, Result};
use colored::*;
use dialoguer::MultiSelect;

use crate::techno::{self, Technology};

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub latest_app_version: Option<String>,
    pub file_path: String,
    pub line_number: usize,
    pub dep_type: DependencyType,
}

#[derive(Debug, Clone)]
pub enum DependencyType {
    TerraformModule { source: String, constraint: String },
    HelmChart { repository: String },
}

impl Dependency {
    pub fn display_name(&self) -> String {
        format!(
            "{} ({}:{}) {} → {}",
            self.name.bright_cyan(),
            self.file_path.purple(),
            self.line_number,
            self.current_version.yellow(),
            self.latest_version.green()
        )
    }
}

pub fn run_bump(
    project_path: &str,
    verbose: bool,
    include_prereleases: bool,
    recursive: bool,
    no_ignore: bool,
) -> Result<()> {
    if recursive {
        run_bump_recursive(project_path, verbose, include_prereleases, no_ignore)
    } else {
        run_bump_single(project_path, verbose, include_prereleases)
    }
}

fn run_bump_single(project_path: &str, verbose: bool, include_prereleases: bool) -> Result<()> {
    eprintln!(
        "{} Scanning for dependencies in: {}",
        "INFO:".cyan(),
        project_path
    );

    // Try direct detection first, fallback to hierarchical detection if needed
    let (techno, actual_path) = if let Some(tech) = techno::detect_technology_direct(project_path) {
        // Direct detection succeeded (e.g., Chart.yaml found or directory named "terraform")
        (tech, project_path.to_string())
    } else {
        // Fallback to hierarchical detection with silent mode to avoid info messages
        techno::detect_technology(project_path, None, true)
            .context("Failed to detect technology")?
    };

    let mut all_dependencies = Vec::new();

    // Scan for dependencies based on technology
    match techno {
        Technology::Terraform => {
            eprintln!("{} Detected Terraform project", "INFO:".cyan());
            let deps =
                terraform::scan_terraform_modules(&actual_path, verbose, include_prereleases)
                    .context("Failed to scan Terraform modules")?;
            all_dependencies.extend(deps);
        }
        Technology::Helm => {
            eprintln!("{} Detected Helm project", "INFO:".cyan());
            let deps = helm::scan_helm_charts(&actual_path, verbose, include_prereleases)
                .context("Failed to scan Helm charts")?;
            all_dependencies.extend(deps);
        }
        _ => {
            anyhow::bail!("Bump command is only supported for Terraform and Helm projects");
        }
    }

    // Separate successful dependencies from errors
    let (successful_deps, error_deps): (Vec<_>, Vec<_>) = all_dependencies
        .iter()
        .partition(|dep| !dep.latest_version.starts_with("ERROR:"));

    if all_dependencies.is_empty() {
        eprintln!("{} No dependencies found", "INFO:".cyan());
        return Ok(());
    }

    // Report errors if any
    if !error_deps.is_empty() {
        eprintln!(
            "{} {} dependencies found, {} with fetch errors",
            "WARNING:".yellow(),
            all_dependencies.len(),
            error_deps.len()
        );
    }

    // Filter dependencies with updates available (excluding errors)
    let updates_available: Vec<_> = successful_deps
        .iter()
        .filter(|dep| dep.current_version != dep.latest_version)
        .copied()
        .collect();

    // Check if all dependencies failed
    if !error_deps.is_empty() && successful_deps.is_empty() {
        eprintln!(
            "{} Could not verify any dependencies due to fetch errors",
            "WARNING:".yellow()
        );
        eprintln!(
            "  {} found but all failed to fetch",
            if all_dependencies.len() == 1 {
                "1 dependency".to_string()
            } else {
                format!("{} dependencies", all_dependencies.len())
            }
        );
        eprintln!(
            "  {}",
            "Tip: Configure OCI authentication in ~/.config/mk/config.toml or use --verbose for details"
                .dimmed()
        );
        return Ok(());
    }

    if updates_available.is_empty() {
        eprintln!("{} All dependencies are up to date!", "SUCCESS:".green());
        return Ok(());
    }

    eprintln!(
        "{} Found {} dependencies with updates available\n",
        "INFO:".cyan(),
        updates_available.len()
    );

    // Create multi-select prompt
    let items: Vec<String> = updates_available
        .iter()
        .map(|dep| dep.display_name())
        .collect();

    // Pre-select if only one dependency is available
    let defaults = if updates_available.len() == 1 {
        vec![true]
    } else {
        vec![false; updates_available.len()]
    };

    let selections = MultiSelect::new()
        .with_prompt("Select dependencies to update (Space to select, Enter to confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact()
        .context("Failed to get user selection")?;

    if selections.is_empty() {
        eprintln!("{} No dependencies selected", "INFO:".cyan());
        return Ok(());
    }

    // Apply updates
    eprintln!("\n{} Updating selected dependencies...", "INFO:".cyan());
    let selected_deps: Vec<_> = selections.iter().map(|&i| updates_available[i]).collect();

    for dep in &selected_deps {
        match &dep.dep_type {
            DependencyType::TerraformModule { source, constraint } => {
                terraform::update_terraform_module(
                    &dep.file_path,
                    source,
                    constraint,
                    &dep.latest_version,
                )
                .context(format!("Failed to update {}", dep.name))?;
                eprintln!(
                    "  {} Updated {} in {}",
                    "✓".green(),
                    dep.name.cyan(),
                    dep.file_path.purple()
                );
            }
            DependencyType::HelmChart { repository } => {
                if verbose {
                    eprintln!("  Updating {} from repository: {}", dep.name, repository);
                }
                helm::update_helm_chart(
                    &actual_path,
                    &dep.name,
                    &dep.current_version,
                    &dep.latest_version,
                    dep.latest_app_version.as_deref(),
                )
                .context(format!("Failed to update {}", dep.name))?;
                eprintln!("  {} Updated {} in Chart.yaml", "✓".green(), dep.name);
            }
        }
    }

    eprintln!(
        "\n{} {} dependencies updated",
        "SUCCESS:".green(),
        selected_deps.len()
    );

    Ok(())
}

fn run_bump_recursive(
    root_path: &str,
    verbose: bool,
    include_prereleases: bool,
    no_ignore: bool,
) -> Result<()> {
    use std::collections::HashMap;

    // Load config to get max_depth
    let config = crate::config::Config::load().unwrap_or_default();
    let max_depth = config.bump.max_depth;

    eprintln!(
        "{} Scanning recursively (max depth: {}): {}",
        "INFO:".cyan(),
        max_depth,
        root_path
    );

    // Use ignore crate's WalkBuilder which properly handles .gitignore
    use ignore::WalkBuilder;

    let mut projects = Vec::new();

    for result in WalkBuilder::new(root_path)
        .max_depth(Some(max_depth))
        .git_ignore(!no_ignore) // Respect .gitignore unless --no-ignore is set
        .git_exclude(!no_ignore) // Respect .git/info/exclude unless --no-ignore is set
        .git_global(!no_ignore) // Respect global gitignore unless --no-ignore is set
        .build()
        .filter_map(|e| e.ok())
    {
        let path = result.path();

        // Only process directories
        if !path.is_dir() {
            continue;
        }

        let path_str = path.to_str().unwrap_or("");

        // Direct file-based detection to avoid interactive prompts
        // Check for Helm project (Chart.yaml)
        if path.join("Chart.yaml").exists() {
            projects.push((Technology::Helm, path_str.to_string()));
            continue;
        }

        // Check for Terraform project (directory named "terraform" or containing .tf files)
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name == "terraform" {
                projects.push((Technology::Terraform, path_str.to_string()));
                continue;
            }
        }

        // Check for .tf files in the directory
        if let Ok(entries) = std::fs::read_dir(path) {
            let has_tf_files = entries.filter_map(|e| e.ok()).any(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "tf")
                    .unwrap_or(false)
            });
            if has_tf_files {
                projects.push((Technology::Terraform, path_str.to_string()));
            }
        }
    }

    if projects.is_empty() {
        eprintln!("{} No Terraform or Helm projects found", "INFO:".cyan());
        return Ok(());
    }

    let terraform_count = projects
        .iter()
        .filter(|(t, _)| matches!(t, Technology::Terraform))
        .count();
    let helm_count = projects
        .iter()
        .filter(|(t, _)| matches!(t, Technology::Helm))
        .count();

    eprintln!(
        "{} Found {} Terraform project(s), {} Helm project(s)",
        "INFO:".cyan(),
        terraform_count,
        helm_count
    );

    // Scan all projects and aggregate dependencies
    // Use a cache to avoid querying the same module/chart version twice
    let mut version_cache: HashMap<String, String> = HashMap::new();
    let mut all_dependencies = Vec::new();
    let total_projects = projects.len();

    for (techno, actual_path) in &projects {
        if verbose {
            eprintln!("  Scanning: {}", actual_path);
        }

        match *techno {
            Technology::Terraform => {
                match terraform::scan_terraform_modules(actual_path, verbose, include_prereleases) {
                    Ok(deps) => {
                        for dep in deps {
                            // Check cache for this source
                            let cache_key =
                                if let DependencyType::TerraformModule { ref source, .. } =
                                    dep.dep_type
                                {
                                    format!("tf:{}", source)
                                } else {
                                    continue;
                                };

                            let (latest_version, used_cache) =
                                if let Some(cached_version) = version_cache.get(&cache_key) {
                                    (cached_version.clone(), true)
                                } else {
                                    let version = dep.latest_version.clone();
                                    version_cache.insert(cache_key.clone(), version.clone());
                                    (version, false)
                                };

                            // Log dependency status
                            if latest_version.starts_with("ERROR:") {
                                // Show error inline
                                let error_msg = latest_version
                                    .strip_prefix("ERROR: ")
                                    .unwrap_or(&latest_version);
                                // Truncate long error messages
                                let short_error = if error_msg.len() > 60 {
                                    format!("{}...", &error_msg[..57])
                                } else {
                                    error_msg.to_string()
                                };
                                eprintln!(
                                    "  {} {} ({}:{}){} - {}",
                                    "⚠".yellow(),
                                    dep.name.cyan(),
                                    dep.file_path.purple(),
                                    dep.line_number,
                                    if used_cache {
                                        " [cached]".dimmed().to_string()
                                    } else {
                                        "".to_string()
                                    },
                                    short_error.dimmed()
                                );
                            } else if dep.current_version == latest_version {
                                eprintln!(
                                    "  {} {} {} ({}:{}){} - already up to date",
                                    "✓".green(),
                                    dep.name.cyan(),
                                    dep.current_version.yellow(),
                                    dep.file_path.purple(),
                                    dep.line_number,
                                    if used_cache {
                                        " [cached]".dimmed().to_string()
                                    } else {
                                        "".to_string()
                                    }
                                );
                            } else {
                                eprintln!(
                                    "  {} {} {} → {} ({}:{}){} - update available",
                                    "↑".yellow(),
                                    dep.name.cyan(),
                                    dep.current_version.yellow(),
                                    latest_version.green(),
                                    dep.file_path.purple(),
                                    dep.line_number,
                                    if used_cache {
                                        " [cached]".dimmed().to_string()
                                    } else {
                                        "".to_string()
                                    }
                                );
                            }

                            let mut final_dep = dep.clone();
                            final_dep.latest_version = latest_version;
                            all_dependencies.push(final_dep);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to scan Terraform modules in {}: {}",
                            "✗".red(),
                            actual_path,
                            e
                        );
                    }
                }
            }
            Technology::Helm => {
                match helm::scan_helm_charts(actual_path, verbose, include_prereleases) {
                    Ok(deps) => {
                        for dep in deps {
                            // Check cache for this chart
                            let cache_key = if let DependencyType::HelmChart { ref repository } =
                                dep.dep_type
                            {
                                format!("helm:{}:{}", repository, dep.name)
                            } else {
                                continue;
                            };

                            let (latest_version, used_cache) =
                                if let Some(cached_version) = version_cache.get(&cache_key) {
                                    (cached_version.clone(), true)
                                } else {
                                    let version = dep.latest_version.clone();
                                    version_cache.insert(cache_key.clone(), version.clone());
                                    (version, false)
                                };

                            // Log dependency status
                            if latest_version.starts_with("ERROR:") {
                                // Show error inline
                                let error_msg = latest_version
                                    .strip_prefix("ERROR: ")
                                    .unwrap_or(&latest_version);
                                // Truncate long error messages
                                let short_error = if error_msg.len() > 60 {
                                    format!("{}...", &error_msg[..57])
                                } else {
                                    error_msg.to_string()
                                };
                                eprintln!(
                                    "  {} {} ({}:{}){} - {}",
                                    "⚠".yellow(),
                                    dep.name.cyan(),
                                    dep.file_path.purple(),
                                    dep.line_number,
                                    if used_cache {
                                        " [cached]".dimmed().to_string()
                                    } else {
                                        "".to_string()
                                    },
                                    short_error.dimmed()
                                );
                            } else if dep.current_version == latest_version {
                                eprintln!(
                                    "  {} {} {} ({}:{}){} - already up to date",
                                    "✓".green(),
                                    dep.name.cyan(),
                                    dep.current_version.yellow(),
                                    dep.file_path.purple(),
                                    dep.line_number,
                                    if used_cache {
                                        " [cached]".dimmed().to_string()
                                    } else {
                                        "".to_string()
                                    }
                                );
                            } else {
                                eprintln!(
                                    "  {} {} {} → {} ({}:{}){} - update available",
                                    "↑".yellow(),
                                    dep.name.cyan(),
                                    dep.current_version.yellow(),
                                    latest_version.green(),
                                    dep.file_path.purple(),
                                    dep.line_number,
                                    if used_cache {
                                        " [cached]".dimmed().to_string()
                                    } else {
                                        "".to_string()
                                    }
                                );
                            }

                            let mut final_dep = dep.clone();
                            final_dep.latest_version = latest_version;
                            all_dependencies.push(final_dep);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to scan Helm charts in {}: {}",
                            "✗".red(),
                            actual_path,
                            e
                        );
                    }
                }
            }
            _ => {}
        }
    }

    if all_dependencies.is_empty() {
        eprintln!("{} No dependencies found", "INFO:".cyan());
        return Ok(());
    }

    // Separate successful dependencies from errors
    let (successful_deps, error_deps): (Vec<&Dependency>, Vec<&Dependency>) = all_dependencies
        .iter()
        .partition(|dep| !dep.latest_version.starts_with("ERROR:"));

    // Filter dependencies with updates available (excluding errors)
    let updates_available: Vec<_> = all_dependencies
        .iter()
        .filter(|dep| {
            !dep.latest_version.starts_with("ERROR:") && dep.current_version != dep.latest_version
        })
        .collect();

    // Check if all dependencies failed
    if !error_deps.is_empty() && successful_deps.is_empty() {
        eprintln!(
            "{} Could not verify any dependencies due to fetch errors",
            "WARNING:".yellow()
        );
        eprintln!(
            "  {} found but all failed to fetch",
            if all_dependencies.len() == 1 {
                "1 dependency".to_string()
            } else {
                format!("{} dependencies", all_dependencies.len())
            }
        );
        eprintln!(
            "  {}",
            "Tip: Configure OCI authentication in ~/.config/mk/config.toml or use --verbose for details"
                .dimmed()
        );
        return Ok(());
    }

    if updates_available.is_empty() {
        eprintln!("{} All dependencies are up to date!", "SUCCESS:".green());
        return Ok(());
    }

    eprintln!(
        "{} Found {} dependencies with updates available\n",
        "INFO:".cyan(),
        updates_available.len()
    );

    // Create multi-select prompt with project path info
    let items: Vec<String> = updates_available
        .iter()
        .map(|dep| dep.display_name())
        .collect();

    // Pre-select if only one dependency is available
    let defaults = if updates_available.len() == 1 {
        vec![true]
    } else {
        vec![false; updates_available.len()]
    };

    let selections = MultiSelect::new()
        .with_prompt("Select dependencies to update (Space to select, Enter to confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact()
        .context("Failed to get user selection")?;

    if selections.is_empty() {
        eprintln!("{} No dependencies selected", "INFO:".cyan());
        return Ok(());
    }

    // Apply updates
    eprintln!("\n{} Updating selected dependencies...", "INFO:".cyan());
    let selected_deps: Vec<_> = selections.iter().map(|&i| updates_available[i]).collect();

    for dep in &selected_deps {
        match &dep.dep_type {
            DependencyType::TerraformModule { source, constraint } => {
                terraform::update_terraform_module(
                    &dep.file_path,
                    source,
                    constraint,
                    &dep.latest_version,
                )
                .context(format!("Failed to update {}", dep.name))?;
                eprintln!(
                    "  {} Updated {} in {}",
                    "✓".green(),
                    dep.name.cyan(),
                    dep.file_path.purple()
                );
            }
            DependencyType::HelmChart { repository } => {
                if verbose {
                    eprintln!("  Updating {} from repository: {}", dep.name, repository);
                }
                // Extract the directory from file_path
                let project_path = std::path::Path::new(&dep.file_path)
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or(&dep.file_path);
                helm::update_helm_chart(
                    project_path,
                    &dep.name,
                    &dep.current_version,
                    &dep.latest_version,
                    dep.latest_app_version.as_deref(),
                )
                .context(format!("Failed to update {}", dep.name))?;
                eprintln!("  {} Updated {} in Chart.yaml", "✓".green(), dep.name);
            }
        }
    }

    eprintln!(
        "\n{} {} dependencies updated across {} project(s)",
        "SUCCESS:".green(),
        selected_deps.len(),
        total_projects
    );

    Ok(())
}
