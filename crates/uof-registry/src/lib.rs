//! OCI Registry client for UOF plugin distribution.

pub mod client;
pub mod digest;
pub mod error;
pub mod manifest;

pub use client::{OciClient, OciRef};
pub use digest::digest_bytes;
pub use error::{RegistryError, RegistryResult};
pub use manifest::{OciManifest, OciManifestLayer};

/// Well-known media types for OCI plugin artifacts.
pub mod media_type {
    /// OCI config blob.
    pub const PLUGIN_CONFIG: &str = "application/vnd.uof.plugin.config.v1+json";
    /// OCI layer blob — eBPF ELF binary.
    pub const EBPF_BINARY: &str = "application/vnd.uof.plugin.ebpf.v1+octet-stream";
    /// OCI layer blob — plugin metadata.
    pub const PLUGIN_META: &str = "application/vnd.uof.plugin.meta.v1+json";
    /// OCI layer blob — signature.
    pub const SIGNATURE: &str = "application/vnd.uof.plugin.signature.v1+json";
}
