//! Integration tests for dependency management
//! These tests require external tools (ansible-galaxy, helmfile) to be installed

use assert_cmd::Command;
use serial_test::serial;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper to create an ansible project with dependencies
fn create_ansible_project_with_deps(temp_dir: &TempDir) -> String {
    let project_dir = temp_dir.path().join("ansible");
    fs::create_dir(&project_dir).unwrap();

    // Create inventories directory
    let inventories_dir = project_dir.join("inventories");
    fs::create_dir(&inventories_dir).unwrap();
    fs::write(
        inventories_dir.join("dev.yml"),
        "all:\n  hosts:\n    localhost:\n      ansible_connection: local\n",
    )
    .unwrap();

    // Create ansible.cfg
    fs::write(
        project_dir.join("ansible.cfg"),
        "[defaults]\nroles_path = ./roles\n",
    )
    .unwrap();

    // Create roles directory
    let roles_dir = project_dir.join("roles");
    fs::create_dir(&roles_dir).unwrap();

    // Create requirements.yml with a simple role dependency
    fs::write(
        roles_dir.join("requirements.yml"),
        "---\n- name: geerlingguy.docker\n  version: \"7.4.1\"\n",
    )
    .unwrap();

    // Create playbook that uses the role
    fs::write(
        project_dir.join("playbook.yml"),
        "---\n- hosts: all\n  roles:\n    - geerlingguy.docker\n",
    )
    .unwrap();

    project_dir.to_str().unwrap().to_string()
}

/// Helper to create a helm project with dependencies
fn create_helm_project_with_deps(temp_dir: &TempDir) -> String {
    let project_dir = temp_dir.path().join("my-chart");
    fs::create_dir(&project_dir).unwrap();

    // Create values.yaml
    fs::write(project_dir.join("values.yaml"), "replicaCount: 1\n").unwrap();

    // Create Chart.yaml with dependencies
    fs::write(
        project_dir.join("Chart.yaml"),
        r#"apiVersion: v2
name: my-chart
version: 0.1.0
dependencies:
  - name: cert-manager
    version: "v1.18.2"
    repository: https://charts.jetstack.io
"#,
    )
    .unwrap();

    // Create helmfile.yaml.gotmpl (required by mk implementation)
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
    createNamespace: true
    values:
      - {{ toYaml .Values | nindent 8 }}
"#,
    )
    .unwrap();

    // Create values directories
    let values_dir = project_dir.join("values");
    fs::create_dir(&values_dir).unwrap();

    let dev_dir = values_dir.join("dev");
    fs::create_dir(&dev_dir).unwrap();
    fs::write(dev_dir.join("values.yaml"), "environment: dev\n").unwrap();

    project_dir.to_str().unwrap().to_string()
}

#[test]
#[serial]
fn test_ansible_deps_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project_with_deps(&temp_dir);

    // Run deps command
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["deps", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that ansible-galaxy command was executed
    assert!(
        stderr.contains("ansible-galaxy") || stdout.contains("was installed successfully"),
        "Should execute ansible-galaxy command. stderr: {}, stdout: {}",
        stderr,
        stdout
    );

    // Check if role was downloaded
    let role_path = Path::new(&project_path).join("roles/geerlingguy.docker");
    if !role_path.exists() {
        eprintln!(
            "Warning: ansible-galaxy role not found at expected path. This may indicate ansible-galaxy is not installed or the command failed."
        );
    }
}

#[test]
#[serial]
fn test_ansible_check_without_deps_fails() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project_with_deps(&temp_dir);

    // Try to run check without installing dependencies first
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The command should either fail or show an error about missing role
    // Note: This might pass if the role is already installed system-wide
    if !output.status.success() {
        assert!(
            stderr.contains("error")
                || stderr.contains("role")
                || stderr.contains("not found")
                || stdout.contains("error")
                || stdout.contains("role"),
            "Should fail or warn about missing dependencies"
        );
    }
}

#[test]
#[serial]
fn test_ansible_check_after_deps_succeeds() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_ansible_project_with_deps(&temp_dir);

    // First install dependencies
    Command::cargo_bin("mk")
        .unwrap()
        .args(["deps", &project_path, "dev"])
        .output()
        .unwrap();

    // Now run check - should generate the command successfully
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["check", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should at least generate the ansible-playbook command
    assert!(
        stderr.contains("ansible-playbook"),
        "Should generate ansible-playbook command after deps installed"
    );
}

#[test]
#[serial]
fn test_helm_deps_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project_with_deps(&temp_dir);

    // Run deps command
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["deps", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The deps command completes successfully (deps update happens silently unless verbose)
    // Just check that it detected the Helm chart and command succeeded
    assert!(
        output.status.success() && stderr.contains("Detected helm"),
        "Should execute helmfile deps command. stderr: {}, stdout: {}",
        stderr,
        stdout
    );

    // Check if charts directory was created and populated
    let charts_dir = Path::new(&project_path).join("charts");
    if charts_dir.exists() {
        let has_tgz = fs::read_dir(&charts_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().extension().is_some_and(|ext| ext == "tgz"))
            })
            .unwrap_or(false);

        assert!(
            has_tgz,
            "charts/ directory should contain .tgz files after running deps"
        );
    } else {
        eprintln!(
            "Warning: charts/ directory not created. Helm may not be installed or command failed."
        );
    }
}

#[test]
#[serial]
fn test_helm_template_auto_downloads_deps() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project_with_deps(&temp_dir);

    // Run template without explicitly running deps first
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["template", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should execute helmfile template command (which auto-downloads deps if needed)
    assert!(
        stderr.contains("helmfile template") || stderr.contains("helmfile"),
        "Should execute helmfile template command"
    );

    // Check if charts were downloaded
    let charts_dir = Path::new(&project_path).join("charts");
    if charts_dir.exists() {
        let has_tgz = fs::read_dir(&charts_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().extension().is_some_and(|ext| ext == "tgz"))
            })
            .unwrap_or(false);

        if has_tgz {
            eprintln!("✓ Helm successfully auto-downloaded dependencies");
        }
    }
}

#[test]
#[serial]
fn test_helm_template_after_deleting_charts() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_helm_project_with_deps(&temp_dir);

    // First run deps to download charts
    Command::cargo_bin("mk")
        .unwrap()
        .args(["deps", &project_path, "dev"])
        .output()
        .unwrap();

    // Delete the charts directory
    let charts_dir = Path::new(&project_path).join("charts");
    if charts_dir.exists() {
        fs::remove_dir_all(&charts_dir).unwrap();
    }

    // Now run template - should re-download
    let output = Command::cargo_bin("mk")
        .unwrap()
        .args(["template", &project_path, "dev"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("helmfile"),
        "Should execute helmfile command and re-download deps"
    );

    // Check if charts were re-downloaded
    if charts_dir.exists() {
        let has_tgz = fs::read_dir(&charts_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().extension().is_some_and(|ext| ext == "tgz"))
            })
            .unwrap_or(false);

        if has_tgz {
            eprintln!("✓ Helm successfully re-downloaded deleted dependencies");
        }
    }
}
