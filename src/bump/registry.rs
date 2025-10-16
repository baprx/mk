use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct TerraformModule {
    versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OciTagsResponse {
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OciTokenResponse {
    token: Option<String>,
    access_token: Option<String>,
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
/// Returns (version, appVersion)
pub fn fetch_helm_chart_version(
    repo_url: &str,
    chart_name: &str,
    verbose: bool,
    include_prereleases: bool,
) -> Result<(String, Option<String>)> {
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

    // Build a map of version string to chart entry for lookup
    let version_to_entry: std::collections::HashMap<String, &Yaml> = versions_array
        .iter()
        .filter_map(|entry| {
            if let Some(hash) = entry.as_hash() {
                hash.get(&Yaml::String("version".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|version_str| (version_str.to_string(), entry))
            } else {
                None
            }
        })
        .collect();

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

    // Find the appVersion for the latest version
    let app_version = version_to_entry
        .get(&latest_str)
        .or_else(|| {
            // Try without 'v' prefix
            version_to_entry.get(&latest.to_string())
        })
        .and_then(|entry| {
            if let Some(hash) = entry.as_hash() {
                hash.get(&Yaml::String("appVersion".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        });

    Ok((latest_str, app_version))
}

/// Parse OCI registry URL to extract registry hostname and repository path
/// Example: oci://ghcr.io/org/chart -> ("ghcr.io", "org/chart")
fn parse_oci_url(oci_url: &str) -> Result<(String, String)> {
    let url = oci_url
        .strip_prefix("oci://")
        .ok_or_else(|| anyhow::anyhow!("Invalid OCI URL format: {}", oci_url))?;

    let parts: Vec<&str> = url.splitn(2, '/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid OCI URL format: {}", oci_url);
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Get authentication token for an OCI registry
/// First tries to use configured token/command, falls back to anonymous token
fn get_oci_token(
    registry: &str,
    repository: &str,
    config: &crate::config::Config,
    verbose: bool,
) -> Result<Option<String>> {
    // Check if there's a configured auth for this registry
    if let Some(auth) = config.bump.oci_registries.get(registry) {
        if let Some(token) = &auth.token {
            if verbose {
                eprintln!("  Using configured token for registry '{}'", registry);
            }
            return Ok(Some(token.clone()));
        }

        if let Some(command) = &auth.command {
            if verbose {
                eprintln!("  Executing command to get token for '{}'", registry);
            }

            let output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .context("Failed to execute token command")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Token command failed: {}", stderr);
            }

            let token = String::from_utf8(output.stdout)
                .context("Token command output is not valid UTF-8")?
                .trim()
                .to_string();

            if verbose {
                eprintln!("  Successfully obtained token from command");
            }

            return Ok(Some(token));
        }
    }

    // Try to get an anonymous token from the registry
    if verbose {
        eprintln!("  Attempting to get anonymous token for '{}'", registry);
    }

    fetch_anonymous_oci_token(registry, repository, verbose)
}

/// Attempt to fetch an anonymous token from the OCI registry
/// Many registries provide anonymous read access via their token endpoint
fn fetch_anonymous_oci_token(
    registry: &str,
    repository: &str,
    verbose: bool,
) -> Result<Option<String>> {
    // Docker Hub uses a different authentication endpoint
    let token_urls = if registry == "registry-1.docker.io" {
        vec![format!(
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
            repository
        )]
    } else {
        // Try common token endpoint patterns for other registries
        vec![
            format!(
                "https://{}/token?scope=repository:{}:pull",
                registry, repository
            ),
            format!(
                "https://{}/v2/token?scope=repository:{}:pull",
                registry, repository
            ),
        ]
    };

    for token_url in token_urls {
        if verbose {
            eprintln!("  Trying token endpoint: {}", token_url);
        }

        match attohttpc::get(&token_url)
            .timeout(Duration::from_secs(10))
            .send()
        {
            Ok(response) if response.status().is_success() => {
                if let Ok(token_response) = response.json::<OciTokenResponse>() {
                    let token = token_response.token.or(token_response.access_token);
                    if token.is_some() {
                        if verbose {
                            eprintln!("  Successfully obtained anonymous token");
                        }
                        return Ok(token);
                    }
                }
            }
            _ => continue,
        }
    }

    if verbose {
        eprintln!("  No anonymous token available, will try without authentication");
    }

    Ok(None)
}

/// Fetch the latest version of a Helm chart from an OCI registry
pub fn fetch_helm_chart_version_oci(
    oci_url: &str,
    chart_name: &str,
    config: &crate::config::Config,
    verbose: bool,
    include_prereleases: bool,
) -> Result<String> {
    let (registry, repository) = parse_oci_url(oci_url)?;

    if verbose {
        eprintln!(
            "  Fetching OCI chart '{}' from {}/{}",
            chart_name, registry, repository
        );
    }

    // For Docker Hub and some ghcr.io repos, the chart name needs to be appended to the repository path
    // Docker Hub format: registry-1.docker.io/v2/bitnamicharts/mariadb/tags/list
    // ghcr.io format (varies):
    //   - ghcr.io/v2/grafana/helm-charts/grafana-operator/tags/list (needs chart name)
    //   - ghcr.io/v2/prometheus-community/charts/prometheus/tags/list (already has chart name)
    let full_repository = if registry == "registry-1.docker.io" {
        // Docker Hub always needs chart name appended
        format!("{}/{}", repository, chart_name)
    } else if registry == "ghcr.io" && !repository.ends_with(&format!("/{}", chart_name)) {
        // ghcr.io: only append if not already present
        format!("{}/{}", repository, chart_name)
    } else {
        repository.clone()
    };

    // Get authentication token (use original repository for token scope)
    let token = get_oci_token(&registry, &full_repository, config, verbose)?;

    // Build the tags list URL
    let tags_url = format!("https://{}/v2/{}/tags/list", registry, full_repository);

    if verbose {
        eprintln!("  Fetching tags from: {}", tags_url);
    }

    // Build request with optional authentication
    let mut request = attohttpc::get(&tags_url).timeout(Duration::from_secs(10));

    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().context(format!(
        "Failed to fetch tags from OCI registry: {}",
        tags_url
    ))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch OCI tags: HTTP {} for {}",
            response.status(),
            tags_url
        );
    }

    let tags_response: OciTagsResponse = response
        .json()
        .context("Failed to parse OCI tags response")?;

    if verbose {
        eprintln!("  Found {} tags", tags_response.tags.len());
        if !tags_response.tags.is_empty() {
            eprintln!(
                "  Sample tags: {:?}",
                &tags_response.tags[..tags_response.tags.len().min(3)]
            );
        }
    }

    // Parse versions
    let mut versions: Vec<Version> = tags_response
        .tags
        .iter()
        .filter_map(|tag| {
            let clean_tag = tag.trim_start_matches('v');
            Version::parse(clean_tag).ok()
        })
        .collect();

    // Filter out pre-releases unless explicitly requested
    if !include_prereleases {
        versions.retain(|v| !is_prerelease(v));
    }

    if versions.is_empty() {
        anyhow::bail!("No valid versions found for OCI chart '{}'", chart_name);
    }

    versions.sort();

    // Return with 'v' prefix if original had it
    let latest = versions.last().unwrap();
    let latest_str = if tags_response
        .tags
        .first()
        .map(|s| s.starts_with('v'))
        .unwrap_or(false)
    {
        format!("v{}", latest)
    } else {
        latest.to_string()
    };

    if verbose {
        eprintln!("  Latest version: {}", latest_str);
    }

    Ok(latest_str)
}
