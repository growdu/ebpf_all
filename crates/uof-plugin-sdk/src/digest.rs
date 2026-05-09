//! SHA-256 digest for plugin artifact integrity verification.

use sha2::{Digest, Sha256};

/// Compute the hex-encoded SHA-256 digest of a byte slice.
pub fn digest_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
