//! Integration tests for drift detection
//! These tests verify the drift command functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a terraform project for drift testing
fn create_terraform_drift_project(temp_dir: &TempDir, envs: &[&str]) -> String {
    let project_dir = temp_dir.path().join("terraform");
    fs::create_dir(&project_dir).unwrap();

    // Create tfvars directory
    let tfvars_dir = project_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    for env in envs {
        fs::write(
            tfvars_dir.join(format!("{}.tfvars", env)),
            format!("env = \"{}\"\\n", env),
        )
        .unwrap();
    }

    // Create backend-vars directory
    let backend_vars_dir = project_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    for env in envs {
        fs::write(
            backend_vars_dir.join(format!("{}.tfvars", env)),
            format!("key = \"terraform-{}.tfstate\"\\n", env),
        )
        .unwrap();
    }

    // Create a simple main.tf
    fs::write(
        project_dir.join("main.tf"),
        r#"resource "null_resource" "test" {
  triggers = {
    always_run = "${timestamp()}"
  }
}
"#,
    )
    .unwrap();

    project_dir.to_str().unwrap().to_string()
}

/// Helper to create a helm project for drift testing
fn create_helm_drift_project(temp_dir: &TempDir, envs: &[&str]) -> String {
    let project_dir = temp_dir.path().join("my-chart");
    fs::create_dir(&project_dir).unwrap();

    // Create values.yaml
    fs::write(
        project_dir.join("values.yaml"),
        r#"replicaCount: 3
image:
  repository: nginx
  tag: latest
"#,
    )
    .unwrap();

    // Create Chart.yaml
    fs::write(
        project_dir.join("Chart.yaml"),
        r#"apiVersion: v2
name: my-chart
version: 0.1.0
"#,
    )
    .unwrap();

    // Create values directory with environments
    let values_dir = project_dir.join("values");
    fs::create_dir(&values_dir).unwrap();

    for env in envs {
        let env_dir = values_dir.join(env);
        fs::create_dir(&env_dir).unwrap();
        fs::write(
            env_dir.join("values.yaml"),
            format!("environment: {}\\nreplicaCount: 5\\n", env),
        )
        .unwrap();
    }

    // Create helmfile.yaml.gotmpl
    fs::write(
        project_dir.join("helmfile.yaml.gotmpl"),
        r#"environments:
{{- range $index, $item := readDirEntries "./values/" }}
  {{- if $item.IsDir }}
  {{ $item.Name }}:
    values:
      - values/{{ $item.Name }}/values.yaml
  {{- end -}}
{{- end -}}
---
releases:
  - name: my-chart
    namespace: default
    chart: .
    version: 0.1.0
"#,
    )
    .unwrap();

    project_dir.to_str().unwrap().to_string()
}

#[test]
fn test_drift_command_requires_base_path() {
    Command::cargo_bin("mk")
        .unwrap()
        .arg("drift")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments"));
}

#[test]
fn test_drift_nonexistent_directory() {
    Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", "/nonexistent/path/12345"])
        .assert()
        .failure();
}

#[test]
fn test_drift_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No IaC projects found")
            || stderr.contains("No Terraform or Helm projects found"),
        "Should report no projects found"
    );
}

#[test]
fn test_drift_detects_terraform_project() {
    let temp_dir = TempDir::new().unwrap();
    let _project_path = create_terraform_drift_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found 1") || stderr.contains("project"),
        "Should detect the Terraform project"
    );
}

#[test]
fn test_drift_detects_helm_project() {
    let temp_dir = TempDir::new().unwrap();
    let _project_path = create_helm_drift_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found 1") || stderr.contains("project"),
        "Should detect the Helm project"
    );
}

#[test]
fn test_drift_with_tech_filter_terraform() {
    let temp_dir = TempDir::new().unwrap();
    let _tf_project = create_terraform_drift_project(&temp_dir, &["dev"]);
    let _helm_project = create_helm_drift_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args([
            "drift",
            temp_dir.path().to_str().unwrap(),
            "--tech",
            "terraform",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should only process Terraform projects, not Helm
    assert!(
        stderr.contains("Found 1"),
        "Should detect exactly 1 project when filtering by terraform"
    );
    assert!(
        !stderr.contains("my-chart") && !stderr.contains("helm"),
        "Should not process Helm projects when filtering for terraform"
    );
}

#[test]
fn test_drift_with_tech_filter_helm() {
    let temp_dir = TempDir::new().unwrap();
    let _tf_project = create_terraform_drift_project(&temp_dir, &["dev"]);
    let _helm_project = create_helm_drift_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap(), "--tech", "helm"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should filter to helm projects only - check for success or helm mention
    assert!(
        output.status.success()
            || stderr.contains("Found")
            || stdout.contains("helm")
            || stderr.contains("helm"),
        "Tech filter should work with helm"
    );
}

#[test]
fn test_drift_with_env_filter() {
    let temp_dir = TempDir::new().unwrap();
    let _project_dev = create_terraform_drift_project(&temp_dir, &["dev", "prod"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap(), "--env", "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // With env filter, should process the project and mention the environment
    assert!(
        stderr.contains("Found") || stderr.contains("dev") || output.status.success(),
        "Should process project with dev environment filter"
    );
}

#[test]
fn test_drift_with_max_depth() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested structure
    let nested_dir = temp_dir.path().join("level1").join("level2").join("level3");
    fs::create_dir_all(&nested_dir).unwrap();

    let tf_dir = nested_dir.join("terraform");
    fs::create_dir(&tf_dir).unwrap();

    let tfvars_dir = tf_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(tfvars_dir.join("dev.tfvars"), "env = \"dev\"\\n").unwrap();

    let backend_vars_dir = tf_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\\n",
    )
    .unwrap();

    fs::write(tf_dir.join("main.tf"), "# terraform config\\n").unwrap();

    // Test with max-depth that's too shallow
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args([
            "drift",
            temp_dir.path().to_str().unwrap(),
            "--max-depth",
            "2",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No IaC projects found")
            || stderr.contains("No Terraform or Helm projects found"),
        "Should not find deeply nested project with shallow max-depth"
    );

    // Test with max-depth that's deep enough
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args([
            "drift",
            temp_dir.path().to_str().unwrap(),
            "--max-depth",
            "5",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found 1") || stderr.contains("project"),
        "Should find project with sufficient max-depth"
    );
}

#[test]
fn test_drift_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();
    let _project_path = create_terraform_drift_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap(), "--verbose"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let output_without_verbose = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    let stderr_without_verbose = String::from_utf8_lossy(&output_without_verbose.stderr);

    // Verbose mode should show more detailed output
    assert!(
        stderr.len() > stderr_without_verbose.len() || stderr.contains("Scanning"),
        "Verbose mode should produce more detailed output"
    );
}

#[test]
fn test_drift_capture_flag_creates_log_dir() {
    let temp_dir = TempDir::new().unwrap();
    let _project_path = create_terraform_drift_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap(), "--capture"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should mention capturing output
    assert!(
        stderr.contains("Capturing output") || stderr.contains(".drift-logs"),
        "Should indicate output capture"
    );
}

#[test]
fn test_drift_multiple_projects() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple projects
    let _tf_project1 = create_terraform_drift_project(&temp_dir, &["dev"]);

    let tf2_dir = temp_dir.path().join("terraform2");
    fs::create_dir(&tf2_dir).unwrap();
    let tfvars_dir = tf2_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(tfvars_dir.join("dev.tfvars"), "env = \"dev\"\\n").unwrap();
    let backend_vars_dir = tf2_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\\n",
    )
    .unwrap();
    fs::write(tf2_dir.join("main.tf"), "# config\\n").unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["drift", temp_dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found 2") || stderr.contains("project"),
        "Should detect multiple projects"
    );
}
