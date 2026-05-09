//! SHA-256 digest computation for content-addressable storage.

use sha2::{Digest, Sha256};

/// Compute the hex-encoded SHA-256 digest of a byte slice.
pub fn digest_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    const KNOWN_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn test_empty_digest() {
        assert_eq!(digest_bytes(&[]), KNOWN_SHA256);
    }

    #[test]
    fn test_hello_digest() {
        assert_eq!(
            digest_bytes(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
