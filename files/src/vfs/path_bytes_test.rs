// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/path_bytes_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::hpi_write::{write_archive, ArchiveFile, ArchiveWriteOptions};
use super::path::clean_path;
use super::{ArchiveOptions, Fs};

#[test]
fn authored_path_bytes_survive_archive_and_overlay() {
    // Distinct legacy bytes must not collapse into the Unicode replacement
    // character. High-byte case equivalence remains unknown.
    let names: [&[u8]; 2] = [b"TEXTURES/\xc0.GAF", b"TEXTURES/\xc1.GAF"];
    let data = write_archive(
        &[
            ArchiveFile::new(names[0], "first"),
            ArchiveFile::new(names[1], "second"),
        ],
        ArchiveWriteOptions::default(),
    )
    .unwrap();
    let mut fs = Fs::new();
    let size = data.len() as u64;
    fs.mount_archive_reader("bytes.hpi", data, size, 10, ArchiveOptions::default())
        .unwrap();
    for (i, name) in names.iter().enumerate() {
        let logical = clean_path(name).unwrap();
        let mut want = b"textures/".to_vec();
        want.push(0xc0 + i as u8);
        want.extend_from_slice(b".gaf");
        assert_eq!(logical, want);
        let got = fs.read_file(&logical).unwrap();
        assert_eq!(got, [b"first".as_slice(), b"second"][i]);
    }
    for entries in [fs.read_dir("TEXTURES"), fs.retail_read_dir("TEXTURES")] {
        let entries = entries.unwrap();
        assert!(
            entries.len() == 2 && entries[0].path != entries[1].path,
            "byte-distinct directory entries = {entries:?}"
        );
    }
}

#[test]
fn archive_writer_rejects_ascii_case_duplicate() {
    let result = write_archive(
        &[
            ArchiveFile::new("textures/A.GAF", b""),
            ArchiveFile::new("TEXTURES/a.gaf", b""),
        ],
        ArchiveWriteOptions::default(),
    );
    assert!(
        result.is_err(),
        "ASCII case variants stopped identifying the same archive entry"
    );
}
