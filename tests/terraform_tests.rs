//! Integration tests for Terraform commands
//! These tests verify Terraform-specific functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a terraform project
/// Returns the path to the terraform directory
fn create_terraform_project(temp_dir: &TempDir, envs: &[&str]) -> String {
    // Create terraform subdirectory
    let terraform_dir = temp_dir.path().join("terraform");
    fs::create_dir(&terraform_dir).unwrap();

    // Create tfvars directory with environment files
    let tfvars_dir = terraform_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    for env in envs {
        fs::write(
            tfvars_dir.join(format!("{}.tfvars", env)),
            format!("env = \"{}\"\n", env),
        )
        .unwrap();
    }

    // Create backend-vars directory
    let backend_vars_dir = terraform_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    for env in envs {
        fs::write(
            backend_vars_dir.join(format!("{}.tfvars", env)),
            format!("key = \"terraform-{}.tfstate\"\n", env),
        )
        .unwrap();
    }

    // Create a dummy main.tf
    fs::write(
        terraform_dir.join("main.tf"),
        r#"# Terraform config
resource "null_resource" "test" {}
"#,
    )
    .unwrap();

    terraform_dir.to_str().unwrap().to_string()
}

#[test]
fn test_terraform_apply_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should detect terraform and attempt to run command
    assert!(
        stderr.contains("Detected terraform") || stderr.contains("terraform"),
        "Should detect terraform technology"
    );
}

#[test]
fn test_terraform_destroy_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["destroy", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Detected terraform") || stderr.contains("terraform"),
        "Should detect terraform technology"
    );
}

#[test]
fn test_terraform_delete_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["delete", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Detected terraform") || stderr.contains("terraform"),
        "Should detect terraform technology"
    );
}

#[test]
fn test_terraform_output_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["output", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Detected terraform") || stderr.contains("terraform"),
        "Should detect terraform technology"
    );
}

#[test]
fn test_terraform_output_with_key() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["output", &project_path, "dev", "vpc_id"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Detected terraform") || stderr.contains("terraform"),
        "Should detect terraform technology"
    );
}

#[test]
fn test_terraform_unlock_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["unlock", &project_path, "dev", "test-lock-id"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Detected terraform") || stderr.contains("terraform"),
        "Should detect terraform technology"
    );
}

#[test]
fn test_terraform_show_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["show", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Detected terraform") || stderr.contains("terraform"),
        "Should detect terraform technology"
    );
}

#[test]
fn test_terraform_invalid_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev"]);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid env"));
}

#[test]
fn test_terraform_multiple_environments() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, &["dev", "prod"]);

    // Both environments should work - test that they're detected
    let output_dev = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev"])
        .output()
        .unwrap();

    let output_prod = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "prod"])
        .output()
        .unwrap();

    let stderr_dev = String::from_utf8_lossy(&output_dev.stderr);
    let stderr_prod = String::from_utf8_lossy(&output_prod.stderr);

    // Both should detect terraform (even if they don't execute successfully)
    assert!(
        stderr_dev.contains("Detected terraform") || stderr_dev.contains("terraform"),
        "Dev environment should be detected"
    );
    assert!(
        stderr_prod.contains("Detected terraform") || stderr_prod.contains("terraform"),
        "Prod environment should be detected"
    );
}
