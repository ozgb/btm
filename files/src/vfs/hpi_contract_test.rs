// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi_contract_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::type_complexity
)]

use super::hpi::{decode_lz77, SliceBytes};
use super::hpi_write::ArchiveFile;
use super::test_support::{authored_archive, put_u32, u32_at};
use super::{ArchiveOptions, Fs};

/// These authored archives lock the low-byte key and flag contracts.
#[test]
fn archive_plain_key_and_flag_bits() {
    for key in [0u32, 0x1200, 0xff, 0x1234_56ff] {
        let mut data = authored_archive(&[ArchiveFile::new("dir/entry", "payload")], false);
        put_u32(&mut data, 12, key);
        let root = u32_at(&data, 16);
        let entries = u32_at(&data, root + 4);
        data[entries + 8] = 0xff; // Bit zero still makes this a directory.
        let child = u32_at(&data, entries + 4);
        let files = u32_at(&data, child + 4);
        data[files + 8] = 0xfe; // Shadow and reserved bits do not make a directory.
        let mut fs = Fs::new();
        let size = data.len() as u64;
        fs.mount_archive_reader("authored.hpi", data, size, 1, ArchiveOptions::default())
            .unwrap_or_else(|e| panic!("key {key:#x}: mount: {e}"));
        let got = fs.read_file("dir/entry");
        fs.close().unwrap();
        assert_eq!(got.unwrap(), b"payload", "key {key:#x}");
    }
}

#[test]
fn lz77_requires_terminator_and_exact_output() {
    let cases: &[(&str, &[u8], u64, Option<&[u8]>)] = &[
        ("literal and terminator", &[2, b'x', 0, 0], 1, Some(b"x")),
        ("missing terminator", &[0, b'x'], 1, None),
        ("early terminator", &[1, 0, 0], 1, None),
        ("extra output", &[4, b'x', b'y', 0, 0], 1, None),
        ("empty terminated stream", &[1, 0, 0], 0, Some(b"")),
        ("empty missing stream", &[], 0, None),
        (
            "zero initialized window",
            &[3, 0x10, 0, 0, 0],
            2,
            Some(&[0, 0]),
        ),
    ];
    for &(name, input, expected, want) in cases {
        let got = decode_lz77(&mut SliceBytes(input.iter()), expected);
        match want {
            Some(want) => assert_eq!(got.as_deref(), Ok(want), "{name}"),
            None => assert!(got.is_err(), "{name}: decode = {got:?}, want an error"),
        }
    }
}
