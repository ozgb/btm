// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/mount_rejection_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::host;
use super::hpi_write::ArchiveFile;
use super::test_support::{authored_archive, first_chunk_offset, put_u32, write_file, TempDir};
use super::Fs;

/// The container gate rejects one discovered candidate, preserving later
/// providers and loose content. Explicit archive mounts still fail.
#[test]
fn discovered_archive_rejection_preserves_later_providers() {
    let valid = authored_archive(&[ArchiveFile::new("archive.txt", "archive")], false);
    let mut marker = b"ZIP!".to_vec();
    marker.extend_from_slice(&valid[4..]);
    let mut version = valid[..4].to_vec();
    version.extend_from_slice(&[0, 0, 2, 0]);
    version.extend_from_slice(&valid[8..]);
    let mut footer = valid[..valid.len() - 1].to_vec();
    footer.push(b'!');
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("short", valid[..10].to_vec()),
        ("marker", marker),
        ("version", version),
        ("footer", footer),
    ];
    for (name, data) in cases {
        let dir = TempDir::new();
        let bad = dir.join("a-rejected.ufo");
        write_file(&bad, &data);
        write_file(dir.join("z-valid.hpi"), &valid);
        write_file(dir.join("loose.txt"), "loose");
        let mut fs = Fs::new();
        fs.mount_game_directory(dir.path()).unwrap();
        for file in ["archive.txt", "loose.txt"] {
            fs.read_file(file)
                .unwrap_or_else(|e| panic!("{name}: surviving content {file}: {e}"));
        }
        let notes = fs.notes().join("\n");
        assert!(
            notes.contains("a-rejected.ufo") && !notes.contains(&host::lossy(dir.path())),
            "{name}: rejection must retain portable provider identity: {notes}"
        );
        // A rejected discovery must not poison direct-call dedup, nor a later retry
        // after the download is repaired.
        let explicit = fs.mount_archive(&bad, 20);
        assert!(
            explicit.as_ref().is_err_and(|e| e.is_rejected_archive()),
            "{name}: explicit rejected archive: {explicit:?}"
        );
        write_file(&bad, &valid);
        let retry = fs.mount_archive(&bad, 20);
        assert!(
            matches!(retry, Ok(Some(_))),
            "{name}: repaired archive retry: {retry:?}"
        );
        fs.close().unwrap();
    }
}

#[test]
fn discovered_archive_directory_failure_remains_fatal() {
    let mut data = authored_archive(&[ArchiveFile::new("file", "data")], false);
    put_u32(&mut data, 16, u32::MAX);
    let dir = TempDir::new();
    write_file(dir.join("invalid-directory.hpi"), &data);
    let mut fs = Fs::new();
    let error = fs.mount_game_directory(dir.path()).unwrap_err();
    assert!(
        error.is_malformed_archive() && !error.is_rejected_archive(),
        "directory failure must not be container-gate rejection: {error}"
    );
}

#[test]
fn discovered_archive_payload_failure_remains_read_error() {
    let mut data = authored_archive(&[ArchiveFile::new("file", b"data".repeat(100))], true);
    let chunk = first_chunk_offset(&data);
    data[chunk] = b'!';
    let dir = TempDir::new();
    write_file(dir.join("bad-payload.hpi"), &data);
    let mut fs = Fs::new();
    fs.mount_game_directory(dir.path()).unwrap();
    let error = fs.read_file("file").unwrap_err();
    assert!(
        error.is_malformed_archive() && !error.is_rejected_archive(),
        "payload failure must remain a read error: {error}"
    );
    fs.close().unwrap();
}
