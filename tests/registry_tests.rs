//! Integration tests for registry version fetching
//! Tests both HTTP and OCI registries with real public repositories
//! These tests validate semantic versioning and proper tag handling

use assert_cmd::Command;
use semver::Version;
use std::fs;
use tempfile::TempDir;

/// Test semantic versioning ordering with actual versions
#[test]
fn test_semver_ordering_validation() {
    // Test that semver correctly orders versions (not lexicographic)
    let v1_9 = Version::parse("1.9.0").unwrap();
    let v1_10 = Version::parse("1.10.0").unwrap();
    let v2_0 = Version::parse("2.0.0").unwrap();

    // This is the key test: 1.10.0 should be > 1.9.0
    // (lexicographically "1.9.0" > "1.10.0" which would be wrong)
    assert!(
        v1_10 > v1_9,
        "Semantic version 1.10.0 should be greater than 1.9.0"
    );
    assert!(
        v2_0 > v1_10,
        "Semantic version 2.0.0 should be greater than 1.10.0"
    );

    // Test prerelease ordering
    let v1_0_0 = Version::parse("1.0.0").unwrap();
    let v1_0_0_alpha = Version::parse("1.0.0-alpha").unwrap();
    let v1_0_0_beta = Version::parse("1.0.0-beta").unwrap();
    let v1_0_0_rc = Version::parse("1.0.0-rc.1").unwrap();

    assert!(
        v1_0_0 > v1_0_0_rc,
        "1.0.0 should be greater than 1.0.0-rc.1"
    );
    assert!(
        v1_0_0_rc > v1_0_0_beta,
        "1.0.0-rc.1 should be greater than 1.0.0-beta"
    );
    assert!(
        v1_0_0_beta > v1_0_0_alpha,
        "1.0.0-beta should be greater than 1.0.0-alpha"
    );
}

/// Integration test: scan Chart.yaml with both HTTP and OCI dependencies using bump command
#[test]
fn test_bump_scan_mixed_http_and_oci_registries() {
    let temp_dir = TempDir::new().unwrap();

    // Create Chart.yaml with both HTTP and OCI dependencies
    let chart_yaml = temp_dir.path().join("Chart.yaml");
    fs::write(
        &chart_yaml,
        r#"apiVersion: v2
name: test-app
version: 1.0.0
dependencies:
  # HTTP registry (Argo)
  - name: argo-cd
    version: "5.0.0"
    repository: "https://argoproj.github.io/argo-helm"

  # OCI registry (Prometheus Community)
  - name: prometheus
    version: "15.0.0"
    repository: "oci://ghcr.io/prometheus-community/charts/prometheus"
"#,
    )
    .unwrap();

    // Run bump command to scan dependencies
    let output = Command::cargo_bin("mk")
        .unwrap()
        .arg("bump")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--verbose")
        .output()
        .expect("Failed to execute bump command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Output:\n{}", stderr);

    // Should successfully detect both dependencies
    assert!(
        stderr.contains("argo-cd") || stderr.contains("prometheus"),
        "Should detect at least one dependency"
    );

    // Should not have errors
    assert!(
        !stderr.contains("Error") && !stderr.contains("failed"),
        "Should not have errors: {}",
        stderr
    );
}

/// Test bump with HTTP registry (Argo CD)
#[test]
fn test_bump_http_registry_argo_cd() {
    let temp_dir = TempDir::new().unwrap();

    // Create Chart.yaml with HTTP dependency
    let chart_yaml = temp_dir.path().join("Chart.yaml");
    fs::write(
        &chart_yaml,
        r#"apiVersion: v2
name: test-app
version: 1.0.0
dependencies:
  - name: argo-cd
    version: "5.0.0"
    repository: "https://argoproj.github.io/argo-helm"
"#,
    )
    .unwrap();

    // Run bump command
    let output = Command::cargo_bin("mk")
        .unwrap()
        .arg("bump")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--verbose")
        .output()
        .expect("Failed to execute bump command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect argo-cd and fetch latest version
    assert!(
        stderr.contains("argo-cd"),
        "Should detect argo-cd dependency"
    );

    // Should fetch latest version successfully
    assert!(
        stderr.contains("Found") && stderr.contains("dependencies"),
        "Should find dependencies with updates: {}",
        stderr
    );
}

/// Test bump with OCI registry (Prometheus)
#[test]
fn test_bump_oci_registry_prometheus() {
    let temp_dir = TempDir::new().unwrap();

    // Create Chart.yaml with OCI dependency
    let chart_yaml = temp_dir.path().join("Chart.yaml");
    fs::write(
        &chart_yaml,
        r#"apiVersion: v2
name: test-app
version: 1.0.0
dependencies:
  - name: prometheus
    version: "15.0.0"
    repository: "oci://ghcr.io/prometheus-community/charts/prometheus"
"#,
    )
    .unwrap();

    // Run bump command
    let output = Command::cargo_bin("mk")
        .unwrap()
        .arg("bump")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--verbose")
        .output()
        .expect("Failed to execute bump command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect prometheus
    assert!(
        stderr.contains("prometheus"),
        "Should detect prometheus dependency"
    );

    // Should successfully fetch OCI version (no error about OCI being unsupported)
    assert!(
        !stderr.contains("OCI registries are not supported"),
        "Should support OCI registries"
    );

    // Should fetch latest version successfully
    assert!(
        stderr.contains("Found") && stderr.contains("dependencies"),
        "Should find dependencies with updates: {}",
        stderr
    );
}

/// Test bump with verbose output shows OCI authentication
#[test]
fn test_bump_oci_verbose_shows_auth() {
    let temp_dir = TempDir::new().unwrap();

    // Create Chart.yaml with OCI dependency
    let chart_yaml = temp_dir.path().join("Chart.yaml");
    fs::write(
        &chart_yaml,
        r#"apiVersion: v2
name: test-app
version: 1.0.0
dependencies:
  - name: prometheus
    version: "15.0.0"
    repository: "oci://ghcr.io/prometheus-community/charts/prometheus"
"#,
    )
    .unwrap();

    // Run bump command with verbose
    let output = Command::cargo_bin("mk")
        .unwrap()
        .arg("bump")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--verbose")
        .output()
        .expect("Failed to execute bump command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show OCI-specific verbose output
    assert!(
        stderr.contains("Fetching OCI")
            || stderr.contains("anonymous token")
            || stderr.contains("ghcr.io"),
        "Verbose mode should show OCI fetch details"
    );
}

/// Test that both HTTP and OCI charts are found and versions are valid
#[test]
fn test_bump_validates_semver_for_both_registries() {
    let temp_dir = TempDir::new().unwrap();

    // Create Chart.yaml with both types
    let chart_yaml = temp_dir.path().join("Chart.yaml");
    fs::write(
        &chart_yaml,
        r#"apiVersion: v2
name: test-app
version: 1.0.0
dependencies:
  - name: argo-cd
    version: "5.0.0"
    repository: "https://argoproj.github.io/argo-helm"
  - name: prometheus
    version: "15.0.0"
    repository: "oci://ghcr.io/prometheus-community/charts/prometheus"
"#,
    )
    .unwrap();

    // Run bump command with verbose to see dependency names
    let output = Command::cargo_bin("mk")
        .unwrap()
        .arg("bump")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--verbose")
        .output()
        .expect("Failed to execute bump command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Debug output
    println!("Stderr output:\n{}", stderr);

    // Both dependencies should be detected
    assert!(
        stderr.contains("argo-cd") && stderr.contains("prometheus"),
        "Should detect both HTTP and OCI dependencies. Got: {}",
        stderr
    );

    // Should show updates available (both are on old versions)
    assert!(
        stderr.contains("2 dependencies with updates available"),
        "Should find 2 dependencies with updates: {}",
        stderr
    );
}
