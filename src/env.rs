use anyhow::Result;
use colored::*;
use ignore::WalkBuilder;
use std::path::Path;

use crate::techno::Technology;

/// Get list of available environments for a given technology and project path
pub fn get_environments(
    project_path: &str,
    techno: Technology,
    no_ignore: bool,
) -> Result<Vec<String>> {
    let path = Path::new(project_path);
    let mut envs = Vec::new();

    match techno {
        Technology::Terraform => {
            // Look for tfvars files in tfvars/ directory
            let tfvars_dir = path.join("tfvars");
            if tfvars_dir.exists() {
                for entry in WalkBuilder::new(&tfvars_dir)
                    .max_depth(Some(1))
                    .git_ignore(!no_ignore)
                    .git_exclude(!no_ignore)
                    .git_global(!no_ignore)
                    .build()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_some_and(|ft| ft.is_file()) {
                        if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                            envs.push(stem.to_string());
                        }
                    }
                }
            }
        }
        Technology::Helm => {
            // Look for directories in values/ directory
            let values_dir = path.join("values");
            if values_dir.exists() {
                for entry in WalkBuilder::new(&values_dir)
                    .max_depth(Some(1))
                    .git_ignore(!no_ignore)
                    .git_exclude(!no_ignore)
                    .git_global(!no_ignore)
                    .build()
                    .filter_map(|e| e.ok())
                {
                    // Filter out the root directory (min_depth equivalent)
                    if entry.depth() > 0 && entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        if let Some(name) = entry.file_name().to_str() {
                            envs.push(name.to_string());
                        }
                    }
                }
            }
        }
        Technology::Kustomize => {
            // Look for directories in overlays/ directory
            let overlays_dir = path.join("overlays");
            if overlays_dir.exists() {
                for entry in WalkBuilder::new(&overlays_dir)
                    .max_depth(Some(1))
                    .git_ignore(!no_ignore)
                    .git_exclude(!no_ignore)
                    .git_global(!no_ignore)
                    .build()
                    .filter_map(|e| e.ok())
                {
                    // Filter out the root directory (min_depth equivalent)
                    if entry.depth() > 0 && entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                            envs.push(stem.to_string());
                        }
                    }
                }
            }
        }
        Technology::Ansible => {
            // Look for inventory files in inventories/ directory
            let inventories_dir = path.join("inventories");
            if inventories_dir.exists() {
                for entry in WalkBuilder::new(&inventories_dir)
                    .max_depth(Some(1))
                    .git_ignore(!no_ignore)
                    .git_exclude(!no_ignore)
                    .git_global(!no_ignore)
                    .build()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_some_and(|ft| ft.is_file()) {
                        if let Some(name) = entry.path().file_name().and_then(|s| s.to_str()) {
                            // Remove all extensions (e.g., "demo.yml" -> "demo")
                            let env_name = name.split('.').next().unwrap_or(name);
                            if !env_name.is_empty() {
                                envs.push(env_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    envs.sort();
    envs.dedup();
    Ok(envs)
}

/// Check if the given environment is valid for the technology and project
pub fn check_environment(
    project_path: &str,
    environment: &str,
    techno: Technology,
    no_ignore: bool,
) -> Result<()> {
    let envs = get_environments(project_path, techno, no_ignore)?;

    if envs.is_empty() {
        anyhow::bail!(
            "{} No environments found for {} in {}",
            "ERROR:".red(),
            techno,
            project_path
        );
    }

    if envs.contains(&environment.to_string()) {
        Ok(())
    } else {
        anyhow::bail!(
            "{} Invalid env provided ({}). Valid options are: [{}]",
            "ERROR:".red(),
            environment,
            envs.join("|")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::fs;
    use tempfile::TempDir;

    // Helper function to create a test project structure
    fn create_terraform_project(temp_dir: &TempDir, envs: &[&str]) -> String {
        let project_dir = temp_dir.path().join("terraform");
        fs::create_dir(&project_dir).unwrap();

        let tfvars_dir = project_dir.join("tfvars");
        fs::create_dir(&tfvars_dir).unwrap();

        for env in envs {
            fs::write(
                tfvars_dir.join(format!("{}.tfvars", env)),
                format!("env = \"{}\"", env),
            )
            .unwrap();
        }

        project_dir.to_str().unwrap().to_string()
    }

    fn create_helm_project(temp_dir: &TempDir, envs: &[&str]) -> String {
        let project_dir = temp_dir.path().join("helm");
        fs::create_dir(&project_dir).unwrap();

        let values_dir = project_dir.join("values");
        fs::create_dir(&values_dir).unwrap();

        for env in envs {
            let env_dir = values_dir.join(env);
            fs::create_dir(&env_dir).unwrap();
            fs::write(env_dir.join("values.yaml"), format!("environment: {}", env)).unwrap();
        }

        project_dir.to_str().unwrap().to_string()
    }

    fn create_kustomize_project(temp_dir: &TempDir, envs: &[&str]) -> String {
        let project_dir = temp_dir.path().join("kustomize");
        fs::create_dir(&project_dir).unwrap();

        let overlays_dir = project_dir.join("overlays");
        fs::create_dir(&overlays_dir).unwrap();

        for env in envs {
            let env_dir = overlays_dir.join(env);
            fs::create_dir(&env_dir).unwrap();
            fs::write(
                env_dir.join("kustomization.yaml"),
                format!("namePrefix: {}-", env),
            )
            .unwrap();
        }

        project_dir.to_str().unwrap().to_string()
    }

    fn create_ansible_project(temp_dir: &TempDir, envs: &[&str]) -> String {
        let project_dir = temp_dir.path().join("ansible");
        fs::create_dir(&project_dir).unwrap();

        let inventories_dir = project_dir.join("inventories");
        fs::create_dir(&inventories_dir).unwrap();

        for env in envs {
            fs::write(
                inventories_dir.join(format!("{}.yml", env)),
                format!("all:\n  hosts:\n    {}:", env),
            )
            .unwrap();
        }

        project_dir.to_str().unwrap().to_string()
    }

    #[test]
    fn test_get_terraform_environments() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = create_terraform_project(&temp_dir, &["dev", "prod", "staging"]);

        let envs = get_environments(&project_path, Technology::Terraform, false).unwrap();
        assert_eq!(envs, vec!["dev", "prod", "staging"]);
    }

    #[test]
    fn test_get_helm_environments() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = create_helm_project(&temp_dir, &["dev", "prod"]);

        let envs = get_environments(&project_path, Technology::Helm, false).unwrap();
        assert_eq!(envs, vec!["dev", "prod"]);
    }

    #[test]
    fn test_get_kustomize_environments() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = create_kustomize_project(&temp_dir, &["dev", "prod", "test"]);

        let envs = get_environments(&project_path, Technology::Kustomize, false).unwrap();
        assert_eq!(envs, vec!["dev", "prod", "test"]);
    }

    #[test]
    fn test_get_ansible_environments() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = create_ansible_project(&temp_dir, &["dev", "prod"]);

        let envs = get_environments(&project_path, Technology::Ansible, false).unwrap();
        assert_eq!(envs, vec!["dev", "prod"]);
    }

    #[test]
    fn test_get_environments_empty() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("terraform");
        fs::create_dir(&project_dir).unwrap();
        fs::create_dir(project_dir.join("tfvars")).unwrap();

        let envs =
            get_environments(project_dir.to_str().unwrap(), Technology::Terraform, false).unwrap();
        assert_eq!(envs, Vec::<String>::new());
    }

    #[test]
    fn test_get_environments_sorted() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = create_terraform_project(&temp_dir, &["prod", "dev", "staging", "test"]);

        let envs = get_environments(&project_path, Technology::Terraform, false).unwrap();
        assert_eq!(envs, vec!["dev", "prod", "staging", "test"]);
    }

    #[test]
    fn test_check_valid_environment() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = create_terraform_project(&temp_dir, &["dev", "prod"]);

        let result = check_environment(&project_path, "dev", Technology::Terraform, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_invalid_environment() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = create_terraform_project(&temp_dir, &["dev", "prod"]);

        let result = check_environment(&project_path, "staging", Technology::Terraform, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid env"));
        assert!(err_msg.contains("dev|prod"));
    }

    #[test]
    fn test_check_environment_no_environments_found() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("terraform");
        fs::create_dir(&project_dir).unwrap();
        fs::create_dir(project_dir.join("tfvars")).unwrap();

        let result = check_environment(
            project_dir.to_str().unwrap(),
            "dev",
            Technology::Terraform,
            false,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No environments found"));
    }

    #[rstest]
    #[case(Technology::Terraform, &["dev", "prod", "staging"])]
    #[case(Technology::Helm, &["dev", "prod"])]
    #[case(Technology::Kustomize, &["dev", "prod", "test"])]
    #[case(Technology::Ansible, &["dev", "prod"])]
    fn test_get_environments_for_all_technologies(
        #[case] tech: Technology,
        #[case] expected_envs: &[&str],
    ) {
        let temp_dir = TempDir::new().unwrap();

        let project_path = match tech {
            Technology::Terraform => create_terraform_project(&temp_dir, expected_envs),
            Technology::Helm => create_helm_project(&temp_dir, expected_envs),
            Technology::Kustomize => create_kustomize_project(&temp_dir, expected_envs),
            Technology::Ansible => create_ansible_project(&temp_dir, expected_envs),
        };

        let envs = get_environments(&project_path, tech, false).unwrap();
        let expected: Vec<String> = expected_envs.iter().map(|s| s.to_string()).collect();
        assert_eq!(envs, expected);
    }

    #[test]
    fn test_ansible_inventory_multiple_extensions() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("ansible");
        fs::create_dir(&project_dir).unwrap();

        let inventories_dir = project_dir.join("inventories");
        fs::create_dir(&inventories_dir).unwrap();

        // Create files with different extensions
        fs::write(inventories_dir.join("dev.yml"), "hosts: dev").unwrap();
        fs::write(inventories_dir.join("prod.yaml"), "hosts: prod").unwrap();
        fs::write(inventories_dir.join("staging.ini"), "hosts: staging").unwrap();

        let envs =
            get_environments(project_dir.to_str().unwrap(), Technology::Ansible, false).unwrap();
        assert_eq!(envs, vec!["dev", "prod", "staging"]);
    }

    #[test]
    fn test_environments_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("terraform");
        fs::create_dir(&project_dir).unwrap();

        let tfvars_dir = project_dir.join("tfvars");
        fs::create_dir(&tfvars_dir).unwrap();

        // Create multiple files with same env name (shouldn't happen but test it)
        fs::write(tfvars_dir.join("dev.tfvars"), "env = dev").unwrap();

        let envs =
            get_environments(project_dir.to_str().unwrap(), Technology::Terraform, false).unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0], "dev");
    }
}
