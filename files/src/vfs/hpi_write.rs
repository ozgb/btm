// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/hpi_write.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Test-only archive writer: just enough of Go's `WriteArchive` to author fixtures.
//! Writing archives is not part of the crate.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::collections::BTreeMap;

use super::path::fold_logical_name;

/// One file to store. `path` is a logical path; the directory tree is derived
/// from it.
#[derive(Debug, Clone)]
pub(crate) struct ArchiveFile {
    pub(crate) path: Vec<u8>,
    pub(crate) data: Vec<u8>,
}

impl ArchiveFile {
    pub(crate) fn new(path: impl AsRef<[u8]>, data: impl AsRef<[u8]>) -> ArchiveFile {
        ArchiveFile {
            path: path.as_ref().to_vec(),
            data: data.as_ref().to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ArchiveWriteOptions {
    /// Write every non-empty file as zlib SQSH chunks; otherwise store it.
    pub(crate) compress: bool,
    /// The edition year of the trailer; 0 writes 1997.
    pub(crate) year: u32,
}

#[derive(Default)]
struct WriteDir {
    name: Vec<u8>,
    children: BTreeMap<Vec<u8>, WriteDir>,
    files: BTreeMap<Vec<u8>, Vec<u8>>,
    file_names: BTreeMap<Vec<u8>, Vec<u8>>,
}

fn put(blob: &mut [u8], at: usize, value: usize) {
    blob[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
}

/// Serialises files into the HAPI layout: the 20-byte plaintext header, the
/// directory right after it, file data in directory order, and the trailer. The
/// header key is zero, so nothing is obfuscated.
pub(crate) fn write_archive(
    files: &[ArchiveFile],
    options: ArchiveWriteOptions,
) -> Result<Vec<u8>, String> {
    let mut root = WriteDir::default();
    for file in files {
        let replaced: Vec<u8> = file
            .path
            .iter()
            .map(|&b| if b == b'\\' { b'/' } else { b })
            .collect();
        let start = replaced
            .iter()
            .position(|&b| b != b'/')
            .unwrap_or(replaced.len());
        let end = replaced
            .iter()
            .rposition(|&b| b != b'/')
            .map_or(start, |e| e + 1);
        let clean = &replaced[start..end.max(start)];
        if clean.is_empty() {
            return Err("hpi write: empty path".to_string());
        }
        let parts: Vec<&[u8]> = clean.split(|&b| b == b'/').collect();
        let mut dir = &mut root;
        for part in &parts[..parts.len() - 1] {
            if part.is_empty() || *part == b"." || *part == b".." {
                return Err(format!("hpi write: invalid path {:?}", file.path));
            }
            let key = fold_logical_name(part);
            dir = dir.children.entry(key).or_insert_with(|| WriteDir {
                name: part.to_vec(),
                ..WriteDir::default()
            });
        }
        let leaf = parts[parts.len() - 1];
        let key = fold_logical_name(leaf);
        if dir.files.contains_key(&key) || dir.children.contains_key(&key) {
            return Err(format!("hpi write: duplicate entry {:?}", file.path));
        }
        dir.files.insert(key.clone(), file.data.clone());
        dir.file_names.insert(key, leaf.to_vec());
    }

    let mut blob = vec![0u8; 20];
    let mut ordered: Vec<Vec<u8>> = Vec::new();
    let mut patches: Vec<usize> = Vec::new();
    fn layout(
        dir: &WriteDir,
        blob: &mut Vec<u8>,
        ordered: &mut Vec<Vec<u8>>,
        patches: &mut Vec<usize>,
        compress: bool,
    ) -> usize {
        let mut names: Vec<&Vec<u8>> = dir.children.keys().chain(dir.files.keys()).collect();
        names.sort();
        let node = blob.len();
        blob.extend_from_slice(&(names.len() as u32).to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        let entries = blob.len();
        blob.resize(blob.len() + 9 * names.len(), 0);
        put(blob, node + 4, entries);
        for (i, key) in names.iter().enumerate() {
            let entry = entries + 9 * i;
            if let Some(child) = dir.children.get(*key) {
                let name_at = blob.len();
                blob.extend_from_slice(&child.name);
                blob.push(0);
                let child_node = layout(child, blob, ordered, patches, compress);
                put(blob, entry, name_at);
                put(blob, entry + 4, child_node);
                blob[entry + 8] = 1;
                continue;
            }
            let data = &dir.files[*key];
            let name_at = blob.len();
            blob.extend_from_slice(&dir.file_names[*key]);
            blob.push(0);
            let record = blob.len();
            blob.resize(blob.len() + 9, 0);
            put(blob, record + 4, data.len());
            if compress && !data.is_empty() {
                blob[record + 8] = 2;
            }
            put(blob, entry, name_at);
            put(blob, entry + 4, record);
            patches.push(record);
            ordered.push(data.clone());
        }
        node
    }
    let root_node = layout(
        &root,
        &mut blob,
        &mut ordered,
        &mut patches,
        options.compress,
    );
    let directory_end = blob.len();
    blob[0..4].copy_from_slice(b"HAPI");
    put(&mut blob, 4, 0x0001_0000);
    put(&mut blob, 8, directory_end);
    put(&mut blob, 12, 0);
    put(&mut blob, 16, root_node);

    let mut out = blob;
    for (data, &patch) in ordered.iter().zip(&patches) {
        let here = out.len();
        put(&mut out, patch, here);
        if !options.compress || data.is_empty() {
            out.extend_from_slice(data);
            continue;
        }
        let chunks = zlib_chunks(data);
        for chunk in &chunks {
            out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        }
        for chunk in &chunks {
            out.extend_from_slice(chunk);
        }
    }
    let year = if options.year == 0 {
        1997
    } else {
        options.year
    };
    out.extend_from_slice(
        format!("Copyright {:04} Cavedog Entertainment", year % 10000).as_bytes(),
    );
    Ok(out)
}

/// Splits data into 64 KiB chunks and wraps each in a SQSH header: marker, 2,
/// method 2 (zlib), not encoded, compressed size, decompressed size, and the
/// wrapping byte sum of the payload.
pub(crate) fn zlib_chunks(data: &[u8]) -> Vec<Vec<u8>> {
    data.chunks(65536)
        .map(|piece| {
            let payload = miniz_oxide::deflate::compress_to_vec_zlib(piece, 9);
            let sum = payload
                .iter()
                .fold(0u32, |sum, &b| sum.wrapping_add(u32::from(b)));
            let mut chunk = Vec::with_capacity(19 + payload.len());
            chunk.extend_from_slice(b"SQSH");
            chunk.extend_from_slice(&[2, 2, 0]);
            chunk.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            chunk.extend_from_slice(&(piece.len() as u32).to_le_bytes());
            chunk.extend_from_slice(&sum.to_le_bytes());
            chunk.extend_from_slice(&payload);
            chunk
        })
        .collect()
}
