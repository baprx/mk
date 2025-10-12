use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct TerraformModule {
    versions: Vec<String>,
}

/// Check if a version string is a pre-release
fn is_prerelease(version: &Version) -> bool {
    !version.pre.is_empty()
}

/// Fetch the latest version of a Terraform module from the Terraform Registry
pub fn fetch_terraform_module_version(
    namespace: &str,
    name: &str,
    provider: &str,
    verbose: bool,
    include_prereleases: bool,
) -> Result<String> {
    let url = format!(
        "https://registry.terraform.io/v1/modules/{}/{}/{}",
        namespace, name, provider
    );

    if verbose {
        eprintln!("  Fetching versions from: {}", url);
    }

    let response = attohttpc::get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .context(format!("Failed to fetch module info from {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch module: HTTP {}", response.status());
    }

    let module: TerraformModule = response
        .json()
        .context("Failed to parse Terraform Registry response")?;

    // Parse versions and find the latest
    let mut versions: Vec<Version> = module
        .versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .collect();

    // Filter out pre-releases unless explicitly requested
    if !include_prereleases {
        versions.retain(|v| !is_prerelease(v));
    }

    versions.sort();

    versions
        .last()
        .map(|v| v.to_string())
        .ok_or_else(|| anyhow::anyhow!("No valid versions found"))
}

/// Fetch the latest version of a Helm chart from a Helm repository
pub fn fetch_helm_chart_version(
    repo_url: &str,
    chart_name: &str,
    verbose: bool,
    include_prereleases: bool,
) -> Result<String> {
    // Ensure repo_url ends with /index.yaml
    let index_url = if repo_url.ends_with("/index.yaml") {
        repo_url.to_string()
    } else if repo_url.ends_with('/') {
        format!("{}index.yaml", repo_url)
    } else {
        format!("{}/index.yaml", repo_url)
    };

    if verbose {
        eprintln!("  Fetching versions from: {}", index_url);
    }

    let response = attohttpc::get(&index_url)
        .timeout(Duration::from_secs(10))
        .send()
        .context(format!("Failed to fetch Helm index from {}", index_url))?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch Helm index: HTTP {}", response.status());
    }

    let text = response
        .text()
        .context("Failed to read Helm index response")?;

    // Parse YAML
    use yaml_rust2::{Yaml, YamlLoader};
    let docs = YamlLoader::load_from_str(&text).context("Failed to parse Helm index YAML")?;

    let doc = docs
        .first()
        .ok_or_else(|| anyhow::anyhow!("Empty YAML document"))?;

    // Navigate to entries[chart_name]
    if verbose {
        eprintln!("  Parsing index.yaml for chart '{}'", chart_name);
    }

    let entries = if let Some(hash) = doc.as_hash() {
        hash.get(&Yaml::String("entries".to_string()))
    } else {
        None
    };

    if verbose {
        eprintln!("  Entries field found: {}", entries.is_some());
    }

    let chart_entries = entries.and_then(|e| {
        if let Some(entries_hash) = e.as_hash() {
            entries_hash.get(&Yaml::String(chart_name.to_string()))
        } else {
            None
        }
    });

    if chart_entries.is_none() {
        anyhow::bail!("Chart '{}' not found in repository", chart_name);
    }

    let chart_entries = chart_entries.unwrap();

    // Get all versions
    let versions_array = chart_entries
        .as_vec()
        .ok_or_else(|| anyhow::anyhow!("Invalid chart entries format"))?;

    if verbose {
        eprintln!("  Found {} chart entries", versions_array.len());
    }

    let version_strings: Vec<String> = versions_array
        .iter()
        .filter_map(|entry| {
            if let Some(hash) = entry.as_hash() {
                hash.get(&Yaml::String("version".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    if verbose {
        eprintln!("  Found {} version strings", version_strings.len());
        if !version_strings.is_empty() {
            eprintln!(
                "  Sample versions: {:?}",
                &version_strings[..version_strings.len().min(3)]
            );
        }
    }

    let mut versions: Vec<Version> = version_strings
        .iter()
        .filter_map(|v| {
            // Try with and without 'v' prefix
            let clean_v = v.trim_start_matches('v');
            Version::parse(clean_v).ok()
        })
        .collect();

    // Filter out pre-releases unless explicitly requested
    if !include_prereleases {
        versions.retain(|v| !is_prerelease(v));
    }

    if versions.is_empty() {
        anyhow::bail!("No valid versions found for chart '{}'", chart_name);
    }

    versions.sort();

    // Return with 'v' prefix if original had it
    let latest = versions.last().unwrap();
    let latest_str = if version_strings
        .first()
        .map(|s| s.starts_with('v'))
        .unwrap_or(false)
    {
        format!("v{}", latest)
    } else {
        latest.to_string()
    };

    Ok(latest_str)
}
