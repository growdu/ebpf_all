//! OCI Registry HTTP client (distribution-spec v1.2).

use reqwest::Client;
use serde::Deserialize;

use crate::digest::digest_bytes;
use crate::error::{RegistryError, RegistryResult};
use crate::manifest::OciManifest;

// ---------------------------------------------------------------------------
// Auth types (module-level, not inside impl)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct OciAuth {
    username: Option<String>,
    password: Option<String>,
}

// ---------------------------------------------------------------------------
// OciRef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum OciRef {
    Tag(String),
    Digest(String),
}

impl OciRef {
    pub fn parse(s: &str) -> Self {
        if s.starts_with("sha256:") {
            Self::Digest(s.to_string())
        } else {
            Self::Tag(s.to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        match self { Self::Tag(s) => s, Self::Digest(s) => s }
    }
}

// ---------------------------------------------------------------------------
// OciClient
// ---------------------------------------------------------------------------

pub struct OciClient {
    registry: String,
    client: Client,
    auth: Option<OciAuth>,
}

const MAX_BLOB_SIZE: usize = 10 * 1024 * 1024 * 1024;

impl OciClient {
    pub fn new(registry: &str) -> RegistryResult<Self> {
        let registry = registry.trim_end_matches('/').to_string();
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(RegistryError::Request)?;
        Ok(Self { registry, client, auth: None })
    }

    pub fn with_basic_auth(mut self, username: &str, password: &str) -> Self {
        self.auth = Some(OciAuth {
            username: Some(username.to_string()),
            password: Some(password.to_string()),
        });
        self
    }

    // -------------------------------------------------------------------------
    // High-level API
    // -------------------------------------------------------------------------

    pub async fn exists(&self, repo: &str, reference: OciRef) -> RegistryResult<bool> {
        let url = self.manifest_url(repo, &reference);
        let resp = self.http_get(&url).await?;
        Ok(resp.status().as_u16() == 200)
    }

    pub async fn tags(&self, repo: &str) -> RegistryResult<Vec<String>> {
        let url = format!("https://{}/v2/{}/tags/list", self.registry, repo);
        let resp = self.http_get(&url).await?;
        #[derive(Deserialize)]
        struct TagList { tags: Vec<String> }
        let body: TagList = self.json(resp).await?;
        Ok(body.tags)
    }

    pub async fn pull(
        &self,
        repo: &str,
        reference: OciRef,
        media_type: &str,
    ) -> RegistryResult<Vec<u8>> {
        let manifest: OciManifest = self.pull_manifest(repo, reference).await?;
        let layer = manifest.layers.iter()
            .find(|l| l.media_type == media_type)
            .ok_or_else(|| RegistryError::LayerNotFound {
                digest: format!("media_type={media_type}"),
            })?;
        self.pull_blob(repo, &layer.digest, layer.size).await
    }

    pub async fn pull_manifest<T: for<'de> Deserialize<'de>>(
        &self,
        repo: &str,
        reference: OciRef,
    ) -> RegistryResult<T> {
        let url = self.manifest_url(repo, &reference);
        let resp = self.http_get(&url).await?;
        if resp.status().as_u16() == 404 {
            return Err(RegistryError::ManifestNotFound(url));
        }
        let body = self.text(resp).await?;
        serde_json::from_str(&body).map_err(RegistryError::Json)
    }

    pub async fn pull_blob(
        &self,
        repo: &str,
        digest: &str,
        expected_size: u64,
    ) -> RegistryResult<Vec<u8>> {
        let url = format!("https://{}/v2/{}/blobs/{digest}", self.registry, repo);
        let resp = self.http_get(&url).await?;
        if resp.status().as_u16() == 404 {
            return Err(RegistryError::LayerNotFound { digest: digest.to_string() });
        }
        let size = resp.content_length().unwrap_or(0);
        if size > MAX_BLOB_SIZE as u64 {
            return Err(RegistryError::HttpError {
                status: 413,
                body: format!("layer too large: {size} bytes"),
            });
        }
        let bytes = resp.bytes().await.map_err(RegistryError::Request)?.to_vec();
        let actual = digest_bytes(&bytes);
        if !actual.starts_with(digest.trim_start_matches("sha256:")) {
            return Err(RegistryError::DigestMismatch {
                expected: digest.to_string(),
                actual: format!("sha256:{actual}"),
            });
        }
        if expected_size > 0 && bytes.len() as u64 != expected_size {
            return Err(RegistryError::HttpError {
                status: 400,
                body: format!("size mismatch: expected={expected_size} actual={}", bytes.len()),
            });
        }
        Ok(bytes)
    }

    pub async fn push_blob(
        &self,
        repo: &str,
        media_type: &str,
        data: Vec<u8>,
    ) -> RegistryResult<String> {
        let digest = format!("sha256:{}", digest_bytes(&data));
        let stat_url = format!("https://{}/v2/{}/blobs/{digest}", self.registry, repo);
        let stat_resp = self.http_head(&stat_url).await?;
        if stat_resp.status().as_u16() == 200 {
            return Ok(digest);
        }
        let upload_url = format!("https://{}/v2/{}/blobs/uploads/", self.registry, repo);
        let init_resp = self.http_post_empty(&upload_url).await?;
        let location = init_resp
            .headers().get("location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| RegistryError::HttpError {
                status: 500,
                body: "missing Location header".into(),
            })?
            .to_string();
        let complete_url = format!("{location}&digest={digest}");
        let put_resp = self.http_put(&complete_url, data, media_type).await?;
        let put_status = put_resp.status();
        if !put_status.is_success() {
            let body = put_resp.text().await.unwrap_or_default();
            return Err(RegistryError::HttpError { status: put_status.as_u16(), body });
        }
        Ok(digest)
    }

    pub async fn push_manifest<T: serde::Serialize>(
        &self,
        repo: &str,
        reference: &str,
        manifest: &T,
    ) -> RegistryResult<String> {
        let url = self.manifest_url(repo, &OciRef::parse(reference));
        let body = serde_json::to_vec(manifest).map_err(RegistryError::Json)?;
        let resp = self.http_put(&url, body, "application/vnd.oci.image.manifest.v1+json").await?;
        let resp_status = resp.status();
        if !resp_status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(RegistryError::HttpError { status: resp_status.as_u16(), body });
        }
        resp.headers()
            .get("docker-content-digest")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| RegistryError::HttpError {
                status: 200,
                body: "missing docker-content-digest header".into(),
            })
    }

    // -------------------------------------------------------------------------
    // Low-level HTTP
    // -------------------------------------------------------------------------

    fn manifest_url(&self, repo: &str, reference: &OciRef) -> String {
        format!(
            "https://{}/v2/{}/manifests/{}",
            self.registry, repo, reference.as_str()
        )
    }

    async fn http_get(&self, url: &str) -> RegistryResult<reqwest::Response> {
        let mut req = self.client.get(url);
        if let Some(ref auth) = self.auth {
            if let (Some(ref u), Some(ref p)) = (&auth.username, &auth.password) {
                req = req.basic_auth(u, Some(p));
            }
        }
        req.send().await.map_err(RegistryError::Request)
    }

    async fn http_head(&self, url: &str) -> RegistryResult<reqwest::Response> {
        self.client.head(url).send().await.map_err(RegistryError::Request)
    }

    async fn http_post_empty(&self, url: &str) -> RegistryResult<reqwest::Response> {
        self.client.post(url).send().await.map_err(RegistryError::Request)
    }

    async fn http_put(
        &self,
        url: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> RegistryResult<reqwest::Response> {
        self.client
            .put(url)
            .header("content-type", content_type)
            .body(body)
            .send()
            .await
            .map_err(RegistryError::Request)
    }

    async fn text(&self, resp: reqwest::Response) -> RegistryResult<String> {
        resp.text().await.map_err(RegistryError::Request)
    }

    async fn json<T: for<'de> Deserialize<'de>>(&self, resp: reqwest::Response) -> RegistryResult<T> {
        resp.json().await.map_err(RegistryError::Request)
    }
}
