//! Integration tests for Kustomize commands
//! These tests verify Kustomize-specific functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a kustomize project
/// Returns the path to the kustomize directory
fn create_kustomize_project(temp_dir: &TempDir, envs: &[&str]) -> String {
    // Create kustomize project directory
    let kustomize_dir = temp_dir.path().join("my-kustomize");
    fs::create_dir(&kustomize_dir).unwrap();

    // Create base directory
    let base_dir = kustomize_dir.join("base");
    fs::create_dir(&base_dir).unwrap();

    // Create base kustomization.yaml
    fs::write(
        base_dir.join("kustomization.yaml"),
        r#"apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
  - deployment.yaml
"#,
    )
    .unwrap();

    // Create base deployment
    fs::write(
        base_dir.join("deployment.yaml"),
        r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
spec:
  replicas: 1
"#,
    )
    .unwrap();

    // Create overlays directory (required for Kustomize detection)
    let overlays_dir = kustomize_dir.join("overlays");
    fs::create_dir(&overlays_dir).unwrap();

    // Create environment overlays
    for env in envs {
        let env_dir = overlays_dir.join(env);
        fs::create_dir(&env_dir).unwrap();

        fs::write(
            env_dir.join("kustomization.yaml"),
            format!(
                "apiVersion: kustomize.config.k8s.io/v1beta1\nkind: Kustomization\nnamespace: {}\nresources:\n  - ../../base\n",
                env
            ),
        )
        .unwrap();
    }

    kustomize_dir.to_str().unwrap().to_string()
}

#[test]
fn test_kustomize_apply_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("kustomize build overlays/dev | kubectl apply"),
        "Should reference kustomize or kubectl command"
    );
}

#[test]
fn test_kustomize_apply_with_options() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev", "--", "--dry-run=client"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("kustomize build overlays/dev | kubectl apply --dry-run=client"),
        "Should generate kustomize/kubectl command with options"
    );
}

#[test]
fn test_kustomize_diff_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["diff", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should reference kustomize build and kubectl diff
    assert!(
        stderr.contains("kustomize build overlays/dev | kubectl diff"),
        "Should reference kustomize/kubectl diff command"
    );
}

#[test]
fn test_kustomize_diff_with_options() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["diff", &project_path, "dev", "--", "--server-side"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("kustomize build overlays/dev") && stderr.contains("--server-side"),
        "Should generate kustomize/kubectl command with options"
    );
}

#[test]
fn test_kustomize_invalid_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_project(&temp_dir, &["dev"]);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid env"));
}

#[test]
fn test_kustomize_multiple_environments() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_project(&temp_dir, &["dev", "prod"]);

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

    assert!(output_dev.status.success(), "Dev environment should work");
    assert!(output_prod.status.success(), "Prod environment should work");
}

#[test]
fn test_kustomize_not_in_kustomize_directory() {
    let temp_dir = TempDir::new().unwrap();

    Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", temp_dir.path().to_str().unwrap(), "dev"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No technology detected")
                .or(predicate::str::contains("not a Kustomize"))
                .or(predicate::str::contains("Cannot")),
        );
}

#[test]
fn test_kustomize_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_kustomize_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Detected kustomize"),
        "Should detect kustomize technology"
    );
}
