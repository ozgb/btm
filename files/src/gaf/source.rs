// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/gaf_source.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! An on-demand source: the validated encoded bank, decoded one frame at a time.
//!
//! Go reads the file through the VFS with a byte cap (`LoadGAFSourceFile`); here the
//! caller hands over the bytes, so any read cap is the caller's. The source takes
//! ownership of the `Vec`, so later changes to the caller's data cannot reach it.

use std::collections::{HashMap, HashSet};
use std::mem::size_of;
use std::sync::Arc;

use super::metadata::materialize_frame;
use super::{GafError, GafFrame, GafLimits, GafMetadata, GafMetadataFrame};
use crate::digest::sha256;

/// A validated encoded bank for on-demand presentation. Metadata and returned
/// frames are immutable. Unlike [`Gaf::parse`](super::Gaf::parse), retaining a
/// source does not retain decoded rasters. It is `Send + Sync`: nothing in it is
/// ever mutated, which is what Go's source relied on.
#[derive(Debug, Clone)]
pub struct GafSource {
    data: Vec<u8>,
    meta: GafMetadata,
    digest: [u8; 32],
}

impl GafSource {
    /// Validates `data` under `limits` and takes ownership of it.
    pub fn new(data: Vec<u8>, limits: GafLimits) -> Result<GafSource, GafError> {
        let meta = GafMetadata::parse_with_limits(&data, limits)?;
        let digest = sha256(&data);
        Ok(GafSource { data, meta, digest })
    }

    /// The immutable index. It holds no decoded pixels.
    pub fn metadata(&self) -> &GafMetadata {
        &self.meta
    }

    /// The retained source buffer size.
    pub fn encoded_bytes(&self) -> u64 {
        self.data.len() as u64
    }

    /// SHA-256 of the validated bytes, to identify them across cache eviction.
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Decodes one entry frame and its reachable child graph. `max_pixels` bounds
    /// both the unique geometry and the expanded traversal before any raster is
    /// allocated, so a consumer that copies aliased children is bounded too. The
    /// whole bank was already checked at load, including frames never requested.
    /// Aliased children share one decoded frame within the result; the source
    /// retains none, and the caller owns cache policy. Every returned frame has
    /// `transient` set.
    pub fn frame(
        &self,
        name: impl AsRef<[u8]>,
        index: usize,
        max_pixels: u64,
    ) -> Result<Arc<GafFrame>, GafError> {
        if max_pixels == 0 {
            return Err(GafError::InvalidFrameRequest);
        }
        let name = name.as_ref();
        let missing = || GafError::MissingEntryFrame {
            name: String::from_utf8_lossy(name).into_owned(),
            index,
        };
        let entry = self.meta.find(name).ok_or_else(missing)?;
        let root = &entry.frames.get(index).ok_or_else(missing)?.frame;
        if root.expanded_pixels > max_pixels {
            return Err(GafError::SelectedExpandedPixelsExceed);
        }
        // Sum the unique frames reachable from the root. The order of visits cannot
        // change whether the total exceeds the budget, so a plain stack suffices.
        let mut seen: HashSet<usize> = HashSet::new();
        let mut pixels: u64 = 0;
        let mut pending: Vec<&Arc<GafMetadataFrame>> = vec![root];
        while let Some(f) = pending.pop() {
            if !seen.insert(Arc::as_ptr(f) as usize) {
                continue;
            }
            let n = u64::from(f.width) * u64::from(f.height);
            if n > max_pixels.saturating_sub(pixels) {
                return Err(GafError::SelectedPixelsExceed);
            }
            pixels += n;
            pending.extend(f.subframes.iter());
        }
        let mut frames: HashMap<usize, Arc<GafFrame>> = HashMap::with_capacity(seen.len());
        materialize_frame(&self.data, root, &mut frames, true, true)
    }
}

/// Counts frame headers, child slices and owned raster storage once per shared
/// node, for cache accounting. It is not authored metadata, and the sizes are
/// Rust's (`size_of`), not Go's `unsafe.Sizeof`, so only comparisons are meaningful.
/// A direct RLE view owns one extra byte plane.
pub fn gaf_frame_bytes(root: &GafFrame) -> u64 {
    let mut seen: HashSet<usize> = HashSet::new();
    let mut total: u64 = 0;
    let mut pending: Vec<&GafFrame> = vec![root];
    while let Some(f) = pending.pop() {
        if !seen.insert(f as *const GafFrame as usize) {
            continue;
        }
        let mut n = size_of::<GafFrame>() as u64
            + (f.subframes.capacity() * size_of::<Arc<GafFrame>>()) as u64
            + (f.pixels.capacity() + f.transparent.capacity()) as u64;
        if let Some(d) = &f.direct_raster {
            n += size_of::<GafFrame>() as u64 + d.pixels.capacity() as u64;
        }
        total = total.saturating_add(n);
        pending.extend(f.subframes.iter().map(|c| c.as_ref()));
    }
    total
}
