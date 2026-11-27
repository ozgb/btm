// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi_bounds_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::type_complexity
)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::hpi::{HPI_CHUNK_HEADER_SIZE, HPI_CHUNK_SIZE};
use super::hpi_write::ArchiveFile;
use super::test_support::{
    authored_archive, first_chunk_offset, put_u32, u32_at, write_file, CountingReader,
    SparseReader, TempDir,
};
use super::{Archive, ArchiveOptions, Fs, FsOps};

const CHUNK: usize = HPI_CHUNK_SIZE as usize;

#[test]
fn archive_rejects_directory_pointers_outside_blob() {
    let data = authored_archive(&[ArchiveFile::new("entry", "data")], false);
    let root = u32_at(&data, 16);
    let entries = u32_at(&data, root + 4);
    let directory_end = u32_at(&data, 8) as u32;
    let cases: Vec<(&str, Box<dyn Fn(&mut Vec<u8>)>)> = vec![
        (
            "name pointer at end",
            Box::new(move |b| put_u32(b, entries, directory_end)),
        ),
        (
            "record pointer beyond end",
            Box::new(move |b| put_u32(b, entries + 4, directory_end + 1)),
        ),
        (
            "child pointer at end",
            Box::new(move |b| {
                b[entries + 8] = 1;
                put_u32(b, entries + 4, directory_end);
            }),
        ),
        (
            "entry list crosses end",
            Box::new(move |b| put_u32(b, root + 4, directory_end - 1)),
        ),
    ];
    for (name, mutate) in cases {
        let mut mutated = data.clone();
        mutate(&mut mutated);
        let size = mutated.len() as u64;
        let result = Archive::new("malformed.hpi", mutated, size, ArchiveOptions::default());
        assert!(
            result.as_ref().is_err_and(|e| e.is_malformed_archive()),
            "{name}: Archive::new = {result:?}, want a malformed archive"
        );
    }
}

#[test]
fn archive_chunk_spans_and_limit() {
    let data: Vec<u8> = (0..2 * CHUNK + 19).map(|i| i as u8).collect();
    let archive = authored_archive(&[ArchiveFile::new("large.bin", &data)], true);

    for output_size in [HPI_CHUNK_SIZE as u32 - 1, HPI_CHUNK_SIZE as u32 + 1] {
        // Reject a non-final output span.
        let mut mutated = archive.clone();
        let chunk = first_chunk_offset(&mutated);
        put_u32(&mut mutated, chunk + 11, output_size);
        let mut fs = Fs::new();
        let size = mutated.len() as u64;
        fs.mount_archive_reader("bad.hpi", mutated, size, 1, ArchiveOptions::default())
            .unwrap();
        let read = fs.read_file("large.bin");
        assert!(
            read.as_ref().is_err_and(|e| e.is_malformed_archive()),
            "read_file = {read:?}, want a malformed archive"
        );
    }

    let mut fs = Fs::new();
    let size = archive.len() as u64;
    fs.mount_archive_reader(
        "valid.hpi",
        archive.clone(),
        size,
        1,
        ArchiveOptions::default(),
    )
    .unwrap();
    let whole = fs.read_file("large.bin").unwrap();
    assert!(
        whole == data,
        "whole-file decode differs from authored bytes"
    );
    for (offset, length) in [(0, 32), (CHUNK - 5, 17), (2 * CHUNK, 19)] {
        let got = fs
            .read_file_range("large.bin", offset as u64, Some(length))
            .unwrap();
        assert!(
            got == whole[offset..offset + length],
            "range {offset}:+{length} differs from whole read"
        );
    }

    let tracked = CountingReader::new(archive.clone());
    let mut limited = Fs::new();
    limited
        .mount_archive_reader(
            "limited.hpi",
            Arc::clone(&tracked),
            size,
            1,
            ArchiveOptions::default(),
        )
        .unwrap();
    tracked.reset();
    let read = limited.read_file_limit("large.bin", 1);
    assert!(read.as_ref().is_err_and(|e| e.is_too_large()), "{read:?}");
    assert_eq!(tracked.calls(), 0, "over-limit read opened archive payload");
}

#[test]
fn archive_limit_streams_file_backed_chunk_payload() {
    let mut base = authored_archive(&[ArchiveFile::new("small.bin", "x")], true);
    let chunk = first_chunk_offset(&base);
    let stored_size = u32::MAX;
    put_u32(&mut base, chunk - 4, stored_size);
    put_u32(
        &mut base,
        chunk + 7,
        stored_size - HPI_CHUNK_HEADER_SIZE as u32,
    );

    let logical_size = chunk as u64 + u64::from(stored_size) + 36;
    let reader = Arc::new(SparseReader {
        tail: base[base.len() - 36..].to_vec(),
        prefix: base,
        size: logical_size,
        tail_at: logical_size - 36,
        block_at: (chunk as u64) + HPI_CHUNK_HEADER_SIZE,
        max_request: AtomicUsize::new(0),
    });
    let mut fs = Fs::new();
    fs.mount_archive_reader(
        "sparse.hpi",
        Arc::clone(&reader),
        logical_size,
        1,
        ArchiveOptions::default(),
    )
    .unwrap();
    reader.reset();
    let read = fs.read_file_limit("small.bin", 1);
    assert!(
        read.as_ref().is_err_and(|e| e.is_malformed_archive()),
        "{read:?}"
    );
    let max_request = reader.max_request.load(Ordering::SeqCst);
    assert!(
        max_request <= 32 << 10,
        "tiny limited read requested {max_request}-byte encoded payload buffer"
    );
}

#[test]
fn pinned_mount_uses_metadata_and_ordinary_read_guards() {
    let archive = authored_archive(&[ArchiveFile::new("same.txt", "lower")], true);
    let tracked = CountingReader::new(archive.clone());
    let dir = TempDir::new();
    let mut fs = Fs::new();
    fs.mount_archive_reader(
        "lower.hpi",
        Arc::clone(&tracked),
        archive.len() as u64,
        1,
        ArchiveOptions::default(),
    )
    .unwrap();
    write_file(dir.join("same.txt"), "upper");
    write_file(dir.join("only-upper.txt"), [b'x'; 16]);
    fs.mount_directory(dir.path(), 2).unwrap();

    // The higher-priority loose directory is mount 0; the archive is the pinned
    // lower provider. Metadata must not open or decode that archive.
    let pinned = fs.pinned(1);
    tracked.reset();
    let info = pinned.stat(b"same.txt").unwrap();
    assert_eq!(info.source.provider_id(), "lower.hpi");
    pinned.cache_stamp(b"same.txt").unwrap();
    assert_eq!(tracked.calls(), 0, "pinned metadata read archive payload");
    let read = pinned.read_file_limit(b"same.txt", 2);
    assert!(read.as_ref().is_err_and(|e| e.is_too_large()), "{read:?}");
    assert_eq!(tracked.calls(), 0, "pinned over-limit read archive payload");
    tracked.fail.store(true, Ordering::SeqCst);
    assert!(
        pinned.open(b"same.txt").is_err(),
        "pinned open succeeded with an unreadable archive"
    );
    let info = pinned.stat(b"same.txt").unwrap();
    assert_eq!(info.source.provider_id(), "lower.hpi");
    let stamp = pinned.cache_stamp(b"same.txt").unwrap();
    assert!(
        stamp.starts_with("hpi|lower.hpi|"),
        "unreadable pinned stamp = {stamp:?}, want lower archive identity"
    );
    let stat = pinned.stat(b"only-upper.txt");
    assert!(stat.as_ref().is_err_and(|e| e.is_not_found()), "{stat:?}");
    let read = pinned.read_file_limit(b"only-upper.txt", 1);
    assert!(read.as_ref().is_err_and(|e| e.is_not_found()), "{read:?}");

    let mut dir_fs = Fs::new();
    dir_fs.mount_directory(dir.path(), 1).unwrap();
    let open = dir_fs.open_mount(0, "");
    assert!(open.as_ref().is_err_and(|e| e.is_dir()), "{open:?}");
    fs.close().unwrap();
    dir_fs.close().unwrap();
}

/// A chunk read whole decodes exactly as the streamed path does, scrambled payload
/// and checksum included, and costs one read per chunk.
#[test]
fn archive_chunk_read_whole_matches_streamed() {
    let data: Vec<u8> = (0..3 * CHUNK - 7)
        .map(|i| (i * 7 + i / 251) as u8)
        .collect();
    let mut archive = authored_archive(&[ArchiveFile::new("art.bin", &data)], true);
    // Scramble the first chunk's payload as the SQSH transform's inverse and set
    // its flag, with the checksum taken over the stored bytes. The record's data
    // opens with a table of three chunk sizes.
    let table = first_chunk_offset(&archive) - 4;
    let stored = u32_at(&archive, table) as u64;
    let chunk = table + 3 * 4;
    let payload_size = u32_at(&archive, chunk + 7);
    let start = chunk + HPI_CHUNK_HEADER_SIZE as usize;
    let mut sum = 0u32;
    for (i, value) in archive[start..start + payload_size].iter_mut().enumerate() {
        *value = (*value ^ i as u8).wrapping_add(i as u8);
        sum = sum.wrapping_add(u32::from(*value));
    }
    archive[chunk + 6] = 1;
    put_u32(&mut archive, chunk + 15, sum);

    let tracked = CountingReader::new(archive.clone());
    let verify = ArchiveOptions {
        verify_checksums: true,
        ..ArchiveOptions::default()
    };
    let mut fs = Fs::new();
    fs.mount_archive_reader(
        "art.hpi",
        Arc::clone(&tracked),
        archive.len() as u64,
        1,
        verify,
    )
    .unwrap();
    tracked.reset();
    let got = fs.read_file("art.bin").unwrap();
    assert!(got == data, "whole-chunk reads decoded different bytes");
    // The chunk table and one read for each of the three chunks.
    assert!(
        tracked.calls() <= 4,
        "a three-chunk file took {} reads",
        tracked.calls()
    );

    let size = archive.len() as u64;
    let a = Archive::new("art.hpi", archive, size, verify).unwrap();
    let whole = a
        .decode_chunk(0, chunk as u64, stored, HPI_CHUNK_SIZE)
        .unwrap();
    let streamed = a
        .decode_chunk_streamed(0, chunk as u64, stored, HPI_CHUNK_SIZE)
        .unwrap();
    assert!(
        whole == streamed && whole == data[..CHUNK],
        "the whole-chunk and streamed decoders disagree"
    );
}
