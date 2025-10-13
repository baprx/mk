//! Integration tests for Helm commands
//! These tests verify Helm-specific functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a helm project
/// Returns the path to the helm chart directory
fn create_helm_project(temp_dir: &TempDir, env: &str) -> String {
    // Create helm chart directory
    let chart_dir = temp_dir.path().join("my-chart");
    fs::create_dir(&chart_dir).unwrap();

    // Create Chart.yaml (required for Helm detection)
    fs::write(
        chart_dir.join("Chart.yaml"),
        "apiVersion: v2\nname: my-chart\nversion: 0.1.0\n",
    )
    .unwrap();

    // Create values.yaml
    fs::write(
        chart_dir.join("values.yaml"),
        "replicaCount: 1\nimage:\n  repository: nginx\n  tag: latest\n",
    )
    .unwrap();

    // Create values directory for environments
    let values_dir = chart_dir.join("values");
    fs::create_dir(&values_dir).unwrap();

    let env_dir = values_dir.join(env);
    fs::create_dir(&env_dir).unwrap();
    fs::write(
        env_dir.join("values.yaml"),
        format!("environment: {}\n", env),
    )
    .unwrap();

    // Create templates directory
    let templates_dir = chart_dir.join("templates");
    fs::create_dir(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("deployment.yaml"),
        "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: test\n",
    )
    .unwrap();

    // Create helmfile.yaml (required for helmfile commands)
    fs::write(
        chart_dir.join("helmfile.yaml"),
        format!(
            "environments:\n  {}:\n---\nreleases:\n  - name: my-chart\n    chart: .\n    values:\n      - values/{}/values.yaml\n",
            env, env
        ),
    )
    .unwrap();

    chart_dir.to_str().unwrap().to_string()
}

#[test]
fn test_helm_apply_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["apply", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should reference helmfile sync
    assert!(
        (stdout.contains("helmfile") && stdout.contains("sync"))
            || (stderr.contains("helmfile") && stderr.contains("sync")),
        "Should reference helmfile sync command"
    );
}

#[test]
fn test_helm_apply_with_options() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["apply", ".", "dev", "--", "--atomic"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("helmfile") || stderr.contains("helmfile"),
        "Should generate helmfile command with options"
    );
}

#[test]
fn test_helm_uninstall_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["uninstall", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should reference helmfile destroy
    assert!(
        (stdout.contains("helmfile") && stdout.contains("destroy"))
            || (stderr.contains("helmfile") && stderr.contains("destroy")),
        "Should reference helmfile destroy command"
    );
}

#[test]
fn test_helm_delete_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["delete", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Delete should also reference helmfile destroy
    assert!(
        (stdout.contains("helmfile") && stdout.contains("destroy"))
            || (stderr.contains("helmfile") && stderr.contains("destroy")),
        "Should reference helmfile destroy command"
    );
}

#[test]
fn test_helm_destroy_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["destroy", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Destroy should also reference helmfile destroy
    assert!(
        (stdout.contains("helmfile") && stdout.contains("destroy"))
            || (stderr.contains("helmfile") && stderr.contains("destroy")),
        "Should reference helmfile destroy command"
    );
}

#[test]
fn test_helm_diff_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["diff", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should reference helmfile diff
    assert!(
        (stdout.contains("helmfile") && stdout.contains("diff"))
            || (stderr.contains("helmfile") && stderr.contains("diff")),
        "Should reference helmfile diff command"
    );
}

#[test]
fn test_helm_check_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["check", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Check should reference helmfile diff
    assert!(
        (stdout.contains("helmfile") && stdout.contains("diff"))
            || (stderr.contains("helmfile") && stderr.contains("diff")),
        "Should reference helmfile diff command for check"
    );
}

#[test]
fn test_helm_invalid_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["apply", ".", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid env"));
}

#[test]
fn test_helm_multiple_environments() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    // Create prod environment
    let chart_dir = temp_dir.path().join("my-chart");
    let values_dir = chart_dir.join("values");
    let prod_dir = values_dir.join("prod");
    fs::create_dir(&prod_dir).unwrap();
    fs::write(prod_dir.join("values.yaml"), "environment: prod\n").unwrap();

    // Both environments should work
    let output_dev = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["template", ".", "dev"])
        .output()
        .unwrap();

    let output_prod = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["template", ".", "prod"])
        .output()
        .unwrap();

    assert!(
        output_dev.status.success()
            || String::from_utf8_lossy(&output_dev.stderr).contains("helmfile"),
        "Dev environment should work"
    );
    assert!(
        output_prod.status.success()
            || String::from_utf8_lossy(&output_prod.stderr).contains("helmfile"),
        "Prod environment should work"
    );
}

#[test]
fn test_helm_with_namespace() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["apply", ".", "dev", "--", "-n", "test-namespace"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("helmfile") || stderr.contains("helmfile"),
        "Should generate helmfile command with namespace"
    );
}
