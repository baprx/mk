use anyhow::Result;
use std::fs;
use std::path::Path;

use super::Action;

pub fn get_command(
    action: &Action,
    project_path: &str,
    environment: &str,
    options: &[String],
) -> Result<Vec<String>> {
    let path = Path::new(project_path);

    // Determine backend directory name (backend-vars or backend_vars)
    let backend_dir = if path.join("backend-vars").exists() {
        "backend-vars"
    } else {
        "backend_vars"
    };

    match action {
        Action::Duplicate { target_env } => {
            // Perform the duplication using native Rust
            duplicate_terraform_env(project_path, environment, target_env)?;
            Ok(vec![]) // No commands to execute
        }
        Action::Output { key: None } => {
            // For --all flag, get all output keys and create individual commands
            let output_keys = get_output_keys(project_path)?;
            let mut commands = vec![
                "tfswitch".to_string(),
                format!(
                    "terraform init -reconfigure -backend-config={}/{}.tfvars",
                    backend_dir, environment
                ),
            ];

            // Add a terraform output command for each key
            for key in output_keys {
                commands.push(format!("terraform output {}", key));
            }

            Ok(commands)
        }
        _ => Ok(build_terraform_commands(
            action,
            backend_dir,
            environment,
            options,
        )),
    }
}

/// Build the sequence of terraform commands for a given action
fn build_terraform_commands(
    action: &Action,
    backend_dir: &str,
    environment: &str,
    options: &[String],
) -> Vec<String> {
    // Common setup commands that all terraform operations need
    let mut commands = vec![
        "tfswitch".to_string(),
        format!(
            "terraform init -reconfigure -backend-config={}/{}.tfvars",
            backend_dir, environment
        ),
    ];

    // Build options string
    let opts = if options.is_empty() {
        String::new()
    } else {
        format!(" {}", options.join(" "))
    };

    // Build the terraform operation command based on action
    let operation = match action {
        Action::Apply => format!(
            "terraform apply -lock-timeout=60s -var-file=tfvars/{}.tfvars{}",
            environment, opts
        ),
        Action::Check | Action::Plan | Action::Diff => format!(
            "terraform plan -lock-timeout=60s -var-file=tfvars/{}.tfvars{}",
            environment, opts
        ),
        Action::Delete | Action::Destroy | Action::Uninstall => format!(
            "terraform destroy -lock-timeout=60s -var-file=tfvars/{}.tfvars{}",
            environment, opts
        ),
        Action::Output { key } => {
            // When key is provided, output that specific key
            // When key is None (--all flag), we'll handle it separately
            // to call terraform output for each key individually
            if let Some(k) = key {
                format!("terraform output {}", k)
            } else {
                // Return empty string as a marker - we'll handle --all differently
                String::new()
            }
        }
        Action::Unlock { lock_id } => format!("terraform force-unlock -force {}", lock_id),
        Action::Show => "terraform show".to_string(),
        _ => {
            // For unsupported actions, return just the init commands
            return commands;
        }
    };

    commands.push(operation);
    commands
}

/// Get all output keys from terraform files in the project
fn get_output_keys(project_path: &str) -> Result<Vec<String>> {
    use ignore::WalkBuilder;
    use regex::Regex;
    use std::collections::HashSet;

    let output_regex = Regex::new(r#"^output\s+"([^"]+)"\s+\{"#)?;
    let mut output_keys = HashSet::new();

    // Walk through the project directory looking for .tf files
    for entry in WalkBuilder::new(project_path)
        .max_depth(Some(3))
        .build()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            if let Some(ext) = entry.path().extension() {
                if ext == "tf" {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        for line in content.lines() {
                            if let Some(captures) = output_regex.captures(line) {
                                if let Some(key) = captures.get(1) {
                                    output_keys.insert(key.as_str().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Convert to sorted Vec for consistent ordering
    let mut keys: Vec<String> = output_keys.into_iter().collect();
    keys.sort();
    Ok(keys)
}

/// Duplicate terraform environment configuration
fn duplicate_terraform_env(project_path: &str, source_env: &str, target_env: &str) -> Result<()> {
    let path = Path::new(project_path);

    // Determine backend directory name
    let backend_dir = if path.join("backend-vars").exists() {
        "backend-vars"
    } else {
        "backend_vars"
    };

    // Copy backend-vars file
    let backend_source = path
        .join(backend_dir)
        .join(format!("{}.tfvars", source_env));
    let backend_target = path
        .join(backend_dir)
        .join(format!("{}.tfvars", target_env));

    if backend_source.exists() {
        let content = fs::read_to_string(&backend_source)?;
        let updated_content = content.replace(source_env, target_env);
        fs::write(&backend_target, updated_content)?;
    }

    // Copy tfvars file
    let tfvars_source = path.join("tfvars").join(format!("{}.tfvars", source_env));
    let tfvars_target = path.join("tfvars").join(format!("{}.tfvars", target_env));

    if tfvars_source.exists() {
        let content = fs::read_to_string(&tfvars_source)?;
        let updated_content = content.replace(source_env, target_env);
        fs::write(&tfvars_target, updated_content)?;
    }

    // Walk through project directory and update any .tfvars files with target_env name
    for entry in ignore::WalkBuilder::new(path)
        .build()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name == format!("{}.tfvars", target_env) {
                    let content = fs::read_to_string(entry.path())?;
                    let updated_content = content.replace(source_env, target_env);
                    fs::write(entry.path(), updated_content)?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_terraform_commands_apply() {
        let commands = build_terraform_commands(
            &Action::Apply,
            "backend-vars",
            "dev",
            &["-auto-approve".to_string()],
        );

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "tfswitch");
        assert_eq!(
            commands[1],
            "terraform init -reconfigure -backend-config=backend-vars/dev.tfvars"
        );
        assert_eq!(
            commands[2],
            "terraform apply -lock-timeout=60s -var-file=tfvars/dev.tfvars -auto-approve"
        );
    }

    #[test]
    fn test_build_terraform_commands_plan() {
        let commands = build_terraform_commands(&Action::Plan, "backend_vars", "prod", &[]);

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "tfswitch");
        assert_eq!(
            commands[1],
            "terraform init -reconfigure -backend-config=backend_vars/prod.tfvars"
        );
        assert_eq!(
            commands[2],
            "terraform plan -lock-timeout=60s -var-file=tfvars/prod.tfvars"
        );
    }

    #[test]
    fn test_build_terraform_commands_destroy() {
        let commands = build_terraform_commands(&Action::Destroy, "backend-vars", "staging", &[]);

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "tfswitch");
        assert_eq!(
            commands[1],
            "terraform init -reconfigure -backend-config=backend-vars/staging.tfvars"
        );
        assert_eq!(
            commands[2],
            "terraform destroy -lock-timeout=60s -var-file=tfvars/staging.tfvars"
        );
    }

    #[test]
    fn test_build_terraform_commands_output() {
        let commands = build_terraform_commands(
            &Action::Output {
                key: Some("vpc_id".to_string()),
            },
            "backend-vars",
            "dev",
            &[],
        );

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "tfswitch");
        assert_eq!(
            commands[1],
            "terraform init -reconfigure -backend-config=backend-vars/dev.tfvars"
        );
        assert_eq!(commands[2], "terraform output vpc_id");
    }

    #[test]
    fn test_build_terraform_commands_with_multiple_options() {
        let commands = build_terraform_commands(
            &Action::Apply,
            "backend-vars",
            "dev",
            &["-auto-approve".to_string(), "-compact-warnings".to_string()],
        );

        assert_eq!(commands.len(), 3);
        assert_eq!(
            commands[2],
            "terraform apply -lock-timeout=60s -var-file=tfvars/dev.tfvars -auto-approve -compact-warnings"
        );
    }

    #[test]
    fn test_build_terraform_commands_check_same_as_plan() {
        let commands_check = build_terraform_commands(&Action::Check, "backend-vars", "dev", &[]);
        let commands_plan = build_terraform_commands(&Action::Plan, "backend-vars", "dev", &[]);

        assert_eq!(commands_check, commands_plan);
    }

    #[test]
    fn test_build_terraform_commands_diff_same_as_plan() {
        let commands_diff = build_terraform_commands(&Action::Diff, "backend-vars", "dev", &[]);
        let commands_plan = build_terraform_commands(&Action::Plan, "backend-vars", "dev", &[]);

        assert_eq!(commands_diff, commands_plan);
    }

    #[test]
    fn test_build_terraform_commands_show() {
        let commands = build_terraform_commands(&Action::Show, "backend-vars", "dev", &[]);

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "tfswitch");
        assert_eq!(
            commands[1],
            "terraform init -reconfigure -backend-config=backend-vars/dev.tfvars"
        );
        assert_eq!(commands[2], "terraform show");
    }
}
