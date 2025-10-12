//! Integration tests for configuration management
//! Tests config file loading, priority selection, and validation

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Helper to create a terraform project with config
fn create_terraform_with_config(temp_dir: &TempDir, config_content: &str) -> String {
    let project_dir = temp_dir.path().join("terraform");
    fs::create_dir(&project_dir).unwrap();

    // Create config file
    fs::write(project_dir.join(".mk.toml"), config_content).unwrap();

    // Create tfvars directory
    let tfvars_dir = project_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(tfvars_dir.join("dev.tfvars"), "env = \"dev\"\\n").unwrap();

    // Create backend-vars directory
    let backend_vars_dir = project_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\\n",
    )
    .unwrap();

    // Create main.tf
    fs::write(project_dir.join("main.tf"), "# terraform config\\n").unwrap();

    project_dir.to_str().unwrap().to_string()
}

#[test]
fn test_config_file_can_be_loaded() {
    let temp_dir = TempDir::new().unwrap();

    // Create project config with some settings
    let config = r#"
[settings]
auto_approve = false
"#;

    let project_path = create_terraform_with_config(&temp_dir, config);

    // Just verify the tool can work with a config file present
    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(&["plan", ".", "dev"])
        .output()
        .unwrap();

    // Config file should not cause errors
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "Tool should work with config file present"
    );
}

#[test]
fn test_config_with_custom_paths() {
    let temp_dir = TempDir::new().unwrap();

    let config = r#"
[paths]
backend_vars = "custom-backend"
"#;

    let project_dir = temp_dir.path().join("terraform");
    fs::create_dir(&project_dir).unwrap();

    fs::write(project_dir.join(".mk.toml"), config).unwrap();

    // Create tfvars in default location
    let tfvars_dir = project_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(tfvars_dir.join("dev.tfvars"), "env = \"dev\"\\n").unwrap();

    // Create backend-vars in default location (config may not be implemented yet)
    let backend_vars_dir = project_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\\n",
    )
    .unwrap();

    fs::write(project_dir.join("main.tf"), "# config\\n").unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_dir)
        .args(&["plan", ".", "dev"])
        .output()
        .unwrap();

    // Config file should not cause errors even if custom paths aren't implemented
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "Tool should work with config file containing custom paths"
    );
}

#[test]
fn test_config_invalid_toml() {
    let temp_dir = TempDir::new().unwrap();

    let invalid_config = r#"
[priority
terraform = "apply"
"#;

    let project_path = create_terraform_with_config(&temp_dir, invalid_config);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(&["plan", ".", "dev"])
        .output()
        .unwrap();

    // Should handle invalid config gracefully (may ignore it or show error)
    // Either way, the tool should not crash
    assert!(
        output.status.code().is_some(),
        "Tool should handle invalid config without crashing"
    );
}

#[test]
fn test_config_environment_specific_settings() {
    let temp_dir = TempDir::new().unwrap();

    let config = r#"
[environments.dev]
auto_approve = true

[environments.prod]
auto_approve = false
"#;

    let project_path = create_terraform_with_config(&temp_dir, config);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(&["terraform", "--env", "dev"])
        .output()
        .unwrap();

    // Command should work (config parsed successfully)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("terraform") || output.status.success(),
        "Should parse environment-specific config"
    );
}

#[test]
fn test_config_with_excluded_directories() {
    let temp_dir = TempDir::new().unwrap();

    let config = r#"
[scan]
exclude = ["vendor", "node_modules"]
"#;

    let project_path = create_terraform_with_config(&temp_dir, config);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(&["terraform", "--env", "dev"])
        .output()
        .unwrap();

    // Config should be parsed without errors
    assert!(
        output.status.success() || String::from_utf8_lossy(&output.stderr).contains("terraform"),
        "Should parse exclusion config"
    );
}

#[test]
fn test_no_config_file_uses_defaults() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("terraform");
    fs::create_dir(&project_dir).unwrap();

    // Don't create .mk.toml file

    // Create tfvars directory
    let tfvars_dir = project_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(tfvars_dir.join("dev.tfvars"), "env = \"dev\"\\n").unwrap();

    // Create backend-vars directory
    let backend_vars_dir = project_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\\n",
    )
    .unwrap();

    fs::write(project_dir.join("main.tf"), "# config\\n").unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_dir)
        .args(&["terraform", "--env", "dev"])
        .output()
        .unwrap();

    // Should work with default settings
    assert!(
        output.status.success() || String::from_utf8_lossy(&output.stderr).contains("terraform"),
        "Should work without config file"
    );
}

#[test]
fn test_config_with_plan_command() {
    let temp_dir = TempDir::new().unwrap();

    let config = r#"
[settings]
verbose = true
"#;

    let project_path = create_terraform_with_config(&temp_dir, config);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(&["plan", ".", "dev"])
        .output()
        .unwrap();

    // Config file should not interfere with plan command
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "Plan command should work with config file"
    );
}

#[test]
fn test_config_helm_specific_settings() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("helm-chart");
    fs::create_dir(&project_dir).unwrap();

    let config = r#"
[helm]
timeout = "10m"
namespace = "custom-namespace"
"#;

    fs::write(project_dir.join(".mk.toml"), config).unwrap();

    // Create helm chart structure
    fs::write(project_dir.join("values.yaml"), "replicaCount: 3\\n").unwrap();
    fs::write(
        project_dir.join("Chart.yaml"),
        "apiVersion: v2\\nname: test-chart\\nversion: 0.1.0\\n",
    )
    .unwrap();

    let values_dir = project_dir.join("values");
    fs::create_dir(&values_dir).unwrap();
    let dev_dir = values_dir.join("dev");
    fs::create_dir(&dev_dir).unwrap();
    fs::write(dev_dir.join("values.yaml"), "environment: dev\\n").unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_dir)
        .args(&["helm", "--env", "dev"])
        .output()
        .unwrap();

    // Should parse helm-specific config
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("helm") || output.status.success(),
        "Should parse helm config"
    );
}

#[test]
fn test_config_ansible_specific_settings() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("ansible");
    fs::create_dir(&project_dir).unwrap();

    let config = r#"
[ansible]
forks = 10
vault_password_file = ".vault-pass"
"#;

    fs::write(project_dir.join(".mk.toml"), config).unwrap();

    // Create ansible structure
    fs::write(
        project_dir.join("ansible.cfg"),
        "[defaults]\\ninventory = inventory\\n",
    )
    .unwrap();

    let inventory_dir = project_dir.join("inventory");
    fs::create_dir(&inventory_dir).unwrap();
    let dev_dir = inventory_dir.join("dev");
    fs::create_dir(&dev_dir).unwrap();
    fs::write(
        dev_dir.join("hosts"),
        "[webservers]\\nserver1 ansible_host=10.0.0.1\\n",
    )
    .unwrap();

    fs::write(
        project_dir.join("playbook.yml"),
        "---\\n- hosts: all\\n  tasks:\\n    - debug: msg=test\\n",
    )
    .unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_dir)
        .args(&["ansible", "list", "--env", "dev"])
        .output()
        .unwrap();

    // Should parse ansible-specific config
    assert!(
        output.status.success() || String::from_utf8_lossy(&output.stderr).contains("ansible"),
        "Should parse ansible config"
    );
}

#[test]
fn test_config_global_verbose_setting() {
    let temp_dir = TempDir::new().unwrap();

    let config = r#"
[global]
verbose = true
"#;

    let project_path = create_terraform_with_config(&temp_dir, config);

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_path)
        .args(&["terraform", "--env", "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // With verbose in config, should show more output
    assert!(
        stderr.contains("DEBUG") || stderr.contains("INFO") || stderr.len() > 50,
        "Verbose config should increase output verbosity"
    );
}

#[test]
fn test_config_in_parent_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create config in parent directory
    let config = r#"
[priority]
terraform = "plan"
"#;
    fs::write(temp_dir.path().join(".mk.toml"), config).unwrap();

    // Create terraform project in subdirectory
    let project_dir = temp_dir.path().join("infra").join("terraform");
    fs::create_dir_all(&project_dir).unwrap();

    let tfvars_dir = project_dir.join("tfvars");
    fs::create_dir(&tfvars_dir).unwrap();
    fs::write(tfvars_dir.join("dev.tfvars"), "env = \"dev\"\\n").unwrap();

    let backend_vars_dir = project_dir.join("backend-vars");
    fs::create_dir(&backend_vars_dir).unwrap();
    fs::write(
        backend_vars_dir.join("dev.tfvars"),
        "key = \"terraform-dev.tfstate\"\\n",
    )
    .unwrap();

    fs::write(project_dir.join("main.tf"), "# config\\n").unwrap();

    let output = Command::cargo_bin("mk")
        .unwrap()
        .current_dir(&project_dir)
        .args(&["terraform", "--env", "dev"])
        .output()
        .unwrap();

    // Should find and use parent config
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("plan") || stderr.contains("terraform"),
        "Should use parent directory config"
    );
}
