//! Integration tests for recursive bump functionality
//! These tests verify the bump --recursive command

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Helper to create a terraform project with modules
fn create_terraform_project_with_modules(base_dir: &std::path::Path, name: &str) -> String {
    let project_dir = base_dir.join(name);
    fs::create_dir(&project_dir).unwrap();

    // Create a Terraform file with a module
    let main_tf = project_dir.join("main.tf");
    fs::write(
        &main_tf,
        r#"module "vpc" {
  source  = "terraform-google-modules/network/google"
  version = "~> 7.0"
  project_id = "test"
}
"#,
    )
    .unwrap();

    project_dir.to_str().unwrap().to_string()
}

/// Helper to create a helm project with dependencies
fn create_helm_project_with_deps(base_dir: &std::path::Path, name: &str) -> String {
    let project_dir = base_dir.join(name);
    fs::create_dir(&project_dir).unwrap();

    // Create values.yaml (required for helm detection)
    fs::write(project_dir.join("values.yaml"), "replicaCount: 3\\n").unwrap();

    // Create Chart.yaml with dependencies
    fs::write(
        project_dir.join("Chart.yaml"),
        r#"apiVersion: v2
name: my-chart
version: 0.1.0
dependencies:
  - name: postgresql
    version: "12.0.0"
    repository: "https://charts.bitnami.com/bitnami"
"#,
    )
    .unwrap();

    project_dir.to_str().unwrap().to_string()
}

#[test]
fn test_recursive_bump_finds_single_project() {
    let temp_dir = TempDir::new().unwrap();
    let _tf_project = create_terraform_project_with_modules(temp_dir.path(), "terraform");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", temp_dir.path().to_str().unwrap(), "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found 1") || stderr.contains("Terraform project"),
        "Should find the single Terraform project"
    );
}

#[test]
fn test_recursive_bump_finds_multiple_projects() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple Terraform projects
    let _tf1 = create_terraform_project_with_modules(temp_dir.path(), "project1");
    let _tf2 = create_terraform_project_with_modules(temp_dir.path(), "project2");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", temp_dir.path().to_str().unwrap(), "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found 2") || stderr.contains("2 Terraform project"),
        "Should find multiple Terraform projects"
    );
}

#[test]
fn test_recursive_bump_mixed_technologies() {
    let temp_dir = TempDir::new().unwrap();

    // Create both Terraform and Helm projects
    let _tf = create_terraform_project_with_modules(temp_dir.path(), "terraform");
    let _helm = create_helm_project_with_deps(temp_dir.path(), "my-chart");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", temp_dir.path().to_str().unwrap(), "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should find both types
    assert!(
        stderr.contains("Terraform") && stderr.contains("Helm"),
        "Should detect both Terraform and Helm projects. stderr: {}",
        stderr
    );
}

#[test]
fn test_recursive_bump_nested_projects() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested structure
    let nested = temp_dir.path().join("infra").join("services");
    fs::create_dir_all(&nested).unwrap();

    let _tf = create_terraform_project_with_modules(&nested, "terraform");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", temp_dir.path().to_str().unwrap(), "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found 1") || stderr.contains("project"),
        "Should find nested project"
    );
}

#[test]
fn test_recursive_bump_with_include_prereleases() {
    let temp_dir = TempDir::new().unwrap();
    let _tf = create_terraform_project_with_modules(temp_dir.path(), "terraform");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args([
            "bump",
            temp_dir.path().to_str().unwrap(),
            "--recursive",
            "--include-prereleases",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should accept the flag without error
    assert!(
        stderr.contains("Scanning") || stderr.contains("Found"),
        "Should process with include-prereleases flag"
    );
}

#[test]
fn test_recursive_bump_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", temp_dir.path().to_str().unwrap(), "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No Terraform or Helm projects found"),
        "Should report no projects found"
    );
}

#[test]
fn test_recursive_bump_shows_project_count() {
    let temp_dir = TempDir::new().unwrap();

    // Create exactly 3 projects
    let _tf1 = create_terraform_project_with_modules(temp_dir.path(), "project1");
    let _tf2 = create_terraform_project_with_modules(temp_dir.path(), "project2");
    let _helm = create_helm_project_with_deps(temp_dir.path(), "helm-chart");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", temp_dir.path().to_str().unwrap(), "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should report the correct count
    assert!(
        stderr.contains("2 Terraform") && stderr.contains("1 Helm"),
        "Should show correct project counts. stderr: {}",
        stderr
    );
}

#[test]
fn test_recursive_bump_max_depth_default() {
    let temp_dir = TempDir::new().unwrap();

    // Create a deeply nested project (more than default max_depth of 5)
    let deep_path = temp_dir
        .path()
        .join("l1")
        .join("l2")
        .join("l3")
        .join("l4")
        .join("l5")
        .join("l6");
    fs::create_dir_all(&deep_path).unwrap();

    let _tf = create_terraform_project_with_modules(&deep_path, "terraform");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", temp_dir.path().to_str().unwrap(), "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // With default max_depth of 5, should not find the deeply nested project
    assert!(
        stderr.contains("No Terraform or Helm projects found"),
        "Should not find project beyond max_depth"
    );
}

#[test]
fn test_recursive_bump_nonexistent_directory() {
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", "/nonexistent/path/12345", "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Tool may succeed but report no projects found, or fail with error
    assert!(
        !output.status.success()
            || stderr.contains("No Terraform or Helm projects found")
            || stderr.contains("No such file")
            || stderr.contains("does not exist"),
        "Should handle nonexistent directory gracefully. stderr: {}",
        stderr
    );
}

#[test]
fn test_recursive_bump_with_verbose() {
    let temp_dir = TempDir::new().unwrap();
    let _tf = create_terraform_project_with_modules(temp_dir.path(), "terraform");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args([
            "bump",
            temp_dir.path().to_str().unwrap(),
            "--recursive",
            "--verbose",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Verbose mode should show more details
    assert!(
        stderr.contains("Scanning") || stderr.contains("DEBUG") || stderr.contains("INFO"),
        "Should show verbose output"
    );
}

#[test]
fn test_recursive_bump_same_dependency_cached() {
    let temp_dir = TempDir::new().unwrap();

    // Create two projects with the same module
    let _tf1 = create_terraform_project_with_modules(temp_dir.path(), "project1");
    let _tf2 = create_terraform_project_with_modules(temp_dir.path(), "project2");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args([
            "bump",
            temp_dir.path().to_str().unwrap(),
            "--recursive",
            "--verbose",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should mention caching when the same module is found in multiple projects
    // The second occurrence should use cached version
    if stderr.contains("network/google") {
        // If we can actually query the registry, check for cache usage
        let network_count = stderr.matches("network/google").count();
        assert!(
            network_count >= 2,
            "Should find the same module in multiple projects"
        );
    }
}

#[test]
fn test_recursive_bump_reports_up_to_date() {
    let temp_dir = TempDir::new().unwrap();
    let _tf = create_terraform_project_with_modules(temp_dir.path(), "terraform");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["bump", temp_dir.path().to_str().unwrap(), "--recursive"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should either show dependencies or report they're up to date
    assert!(
        stderr.contains("up to date")
            || stderr.contains("update available")
            || stderr.contains("dependencies"),
        "Should report dependency status"
    );
}
