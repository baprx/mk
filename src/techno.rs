use anyhow::Result;
use colored::*;
use dialoguer::Select;
use std::path::Path;

use crate::commands::Action;
use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Technology {
    Terraform,
    Helm,
    Kustomize,
    Ansible,
}

impl Technology {
    pub fn as_str(&self) -> &'static str {
        match self {
            Technology::Terraform => "terraform",
            Technology::Helm => "helm",
            Technology::Kustomize => "kustomize",
            Technology::Ansible => "ansible",
        }
    }

    /// Check if this technology supports the given action
    pub fn supports_action(&self, action: &Action) -> bool {
        match (self, action) {
            // Terraform actions
            (Technology::Terraform, Action::Plan) => true,
            (Technology::Terraform, Action::Check) => true,
            (Technology::Terraform, Action::Apply) => true,
            (Technology::Terraform, Action::Destroy) => true,
            (Technology::Terraform, Action::Delete) => true,
            (Technology::Terraform, Action::Output { .. }) => true,
            (Technology::Terraform, Action::Duplicate { .. }) => true,
            (Technology::Terraform, Action::Unlock { .. }) => true,
            (Technology::Terraform, Action::Show) => true,

            // Helm actions
            (Technology::Helm, Action::Apply) => true,
            (Technology::Helm, Action::Uninstall) => true,
            (Technology::Helm, Action::Delete) => true,
            (Technology::Helm, Action::Destroy) => true,
            (Technology::Helm, Action::Template) => true,
            (Technology::Helm, Action::Deps) => true,
            (Technology::Helm, Action::Diff) => true,
            (Technology::Helm, Action::Check) => true,
            (Technology::Helm, Action::Duplicate { .. }) => true,

            // Ansible actions
            (Technology::Ansible, Action::Diff) => true,
            (Technology::Ansible, Action::Apply) => true,
            (Technology::Ansible, Action::List) => true,
            (Technology::Ansible, Action::Deps) => true,

            // Kustomize actions
            (Technology::Kustomize, Action::Apply) => true,
            (Technology::Kustomize, Action::Diff) => true,

            _ => false,
        }
    }
}

impl std::fmt::Display for Technology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Try to detect technology in the given path without fallback
fn try_detect_technology_direct(path: &Path) -> Option<Technology> {
    // Check based on directory name first (ansible or terraform)
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name == "ansible" {
            return Some(Technology::Ansible);
        } else if name == "terraform" {
            return Some(Technology::Terraform);
        }
    }

    // Check for helm (Chart.yaml is required in every Helm chart)
    if path.join("Chart.yaml").exists() {
        return Some(Technology::Helm);
    }

    // Check for kustomize (overlays directory)
    if path.join("overlays").exists() {
        return Some(Technology::Kustomize);
    }

    None
}

/// Detect technology directly in the given path, without any fallback or child scanning.
/// This is used for drift scanning where we want to detect only at the leaf level.
/// Returns None if no technology is detected at this exact path.
pub fn detect_technology_direct(project_path: &str) -> Option<Technology> {
    let path = Path::new(project_path);

    if !path.exists() || !path.is_dir() {
        return None;
    }

    try_detect_technology_direct(path)
}

/// Scan child directories for technology folders
fn scan_child_technologies(path: &Path) -> Result<Vec<(String, Technology)>> {
    let mut found_technologies = Vec::new();

    // Read directory entries
    let entries = std::fs::read_dir(path)
        .map_err(|e| anyhow::anyhow!("Failed to read directory {}: {}", path.display(), e))?;

    for entry in entries {
        let entry = entry?;
        let child_path = entry.path();

        // Only process directories
        if !child_path.is_dir() {
            continue;
        }

        // Try to detect technology in this child directory
        if let Some(tech) = try_detect_technology_direct(&child_path) {
            if let Some(dir_name) = child_path.file_name().and_then(|n| n.to_str()) {
                found_technologies.push((dir_name.to_string(), tech));
            }
        }
    }

    Ok(found_technologies)
}

/// Detect the technology type based on the project path structure
///
/// This function implements hierarchical detection:
/// 1. First tries to detect technology in the exact path provided
/// 2. If that fails, scans child directories for technology folders
/// 3. If exactly one technology is found in children, uses it automatically
/// 4. If multiple technologies found, filters by action support if action is provided
/// 5. Falls back to priority-based or interactive selection if needed
///
/// Returns a tuple of (Technology, actual_path) where actual_path is the directory
/// where the technology was found. This is important when technology is detected
/// in a child directory, as subsequent operations need to use the correct path.
///
/// # Parameters
/// * `project_path` - The path to analyze for technology detection
/// * `action` - Optional action to filter technologies by support
/// * `silent` - If true, suppresses all informational output (useful for shell completion)
pub fn detect_technology(
    project_path: &str,
    action: Option<&Action>,
    silent: bool,
) -> Result<(Technology, String)> {
    let path = Path::new(project_path);

    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", project_path);
    }

    if !path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", project_path);
    }

    // Try direct detection first (maintains backward compatibility and precedence)
    if let Some(tech) = try_detect_technology_direct(path) {
        if !silent {
            eprintln!(
                "{} Detected {} in {}",
                "INFO:".cyan(),
                tech.to_string().bold(),
                project_path
            );
        }
        return Ok((tech, project_path.to_string()));
    }

    // If direct detection failed, try scanning child directories
    let child_technologies = scan_child_technologies(path)?;

    match child_technologies.len() {
        0 => {
            anyhow::bail!("No technology detected in {}", project_path);
        }
        1 => {
            // Exactly one technology found in children, use it automatically
            let (dir_name, tech) = &child_technologies[0];
            let full_path = path.join(dir_name);
            let full_path_str = full_path.to_string_lossy().to_string();
            if !silent {
                eprintln!(
                    "{} Detected {} in {} (discovered from parent directory)",
                    "INFO:".cyan(),
                    tech.to_string().bold(),
                    full_path.display()
                );
            }
            Ok((*tech, full_path_str))
        }
        _ => {
            // Multiple technologies found

            // Filter by action if provided
            let filtered_technologies = if let Some(action) = action {
                let filtered: Vec<_> = child_technologies
                    .iter()
                    .filter(|(_, tech)| tech.supports_action(action))
                    .cloned()
                    .collect();

                // If filtering resulted in exactly one match, use it
                if filtered.len() == 1 {
                    let (dir_name, tech) = &filtered[0];
                    let full_path = path.join(dir_name);
                    let full_path_str = full_path.to_string_lossy().to_string();
                    eprintln!(
                        "{} Multiple technologies detected. Auto-selected {} for action (from {})",
                        "INFO:".cyan(),
                        tech.to_string().bold(),
                        full_path.display()
                    );
                    return Ok((*tech, full_path_str));
                }

                // If filtering resulted in zero matches, inform user
                if filtered.is_empty() {
                    eprintln!(
                        "{} None of the detected technologies support this action",
                        "WARN:".yellow()
                    );
                    // Fall through to show all technologies
                    child_technologies.clone()
                } else {
                    // Multiple technologies still support this action
                    filtered
                }
            } else {
                // No action provided, use all detected technologies
                child_technologies.clone()
            };

            // Try to load config
            let config = Config::load().unwrap_or_default();

            // Check if user has configured priority order
            if let Some(priority_list) = config.get_technology_priority() {
                // Use priority-based selection on filtered technologies
                for priority_tech in priority_list {
                    if let Some((dir_name, _)) = filtered_technologies
                        .iter()
                        .find(|(_, tech)| *tech == priority_tech)
                    {
                        let full_path = path.join(dir_name);
                        let full_path_str = full_path.to_string_lossy().to_string();
                        eprintln!(
                            "{} Multiple technologies detected. Using {} based on configured priority (from {})",
                            "INFO:".cyan(),
                            priority_tech.to_string().bold(),
                            full_path.display()
                        );
                        return Ok((priority_tech, full_path_str));
                    }
                }
            }

            // No priority configured or priority didn't match
            // Log info message
            eprintln!(
                "{} Multiple technologies detected in {}",
                "INFO:".cyan(),
                project_path
            );

            // Try interactive selection only if not in silent mode
            if !silent {
                let items: Vec<String> = filtered_technologies
                    .iter()
                    .map(|(dir_name, tech)| format!("{} ({})", dir_name, tech))
                    .collect();

                let selection = Select::new()
                    .with_prompt("Select technology")
                    .items(&items)
                    .default(0)
                    .interact();

                match selection {
                    Ok(idx) => {
                        let (dir_name, tech) = &filtered_technologies[idx];
                        let full_path = path.join(dir_name);
                        let full_path_str = full_path.to_string_lossy().to_string();
                        eprintln!(
                            "{} Selected {} in {}",
                            "INFO:".cyan(),
                            tech.to_string().bold(),
                            full_path.display()
                        );
                        return Ok((*tech, full_path_str));
                    }
                    Err(_) => {
                        // Fall through to error below
                    }
                }
            }

            // Silent mode or interactive selection failed
            anyhow::bail!("Multiple technologies detected in {}. Please specify which technology to use or configure a priority in your config.", project_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_technology_display() {
        assert_eq!(Technology::Terraform.to_string(), "terraform");
        assert_eq!(Technology::Helm.to_string(), "helm");
        assert_eq!(Technology::Kustomize.to_string(), "kustomize");
        assert_eq!(Technology::Ansible.to_string(), "ansible");
    }

    #[test]
    fn test_technology_as_str() {
        assert_eq!(Technology::Terraform.as_str(), "terraform");
        assert_eq!(Technology::Helm.as_str(), "helm");
        assert_eq!(Technology::Kustomize.as_str(), "kustomize");
        assert_eq!(Technology::Ansible.as_str(), "ansible");
    }

    #[test]
    fn test_detect_terraform_by_directory_name() {
        let temp_dir = TempDir::new().unwrap();
        let terraform_dir = temp_dir.path().join("terraform");
        fs::create_dir(&terraform_dir).unwrap();

        let (tech, _path) =
            detect_technology(terraform_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Terraform);
    }

    #[test]
    fn test_detect_ansible_by_directory_name() {
        let temp_dir = TempDir::new().unwrap();
        let ansible_dir = temp_dir.path().join("ansible");
        fs::create_dir(&ansible_dir).unwrap();

        let (tech, _path) = detect_technology(ansible_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Ansible);
    }

    #[test]
    fn test_detect_helm_by_values_file() {
        let temp_dir = TempDir::new().unwrap();
        let helm_dir = temp_dir.path().join("my-chart");
        fs::create_dir(&helm_dir).unwrap();
        // Create Chart.yaml (required for Helm chart detection)
        fs::write(
            helm_dir.join("Chart.yaml"),
            "name: my-chart\nversion: 1.0.0",
        )
        .unwrap();
        fs::write(helm_dir.join("values.yaml"), "replicaCount: 3").unwrap();

        let (tech, _path) = detect_technology(helm_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Helm);
    }

    #[test]
    fn test_detect_kustomize_by_overlays_directory() {
        let temp_dir = TempDir::new().unwrap();
        let kustomize_dir = temp_dir.path().join("my-kustomize");
        fs::create_dir(&kustomize_dir).unwrap();
        fs::create_dir(kustomize_dir.join("overlays")).unwrap();

        let (tech, _path) =
            detect_technology(kustomize_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Kustomize);
    }

    #[test]
    fn test_detect_technology_nonexistent_path() {
        let result = detect_technology("/nonexistent/path/12345", None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_detect_technology_file_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let result = detect_technology(file_path.to_str().unwrap(), None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_detect_technology_no_indicators() {
        let temp_dir = TempDir::new().unwrap();
        let unknown_dir = temp_dir.path().join("unknown");
        fs::create_dir(&unknown_dir).unwrap();

        let result = detect_technology(unknown_dir.to_str().unwrap(), None, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No technology detected"));
    }

    #[rstest]
    #[case(Technology::Terraform, "terraform")]
    #[case(Technology::Helm, "helm")]
    #[case(Technology::Kustomize, "kustomize")]
    #[case(Technology::Ansible, "ansible")]
    fn test_technology_string_conversion(#[case] tech: Technology, #[case] expected: &str) {
        assert_eq!(tech.to_string(), expected);
        assert_eq!(tech.as_str(), expected);
    }

    #[test]
    fn test_terraform_takes_precedence_over_helm() {
        // If a directory is named "terraform", it should be detected as Terraform
        // even if it has a values.yaml file
        let temp_dir = TempDir::new().unwrap();
        let terraform_dir = temp_dir.path().join("terraform");
        fs::create_dir(&terraform_dir).unwrap();
        fs::write(terraform_dir.join("values.yaml"), "content").unwrap();

        let (tech, _path) =
            detect_technology(terraform_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Terraform);
    }

    #[test]
    fn test_ansible_takes_precedence_over_helm() {
        // If a directory is named "ansible", it should be detected as Ansible
        // even if it has a values.yaml file
        let temp_dir = TempDir::new().unwrap();
        let ansible_dir = temp_dir.path().join("ansible");
        fs::create_dir(&ansible_dir).unwrap();
        fs::write(ansible_dir.join("values.yaml"), "content").unwrap();

        let (tech, _path) = detect_technology(ansible_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Ansible);
    }

    #[test]
    fn test_helm_takes_precedence_over_kustomize() {
        // If both Chart.yaml and overlays/ exist, Helm should be detected first
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("project");
        fs::create_dir(&project_dir).unwrap();
        fs::write(
            project_dir.join("Chart.yaml"),
            "name: my-chart\nversion: 1.0.0",
        )
        .unwrap();
        fs::write(project_dir.join("values.yaml"), "content").unwrap();
        fs::create_dir(project_dir.join("overlays")).unwrap();

        let (tech, _path) = detect_technology(project_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Helm);
    }

    // Tests for hierarchical discovery with fallback

    #[test]
    fn test_fallback_discovery_single_terraform() {
        // Parent directory with single terraform child should be detected
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("tests");
        fs::create_dir(&parent_dir).unwrap();

        let terraform_dir = parent_dir.join("terraform");
        fs::create_dir(&terraform_dir).unwrap();

        let (tech, _path) = detect_technology(parent_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Terraform);
    }

    #[test]
    fn test_fallback_discovery_single_ansible() {
        // Parent directory with single ansible child should be detected
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("tests");
        fs::create_dir(&parent_dir).unwrap();

        let ansible_dir = parent_dir.join("ansible");
        fs::create_dir(&ansible_dir).unwrap();

        let (tech, _path) = detect_technology(parent_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Ansible);
    }

    #[test]
    fn test_fallback_discovery_single_helm() {
        // Parent directory with single helm child should be detected
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("tests");
        fs::create_dir(&parent_dir).unwrap();

        let helm_dir = parent_dir.join("my-chart");
        fs::create_dir(&helm_dir).unwrap();
        fs::write(
            helm_dir.join("Chart.yaml"),
            "name: my-chart\nversion: 1.0.0",
        )
        .unwrap();
        fs::write(helm_dir.join("values.yaml"), "replicaCount: 3").unwrap();

        let (tech, _path) = detect_technology(parent_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Helm);
    }

    #[test]
    fn test_exact_path_takes_precedence_over_fallback() {
        // Direct detection should take precedence over child scanning
        let temp_dir = TempDir::new().unwrap();
        let terraform_parent = temp_dir.path().join("terraform");
        fs::create_dir(&terraform_parent).unwrap();

        // Create a child ansible directory inside terraform directory
        let ansible_child = terraform_parent.join("ansible");
        fs::create_dir(&ansible_child).unwrap();

        // Should detect terraform (parent directory name), not ansible (child)
        let (tech, _path) =
            detect_technology(terraform_parent.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Terraform);
    }

    #[test]
    fn test_fallback_ignores_non_technology_directories() {
        // Should ignore directories that don't contain technology indicators
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("tests");
        fs::create_dir(&parent_dir).unwrap();

        // Create some non-technology directories
        fs::create_dir(parent_dir.join("docs")).unwrap();
        fs::create_dir(parent_dir.join("scripts")).unwrap();

        // Create one terraform directory
        let terraform_dir = parent_dir.join("terraform");
        fs::create_dir(&terraform_dir).unwrap();

        let (tech, _path) = detect_technology(parent_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Terraform);
    }

    #[test]
    fn test_fallback_ignores_files_in_parent() {
        // Should only scan child directories, not be affected by files in parent
        // Note: If parent has values.yaml, it will be detected as Helm (by design)
        // This test verifies that files don't interfere with child directory scanning
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("tests");
        fs::create_dir(&parent_dir).unwrap();

        // Create some files in parent (but NOT values.yaml which would make it Helm)
        fs::write(parent_dir.join("README.md"), "content").unwrap();
        fs::write(parent_dir.join("config.yaml"), "should be ignored").unwrap();

        // Create terraform child directory
        let terraform_dir = parent_dir.join("terraform");
        fs::create_dir(&terraform_dir).unwrap();

        let (tech, _path) = detect_technology(parent_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Terraform);
    }

    #[test]
    fn test_no_fallback_when_direct_detection_succeeds() {
        // If direct detection works, fallback should not be attempted
        let temp_dir = TempDir::new().unwrap();
        let helm_dir = temp_dir.path().join("my-chart");
        fs::create_dir(&helm_dir).unwrap();
        fs::write(
            helm_dir.join("Chart.yaml"),
            "name: my-chart\nversion: 1.0.0",
        )
        .unwrap();
        fs::write(helm_dir.join("values.yaml"), "replicaCount: 3").unwrap();

        // Create a terraform subdirectory (which should be ignored)
        let terraform_child = helm_dir.join("terraform");
        fs::create_dir(&terraform_child).unwrap();

        // Should detect Helm (parent), not Terraform (child)
        let (tech, _path) = detect_technology(helm_dir.to_str().unwrap(), None, false).unwrap();
        assert_eq!(tech, Technology::Helm);
    }

    #[test]
    fn test_fallback_with_mixed_technology_types() {
        // Test with multiple different technology types
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("tests");
        fs::create_dir(&parent_dir).unwrap();

        // Create terraform directory
        fs::create_dir(parent_dir.join("terraform")).unwrap();

        // Create helm directory
        let helm_dir = parent_dir.join("my-helm");
        fs::create_dir(&helm_dir).unwrap();
        fs::write(helm_dir.join("Chart.yaml"), "name: my-helm\nversion: 1.0.0").unwrap();
        fs::write(helm_dir.join("values.yaml"), "content").unwrap();

        // Verify that scan_child_technologies finds both technologies
        let result = scan_child_technologies(&parent_dir);
        assert!(result.is_ok());
        let technologies = result.unwrap();
        assert_eq!(technologies.len(), 2);

        // Verify both technologies are found
        let tech_types: Vec<Technology> = technologies.iter().map(|(_, t)| *t).collect();
        assert!(tech_types.contains(&Technology::Terraform));
        assert!(tech_types.contains(&Technology::Helm));
    }

    // Tests for action-based filtering

    #[test]
    fn test_action_filtering_single_match() {
        // When multiple technologies are detected but only one supports the action
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("project");
        fs::create_dir(&parent_dir).unwrap();

        // Create terraform directory
        fs::create_dir(parent_dir.join("terraform")).unwrap();

        // Create ansible directory
        fs::create_dir(parent_dir.join("ansible")).unwrap();

        // Plan action should auto-select Terraform (only Terraform supports Plan)
        let (tech, path) =
            detect_technology(parent_dir.to_str().unwrap(), Some(&Action::Plan), false).unwrap();
        assert_eq!(tech, Technology::Terraform);
        assert!(path.contains("terraform"));
    }

    #[test]
    fn test_action_filtering_list_action() {
        // List action is only supported by Ansible
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("project");
        fs::create_dir(&parent_dir).unwrap();

        // Create terraform directory
        fs::create_dir(parent_dir.join("terraform")).unwrap();

        // Create ansible directory
        fs::create_dir(parent_dir.join("ansible")).unwrap();

        // List action should auto-select Ansible
        let (tech, path) =
            detect_technology(parent_dir.to_str().unwrap(), Some(&Action::List), false).unwrap();
        assert_eq!(tech, Technology::Ansible);
        assert!(path.contains("ansible"));
    }

    #[test]
    fn test_action_filtering_template_action() {
        // Template action supported by Helm only (among common technologies)
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().join("project");
        fs::create_dir(&parent_dir).unwrap();

        // Create terraform directory
        fs::create_dir(parent_dir.join("terraform")).unwrap();

        // Create helm directory
        let helm_dir = parent_dir.join("my-chart");
        fs::create_dir(&helm_dir).unwrap();
        fs::write(
            helm_dir.join("Chart.yaml"),
            "name: my-chart\nversion: 1.0.0",
        )
        .unwrap();
        fs::write(helm_dir.join("values.yaml"), "content").unwrap();

        // Template action should auto-select Helm
        let (tech, path) =
            detect_technology(parent_dir.to_str().unwrap(), Some(&Action::Template), false)
                .unwrap();
        assert_eq!(tech, Technology::Helm);
        assert!(path.contains("my-chart"));
    }
}
