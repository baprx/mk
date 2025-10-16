use anyhow::{Context, Result};
use ignore::WalkBuilder;
use regex::Regex;
use std::fs;

use super::registry;
use super::{Dependency, DependencyType};

/// Scan Terraform files for module dependencies
pub fn scan_terraform_modules(
    project_path: &str,
    verbose: bool,
    include_prereleases: bool,
) -> Result<Vec<Dependency>> {
    let mut dependencies = Vec::new();

    // Walk through .tf files in the project, respecting .gitignore
    for entry in WalkBuilder::new(project_path)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && e.path().extension().map(|ext| ext == "tf").unwrap_or(false)
        })
    {
        let file_path = entry.path();
        let full_path = file_path.to_string_lossy().to_string();
        let relative_path = file_path
            .strip_prefix(project_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        if verbose {
            eprintln!("  Scanning: {}", relative_path);
        }

        let content =
            fs::read_to_string(file_path).context(format!("Failed to read {}", relative_path))?;

        // Parse modules from the file - pass both full path and relative path
        let file_deps =
            parse_terraform_modules(&content, &full_path, verbose, include_prereleases)?;
        dependencies.extend(file_deps);
    }

    Ok(dependencies)
}

/// Parse Terraform module blocks and fetch latest versions
fn parse_terraform_modules(
    content: &str,
    full_path: &str,
    verbose: bool,
    include_prereleases: bool,
) -> Result<Vec<Dependency>> {
    let mut dependencies = Vec::new();

    // Regex to match module blocks with registry sources
    // Example: source = "terraform-google-modules/cloud-nat/google"
    // Matches content up to a line starting with } (module closing brace)
    let module_regex = Regex::new(r#"(?ms)module\s+"([^"]+)"\s*\{(.*?)^\}$"#).unwrap();

    // Extract source and version from the module block
    let source_regex = Regex::new(r#"source\s*=\s*"([^"]+)""#).unwrap();
    let version_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();

    for cap in module_regex.captures_iter(content) {
        let module_name = cap.get(1).unwrap().as_str();
        let module_block = cap.get(2).unwrap().as_str();

        let source = match source_regex.captures(module_block) {
            Some(cap) => cap.get(1).unwrap().as_str(),
            None => continue, // Skip if no source found
        };

        let version_constraint = match version_regex.captures(module_block) {
            Some(cap) => cap.get(1).unwrap().as_str(),
            None => continue, // Skip if no version found
        };

        // Only handle Terraform Registry modules (format: namespace/name/provider)
        if source.contains('/') && !source.starts_with("git::") && !source.starts_with("./") {
            // Handle submodules: split on '//' and use only the first part for registry lookup
            let registry_source = source.split("//").next().unwrap_or(source);

            let parts: Vec<&str> = registry_source.split('/').collect();
            if parts.len() == 3 {
                let namespace = parts[0];
                let name = parts[1];
                let provider = parts[2];

                // Extract current version from constraint
                let current_version = extract_version_from_constraint(version_constraint);

                if verbose {
                    eprintln!(
                        "  Found module: {} ({}), current: {}",
                        module_name, source, current_version
                    );
                }

                // Fetch latest version
                match registry::fetch_terraform_module_version(
                    namespace,
                    name,
                    provider,
                    verbose,
                    include_prereleases,
                ) {
                    Ok(latest_version) => {
                        // Find line number
                        let line_number = content
                            .lines()
                            .enumerate()
                            .find(|(_, line)| {
                                line.contains(&format!(r#"module "{}""#, module_name))
                            })
                            .map(|(i, _)| i + 1)
                            .unwrap_or(1);

                        dependencies.push(Dependency {
                            name: module_name.to_string(),
                            current_version: current_version.clone(),
                            latest_version,
                            latest_app_version: None, // Terraform modules don't have appVersion
                            file_path: full_path.to_string(),
                            line_number,
                            dep_type: DependencyType::TerraformModule {
                                source: source.to_string(),
                                constraint: version_constraint.to_string(),
                            },
                        });
                    }
                    Err(e) => {
                        if verbose {
                            eprintln!("  Warning: Failed to fetch version for {}: {}", source, e);
                        }
                    }
                }
            }
        }
    }

    Ok(dependencies)
}

/// Extract the actual version number from a version constraint
/// Examples: "~> 5.0" -> "5.0", ">= 1.2.3" -> "1.2.3", "1.0.0" -> "1.0.0"
pub(crate) fn extract_version_from_constraint(constraint: &str) -> String {
    let version_regex = Regex::new(r"[0-9]+\.[0-9]+(?:\.[0-9]+)?").unwrap();
    version_regex
        .find(constraint)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| constraint.to_string())
}

/// Update a Terraform module version in a file
pub fn update_terraform_module(
    file_path: &str,
    source: &str,
    old_constraint: &str,
    new_version: &str,
) -> Result<()> {
    let content = fs::read_to_string(file_path).context(format!("Failed to read {}", file_path))?;

    // Preserve the constraint operator (e.g., ~>, >=)
    let new_constraint = if old_constraint.starts_with("~>") {
        format!("~> {}", new_version)
    } else if old_constraint.starts_with(">=") {
        format!(">= {}", new_version)
    } else if old_constraint.starts_with(">") {
        format!("> {}", new_version)
    } else {
        new_version.to_string()
    };

    // Find and replace the version constraint
    // We need to be careful to only replace within the correct module block
    let module_regex = Regex::new(&format!(
        r#"(?m)(module\s+"[^"]+"\s*\{{[^}}]*?source\s*=\s*"{}"[^}}]*?version\s*=\s*")([^"]*)(")"#,
        regex::escape(source)
    ))
    .context("Failed to create regex")?;

    let updated_content = module_regex.replace(&content, |caps: &regex::Captures| {
        format!("{}{}{}", &caps[1], new_constraint, &caps[3])
    });

    fs::write(file_path, updated_content.as_ref())
        .context(format!("Failed to write {}", file_path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_version_from_constraint_exact() {
        assert_eq!(extract_version_from_constraint("1.0.0"), "1.0.0");
        assert_eq!(extract_version_from_constraint("2.5.3"), "2.5.3");
    }

    #[test]
    fn test_extract_version_from_constraint_pessimistic() {
        assert_eq!(extract_version_from_constraint("~> 5.0"), "5.0");
        assert_eq!(extract_version_from_constraint("~> 7.0"), "7.0");
        assert_eq!(extract_version_from_constraint("~> 1.2.3"), "1.2.3");
    }

    #[test]
    fn test_extract_version_from_constraint_gte() {
        assert_eq!(extract_version_from_constraint(">= 4.0.0"), "4.0.0");
        assert_eq!(extract_version_from_constraint(">= 1.2"), "1.2");
    }

    #[test]
    fn test_extract_version_from_constraint_gt() {
        assert_eq!(extract_version_from_constraint("> 3.0"), "3.0");
        assert_eq!(extract_version_from_constraint("> 2.5.1"), "2.5.1");
    }

    #[test]
    fn test_update_terraform_module_preserves_constraint_operator() {
        let temp_dir = TempDir::new().unwrap();
        let tf_file = temp_dir.path().join("test.tf");

        // Test with ~> operator
        fs::write(
            &tf_file,
            r#"
module "vpc" {
  source  = "terraform-google-modules/network/google"
  version = "~> 7.0"
  project_id = "test"
}
"#,
        )
        .unwrap();

        let result = update_terraform_module(
            tf_file.to_str().unwrap(),
            "terraform-google-modules/network/google",
            "~> 7.0",
            "8.0.0",
        );

        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tf_file).unwrap();
        assert!(updated_content.contains(r#"version = "~> 8.0.0""#));

        // Test with >= operator
        fs::write(
            &tf_file,
            r#"
module "nat" {
  source  = "terraform-google-modules/cloud-nat/google"
  version = ">= 4.0.0"
  project_id = "test"
}
"#,
        )
        .unwrap();

        let result = update_terraform_module(
            tf_file.to_str().unwrap(),
            "terraform-google-modules/cloud-nat/google",
            ">= 4.0.0",
            "5.0.0",
        );

        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tf_file).unwrap();
        assert!(updated_content.contains(r#"version = ">= 5.0.0""#));
    }

    #[test]
    fn test_update_terraform_module_exact_version() {
        let temp_dir = TempDir::new().unwrap();
        let tf_file = temp_dir.path().join("test.tf");

        fs::write(
            &tf_file,
            r#"
module "bastion" {
  source  = "terraform-google-modules/bastion-host/google"
  version = "5.0.0"
  project_id = "test"
}
"#,
        )
        .unwrap();

        let result = update_terraform_module(
            tf_file.to_str().unwrap(),
            "terraform-google-modules/bastion-host/google",
            "5.0.0",
            "6.0.0",
        );

        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tf_file).unwrap();
        assert!(updated_content.contains(r#"version = "6.0.0""#));
    }
}
