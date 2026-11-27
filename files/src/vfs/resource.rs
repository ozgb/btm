// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/resource.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Resource request paths.

/// The resource request the retail unit compiler forms, over the raw bytes of a
/// logical name. Its extension replacement scans the whole assembled path,
/// directory components included: it cuts at the last `.` anywhere, which is not
/// `Path::extension` semantics. Canonicalization and validation stay with the
/// receiving namespace.
pub fn resource_path(directory: &[u8], name: &[u8], extension: &[u8]) -> Vec<u8> {
    let mut logical = Vec::with_capacity(directory.len() + name.len() + extension.len() + 2);
    logical.extend_from_slice(directory);
    logical.push(b'/');
    logical.extend_from_slice(name);
    if let Some(dot) = logical.iter().rposition(|&b| b == b'.') {
        logical.truncate(dot);
    }
    logical.push(b'.');
    logical.extend_from_slice(extension);
    logical
}
