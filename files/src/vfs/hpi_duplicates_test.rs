// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi_duplicates_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::hpi_write::ArchiveFile;
use super::test_support::{authored_archive, u32_at};
use super::{ArchiveOptions, Fs};

/// Every path component selects the last matching directory entry. A replaced
/// directory does not merge its children into its replacement.
#[test]
fn archive_duplicate_directory_does_not_merge_subtrees() {
    for replacement in [
        ArchiveFile::new("second/new.txt", "new"),
        ArchiveFile::new("second", "file"),
    ] {
        let label = String::from_utf8_lossy(&replacement.path).into_owned();
        let mut data = authored_archive(
            &[
                ArchiveFile::new("first/old.txt", "old"),
                replacement.clone(),
            ],
            false,
        );
        let root = u32_at(&data, 16);
        let entries = u32_at(&data, root + 4);
        // The authored writer orders first before second. Point the second entry's
        // name at the first name to author a duplicate component.
        let first_name = data[entries..entries + 4].to_vec();
        data[entries + 9..entries + 13].copy_from_slice(&first_name);
        let mut fs = Fs::new();
        let size = data.len() as u64;
        fs.mount_archive_reader("duplicate.hpi", data, size, 1, ArchiveOptions::default())
            .unwrap();
        let read = fs.read_file("first/old.txt");
        assert!(
            read.as_ref().is_err_and(|e| e.is_not_found()),
            "{label}: shadowed child read = {read:?}; want not found"
        );
        let stat = fs.stat("first/old.txt");
        assert!(
            stat.as_ref().is_err_and(|e| e.is_not_found()),
            "{label}: shadowed child stat = {stat:?}; want not found"
        );
        assert!(
            fs.entries()
                .iter()
                .any(|entry| entry.path == b"first/old.txt"),
            "{label}: raw diagnostic index discarded the shadowed subtree"
        );
        if replacement.path == b"second/new.txt" {
            assert_eq!(fs.read_file("first/new.txt").unwrap(), b"new", "{label}");
            for got in [fs.read_dir("first"), fs.retail_read_dir("first")] {
                let got = got.unwrap();
                assert!(
                    got.len() == 1 && got[0].path == b"first/new.txt",
                    "{label}: winning subtree = {got:?}"
                );
            }
        } else {
            assert_eq!(fs.read_file("first").unwrap(), b"file", "{label}");
            let got = fs.retail_read_dir("first").unwrap();
            assert!(
                got.is_empty(),
                "{label}: file-shadowed subtree enumerated = {got:?}"
            );
        }
        fs.close().unwrap();
    }
}
