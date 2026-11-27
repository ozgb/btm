// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi_bounds_test.go, vfs/mount_rejection_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Shared test fixtures: temporary directories (Go's `t.TempDir`), authored
//! archives, and instrumented readers.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use super::hpi::ReadAt;
use super::hpi_write::{write_archive, ArchiveFile, ArchiveWriteOptions};

/// A directory removed when dropped.
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new() -> TempDir {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "btm-vfs-test-{}-{}-{nanos}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        let path = std::path::absolute(&path).unwrap();
        TempDir(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn join(&self, part: impl AsRef<Path>) -> PathBuf {
        self.0.join(part)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn write_file(path: impl AsRef<Path>, data: impl AsRef<[u8]>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, data).unwrap();
}

pub(crate) fn authored_archive(files: &[ArchiveFile], compress: bool) -> Vec<u8> {
    write_archive(files, ArchiveWriteOptions { compress, year: 0 }).unwrap()
}

pub(crate) fn u32_at(data: &[u8], at: usize) -> usize {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap()) as usize
}

pub(crate) fn put_u32(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// The offset of the first SQSH chunk of the first root entry's record.
pub(crate) fn first_chunk_offset(data: &[u8]) -> usize {
    let root = u32_at(data, 16);
    let entries = u32_at(data, root + 4);
    let record = u32_at(data, entries + 4);
    u32_at(data, record) + 4
}

/// Counts reads, and fails them on request.
pub(crate) struct CountingReader {
    data: Vec<u8>,
    pub(crate) calls: AtomicUsize,
    pub(crate) fail: AtomicBool,
}

impl CountingReader {
    pub(crate) fn new(data: Vec<u8>) -> Arc<CountingReader> {
        Arc::new(CountingReader {
            data,
            calls: AtomicUsize::new(0),
            fail: AtomicBool::new(false),
        })
    }

    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub(crate) fn reset(&self) {
        self.calls.store(0, Ordering::SeqCst);
    }
}

impl ReadAt for CountingReader {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        self.data.read_exact_at(buf, offset)
    }
}

/// A file-backed chunk payload whose declared extent is large while only its
/// metadata is readable. It records request sizes without allocating the payload.
pub(crate) struct SparseReader {
    pub(crate) prefix: Vec<u8>,
    pub(crate) size: u64,
    pub(crate) tail: Vec<u8>,
    pub(crate) tail_at: u64,
    pub(crate) block_at: u64,
    pub(crate) max_request: AtomicUsize,
}

impl SparseReader {
    pub(crate) fn reset(&self) {
        self.max_request.store(0, Ordering::SeqCst);
    }
}

impl ReadAt for SparseReader {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        self.max_request.fetch_max(buf.len(), Ordering::SeqCst);
        if offset == self.tail_at && buf.len() == self.tail.len() {
            buf.copy_from_slice(&self.tail);
            return Ok(());
        }
        let end = offset + buf.len() as u64;
        if end > self.block_at || end > self.size {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        buf.copy_from_slice(&self.prefix[offset as usize..end as usize]);
        Ok(())
    }
}
