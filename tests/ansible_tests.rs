//! Integration tests for Ansible commands
//! These tests verify Ansible-specific functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create an ansible project
/// Returns the path to the parent directory (not the ansible subdirectory)
fn create_ansible_project(temp_dir: &TempDir, envs: &[&str]) -> String {
    // Create ansible subdirectory
    let ansible_dir = temp_dir.path().join("ansible");
    fs::create_dir(&ansible_dir).unwrap();

    // Create ansible.cfg
    fs::write(
        ansible_dir.join("ansible.cfg"),
        r#"[defaults]
inventory = inventories
"#,
    )
    .unwrap();

    // Create inventories directory with environment files
    let inventories_dir = ansible_dir.join("inventories");
    fs::create_dir(&inventories_dir).unwrap();

    for env in envs {
        fs::write(
            inventories_dir.join(format!("{}.yml", env)),
            r#"all:
  hosts:
    server1:
      ansible_host: 10.0.0.1
"#,
        )
        .unwrap();
    }

    // Create a simple playbook
    fs::write(
        ansible_dir.join("playbook.yml"),
        r#"---
- hosts: all
  tasks:
    - name: Test
      debug:
        msg: test
"#,
    )
    .unwrap();

    // Return the parent directory path
    temp_dir.path().to_str().unwrap().to_string()
}

#[test]
fn test_ansible_list_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["list", &project_path, "dev"])
        .assert()
        .success();
}

#[test]
fn test_ansible_list_with_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev", "prod"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["list", &project_path, "dev"])
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn test_ansible_list_invalid_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    Command::cargo_bin("mk")
        .unwrap()
        .args(["list", &project_path, "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Invalid")));
}

#[test]
fn test_ansible_list_shows_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["list", &project_path, "dev"])
        .output()
        .unwrap();

    // The command should succeed (whether it prints or executes the command)
    assert!(
        output.status.success(),
        "List command should succeed for valid Ansible project"
    );
}

#[test]
fn test_ansible_apply_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should generate an ansible-playbook command with proper structure
    assert!(
        stderr.contains("ansible-playbook"),
        "Should generate ansible-playbook command in stderr"
    );
    assert!(
        stderr.contains("playbook.yml"),
        "Should reference the playbook file"
    );
    assert!(
        stderr.contains("-i") || stderr.contains("--inventory"),
        "Should include inventory flag"
    );
}

#[test]
fn test_ansible_apply_with_check_option() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["apply", &project_path, "dev", "--", "--check"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show the command with --check flag passed through
    assert!(
        stderr.contains("ansible-playbook"),
        "Should generate ansible-playbook command"
    );
    assert!(
        stderr.contains("--check"),
        "Should include the --check flag in the command"
    );
}

#[test]
fn test_ansible_not_in_ansible_directory() {
    let temp_dir = TempDir::new().unwrap();

    Command::cargo_bin("mk")
        .unwrap()
        .args(["list", temp_dir.path().to_str().unwrap(), "dev"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No technology detected")
                .or(predicate::str::contains("not an Ansible"))
                .or(predicate::str::contains("Cannot")),
        );
}

#[test]
fn test_ansible_multiple_environments_available() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev", "staging", "prod"]);

    // With valid environment, should succeed
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["list", &project_path, "dev"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Should work with multiple environments when one is specified"
    );
}

#[test]
fn test_ansible_diff_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["diff", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should generate an ansible-playbook command with diff and check flags
    assert!(
        stderr.contains("ansible-playbook"),
        "Should generate ansible-playbook command"
    );
    assert!(
        stderr.contains("-DC") || (stderr.contains("-D") && stderr.contains("-C")),
        "Should include -DC or separate -D -C flags for diff and check"
    );
}

#[test]
fn test_ansible_diff_with_options() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["diff", &project_path, "dev", "--", "--limit", "webservers"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should generate ansible-playbook command with additional options passed through
    assert!(
        stderr.contains("ansible-playbook"),
        "Should generate ansible-playbook command"
    );
    assert!(
        stderr.contains("--limit") && stderr.contains("webservers"),
        "Should include the --limit webservers option"
    );
}

#[test]
fn test_ansible_deps_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    // Create requirements.yml for dependencies
    let ansible_dir = temp_dir.path().join("ansible");
    fs::write(
        ansible_dir.join("requirements.yml"),
        "---\ncollections:\n  - name: community.general\n    version: \">=1.0.0\"\n",
    )
    .unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["deps", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should generate ansible-galaxy command with requirements file
    assert!(
        stderr.contains("ansible-galaxy"),
        "Should generate ansible-galaxy command"
    );
    assert!(
        stderr.contains("install") && stderr.contains("requirements"),
        "Should include install command with requirements file"
    );
}

#[test]
fn test_ansible_deps_without_requirements_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, &["dev"]);

    // Don't create requirements.yml
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["deps", &project_path, "dev"])
        .output()
        .unwrap();

    // Should either succeed (no-op) or reference ansible-galaxy
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Command should execute or provide appropriate feedback
    assert!(
        output.status.success()
            || stdout.contains("ansible-galaxy")
            || stderr.contains("ansible-galaxy")
            || stderr.contains("requirements"),
        "Should handle missing requirements file gracefully"
    );
}
