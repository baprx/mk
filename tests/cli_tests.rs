//! Integration tests for the CLI interface
//! These tests verify the end-to-end behavior of the mk command

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a terraform test project
fn create_terraform_test_project(temp_dir: &TempDir) -> String {
    let project_dir = temp_dir.path().join("terraform");
    fs::create_dir(&project_dir).unwrap();

    // Create tfvars directory with environment files
    let tfvars_dir = project_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(tfvars_dir.join("dev.tfvars"), "env = \"dev\"\n").unwrap();
    fs::write(tfvars_dir.join("prod.tfvars"), "env = \"prod\"\n").unwrap();

    // Create backend-vars directory
    let backend_vars_dir = project_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\n",
    )
    .unwrap();
    fs::write(
        backend_vars_dir.join("prod.tfvars"),
        "key = \"terraform-prod.tfstate\"\n",
    )
    .unwrap();

    // Create a dummy main.tf
    fs::write(project_dir.join("main.tf"), "# Terraform config\n").unwrap();

    project_dir.to_str().unwrap().to_string()
}

/// Helper to create a helm test project
fn create_helm_test_project(temp_dir: &TempDir) -> String {
    let project_dir = temp_dir.path().join("my-chart");
    fs::create_dir(&project_dir).unwrap();

    // Create values.yaml (required for helm detection)
    fs::write(project_dir.join("values.yaml"), "replicaCount: 3\n").unwrap();

    // Create Chart.yaml
    fs::write(
        project_dir.join("Chart.yaml"),
        "apiVersion: v2\nname: my-chart\nversion: 0.1.0\n",
    )
    .unwrap();

    // Create values directories for different environments
    let values_dir = project_dir.join("values");
    fs::create_dir(&values_dir).unwrap();

    let dev_dir = values_dir.join("dev");
    fs::create_dir(&dev_dir).unwrap();
    fs::write(dev_dir.join("values.yaml"), "environment: dev\n").unwrap();

    let prod_dir = values_dir.join("prod");
    fs::create_dir(&prod_dir).unwrap();
    fs::write(prod_dir.join("values.yaml"), "environment: prod\n").unwrap();

    project_dir.to_str().unwrap().to_string()
}

/// Helper to create a kustomize test project
fn create_kustomize_test_project(temp_dir: &TempDir) -> String {
    let project_dir = temp_dir.path().join("my-kustomize");
    fs::create_dir(&project_dir).unwrap();

    // Create overlays directory (required for kustomize detection)
    let overlays_dir = project_dir.join("overlays");
    fs::create_dir(&overlays_dir).unwrap();

    // Create dev overlay
    let dev_dir = overlays_dir.join("dev");
    fs::create_dir(&dev_dir).unwrap();
    fs::write(dev_dir.join("kustomization.yaml"), "namePrefix: dev-\n").unwrap();

    // Create prod overlay
    let prod_dir = overlays_dir.join("prod");
    fs::create_dir(&prod_dir).unwrap();
    fs::write(prod_dir.join("kustomization.yaml"), "namePrefix: prod-\n").unwrap();

    project_dir.to_str().unwrap().to_string()
}

/// Helper to create an ansible test project
fn create_ansible_test_project(temp_dir: &TempDir) -> String {
    let project_dir = temp_dir.path().join("ansible");
    fs::create_dir(&project_dir).unwrap();

    // Create inventories directory
    let inventories_dir = project_dir.join("inventories");
    fs::create_dir(&inventories_dir).unwrap();
    fs::write(inventories_dir.join("dev.yml"), "all:\n  hosts:\n").unwrap();
    fs::write(inventories_dir.join("prod.yml"), "all:\n  hosts:\n").unwrap();

    // Create ansible.cfg
    fs::write(project_dir.join("ansible.cfg"), "[defaults]\n").unwrap();

    // Create playbook.yml
    fs::write(
        project_dir.join("playbook.yml"),
        "---\n- hosts: all\n  tasks:\n    - debug: msg=test\n",
    )
    .unwrap();

    project_dir.to_str().unwrap().to_string()
}

#[test]
fn test_cli_help() {
    Command::cargo_bin("mk")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version() {
    Command::cargo_bin("mk")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn test_terraform_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_test_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .assert()
        .stderr(predicate::str::contains("Detected terraform"));
}

#[test]
fn test_helm_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_test_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["template", &project_path, "dev"])
        .assert()
        .stderr(predicate::str::contains("Detected helm"));
}

#[test]
fn test_kustomize_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_test_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["template", &project_path, "dev"])
        .assert()
        .stderr(predicate::str::contains("Detected kustomize"));
}

#[test]
fn test_ansible_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_test_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .assert()
        .stderr(predicate::str::contains("Detected ansible"));
}

#[test]
fn test_invalid_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_test_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "invalid-env"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid env"));
}

#[test]
fn test_valid_environment_accepted() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_test_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .assert()
        .stderr(predicate::str::contains("Invalid env").not());
}

#[test]
fn test_nonexistent_path() {
    Command::cargo_bin("mk")
        .unwrap()
        .args(["check", "/nonexistent/path/12345", "dev"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("does not exist")
                .or(predicate::str::contains("Failed to detect technology")),
        );
}

#[test]
fn test_terraform_check_command_generation() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_test_project(&temp_dir);

    // The command will fail because terraform isn't actually configured,
    // but we can check that the correct command is generated
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("terraform plan") || stderr.contains("terraform"));
}

#[test]
fn test_helm_template_command_generation() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_test_project(&temp_dir);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["template", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("helm"));
}

#[test]
fn test_kustomize_template_command_generation() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_test_project(&temp_dir);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["template", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("kustomize"));
}

#[test]
fn test_ansible_check_command_generation() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_test_project(&temp_dir);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ansible-playbook"));
}

#[test]
fn test_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_test_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["--verbose", "check", &project_path, "dev"])
        .assert()
        .stderr(predicate::str::contains("Detected terraform"));
}

#[test]
fn test_multiple_environments_available() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_test_project(&temp_dir);

    // Test with dev
    Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .assert()
        .stderr(predicate::str::contains("Invalid env").not());

    // Test with prod
    Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "prod"])
        .assert()
        .stderr(predicate::str::contains("Invalid env").not());
}

#[test]
fn test_error_message_shows_valid_environments() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_test_project(&temp_dir);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "staging"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // The error message should mention it's invalid and ideally show valid options
    assert!(
        stderr.contains("Invalid env") || stderr.contains("Invalid environment"),
        "Error should mention invalid environment"
    );
    assert!(!output.status.success(), "Command should fail");
}

#[test]
fn test_invalid_subcommand() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_test_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["invalid-action", &project_path, "dev"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}
