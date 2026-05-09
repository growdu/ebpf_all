//! CLI command handlers for interacting with the UOF Control Plane API
//! and local plugin packaging / registry operations.

use clap::Subcommand;
use std::io::{Read, Write};
use reqwest::Client;
use serde::Serialize;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Shared output helpers
// ---------------------------------------------------------------------------

fn print_json<T: Serialize>(v: &T) -> anyhow::Result<()> {
    let s = serde_json::to_string_pretty(v)?;
    println!("{s}");
    Ok(())
}

fn exit_with(code: u8, msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(code as i32);
}

// ---------------------------------------------------------------------------
// Agent commands
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum AgentCommands {
    /// List all registered agents (placeholder — server returns internal state).
    List,
    /// Show the health status of the control plane.
    Health,
}

pub async fn handle_agent(client: &Client, base_url: &str, sub: AgentCommands) -> anyhow::Result<()> {
    match sub {
        AgentCommands::List => {
            let url = format!("{base_url}/healthz");
            let resp = client.get(&url).send().await?;
            if resp.status().is_success() {
                println!("Control plane is reachable at {base_url}.");
                println!("Agent listing is not yet exposed via API.");
            } else {
                exit_with(1, &format!("Control plane returned {}", resp.status()));
            }
        }
        AgentCommands::Health => {
            let url = format!("{base_url}/healthz");
            let resp = client.get(&url).send().await?;
            let body = resp.text().await?;
            println!("{body}");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Plugin commands (API + local pack/push/pull)
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum PluginCommands {
    /// List all plugins.
    List,
    /// Create a new plugin.
    Create {
        /// Plugin name.
        name: String,
        /// Plugin kind (e.g. "observability", "security").
        kind: String,
        /// Publisher identifier.
        publisher: String,
    },
    /// Show details of a specific plugin.
    Get {
        /// Plugin ID.
        #[arg(value_parser = uuid::Uuid::parse_str)]
        plugin_id: String,
    },
    /// Upload a new version metadata record for a plugin.
    AddVersion {
        /// Plugin ID.
        #[arg(value_parser = uuid::Uuid::parse_str)]
        plugin_id: String,
        /// Version string (e.g. "0.1.0").
        version: String,
        /// Content hash of the plugin artifact.
        digest: String,
        /// OCI artifact reference.
        oci_ref: String,
    },
    /// Publish a plugin version.
    Release {
        /// Plugin ID.
        #[arg(value_parser = uuid::Uuid::parse_str)]
        plugin_id: String,
        /// Version to publish.
        version: String,
        /// Set this version as the default.
        #[arg(long)]
        default: bool,
    },
    /// Pack a local plugin directory into a gzipped tarball.
    Pack {
        /// Path to the plugin directory (must contain manifest.yaml).
        #[arg(long)]
        dir: String,
        /// Output file path. Use "-" to write to stdout.
        #[arg(long, default_value = "plugin.tar.gz")]
        output: String,
    },
    /// Push a packed plugin artifact to an OCI registry.
    Push {
        /// OCI registry host (e.g. "ghcr.io" or "registry.example.com").
        #[arg(long)]
        registry: String,
        /// Repository name (e.g. "myorg/postgres-observability").
        #[arg(long)]
        repo: String,
        /// Tag for this version (e.g. "0.1.0").
        #[arg(long)]
        tag: String,
        /// Path to the packed plugin tarball.
        #[arg(long)]
        artifact: String,
        /// Optional username for registry auth.
        #[arg(long)]
        username: Option<String>,
        /// Optional password for registry auth.
        #[arg(long)]
        password: Option<String>,
    },
    /// Pull a plugin artifact from an OCI registry.
    Pull {
        /// OCI registry host (e.g. "ghcr.io" or "registry.example.com").
        #[arg(long)]
        registry: String,
        /// Repository name (e.g. "myorg/postgres-observability").
        #[arg(long)]
        repo: String,
        /// Tag or digest to pull.
        #[arg(long, default_value = "latest")]
        tag: String,
        /// Output file path. Use "-" to write to stdout.
        #[arg(long, default_value = "plugin.tar.gz")]
        output: String,
        /// Optional username for registry auth.
        #[arg(long)]
        username: Option<String>,
        /// Optional password for registry auth.
        #[arg(long)]
        password: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct CreatePluginRequest {
    name: String,
    kind: String,
    publisher: String,
}

#[derive(Debug, Serialize)]
struct CreatePluginVersionRequest {
    version: String,
    digest: String,
    oci_ref: String,
    manifest: serde_json::Value,
    #[serde(default)]
    compat_matrix: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ReleasePluginRequest {
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    make_default: Option<bool>,
}

pub async fn handle_plugin(
    client: &Client,
    base_url: &str,
    sub: PluginCommands,
) -> anyhow::Result<()> {
    match sub {
        // ---- API-backed commands ----
        PluginCommands::List => {
            let url = format!("{base_url}/api/v1/plugins");
            let resp = client.get(&url).send().await?.error_for_status()?;
            let plugins: Vec<serde_json::Value> = resp.json().await?;
            if plugins.is_empty() {
                println!("No plugins found.");
            } else {
                print_json(&plugins)?;
            }
        }
        PluginCommands::Create { name, kind, publisher } => {
            let url = format!("{base_url}/api/v1/plugins");
            let body = CreatePluginRequest { name, kind, publisher };
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
            let plugin: serde_json::Value = resp.json().await?;
            print_json(&plugin)?;
        }
        PluginCommands::Get { plugin_id } => {
            let url = format!("{base_url}/api/v1/plugins/{plugin_id}");
            let resp = client.get(&url).send().await?;
            if resp.status() == 404 {
                exit_with(1, &format!("Plugin {plugin_id} not found"));
            }
            let plugin: serde_json::Value = resp.error_for_status()?.json().await?;
            print_json(&plugin)?;
        }
        PluginCommands::AddVersion { plugin_id, version, digest, oci_ref } => {
            let url = format!("{base_url}/api/v1/plugins/{plugin_id}/versions");
            let body = CreatePluginVersionRequest {
                version,
                digest,
                oci_ref,
                manifest: serde_json::json!({}),
                compat_matrix: serde_json::json!({}),
            };
            let resp = client.post(&url).json(&body).send().await?;
            if resp.status() == 404 {
                exit_with(1, &format!("Plugin {plugin_id} not found"));
            }
            let version: serde_json::Value = resp.error_for_status()?.json().await?;
            print_json(&version)?;
        }
        PluginCommands::Release { plugin_id, version, default } => {
            let url = format!("{base_url}/api/v1/plugins/{plugin_id}/release");
            let body = ReleasePluginRequest {
                version,
                make_default: if default { Some(true) } else { None },
            };
            let resp = client.post(&url).json(&body).send().await?;
            if resp.status() == 404 {
                exit_with(1, &format!("Plugin {plugin_id} not found"));
            }
            println!("Release accepted.");
        }

        // ---- Local / offline commands ----
        PluginCommands::Pack { dir, output } => {
            let dir = PathBuf::from(&dir);
            let mut packager = uof_plugin_sdk::PluginPackager::new(&dir)?;

            // Load and validate manifest.yaml
            let manifest_path = dir.join("manifest.yaml");
            if !manifest_path.exists() {
                exit_with(1, &format!("manifest.yaml not found in {}", dir.display()));
            }
            let yaml = std::fs::read_to_string(&manifest_path)?;
            let manifest = uof_plugin_sdk::PluginManifest::from_yaml(&yaml)
                .map_err(|e| anyhow::anyhow!("invalid manifest: {e}"))?;
            packager.set_manifest(manifest)?;

            // Collect artifacts
            let artifacts_dir = dir.join("artifacts");
            if artifacts_dir.is_dir() {
                collect_files(&artifacts_dir, "artifacts", &mut packager)?;
            }

            let mut buf = Vec::new();
            packager.pack(&mut buf)?;
            let digest = uof_plugin_sdk::digest_bytes(&buf);
            println!("Packed {} bytes, digest: sha256:{}", buf.len(), digest);

            if output == "-" {
                std::io::stdout().write_all(&buf)?;
            } else {
                std::fs::write(&output, &buf)?;
                println!("Written: {output}");
            }
        }

        PluginCommands::Push { registry, repo, tag, artifact, username, password } => {
            let artifact_bytes = if artifact == "-" {
                { let mut tmp = Vec::new(); std::io::stdin().read_to_end(&mut tmp)?; tmp }
            } else {
                std::fs::read(&artifact)?
            };

            let digest = uof_plugin_sdk::digest_bytes(&artifact_bytes);
            println!("Artifact digest: sha256:{digest}");
            println!("Pushing to {registry}/{repo}:{tag} ...");

            let mut oci_client = uof_registry::OciClient::new(&registry)
                .map_err(|e| anyhow::anyhow!("failed to create OCI client: {e}"))?;

            if let (Some(user), Some(pass)) = (&username, &password) {
                oci_client = oci_client.with_basic_auth(user, pass);
            }

            // Push blob
            let blob_digest = oci_client.push_blob(
                &repo,
                uof_registry::media_type::EBPF_BINARY,
                artifact_bytes,
            ).await
            .map_err(|e| anyhow::anyhow!("failed to push blob: {e}"))?;

            // Build and push manifest
            let mut manifest = uof_registry::OciManifest::new();
            manifest.add_layer(uof_registry::OciManifestLayer {
                media_type: uof_registry::media_type::EBPF_BINARY.to_string(),
                size: blob_digest.len() as u64,
                digest: blob_digest.clone(),
                urls: None,
                platform: None,
                annotations: None,
            });

            let manifest_digest = oci_client.push_manifest(&repo, &tag, &manifest).await
                .map_err(|e| anyhow::anyhow!("failed to push manifest: {e}"))?;

            println!("Pushed successfully.");
            println!("  Blob digest:  sha256:{}", blob_digest);
            println!("  Manifest ref: {tag} -> sha256:{manifest_digest}");
        }

        PluginCommands::Pull { registry, repo, tag, output, username, password } => {
            let mut oci_client = uof_registry::OciClient::new(&registry)
                .map_err(|e| anyhow::anyhow!("failed to create OCI client: {e}"))?;

            if let (Some(user), Some(pass)) = (&username, &password) {
                oci_client = oci_client.with_basic_auth(user, pass);
            }

            println!("Pulling {registry}/{repo}:{tag} ...");

            let bytes = oci_client.pull(
                &repo,
                uof_registry::OciRef::parse(&tag),
                uof_registry::media_type::EBPF_BINARY,
            ).await
            .map_err(|e| anyhow::anyhow!("failed to pull: {e}"))?;

            let digest = uof_plugin_sdk::digest_bytes(&bytes);
            println!("Downloaded {} bytes, digest: sha256:{}", bytes.len(), digest);

            if output == "-" {
                std::io::stdout().write_all(&bytes)?;
            } else {
                std::fs::write(&output, &bytes)?;
                println!("Written: {output}");
            }
        }
    }
    Ok(())
}

/// Recursively collect files under a directory prefix into the packager.
fn collect_files(
    dir: &std::path::Path,
    prefix: &str,
    packager: &mut uof_plugin_sdk::PluginPackager,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = format!("{}/{}", prefix, path.file_name().unwrap().to_string_lossy());
        if path.is_dir() {
            collect_files(&path, &rel, packager)?;
        } else {
            packager.add_meta_file(&rel)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Template commands
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum TemplateCommands {
    /// List all templates.
    List,
    /// Create a new template.
    Create {
        /// Plugin ID this template belongs to.
        #[arg(long, value_parser = uuid::Uuid::parse_str)]
        plugin_id: String,
        /// Template name.
        #[arg(long)]
        name: String,
        /// Template version.
        #[arg(long)]
        version: String,
        /// Target software name (e.g. "postgresql", "nginx").
        #[arg(long)]
        target_software: String,
        /// Optional scenario label.
        #[arg(long)]
        scenario: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct CreateTemplateRequest {
    plugin_id: uuid::Uuid,
    name: String,
    version: String,
    target_software: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scenario: Option<String>,
    manifest: serde_json::Value,
}

pub async fn handle_template(
    client: &Client,
    base_url: &str,
    sub: TemplateCommands,
) -> anyhow::Result<()> {
    match sub {
        TemplateCommands::List => {
            let url = format!("{base_url}/api/v1/templates");
            let resp = client.get(&url).send().await?.error_for_status()?;
            let templates: Vec<serde_json::Value> = resp.json().await?;
            if templates.is_empty() {
                println!("No templates found.");
            } else {
                print_json(&templates)?;
            }
        }
        TemplateCommands::Create { plugin_id, name, version, target_software, scenario } => {
            let url = format!("{base_url}/api/v1/templates");
            let body = CreateTemplateRequest {
                plugin_id: uuid::Uuid::parse_str(&plugin_id)
                    .map_err(|e| anyhow::anyhow!("invalid plugin_id: {e}"))?,
                name,
                version,
                target_software,
                scenario,
                manifest: serde_json::json!({}),
            };
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
            let tmpl: serde_json::Value = resp.json().await?;
            print_json(&tmpl)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Template binding commands
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum BindingCommands {
    /// Create a template binding.
    Create {
        /// Template ID to bind.
        #[arg(long, value_parser = uuid::Uuid::parse_str)]
        template_id: String,
        /// JSON selector (e.g. '{"env":"prod"}').
        #[arg(long)]
        selector: String,
        /// JSON target (e.g. '{"host":"db-1"}').
        #[arg(long)]
        target: String,
        /// Optional JSON policy.
        #[arg(long)]
        policy: Option<String>,
        /// Whether the binding is enabled.
        #[arg(long, default_value = "true")]
        enabled: bool,
    },
    /// Delete a template binding.
    Delete {
        /// Binding ID to delete.
        #[arg(value_parser = uuid::Uuid::parse_str)]
        binding_id: String,
    },
}

fn parse_json(s: &str) -> anyhow::Result<serde_json::Value> {
    serde_json::from_str(s).map_err(|e| anyhow::anyhow!("invalid JSON: {e}"))
}

#[derive(Debug, Serialize)]
struct CreateTemplateBindingRequest {
    template_id: uuid::Uuid,
    selector: serde_json::Value,
    target: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
}

pub async fn handle_binding(
    client: &Client,
    base_url: &str,
    sub: BindingCommands,
) -> anyhow::Result<()> {
    match sub {
        BindingCommands::Create { template_id, selector, target, policy, enabled } => {
            let url = format!("{base_url}/api/v1/template-bindings");
            let body = CreateTemplateBindingRequest {
                template_id: uuid::Uuid::parse_str(&template_id)
                    .map_err(|e| anyhow::anyhow!("invalid template_id: {e}"))?,
                selector: parse_json(&selector)?,
                target: parse_json(&target)?,
                policy: policy.as_ref().map(|s| parse_json(s)).transpose()?,
                enabled: Some(enabled),
            };
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
            let binding: serde_json::Value = resp.json().await?;
            print_json(&binding)?;
        }
        BindingCommands::Delete { binding_id } => {
            let url = format!("{base_url}/api/v1/template-bindings/{binding_id}");
            let resp = client.delete(&url).send().await?;
            if resp.status() == 404 {
                exit_with(1, &format!("Binding {binding_id} not found"));
            }
            if resp.status() == 204 {
                println!("Binding {binding_id} deleted.");
            } else {
                println!("Unexpected status: {}", resp.status());
            }
        }
    }
    Ok(())
}
