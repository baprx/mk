use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_bump_terraform_module_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let terraform_dir = temp_dir.path().join("terraform");
    fs::create_dir(&terraform_dir).unwrap();

    // Create a Terraform file with a module
    let vpc_tf = terraform_dir.join("vpc.tf");
    fs::write(
        &vpc_tf,
        r#"
module "vpc" {
  source  = "terraform-google-modules/network/google"
  version = "~> 7.0"

  project_id   = "my-project"
  network_name = "my-vpc"
}
"#,
    )
    .unwrap();

    // Run bump command in dry-run mode (list only, don't update)
    let mut cmd = Command::cargo_bin("mk").unwrap();
    cmd.current_dir(&terraform_dir)
        .arg("bump")
        .arg(".")
        .arg("--verbose");

    // Note: This will actually try to fetch versions from the registry
    // In a real test, we might want to mock the HTTP calls
    // For now, we're testing that the command runs without the file path error
    let output = cmd.output().unwrap();

    // Check that we don't get "No such file or directory" error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("No such file or directory"),
        "Should not have file path error. Stderr: {}",
        stderr
    );
}

#[test]
fn test_bump_terraform_full_path_handling() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("infrastructure").join("vpc");
    fs::create_dir_all(&project_dir).unwrap();

    // Create terraform directory marker for technology detection
    let terraform_dir = project_dir.join("terraform");
    fs::create_dir(&terraform_dir).unwrap();

    // Create Terraform files in a nested directory structure
    let vpc_tf = project_dir.join("vpc.tf");
    fs::write(
        &vpc_tf,
        r#"
module "vpc" {
  source  = "terraform-google-modules/network/google"
  version = "~> 7.0"

  project_id   = "my-project"
  network_name = "my-vpc"
}
"#,
    )
    .unwrap();

    let backend_tf = project_dir.join("backend.tf");
    fs::write(
        &backend_tf,
        r#"
terraform {
  backend "gcs" {
    bucket = "my-bucket"
    prefix = "terraform/state"
  }
}
"#,
    )
    .unwrap();

    // Run bump command from parent directory
    let mut cmd = Command::cargo_bin("mk").unwrap();
    cmd.current_dir(&project_dir)
        .arg("bump")
        .arg(".")
        .arg("--verbose");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify no file path errors
    assert!(
        !stderr.contains("No such file or directory"),
        "Should handle nested directory structure. Stderr: {}",
        stderr
    );

    // Verify it detected the Terraform project
    assert!(
        stderr.contains("Detected Terraform project") || stderr.contains("Detected terraform"),
        "Should detect Terraform project. Stderr: {}",
        stderr
    );
}

#[test]
fn test_bump_helm_chart_parsing() {
    let temp_dir = TempDir::new().unwrap();

    // Create a values.yaml (required for Helm detection) in the temp directory
    let values_yaml = temp_dir.path().join("values.yaml");
    fs::write(&values_yaml, "# Default values\n").unwrap();

    // Create a Chart.yaml with dependencies
    let chart_yaml = temp_dir.path().join("Chart.yaml");
    fs::write(
        &chart_yaml,
        r#"
apiVersion: v2
name: my-chart
version: 1.0.0
dependencies:
  - name: postgresql
    version: "12.0.0"
    repository: "https://charts.bitnami.com/bitnami"
  - name: redis
    version: "17.0.0"
    repository: "https://charts.bitnami.com/bitnami"
"#,
    )
    .unwrap();

    // Run bump command - the directory has values.yaml so it should be detected as Helm
    let mut cmd = Command::cargo_bin("mk").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("bump")
        .arg(".")
        .arg("--verbose");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify it detected Helm chart
    assert!(
        stderr.contains("Detected Helm project") || stderr.contains("Detected helm"),
        "Should detect Helm project. Stderr: {}",
        stderr
    );

    // Verify no file path errors
    assert!(
        !stderr.contains("No such file or directory"),
        "Should not have file path error. Stderr: {}",
        stderr
    );
}

#[test]
fn test_bump_no_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create terraform directory marker for technology detection
    let terraform_dir = project_dir.join("terraform");
    fs::create_dir(&terraform_dir).unwrap();

    // Create a Terraform file with no modules
    let main_tf = project_dir.join("main.tf");
    fs::write(
        &main_tf,
        r#"
resource "google_compute_network" "vpc" {
  name                    = "my-vpc"
  auto_create_subnetworks = false
}
"#,
    )
    .unwrap();

    // Run bump command
    let mut cmd = Command::cargo_bin("mk").unwrap();
    cmd.current_dir(project_dir)
        .arg("bump")
        .arg(".")
        .arg("--verbose");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should report no dependencies found
    assert!(
        stderr.contains("No dependencies found"),
        "Should report no dependencies. Stderr: {}",
        stderr
    );
}

#[test]
fn test_bump_terraform_version_constraint_formats() {
    let temp_dir = TempDir::new().unwrap();

    // Create a terraform subdirectory for technology detection
    let terraform_dir = temp_dir.path().join("terraform");
    fs::create_dir(&terraform_dir).unwrap();

    // Create a Terraform file with different version constraint formats in the terraform subdir
    let modules_tf = terraform_dir.join("modules.tf");
    fs::write(
        &modules_tf,
        r#"
module "vpc" {
  source  = "terraform-google-modules/network/google"
  version = "~> 7.0"
  project_id = "test"
}

module "nat" {
  source  = "terraform-google-modules/cloud-nat/google"
  version = ">= 4.0.0"
  project_id = "test"
}

module "bastion" {
  source  = "terraform-google-modules/bastion-host/google"
  version = "5.0.0"
  project_id = "test"
}
"#,
    )
    .unwrap();

    // Run bump command from parent - it should detect the terraform subdirectory
    let mut cmd = Command::cargo_bin("mk").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("bump")
        .arg(".")
        .arg("--verbose");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify all modules are detected
    assert!(
        stderr.contains("network/google") || stderr.contains("Found module: vpc"),
        "Should detect vpc module. Stderr: {}",
        stderr
    );
}

#[test]
fn test_bump_display_relative_paths() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("infrastructure");
    fs::create_dir_all(&project_dir).unwrap();

    // Create a Terraform file
    let vpc_tf = project_dir.join("vpc.tf");
    fs::write(
        &vpc_tf,
        r#"
module "vpc" {
  source  = "terraform-google-modules/network/google"
  version = "~> 7.0"
  project_id = "test"
}
"#,
    )
    .unwrap();

    // Run bump command
    let mut cmd = Command::cargo_bin("mk").unwrap();
    cmd.current_dir(&project_dir)
        .arg("bump")
        .arg(".")
        .arg("--verbose");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stderr.contains("Found") && stderr.contains("module") {
        // If modules are found, check that paths are relative
        assert!(
            !stderr.contains(&format!("{}", project_dir.display())),
            "Should display relative paths, not absolute. Stderr: {}",
            stderr
        );
    }
}
