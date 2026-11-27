// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/path.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Logical path normalization.
//!
//! Retail HPI lookup splits on backslash only, treats `/` as a literal name
//! character, has no special handling of `.` or `..`, and scans entries last to
//! first. That contract is preserved inside archive indexing: entry names from the
//! directory contain no separators, and duplicate entries are resolved by last-wins.
//! The overlay intentionally accepts both slash styles, drops `.` and rejects `..`
//! for host-filesystem ergonomics.

use super::{show, Error};

/// The case-folded logical path the overlay uses for `name`.
pub(crate) fn clean_path(name: &[u8]) -> Result<Vec<u8>, Error> {
    let replaced: Vec<u8> = name
        .iter()
        .map(|&b| if b == b'\\' { b'/' } else { b })
        .collect();
    if replaced.is_empty() || replaced == b"." {
        return Ok(Vec::new());
    }
    if replaced.first() == Some(&b'/') {
        return Err(Error::InvalidPath(format!(
            "vfs: absolute path {:?} is not allowed",
            show(&replaced)
        )));
    }
    let mut out: Vec<u8> = Vec::with_capacity(replaced.len());
    for part in replaced.split(|&b| b == b'/') {
        match part {
            b"" | b"." => continue,
            b".." => {
                return Err(Error::InvalidPath(format!(
                    "vfs: parent path {:?} is not allowed",
                    show(&replaced)
                )))
            }
            _ => {
                if !out.is_empty() {
                    out.push(b'/');
                }
                out.extend_from_slice(part);
            }
        }
    }
    fold_logical_name_in_place(&mut out);
    Ok(out)
}

/// `clean_path(parent + "/" + child)`, or `clean_path(child)` at the root.
pub(crate) fn join_path(parent: &[u8], child: &[u8]) -> Result<Vec<u8>, Error> {
    if parent.is_empty() {
        return clean_path(child);
    }
    let mut joined = Vec::with_capacity(parent.len() + 1 + child.len());
    joined.extend_from_slice(parent);
    joined.push(b'/');
    joined.extend_from_slice(child);
    clean_path(&joined)
}

/// The authored path, joined without normalization.
pub(crate) fn original_join(parent: &[u8], child: &[u8]) -> Vec<u8> {
    if parent.is_empty() {
        return child.to_vec();
    }
    let mut joined = Vec::with_capacity(parent.len() + 1 + child.len());
    joined.extend_from_slice(parent);
    joined.push(b'/');
    joined.extend_from_slice(child);
    joined
}

/// Folds ASCII `A`-`Z` and preserves every other byte. Bytes above ASCII keep
/// their authored value: the retail code-page mapping for them is not established,
/// and retaining them avoids inventing a Unicode equivalence.
pub(crate) fn fold_logical_name(name: &[u8]) -> Vec<u8> {
    let mut out = name.to_vec();
    fold_logical_name_in_place(&mut out);
    out
}

fn fold_logical_name_in_place(name: &mut [u8]) {
    name.make_ascii_lowercase();
}
