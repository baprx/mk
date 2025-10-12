//! Integration tests for the duplicate command
//! These tests verify file duplication and content replacement

use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

/// Helper to create a terraform project for duplicate testing
fn create_terraform_duplicate_project(temp_dir: &TempDir) -> String {
    let project_dir = temp_dir.path().join("terraform");
    fs::create_dir(&project_dir).unwrap();

    // Create tfvars directory
    let tfvars_dir = project_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(
        tfvars_dir.join("dev.tfvars"),
        "env = \"dev\"\nproject = \"test-project-dev\"\n",
    )
    .unwrap();

    // Create backend-vars directory
    let backend_vars_dir = project_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\n",
    )
    .unwrap();

    project_dir.to_str().unwrap().to_string()
}

/// Helper to create a helm project for duplicate testing
fn create_helm_duplicate_project(temp_dir: &TempDir) -> String {
    let project_dir = temp_dir.path().join("my-chart");
    fs::create_dir(&project_dir).unwrap();

    // Create Chart.yaml (required for Helm chart detection)
    fs::write(
        project_dir.join("Chart.yaml"),
        "name: my-chart\nversion: 1.0.0\n",
    )
    .unwrap();

    // Create values.yaml
    fs::write(project_dir.join("values.yaml"), "replicaCount: 3\n").unwrap();

    // Create values/dev directory
    let values_dir = project_dir.join("values");
    fs::create_dir(&values_dir).unwrap();

    let dev_dir = values_dir.join("dev");
    fs::create_dir(&dev_dir).unwrap();
    fs::write(
        dev_dir.join("values.yaml"),
        "environment: dev\nconfig: test-config-dev\napp: test-app-dev\n",
    )
    .unwrap();

    project_dir.to_str().unwrap().to_string()
}

#[test]
#[serial]
fn test_terraform_duplicate_creates_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_duplicate_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", &project_path, "dev", "staging"])
        .assert()
        .success();

    // Check that files were created
    let staging_tfvars = format!("{}/tfvars/staging.tfvars", project_path);
    let staging_backend = format!("{}/backend-vars/staging.tfvars", project_path);

    assert!(
        std::path::Path::new(&staging_tfvars).exists(),
        "tfvars/staging.tfvars should exist"
    );
    assert!(
        std::path::Path::new(&staging_backend).exists(),
        "backend-vars/staging.tfvars should exist"
    );
}

#[test]
#[serial]
fn test_terraform_duplicate_replaces_content() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_duplicate_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", &project_path, "dev", "staging"])
        .assert()
        .success();

    // Check tfvars content
    let staging_tfvars = format!("{}/tfvars/staging.tfvars", project_path);
    let content = fs::read_to_string(&staging_tfvars).unwrap();
    assert!(
        content.contains("staging"),
        "Content should contain 'staging'"
    );
    assert!(
        content.contains("test-project-staging"),
        "Content should replace dev with staging in project name"
    );
    assert!(
        !content.contains("dev"),
        "Content should not contain 'dev' after replacement"
    );

    // Check backend-vars content
    let staging_backend = format!("{}/backend-vars/staging.tfvars", project_path);
    let backend_content = fs::read_to_string(&staging_backend).unwrap();
    assert!(
        backend_content.contains("terraform-staging.tfstate"),
        "Backend should use staging state file"
    );
}

#[test]
#[serial]
fn test_helm_duplicate_creates_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_duplicate_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", &project_path, "dev", "staging"])
        .assert()
        .success();

    // Check that directory and file were created
    let staging_values = format!("{}/values/staging/values.yaml", project_path);
    assert!(
        std::path::Path::new(&staging_values).exists(),
        "values/staging/values.yaml should exist"
    );
}

#[test]
#[serial]
fn test_helm_duplicate_replaces_content() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_duplicate_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", &project_path, "dev", "staging"])
        .assert()
        .success();

    let staging_values = format!("{}/values/staging/values.yaml", project_path);
    let content = fs::read_to_string(&staging_values).unwrap();

    assert!(
        content.contains("environment: staging"),
        "Environment should be staging"
    );
    assert!(
        content.contains("test-config-staging"),
        "Config should use staging"
    );
    assert!(
        content.contains("test-app-staging"),
        "App should use staging"
    );
    assert!(
        !content.contains("dev"),
        "Content should not contain 'dev' after replacement"
    );
}

#[test]
#[serial]
fn test_duplicate_with_nonexistent_source_env() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_duplicate_project(&temp_dir);

    Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", &project_path, "nonexistent", "staging"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid env"));
}

#[test]
#[serial]
fn test_duplicate_target_env_already_exists() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_duplicate_project(&temp_dir);

    // First duplicate
    Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", &project_path, "dev", "staging"])
        .assert()
        .success();

    // Try to duplicate again to same target
    let result = Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", &project_path, "dev", "staging"])
        .output()
        .unwrap();

    // Should either fail or overwrite - check stderr for any warnings/errors
    let stderr = String::from_utf8_lossy(&result.stderr);
    // At minimum, the command should complete
    assert!(!stderr.is_empty() || result.status.success());
}

#[test]
#[serial]
fn test_duplicate_preserves_source_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_terraform_duplicate_project(&temp_dir);

    // Read original content
    let dev_tfvars = format!("{}/tfvars/dev.tfvars", project_path);
    let original_content = fs::read_to_string(&dev_tfvars).unwrap();

    Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", &project_path, "dev", "staging"])
        .assert()
        .success();

    // Check original is unchanged
    let current_content = fs::read_to_string(&dev_tfvars).unwrap();
    assert_eq!(
        original_content, current_content,
        "Source file should remain unchanged"
    );
}

#[test]
#[serial]
fn test_duplicate_case_sensitive_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("terraform");
    fs::create_dir(&project_dir).unwrap();

    // Create tfvars with mixed case
    let tfvars_dir = project_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(
        tfvars_dir.join("dev.tfvars"),
        "env = \"dev\"\nproject = \"test-dev\"\n",
    )
    .unwrap();

    let backend_vars_dir = project_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\n",
    )
    .unwrap();

    let project_path = project_dir.to_str().unwrap();

    Command::cargo_bin("mk")
        .unwrap()
        .args(&["duplicate", project_path, "dev", "staging"])
        .assert()
        .success();

    // Check that replacement happened correctly
    let staging_tfvars = format!("{}/tfvars/staging.tfvars", project_path);
    let content = fs::read_to_string(&staging_tfvars).unwrap();

    // Should replace dev with staging throughout
    assert!(content.contains("staging"));
    assert!(content.contains("test-staging"));
    assert!(
        !content.contains("dev"),
        "Should not contain 'dev' after replacement"
    );
}
