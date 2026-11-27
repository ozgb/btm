// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi_limits_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::hpi::{HPI_DIRECTORY_NODE_SIZE, HPI_ENTRY_SIZE, HPI_HEADER_SIZE};
use super::hpi_write::ArchiveFile;
use super::test_support::{authored_archive, u32_at};
use super::{Archive, ArchiveOptions, Fs};

const HEADER: usize = HPI_HEADER_SIZE as usize;
const NODE: usize = HPI_DIRECTORY_NODE_SIZE as usize;
const ENTRY: usize = HPI_ENTRY_SIZE as usize;

/// These authored graphs exercise the host safety policy, not retail acceptance of
/// shared directories. Each level references one next node repeatedly; the last
/// node is empty. Distinct names expose all paths, while identical names hide all
/// but the last subtree.
fn directory_graph(levels: usize, fanout: usize, name: &[u8], duplicate: bool) -> Vec<u8> {
    let stride = NODE + fanout * ENTRY;
    let names = HEADER + levels * stride + NODE;
    let name_size = name.len() + 2;
    let name_count = if duplicate { 1 } else { fanout };
    let mut data = vec![0u8; names + name_count * name_size];
    fn put(data: &mut [u8], at: usize, value: usize) {
        data[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
    }
    data[..4].copy_from_slice(b"HAPI");
    put(&mut data, 4, 0x0001_0000);
    let len = data.len();
    put(&mut data, 8, len);
    put(&mut data, 16, HEADER);
    for i in 0..name_count {
        let at = names + i * name_size;
        data[at..at + name.len()].copy_from_slice(name);
        data[at + name.len()] = b'a' + i as u8;
    }
    for level in 0..levels {
        let node = HEADER + level * stride;
        put(&mut data, node, fanout);
        put(&mut data, node + 4, node + NODE);
        for i in 0..fanout {
            let entry = node + NODE + i * ENTRY;
            let name_index = if duplicate { 0 } else { i };
            put(&mut data, entry, names + name_index * name_size);
            put(&mut data, entry + 4, node + stride);
            data[entry + 8] = 1;
        }
    }
    put(&mut data, HEADER + levels * stride + 4, names);
    data.extend_from_slice(b"Copyright 0000 Cavedog Entertainment");
    data
}

fn require_directory_limit(data: &[u8], options: ArchiveOptions, reason: &str) {
    let mut fs = Fs::new();
    let result =
        fs.mount_archive_reader("limited.hpi", data.to_vec(), data.len() as u64, 0, options);
    let Err(error) = result else {
        panic!("mount succeeded")
    };
    assert!(
        error.is_malformed_archive() && error.to_string().contains(reason),
        "mount = {error}; want a malformed archive and {reason:?}"
    );
    assert!(
        fs.mount_count() == 0 && fs.entries().is_empty(),
        "rejected archive published partial mount state"
    );
}

#[test]
fn archive_directory_expansion_limits() {
    for duplicate in [false, true] {
        // Twenty-eight levels encode hundreds of millions of visits in less than
        // one KiB. The entry budget stops both visible and hidden work.
        require_directory_limit(
            &directory_graph(28, 2, b"", duplicate),
            ArchiveOptions::default(),
            "entry limit",
        );
    }
    // The empty terminal node still consumes depth, even though it has no entries
    // to count. Reject before growing the stack for that directory.
    require_directory_limit(
        &directory_graph(64, 1, b"", false),
        ArchiveOptions::default(),
        "depth limit",
    );
    // The first pass sees repeated long-name pointers before path creation; an
    // entry count alone cannot constrain this repeated scanning work.
    require_directory_limit(
        &directory_graph(1, 32, &vec![b'x'; 1 << 20], true),
        ArchiveOptions::default(),
        "string work limit",
    );
}

#[test]
fn archive_directory_exact_budgets_and_shared_references() {
    let data = directory_graph(2, 2, b"", false);
    // Six entries, root plus two directory levels, twelve two-byte name scans, and
    // joined paths of 1, 1, 3, 3, 3, 3 bytes counted twice.
    let exact = ArchiveOptions {
        max_directory_entries: 6,
        max_directory_depth: 3,
        max_directory_string_bytes: 52,
        ..ArchiveOptions::default()
    };
    let archive = Archive::new("shared.hpi", data.clone(), data.len() as u64, exact).unwrap();
    let want: [&[u8]; 6] = [b"a", b"a/a", b"a/b", b"b", b"b/a", b"b/b"];
    let audit = archive.audit_entries();
    for (i, entry) in audit.iter().enumerate() {
        assert!(
            i < want.len()
                && entry.path == want[i]
                && entry.original_path == want[i]
                && entry.source.source_path.as_os_str() == "shared.hpi",
            "entry {i} = {entry:?}; want authored order/path/provenance"
        );
    }
    assert!(
        audit.len() == want.len() && archive.retail_entries().count() == want.len(),
        "benign shared references lost entries"
    );
    for (name, options) in [
        (
            "entry limit",
            ArchiveOptions {
                max_directory_entries: 5,
                ..ArchiveOptions::default()
            },
        ),
        (
            "depth limit",
            ArchiveOptions {
                max_directory_depth: 2,
                ..ArchiveOptions::default()
            },
        ),
        (
            "string work limit",
            ArchiveOptions {
                max_directory_string_bytes: 51,
                ..ArchiveOptions::default()
            },
        ),
    ] {
        require_directory_limit(&data, options, name);
    }
}

#[test]
fn archive_directory_string_budget_before_normalization() {
    // Normalization removes this repeated prefix. Work must be charged from the
    // joined inputs, not the shorter resulting logical path.
    let mut data = authored_archive(&[ArchiveFile::new("entry", "x")], false);
    let root = u32_at(&data, 16);
    let list = u32_at(&data, root + 4);
    let name = u32_at(&data, list);
    data[name..name + 5].copy_from_slice(b"././a");
    for budget in [5, 6, 11, 12, 21] {
        // A scan needs six bytes including NUL, both scans need twelve, and path
        // materialization needs another ten even though the result is "a".
        require_directory_limit(
            &data,
            ArchiveOptions {
                max_directory_string_bytes: budget,
                ..ArchiveOptions::default()
            },
            "string work limit",
        );
    }
    let options = ArchiveOptions {
        max_directory_string_bytes: 22,
        ..ArchiveOptions::default()
    };
    let a = Archive::new("normalized.hpi", data.clone(), data.len() as u64, options).unwrap();
    assert!(
        a.lookup(b"a").is_some(),
        "accepted path normalization changed"
    );
}

#[test]
fn archive_directory_nonpositive_limits_use_defaults() {
    let defaults = ArchiveOptions::default().with_defaults();
    let negative = ArchiveOptions {
        max_directory_entries: -1,
        max_directory_depth: -1,
        max_directory_string_bytes: -1,
        ..ArchiveOptions::default()
    }
    .with_defaults();
    assert!(
        defaults.max_directory_entries == negative.max_directory_entries
            && defaults.max_directory_depth == negative.max_directory_depth
            && defaults.max_directory_string_bytes == negative.max_directory_string_bytes,
        "nonpositive limits must select the same safe defaults"
    );
    let data = directory_graph(2, 2, b"", false);
    Archive::new("defaults.hpi", data.clone(), data.len() as u64, negative).unwrap();
}
