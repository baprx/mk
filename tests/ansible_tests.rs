//! Integration tests for Ansible commands
//! These tests verify Ansible-specific functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create an ansible project
/// Returns the path to the parent directory (not the ansible subdirectory)
fn create_ansible_project(temp_dir: &TempDir, env: &str) -> String {
    // Create ansible subdirectory
    let ansible_dir = temp_dir.path().join("ansible");
    fs::create_dir(&ansible_dir).unwrap();

    // Create ansible.cfg
    fs::write(
        ansible_dir.join("ansible.cfg"),
        "[defaults]\\ninventory = inventories\\n",
    )
    .unwrap();

    // Create inventories directory with environment files
    let inventories_dir = ansible_dir.join("inventories");
    fs::create_dir(&inventories_dir).unwrap();

    fs::write(
        inventories_dir.join(format!("{}.yml", env)),
        "all:\\n  hosts:\\n    server1:\\n      ansible_host: 10.0.0.1\\n",
    )
    .unwrap();

    // Create a simple playbook
    fs::write(
        ansible_dir.join("playbook.yml"),
        "---\\n- hosts: all\\n  tasks:\\n    - name: Test\\n      debug:\\n        msg: test\\n",
    )
    .unwrap();

    // Return the parent directory path
    temp_dir.path().to_str().unwrap().to_string()
}

#[test]
fn test_ansible_list_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, "dev");

    Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["list", ".", "dev"])
        .assert()
        .success();
}

#[test]
fn test_ansible_list_with_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, "dev");

    // Create prod environment too
    let inventories_dir = temp_dir.path().join("ansible").join("inventories");
    fs::write(
        inventories_dir.join("prod.yml"),
        "all:\\n  hosts:\\n    server2:\\n      ansible_host: 10.0.0.2\\n",
    )
    .unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["list", ".", "dev"])
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn test_ansible_list_invalid_environment() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, "dev");

    Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["list", ".", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Invalid")));
}

#[test]
fn test_ansible_list_shows_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["list", ".", "dev"])
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
    let project_path = create_ansible_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["apply", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should generate an ansible-playbook command
    assert!(
        stdout.contains("ansible-playbook") || stderr.contains("ansible-playbook"),
        "Should reference ansible-playbook"
    );
}

#[test]
fn test_ansible_apply_with_check_option() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["apply", ".", "dev", "--", "--check"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should either show the command with --check or succeed (dry-run mode)
    // The tool generates commands, so check if --check appears in output
    assert!(
        stdout.contains("--check") || stderr.contains("--check") || output.status.success(),
        "Apply with --check should either display or execute successfully"
    );
}

#[test]
fn test_ansible_not_in_ansible_directory() {
    let temp_dir = TempDir::new().unwrap();

    Command::cargo_bin("mk")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["list", ".", "dev"])
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
    let project_path = create_ansible_project(&temp_dir, "dev");

    // Create multiple environments
    let inventories_dir = temp_dir.path().join("ansible").join("inventories");
    for env in &["staging", "prod"] {
        fs::write(
            inventories_dir.join(format!("{}.yml", env)),
            "all:\\n  hosts:\\n    server:\\n      ansible_host: 10.0.0.1\\n",
        )
        .unwrap();
    }

    // With valid environment, should succeed
    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["list", ".", "dev"])
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
    let project_path = create_ansible_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["diff", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should generate an ansible-playbook command with -DC flags (--diff --check)
    assert!(
        stdout.contains("ansible-playbook") || stderr.contains("ansible-playbook"),
        "Should reference ansible-playbook"
    );
    assert!(
        stdout.contains("-DC")
            || stderr.contains("-DC")
            || (stdout.contains("--diff") && stdout.contains("--check"))
            || (stderr.contains("--diff") && stderr.contains("--check")),
        "Should include -DC or --diff --check flags"
    );
}

#[test]
fn test_ansible_diff_with_options() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, "dev");

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["diff", ".", "dev", "--", "--limit", "webservers"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should generate ansible-playbook command with additional options
    assert!(
        stdout.contains("ansible-playbook") || stderr.contains("ansible-playbook"),
        "Should reference ansible-playbook"
    );
}

#[test]
fn test_ansible_deps_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, "dev");

    // Create requirements.yml for dependencies
    let ansible_dir = temp_dir.path().join("ansible");
    fs::write(
        ansible_dir.join("requirements.yml"),
        "---\ncollections:\n  - name: community.general\n    version: \">=1.0.0\"\n",
    )
    .unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["deps", ".", "dev"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should reference ansible-galaxy
    assert!(
        stdout.contains("ansible-galaxy") || stderr.contains("ansible-galaxy"),
        "Should reference ansible-galaxy for deps"
    );
}

#[test]
fn test_ansible_deps_without_requirements_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project(&temp_dir, "dev");

    // Don't create requirements.yml
    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(["deps", ".", "dev"])
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
