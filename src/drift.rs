use anyhow::{Context, Result};
use colored::*;
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::env;
use crate::techno::{self, Technology};

#[derive(Debug, Clone, PartialEq)]
pub enum DriftStatus {
    Ok,
    Drift,
    Error(String),
}

#[derive(Debug)]
pub struct DriftResult {
    pub path: String,
    pub environment: String,
    pub technology: Technology,
    pub status: DriftStatus,
    pub output: Option<String>,
}

#[derive(Debug)]
pub struct DriftSummary {
    pub ok_count: usize,
    pub drift_count: usize,
    pub error_count: usize,
    pub drift_items: Vec<String>,
    pub error_items: Vec<(String, String)>,
}

/// Main entry point for drift detection
pub fn run_drift(
    base_path: &str,
    verbose: bool,
    tech_filter: Option<String>,
    env_filter: Vec<String>,
    capture: bool,
    max_depth: usize,
    no_ignore: bool,
) -> Result<()> {
    eprintln!(
        "{} Scanning: {} (max depth: {})",
        "INFO:".cyan(),
        base_path,
        max_depth
    );

    // Find all IaC projects
    let projects = scan_for_projects(base_path, max_depth, tech_filter.as_deref(), no_ignore)?;

    if projects.is_empty() {
        eprintln!("{} No IaC projects found", "WARNING:".yellow());
        return Ok(());
    }

    // Count total checks to perform
    let mut total_checks = 0;
    let mut project_env_map: HashMap<String, Vec<String>> = HashMap::new();

    for (project_path, techno) in &projects {
        if let Ok(environments) = env::get_environments(project_path, *techno, no_ignore) {
            let filtered_envs: Vec<String> = if env_filter.is_empty() {
                environments
            } else {
                environments
                    .into_iter()
                    .filter(|e| env_filter.contains(e))
                    .collect()
            };
            total_checks += filtered_envs.len();
            project_env_map.insert(project_path.clone(), filtered_envs);
        }
    }

    eprintln!(
        "{} Found {} project(s), {} total check(s) to perform\n",
        "INFO:".cyan(),
        projects.len(),
        total_checks
    );

    if total_checks == 0 {
        eprintln!("{} No environments to check", "WARNING:".yellow());
        return Ok(());
    }

    // Create progress bar
    let pb = ProgressBar::new(total_checks as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░"),
    );

    // Perform drift checks
    let mut results = Vec::new();
    let log_dir = if capture {
        Some(create_log_dir()?)
    } else {
        None
    };

    for (project_path, techno) in &projects {
        if let Some(environments) = project_env_map.get(project_path) {
            for env in environments {
                pb.set_message(format!("{} ({})", project_path, env));

                let result = check_drift(
                    project_path,
                    env,
                    *techno,
                    verbose,
                    capture,
                    log_dir.as_deref(),
                )?;

                results.push(result);
                pb.inc(1);
            }
        }
    }

    pb.finish_and_clear();

    // Print summary
    let summary = generate_summary(&results);
    print_summary(&summary);

    // Exit with appropriate code
    if summary.drift_count > 0 {
        std::process::exit(2);
    } else if summary.error_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Scan directory recursively for IaC projects
fn scan_for_projects(
    base_path: &str,
    max_depth: usize,
    tech_filter: Option<&str>,
    no_ignore: bool,
) -> Result<Vec<(String, Technology)>> {
    let mut projects = Vec::new();
    let base = Path::new(base_path).canonicalize()?;

    for result in WalkBuilder::new(&base)
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

        // Try to detect technology directly (no child scanning)
        // This ensures we only detect at the leaf level (actual chart/project directories)
        if let Some(techno) = techno::detect_technology_direct(path.to_str().unwrap()) {
            // Apply technology filter
            if let Some(filter) = tech_filter {
                let filter_lower = filter.to_lowercase();
                let matches = match techno {
                    Technology::Terraform => filter_lower == "terraform",
                    Technology::Helm => filter_lower == "helm",
                    _ => false,
                };
                if !matches {
                    continue;
                }
            }

            // Only include terraform and helm
            if matches!(techno, Technology::Terraform | Technology::Helm) {
                let project_path = path.to_string_lossy().to_string();
                // Avoid duplicates
                if !projects.iter().any(|(p, _)| p == &project_path) {
                    projects.push((project_path, techno));
                }
            }
        }
    }

    Ok(projects)
}

/// Check for drift in a single project/environment
fn check_drift(
    project_path: &str,
    environment: &str,
    techno: Technology,
    verbose: bool,
    capture: bool,
    log_dir: Option<&Path>,
) -> Result<DriftResult> {
    let result = match techno {
        Technology::Terraform => check_terraform_drift(project_path, environment, verbose)?,
        Technology::Helm => check_helm_drift(project_path, environment, verbose)?,
        _ => {
            return Ok(DriftResult {
                path: project_path.to_string(),
                environment: environment.to_string(),
                technology: techno,
                status: DriftStatus::Error("Unsupported technology".to_string()),
                output: None,
            });
        }
    };

    // Save output if capture is enabled
    if capture && result.output.is_some() {
        if let Some(log_dir) = log_dir {
            save_output(log_dir, &result)?;
        }
    }

    Ok(result)
}

/// Check terraform drift using terraform plan with -detailed-exitcode
fn check_terraform_drift(
    project_path: &str,
    environment: &str,
    verbose: bool,
) -> Result<DriftResult> {
    use crate::Action;

    if verbose {
        eprintln!(
            "\n{} Checking drift for {} ({})",
            "INFO:".cyan(),
            project_path,
            environment
        );
    }

    // Use the unified execute_action_internal in drift mode
    let (exit_code, output) = match crate::execute_action_internal(
        Action::Plan,
        project_path,
        environment,
        &["-detailed-exitcode".to_string(), "-input=false".to_string()],
        verbose,
        true,  // drift_mode = true
        false, // no_ignore = false (drift doesn't need this for env check)
    ) {
        Ok(result) => result,
        Err(e) => {
            return Ok(DriftResult {
                path: project_path.to_string(),
                environment: environment.to_string(),
                technology: Technology::Terraform,
                status: DriftStatus::Error(format!("Execution failed: {}", e)),
                output: None,
            });
        }
    };

    // Interpret exit code from terraform plan -detailed-exitcode
    let status = match exit_code {
        0 => DriftStatus::Ok,
        2 => DriftStatus::Drift,
        _ => DriftStatus::Error(format!("Exit code {}: Plan failed", exit_code)),
    };

    Ok(DriftResult {
        path: project_path.to_string(),
        environment: environment.to_string(),
        technology: Technology::Terraform,
        status,
        output,
    })
}

/// Check helm drift using helm diff
fn check_helm_drift(project_path: &str, environment: &str, verbose: bool) -> Result<DriftResult> {
    use crate::Action;

    if verbose {
        eprintln!(
            "\n{} Checking drift for {} ({})",
            "INFO:".cyan(),
            project_path,
            environment
        );
    }

    // Use the unified execute_action_internal in drift mode
    let (exit_code, output) = match crate::execute_action_internal(
        Action::Diff,
        project_path,
        environment,
        &[],
        verbose,
        true,  // drift_mode = true
        false, // no_ignore = false (drift doesn't need this for env check)
    ) {
        Ok(result) => result,
        Err(e) => {
            return Ok(DriftResult {
                path: project_path.to_string(),
                environment: environment.to_string(),
                technology: Technology::Helm,
                status: DriftStatus::Error(format!("Execution failed: {}", e)),
                output: None,
            });
        }
    };

    // Helmfile diff returns 0 if no changes, 2 if changes detected
    let status = match exit_code {
        0 => DriftStatus::Ok,
        2 => DriftStatus::Drift,
        _ => DriftStatus::Error(format!("Exit code {}: Diff failed", exit_code)),
    };

    Ok(DriftResult {
        path: project_path.to_string(),
        environment: environment.to_string(),
        technology: Technology::Helm,
        status,
        output,
    })
}

/// Generate summary statistics
fn generate_summary(results: &[DriftResult]) -> DriftSummary {
    let mut ok_count = 0;
    let mut drift_count = 0;
    let mut error_count = 0;
    let mut drift_items = Vec::new();
    let mut error_items = Vec::new();

    for result in results {
        match &result.status {
            DriftStatus::Ok => ok_count += 1,
            DriftStatus::Drift => {
                drift_count += 1;
                drift_items.push(format!("{} ({})", result.path, result.environment));
            }
            DriftStatus::Error(msg) => {
                error_count += 1;
                error_items.push((
                    format!("{} ({})", result.path, result.environment),
                    msg.clone(),
                ));
            }
        }
    }

    DriftSummary {
        ok_count,
        drift_count,
        error_count,
        drift_items,
        error_items,
    }
}

/// Print summary
fn print_summary(summary: &DriftSummary) {
    eprintln!("\n{}", "Summary:".bold());
    eprintln!("  {} {} OK", "✓".green(), summary.ok_count);
    eprintln!("  {} {} Drift Detected", "⚠".yellow(), summary.drift_count);
    eprintln!("  {} {} Errors", "✗".red(), summary.error_count);

    if !summary.drift_items.is_empty() {
        eprintln!("\n{}", "Drift detected in:".yellow().bold());
        for item in &summary.drift_items {
            eprintln!("  - {}", item);
        }
    }

    if !summary.error_items.is_empty() {
        eprintln!("\n{}", "Errors in:".red().bold());
        for (item, msg) in &summary.error_items {
            eprintln!("  - {}: {}", item, msg);
        }
    }
}

/// Create log directory for captured output
fn create_log_dir() -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let log_dir = PathBuf::from(".drift-logs").join(format!("drift-{}", timestamp));
    fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    eprintln!(
        "{} Capturing output to: {}",
        "INFO:".cyan(),
        log_dir.display()
    );

    Ok(log_dir)
}

/// Save output to log file
fn save_output(log_dir: &Path, result: &DriftResult) -> Result<()> {
    if let Some(output) = &result.output {
        let filename = format!(
            "{}_{}.log",
            result.path.replace(['/', '.'], "_"),
            result.environment
        );
        let log_file = log_dir.join(filename);

        let content = format!(
            "Project: {}\nEnvironment: {}\nTechnology: {:?}\nStatus: {:?}\n\n{}",
            result.path, result.environment, result.technology, result.status, output
        );

        fs::write(log_file, content)?;
    }
    Ok(())
}
