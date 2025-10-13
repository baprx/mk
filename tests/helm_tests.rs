//! Integration tests for Helm commands
//! These tests verify Helm-specific functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a helm project
/// Returns the path to the helm chart directory
fn create_helm_project(temp_dir: &TempDir, envs: &[&str]) -> String {
    // Create helm chart directory
    let chart_dir = temp_dir.path().join("my-chart");
    fs::create_dir(&chart_dir).unwrap();

    // Create Chart.yaml (required for Helm detection)
    fs::write(
        chart_dir.join("Chart.yaml"),
        r#"apiVersion: v2
name: my-chart
version: 0.1.0
"#,
    )
    .unwrap();

    // Create values.yaml
    fs::write(
        chart_dir.join("values.yaml"),
        r#"replicaCount: 1
image:
  repository: nginx
  tag: latest
"#,
    )
    .unwrap();

    // Create values directory for environments
    let values_dir = chart_dir.join("values");
    fs::create_dir(&values_dir).unwrap();

    // Create environment directories and values files
    for env in envs {
        let env_dir = values_dir.join(env);
        fs::create_dir(&env_dir).unwrap();
        fs::write(
            env_dir.join("values.yaml"),
            format!("environment: {}\n", env),
        )
        .unwrap();
    }

    // Create templates directory
    let templates_dir = chart_dir.join("templates");
    fs::create_dir(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("deployment.yaml"),
        r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-{{ .Values.environment }}
"#,
    )
    .unwrap();

    // Create helmfile.yaml.gotmpl
    fs::write(
        chart_dir.join("helmfile.yaml.gotmpl"),
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
    createNamespace: true
    values:
      - {{ toYaml .Values | nindent 8 }}
"#,
    )
    .unwrap();

    chart_dir.to_str().unwrap().to_string()
}

#[test]
fn test_helm_apply_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should reference helmfile sync
    assert!(
        stderr.contains("helmfile sync -e dev --skip-deps"),
        "Should reference helmfile sync command"
    );
}

#[test]
fn test_helm_template_with_options() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args([
            "template",
            &project_path,
            "dev",
            "--",
            "--args",
            "--set=environment=option",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("name: test-option"),
        "Should generate helmfile command with options"
    );
}

#[test]
fn test_helm_uninstall_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["uninstall", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should reference helmfile destroy
    assert!(
        stderr.contains("helmfile destroy -e dev --skip-deps"),
        "Should reference helmfile destroy command"
    );
}

#[test]
fn test_helm_delete_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["delete", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Delete should also reference helmfile destroy
    assert!(
        stderr.contains("helmfile destroy -e dev --skip-deps"),
        "Should reference helmfile destroy command"
    );
}

#[test]
fn test_helm_destroy_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["destroy", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Destroy should also reference helmfile destroy
    assert!(
        stderr.contains("helmfile destroy -e dev --skip-deps"),
        "Should reference helmfile destroy command"
    );
}

#[test]
fn test_helm_diff_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["diff", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should reference helmfile diff
    assert!(
        stderr.contains("helmfile diff -e dev --skip-deps"),
        "Should reference helmfile diff command"
    );
}

#[test]
fn test_helm_check_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Check should reference helmfile diff
    assert!(
        stderr.contains("helmfile diff -e dev --skip-deps"),
        "Should reference helmfile diff command for check"
    );
}

#[test]
fn test_helm_invalid_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid env"));
}

#[test]
fn test_helm_multiple_environments() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev", "prod"]);

    // Both environments should work
    let output_dev = Command::cargo_bin("mk")
        .unwrap()
        .args(["template", &project_path, "dev"])
        .output()
        .unwrap();

    let output_prod = Command::cargo_bin("mk")
        .unwrap()
        .args(["template", &project_path, "prod"])
        .output()
        .unwrap();

    let stdout_dev = String::from_utf8_lossy(&output_dev.stdout);
    let stdout_prod = String::from_utf8_lossy(&output_prod.stdout);
    assert!(
        output_dev.status.success() && stdout_dev.contains("name: test-dev"),
        "Dev environment should work"
    );
    assert!(
        output_prod.status.success() && stdout_prod.contains("name: test-prod"),
        "Prod environment should work"
    );
}

#[test]
fn test_helm_with_namespace() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev", "--", "-n", "test-namespace"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("helmfile sync -e dev --skip-deps"),
        "Should generate helmfile command with namespace"
    );
}
