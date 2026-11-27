// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), vfs/manifest.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Manifests: the deduplicated identity of the mounted content set.

use std::collections::HashSet;

use super::overlay::Fs;
use super::{show, Error};
use crate::digest::{hex, Sha256};

/// One logical path's identity: who won, who was shadowed, and the hash of the
/// bytes the engine will read. A differing winner can then be named rather than
/// silently used.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestRecord {
    pub logical_path: Vec<u8>,
    pub provider_id: String,
    pub priority: i64,
    pub mount_order: usize,
    pub size: u64,
    /// SHA-256 of the winning bytes in hex; empty if unread.
    pub hash: String,
    /// Provider IDs that also hold this path, in losing order.
    pub shadowed: Vec<String>,
}

/// Bounds the work [`Fs::manifest`] does. Hashing every file of a full install
/// reads about a gigabyte, so it is opt-in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ManifestOptions {
    /// Read and hash the winning bytes.
    pub hash: bool,
    /// Skip hashing entries larger than this; 0 is no limit.
    pub max_hash_size: u64,
}

impl Fs {
    /// One record per logical file, sorted by path. Unlike [`Fs::entries`], which
    /// yields one record per mount, this is the deduplicated identity view.
    pub fn manifest(&self, options: ManifestOptions) -> Result<Vec<ManifestRecord>, Error> {
        let entries = self.entries();
        let mut seen: HashSet<Vec<u8>> = HashSet::with_capacity(entries.len());
        let mut records = Vec::with_capacity(entries.len());
        for entry in &entries {
            if entry.is_dir || !seen.insert(entry.path.clone()) {
                continue;
            }
            let sources = self.sources(&entry.path);
            let Some((winner, shadows)) = sources.split_first() else {
                continue;
            };
            let mut record = ManifestRecord {
                logical_path: winner.path.clone(),
                provider_id: winner.source.provider_id(),
                priority: winner.source.priority,
                mount_order: winner.source.mount_order,
                size: winner.size,
                hash: String::new(),
                shadowed: shadows
                    .iter()
                    .map(|shadow| shadow.source.provider_id())
                    .collect(),
            };
            if options.hash && (options.max_hash_size == 0 || winner.size <= options.max_hash_size)
            {
                let data = self
                    .read_file(&winner.path)
                    .map_err(|source| Error::Manifest {
                        path: show(&winner.path),
                        source: Box::new(source),
                    })?;
                let mut digest = Sha256::new();
                digest.update(&data);
                record.hash = hex(&digest.finish());
            }
            records.push(record);
        }
        // Paths are unique here, so the sort has no ties.
        records.sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
        Ok(records)
    }

    /// A stable identity for the mounted content set over logical paths, winning
    /// providers, priorities and sizes. It reads no file data.
    pub fn manifest_hash(&self) -> Result<String, Error> {
        let records = self.manifest(ManifestOptions::default())?;
        let mut digest = Sha256::new();
        for record in &records {
            digest.update(&record.logical_path);
            digest.update(b"\x00");
            digest.update(record.provider_id.as_bytes());
            digest.update(format!("\x00{}\x00{}\n", record.priority, record.size).as_bytes());
        }
        Ok(hex(&digest.finish()))
    }
}
