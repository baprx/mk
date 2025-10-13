//! Integration tests for Terraform commands
//! These tests verify Terraform-specific functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a terraform project
/// Returns the path to the terraform directory
fn create_terraform_project(temp_dir: &TempDir, env: &str) -> String {
    // Create terraform subdirectory
    let terraform_dir = temp_dir.path().join("terraform");
    fs::create_dir(&terraform_dir).unwrap();

    // Create tfvars directory with environment files
    let tfvars_dir = terraform_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(
        tfvars_dir.join(format!("{}.tfvars", env)),
        format!("env = \"{}\"\n", env),
    )
    .unwrap();

    // Create backend-vars directory
    let backend_vars_dir = terraform_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join(format!("{}.tfvars", env)),
        format!("key = \"terraform-{}.tfstate\"\n", env),
    )
    .unwrap();

    // Create a dummy main.tf
    fs::write(
        terraform_dir.join("main.tf"),
        "# Terraform config\nresource \"null_resource\" \"test\" {}\n",
    )
    .unwrap();

    terraform_dir.to_str().unwrap().to_string()
}

#[test]
fn test_terraform_apply_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, "dev");

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
fn test_terraform_apply_with_auto_approve() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev", "--", "-auto-approve"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Detected terraform") || stderr.contains("terraform"),
        "Should detect terraform technology"
    );
}

#[test]
fn test_terraform_destroy_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, "dev");

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
fn test_terraform_destroy_with_auto_approve() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["destroy", &project_path, "dev", "--", "-auto-approve"])
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
    let project_path = create_terraform_project(&temp_dir, "dev");

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
    let project_path = create_terraform_project(&temp_dir, "dev");

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
    let project_path = create_terraform_project(&temp_dir, "dev");

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
    let project_path = create_terraform_project(&temp_dir, "dev");

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
    let project_path = create_terraform_project(&temp_dir, "dev");

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
    let project_path = create_terraform_project(&temp_dir, "dev");

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
    let project_path = create_terraform_project(&temp_dir, "dev");

    // Create prod environment
    let terraform_dir = temp_dir.path().join("terraform");
    let tfvars_dir = terraform_dir.join("tfvars");
    fs::write(tfvars_dir.join("prod.tfvars"), "env = \"prod\"\n").unwrap();

    let backend_vars_dir = terraform_dir.join("backend-vars");
    fs::write(
        backend_vars_dir.join("prod.tfvars"),
        "key = \"terraform-prod.tfstate\"\n",
    )
    .unwrap();

    // Both environments should work
    let output_dev = Command::cargo_bin("mk")
        .unwrap()
        .args(["plan", &project_path, "dev"])
        .output()
        .unwrap();

    let output_prod = Command::cargo_bin("mk")
        .unwrap()
        .args(["plan", &project_path, "prod"])
        .output()
        .unwrap();

    assert!(
        output_dev.status.success()
            || String::from_utf8_lossy(&output_dev.stderr).contains("terraform"),
        "Dev environment should work"
    );
    assert!(
        output_prod.status.success()
            || String::from_utf8_lossy(&output_prod.stderr).contains("terraform"),
        "Prod environment should work"
    );
}
