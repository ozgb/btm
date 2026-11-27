//! SHA-256 for content hashes and manifests, backed by the `sha2` crate.

use sha2::Digest;

/// The SHA-256 of `data`.
pub(crate) fn sha256(data: &[u8]) -> [u8; 32] {
    sha2::Sha256::digest(data).into()
}

/// An incremental SHA-256.
pub(crate) struct Sha256(sha2::Sha256);

impl Sha256 {
    pub(crate) fn new() -> Sha256 {
        Sha256(sha2::Sha256::new())
    }

    pub(crate) fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub(crate) fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}

/// Lower-case hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        // Writing to a String never fails.
        let _ = write!(out, "{b:02x}");
    }
    out
}
