// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/vfs_range_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::hpi_write::ArchiveFile;
use super::test_support::{authored_archive, write_file, TempDir};
use super::{ArchiveOptions, Fs};

/// The two range providers follow one contract, because the same logical read must
/// not depend on whether the file is loose or archived:
///
/// - a length past the end is clamped to what remains, and the allocation is
///   bounded by the file rather than by the caller's word;
/// - `None` reads to the end;
/// - an offset exactly at the end is an empty read, not an error;
/// - an offset past the end is refused.
#[test]
fn read_file_range_providers_agree() {
    let whole: Vec<u8> = (0..3000).map(|i: usize| (i * 7 + 1) as u8).collect();
    let name = "ranged.bin";
    let dir = TempDir::new();
    write_file(dir.join(name), &whole);
    let mut loose = Fs::new();
    loose.mount_directory(dir.path(), 1).unwrap();

    for compress in [false, true] {
        let mut archived = Fs::new();
        let data = authored_archive(&[ArchiveFile::new(name, &whole)], compress);
        let size = data.len() as u64;
        archived
            .mount_archive_reader("ranged.hpi", data, size, 1, ArchiveOptions::default())
            .unwrap_or_else(|e| panic!("mount archive (compress={compress}): {e}"));

        for (label, fs) in [("loose", &loose), ("archived", &archived)] {
            let label = if label == "archived" && compress {
                "archived-compressed"
            } else {
                label
            };
            // A length far past the end is clamped, and the allocation with it.
            for offset in [0, 1, whole.len() / 2, whole.len() - 1] {
                let got = fs
                    .read_file_range(name, offset as u64, Some(1 << 28))
                    .unwrap_or_else(|e| panic!("{label}: oversize length at {offset}: {e}"));
                assert!(
                    got == whole[offset..],
                    "{label}: oversize length at {offset}"
                );
                assert!(
                    got.capacity() <= whole.len(),
                    "{label}: oversize length at {offset} allocated {} bytes for a {}-byte file",
                    got.capacity(),
                    whole.len()
                );
            }
            // None reads to the end.
            let tail = fs
                .read_file_range(name, whole.len() as u64 - 16, None)
                .unwrap();
            assert!(tail == whole[whole.len() - 16..], "{label}: read to end");
            // An offset exactly at the end is an empty read.
            let empty = fs
                .read_file_range(name, whole.len() as u64, Some(64))
                .unwrap_or_else(|e| panic!("{label}: offset at the end: {e}"));
            assert!(
                empty.is_empty(),
                "{label}: offset at the end returned {} bytes",
                empty.len()
            );
            // An offset past the end is refused by both providers.
            let past = whole.len() as u64 + 1;
            assert!(
                fs.read_file_range(name, past, Some(64)).is_err(),
                "{label}: offset past the end was accepted"
            );
            assert!(
                fs.read_file_range(name, past, None).is_err(),
                "{label}: offset past the end with no length was accepted"
            );
        }
    }

    // The loose provider names itself in its refusal; the archive reader names a
    // malformed archive.
    let error = loose
        .read_file_range(name, whole.len() as u64 + 1, Some(64))
        .unwrap_err();
    assert!(error.is_not_found(), "loose past-the-end error = {error}");
}
