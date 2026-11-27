// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi_write_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::hpi_write::{write_archive, ArchiveFile, ArchiveWriteOptions};
use super::{ArchiveOptions, Fs};

#[test]
fn write_archive_round_trip() {
    // Three chunks, the last a remainder.
    let big: Vec<u8> = (0..65536 * 2 + 123).map(|i: usize| (i * 7) as u8).collect();
    let files = vec![
        ArchiveFile::new("objects3d/corkrog.3do", "model bytes"),
        ArchiveFile::new("textures\\rm_corkrog.gaf", &big),
        ArchiveFile::new("readme.txt", "hello"),
        ArchiveFile::new("textures/empty.gaf", b""),
    ];
    for compress in [false, true] {
        let data = write_archive(
            &files,
            ArchiveWriteOptions {
                compress,
                year: 2026,
            },
        )
        .unwrap();
        assert!(
            data.ends_with(b"Copyright 2026 Cavedog Entertainment"),
            "missing the 36-byte trailer the retail mount validator requires"
        );
        let mut fs = Fs::new();
        let size = data.len() as u64;
        fs.mount_archive_reader(
            "test.hpi",
            data,
            size,
            10,
            ArchiveOptions {
                verify_checksums: true,
                ..ArchiveOptions::default()
            },
        )
        .unwrap_or_else(|e| panic!("compress={compress}: {e}"));
        for file in &files {
            let got = fs
                .read_file(&file.path)
                .unwrap_or_else(|e| panic!("compress={compress}: {:?}: {e}", file.path));
            assert_eq!(got, file.data, "compress={compress}: {:?}", file.path);
        }
        let entries = fs.read_dir("textures").unwrap();
        assert_eq!(
            entries.len(),
            2,
            "compress={compress}: textures dir {entries:?}"
        );
        fs.close().unwrap();
    }
}
