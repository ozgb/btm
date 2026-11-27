// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/gaf_metadata.go (with the frame decoding of formats/gaf.go), commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! The pixel-free index of a bank, and the decoder that turns an index plus the
//! file bytes into pixels. Metadata and full decoding share one validation pass,
//! so choosing metadata never accepts content that a later decode rejects.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::{
    find_by_folded_name, Gaf, GafEntry, GafError, GafFrame, GafFrameRef, GafLimits, RleFault,
    MAX_COMPOSITE_DEPTH_CEILING, MAX_GAF_FRAME_PIXELS,
};

/// The validated, pixel-free index of an animation bank. It keeps every frame
/// fact a consumer can see without decoding a raster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GafMetadata {
    pub version: u32,
    pub entry_count: u32,
    pub unknown: u32,
    /// In file table order. Two table slots with the same offset share one `Arc`.
    pub entries: Vec<Arc<GafMetadataEntry>>,
}

/// One named sequence in a pixel-free index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GafMetadataEntry {
    pub name: Vec<u8>,
    pub frame_count: u16,
    pub unknown1: u16,
    pub unknown2: u32,
    pub frames: Vec<GafMetadataFrameRef>,
    pub(super) source_offset: u32,
}

impl GafMetadataEntry {
    /// The file offset of this entry's header.
    pub fn source_offset(&self) -> u32 {
        self.source_offset
    }
}

/// The authored frame pointer and the display-delay word beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GafMetadataFrameRef {
    pub offset: u32,
    pub value: u32,
    pub frame: Arc<GafMetadataFrame>,
}

/// A frame header and its validated composite graph. Pixel planes are absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GafMetadataFrame {
    pub width: u16,
    pub height: u16,
    pub x_offset: i16,
    pub y_offset: i16,
    pub color_key: u8,
    pub compressed: u8,
    pub unknown2: u32,
    pub data_offset: u32,
    pub unknown3: u32,
    pub subframe_count: u8,
    pub alternate_blitter: u8,
    pub subframes: Vec<Arc<GafMetadataFrame>>,
    pub(super) composite_depth: u32,
    pub(super) expanded_frames: u64,
    pub(super) expanded_pixels: u64,
}

impl GafMetadataFrame {
    /// The maximum number of subframe links below this frame.
    pub fn composite_depth(&self) -> u32 {
        self.composite_depth
    }

    /// Frames visited by a full traversal, counting a shared child once per reference.
    pub fn expanded_frames(&self) -> u64 {
        self.expanded_frames
    }

    /// Pixels visited by a full traversal, counting a shared child once per reference.
    pub fn expanded_pixels(&self) -> u64 {
        self.expanded_pixels
    }
}

fn u16_at(data: &[u8], pos: usize) -> Option<u16> {
    let end = pos.checked_add(2)?;
    match data.get(pos..end)? {
        [a, b] => Some(u16::from_le_bytes([*a, *b])),
        _ => None,
    }
}

fn u32_at(data: &[u8], pos: usize) -> Option<u32> {
    let end = pos.checked_add(4)?;
    match data.get(pos..end)? {
        [a, b, c, d] => Some(u32::from_le_bytes([*a, *b, *c, *d])),
        _ => None,
    }
}

/// `u32` file offsets always fit `usize` on the targets this crate supports; if one
/// ever did not, the saturated value fails every bounds check that follows.
pub(super) fn to_usize(v: u32) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

impl GafMetadata {
    /// Validates and indexes a bank under the default limits, without allocating
    /// decoded pixel or transparency planes.
    pub fn parse(data: &[u8]) -> Result<GafMetadata, GafError> {
        GafMetadata::parse_with_limits(data, GafLimits::default())
    }

    /// Validates and indexes a bank under the same limits as
    /// [`Gaf::parse_with_limits`]. `max_decoded_pixels` still bounds the unique
    /// frame geometry, so metadata and full decoding share one policy.
    pub fn parse_with_limits(data: &[u8], mut limits: GafLimits) -> Result<GafMetadata, GafError> {
        if data.len() < 12 {
            return Err(GafError::FileTooSmall);
        }
        if limits.max_frame_refs == 0
            || limits.max_decoded_pixels == 0
            || limits.max_composite_depth == 0
            || limits.max_composite_depth > MAX_COMPOSITE_DEPTH_CEILING
        {
            return Err(GafError::InvalidLimits);
        }
        let defaults = GafLimits::default();
        if limits.max_expanded_frames == 0 {
            limits.max_expanded_frames = defaults.max_expanded_frames;
        }
        if limits.max_expanded_pixels == 0 {
            limits.max_expanded_pixels = defaults.max_expanded_pixels;
        }
        if limits.max_rle_commands == 0 {
            limits.max_rle_commands = defaults.max_rle_commands;
        }
        let (Some(version), Some(entry_count), Some(unknown)) =
            (u32_at(data, 0), u32_at(data, 4), u32_at(data, 8))
        else {
            return Err(GafError::FileTooSmall);
        };
        // Only the low word of the entry count is read, as a signed 16-bit value;
        // a negative count walks no table at all (retail behaviour).
        let signed_count = (entry_count & 0xffff) as u16 as i16;
        let count = usize::try_from(signed_count).unwrap_or(0);
        if count > (data.len() - 12) / 4 {
            return Err(GafError::EntryTableTruncated);
        }
        // `count` is bounded by the table that fits in the file, so this reserve is too.
        let mut entries: Vec<Arc<GafMetadataEntry>> = Vec::with_capacity(count);
        let mut budget = Budget::new(limits);
        let mut entry_cache: HashMap<u32, Arc<GafMetadataEntry>> = HashMap::new();
        let mut frame_cache: HashMap<u32, Arc<GafMetadataFrame>> = HashMap::new();
        for i in 0..count {
            let offset = u32_at(data, 12 + i * 4).ok_or(GafError::EntryTableTruncated)?;
            let off = to_usize(offset);
            let outside = GafError::EntryOutsideFile { index: i as u64 };
            if off > data.len() || data.len() - off < 40 {
                return Err(outside);
            }
            if let Some(cached) = entry_cache.get(&offset) {
                entries.push(Arc::clone(cached));
                continue;
            }
            let header = data.get(off..off + 40).ok_or(outside)?;
            let name_field = header.get(8..40).unwrap_or(&[]);
            let name: Vec<u8> = name_field.iter().copied().take_while(|b| *b != 0).collect();
            let frame_count = u16_at(header, 0).unwrap_or(0);
            let unknown1 = u16_at(header, 2).unwrap_or(0);
            let unknown2 = u32_at(header, 4).unwrap_or(0);
            let name_text = String::from_utf8_lossy(&name).into_owned();
            let refs_start = off + 40;
            let ref_bytes = usize::from(frame_count) * 8;
            if refs_start > data.len() || ref_bytes > data.len() - refs_start {
                return Err(GafError::FrameTableTruncated { entry: name_text });
            }
            budget
                .add_refs(u64::from(frame_count))
                .map_err(|e| GafError::InEntry {
                    entry: name_text.clone(),
                    source: Box::new(e),
                })?;
            // The reference table fits in the file (checked above), so this is bounded.
            let mut frames: Vec<GafMetadataFrameRef> = Vec::with_capacity(usize::from(frame_count));
            for frame in 0..usize::from(frame_count) {
                let at = refs_start + frame * 8;
                let (Some(frame_offset), Some(value)) = (u32_at(data, at), u32_at(data, at + 4))
                else {
                    return Err(GafError::FrameTableTruncated { entry: name_text });
                };
                let wrap = |e: GafError| GafError::InEntryFrame {
                    entry: name_text.clone(),
                    frame,
                    source: Box::new(e),
                };
                let decoded = index_frame(
                    data,
                    frame_offset,
                    &mut frame_cache,
                    &mut HashSet::new(),
                    &mut budget,
                    0,
                )
                .map_err(wrap)?;
                budget.add_expanded_root(&decoded).map_err(wrap)?;
                frames.push(GafMetadataFrameRef {
                    offset: frame_offset,
                    value,
                    frame: decoded,
                });
            }
            let entry = Arc::new(GafMetadataEntry {
                name,
                frame_count,
                unknown1,
                unknown2,
                frames,
                source_offset: offset,
            });
            entry_cache.insert(offset, Arc::clone(&entry));
            entries.push(entry);
        }
        Ok(GafMetadata {
            version,
            entry_count,
            unknown,
            entries,
        })
    }

    /// The first entry whose name matches, in table order. ASCII letters fold; high
    /// bytes compare unchanged.
    pub fn find(&self, name: impl AsRef<[u8]>) -> Option<&GafMetadataEntry> {
        find_by_folded_name(&self.entries, name.as_ref(), |e| &e.name)
    }
}

/// Aggregate counters for one bank.
pub(super) struct Budget {
    limits: GafLimits,
    refs: u64,
    pixels: u64,
    rle_commands: u64,
    expanded_frames: u64,
    expanded_pixels: u64,
    /// Root frames already charged, by node address (frames are shared by `Arc`).
    roots: HashSet<usize>,
}

impl Budget {
    pub(super) fn new(limits: GafLimits) -> Budget {
        Budget {
            limits,
            refs: 0,
            pixels: 0,
            rle_commands: 0,
            expanded_frames: 0,
            expanded_pixels: 0,
            roots: HashSet::new(),
        }
    }

    fn add_expanded_root(&mut self, frame: &Arc<GafMetadataFrame>) -> Result<(), GafError> {
        let key = Arc::as_ptr(frame) as usize;
        if self.roots.contains(&key) {
            return Ok(());
        }
        // The counters never exceed their limits, so the subtraction cannot wrap;
        // `saturating_sub` makes that explicit.
        if frame.expanded_frames
            > self
                .limits
                .max_expanded_frames
                .saturating_sub(self.expanded_frames)
        {
            return Err(GafError::AggregateExpandedFramesExceed);
        }
        if frame.expanded_pixels
            > self
                .limits
                .max_expanded_pixels
                .saturating_sub(self.expanded_pixels)
        {
            return Err(GafError::AggregateExpandedPixelsExceed);
        }
        self.expanded_frames += frame.expanded_frames;
        self.expanded_pixels += frame.expanded_pixels;
        self.roots.insert(key);
        Ok(())
    }

    fn add_refs(&mut self, n: u64) -> Result<(), GafError> {
        if n > self.limits.max_frame_refs.saturating_sub(self.refs) {
            return Err(GafError::AggregateFrameRefsExceed);
        }
        self.refs += n;
        Ok(())
    }

    fn add_pixels(&mut self, n: u64) -> Result<(), GafError> {
        if n > self.limits.max_decoded_pixels.saturating_sub(self.pixels) {
            return Err(GafError::AggregateDecodedPixelsExceed);
        }
        self.pixels += n;
        Ok(())
    }
}

type MetaCache = HashMap<u32, Arc<GafMetadataFrame>>;

/// Validates the frame at `offset` and its children. `stack` holds the offsets on
/// the current path so a cycle is an error. Recursion depth is bounded by
/// `max_composite_depth`, which is at most [`MAX_COMPOSITE_DEPTH_CEILING`].
fn index_frame(
    data: &[u8],
    offset: u32,
    cache: &mut MetaCache,
    stack: &mut HashSet<u32>,
    budget: &mut Budget,
    depth: u32,
) -> Result<Arc<GafMetadataFrame>, GafError> {
    if stack.contains(&offset) {
        return Err(GafError::FrameCycle { offset });
    }
    if depth > budget.limits.max_composite_depth {
        return Err(GafError::CompositeDepthExceeded);
    }
    if let Some(frame) = cache.get(&offset) {
        // `depth` is at most the limit here, so the subtraction cannot wrap.
        if frame.composite_depth > budget.limits.max_composite_depth.saturating_sub(depth) {
            return Err(GafError::CompositeDepthExceeded);
        }
        return Ok(Arc::clone(frame));
    }
    let off = to_usize(offset);
    if off > data.len() || data.len() - off < 24 {
        return Err(GafError::FramePointerOutsideFile { offset });
    }
    stack.insert(offset);
    let result = index_frame_body(data, offset, cache, stack, budget, depth);
    stack.remove(&offset);
    result
}

fn index_frame_body(
    data: &[u8],
    offset: u32,
    cache: &mut MetaCache,
    stack: &mut HashSet<u32>,
    budget: &mut Budget,
    depth: u32,
) -> Result<Arc<GafMetadataFrame>, GafError> {
    let off = to_usize(offset);
    let outside = GafError::FramePointerOutsideFile { offset };
    let header = data.get(off..off + 24).ok_or(outside.clone())?;
    let (Some(width), Some(height), Some(x), Some(y)) = (
        u16_at(header, 0),
        u16_at(header, 2),
        u16_at(header, 4),
        u16_at(header, 6),
    ) else {
        return Err(outside);
    };
    let (Some(unknown2), Some(data_offset), Some(unknown3)) =
        (u32_at(header, 12), u32_at(header, 16), u32_at(header, 20))
    else {
        return Err(outside);
    };
    let (Some(&color_key), Some(&compressed), Some(&subframe_count), Some(&alternate_blitter)) =
        (header.get(8), header.get(9), header.get(10), header.get(11))
    else {
        return Err(outside);
    };
    let mut frame = GafMetadataFrame {
        width,
        height,
        // Offsets are signed 16-bit values stored as raw words.
        x_offset: x as i16,
        y_offset: y as i16,
        color_key,
        compressed,
        unknown2,
        data_offset,
        unknown3,
        subframe_count,
        alternate_blitter,
        subframes: Vec::new(),
        composite_depth: 0,
        expanded_frames: 0,
        expanded_pixels: 0,
    };
    let data_start = to_usize(data_offset);
    if data_start > data.len() {
        return Err(GafError::FrameDataPointerOutsideFile { offset });
    }
    let pixel_count = u64::from(width) * u64::from(height);
    if pixel_count > MAX_GAF_FRAME_PIXELS {
        return Err(GafError::FrameTooLarge { offset });
    }
    budget
        .add_pixels(pixel_count)
        .map_err(|e| GafError::InFrame {
            offset,
            source: Box::new(e),
        })?;
    frame.expanded_frames = 1;
    frame.expanded_pixels = pixel_count;
    if pixel_count > budget.limits.max_expanded_pixels {
        return Err(GafError::ExpandedPixelsExceed);
    }
    if compressed != 0 && compressed != 1 {
        return Err(GafError::BadCompression {
            offset,
            value: compressed,
        });
    }
    if subframe_count != 0 {
        let count = usize::from(subframe_count);
        let bytes_needed = count * 4;
        if bytes_needed > data.len() - data_start {
            return Err(GafError::SubframeTableTruncated { offset });
        }
        budget
            .add_refs(u64::from(subframe_count))
            .map_err(|e| GafError::InFrame {
                offset,
                source: Box::new(e),
            })?;
        // At most 255 children, whose table fits in the file.
        frame.subframes.reserve_exact(count);
        for i in 0..count {
            let sub_offset = u32_at(data, data_start + i * 4)
                .ok_or(GafError::SubframeTableTruncated { offset })?;
            let sub = index_frame(data, sub_offset, cache, stack, budget, depth + 1)?;
            // Check before adding, cache hits included: a repeated child pointer
            // multiplies downstream work and must not wrap either counter.
            if sub.expanded_frames
                > budget
                    .limits
                    .max_expanded_frames
                    .saturating_sub(frame.expanded_frames)
            {
                return Err(GafError::ExpandedFramesExceed);
            }
            if sub.expanded_pixels
                > budget
                    .limits
                    .max_expanded_pixels
                    .saturating_sub(frame.expanded_pixels)
            {
                return Err(GafError::ExpandedPixelsExceed);
            }
            frame.expanded_frames += sub.expanded_frames;
            frame.expanded_pixels += sub.expanded_pixels;
            // Go wraps `u32::MAX + 1` to zero here; saturating keeps the large value,
            // which the limit check below then rejects. Unreachable under the ceiling.
            let candidate = sub.composite_depth.saturating_add(1);
            if candidate > frame.composite_depth {
                frame.composite_depth = candidate;
            }
            frame.subframes.push(sub);
        }
        if frame.composite_depth > budget.limits.max_composite_depth.saturating_sub(depth) {
            return Err(GafError::CompositeDepthExceeded);
        }
        let frame = Arc::new(frame);
        cache.insert(offset, Arc::clone(&frame));
        return Ok(frame);
    }
    if compressed == 0 {
        let need = usize::try_from(pixel_count).unwrap_or(usize::MAX);
        if need > data.len() - data_start {
            return Err(GafError::RawPixelsTruncated { offset });
        }
    } else {
        walk_rle(data, width, height, data_offset, None, Some(&mut *budget)).map_err(|e| {
            GafError::InFrame {
                offset,
                source: Box::new(e),
            }
        })?;
    }
    let frame = Arc::new(frame);
    cache.insert(offset, Arc::clone(&frame));
    Ok(frame)
}

/// Output planes for an RLE walk. `None` validates only.
pub(super) struct RleOut<'a> {
    pub(super) pixels: &'a mut [u8],
    pub(super) transparent: &'a mut [bool],
}

/// Applies the checked command stream to metadata and pixels alike. The stored row
/// length anchors the next row; only the remaining width stops a nonempty row's
/// commands, so a row may read past its stored length (retail behaviour).
pub(super) fn walk_rle(
    data: &[u8],
    width: u16,
    height: u16,
    data_offset: u32,
    mut out: Option<&mut RleOut<'_>>,
    mut budget: Option<&mut Budget>,
) -> Result<(), GafError> {
    // Empty leaf rectangles never reach the retail pixel reader.
    if width == 0 || height == 0 {
        return Ok(());
    }
    let len = data.len();
    let mut position = to_usize(data_offset);
    let width = usize::from(width);
    let fault = |row: usize, kind: RleFault| GafError::Rle { row, kind };
    for row in 0..usize::from(height) {
        if position > len || len - position < 2 {
            return Err(fault(row, RleFault::RowHeaderTruncated));
        }
        let row_size =
            usize::from(u16_at(data, position).ok_or(fault(row, RleFault::RowHeaderTruncated))?);
        position += 2;
        if row_size > len - position {
            return Err(fault(row, RleFault::RowExtentOutsideFile));
        }
        let row_end = position + row_size;
        let base = row * width;
        if row_size == 0 {
            if let Some(o) = out.as_deref_mut() {
                set_transparent(o, base, width)?;
            }
            continue;
        }
        let mut x = 0usize;
        while x < width {
            let Some(&command) = data.get(position) else {
                return Err(fault(row, RleFault::CommandOutsideFile));
            };
            if let Some(b) = budget.as_deref_mut() {
                if b.rle_commands >= b.limits.max_rle_commands {
                    return Err(GafError::AggregateRleCommandsExceed);
                }
                b.rle_commands += 1;
            }
            position += 1;
            let remaining = width - x;
            let mut n = (usize::from(command >> 2) + 1).min(remaining);
            if command & 1 != 0 {
                // Skip run (the count may be zero, which makes no progress).
                n = usize::from(command >> 1).min(remaining);
                if let Some(o) = out.as_deref_mut() {
                    set_transparent(o, base + x, n)?;
                }
            } else if command & 2 != 0 {
                let Some(&value) = data.get(position) else {
                    return Err(fault(row, RleFault::RepeatByteOutsideFile));
                };
                position += 1;
                if let Some(o) = out.as_deref_mut() {
                    o.pixels
                        .get_mut(base + x..base + x + n)
                        .ok_or(GafError::Inconsistent)?
                        .fill(value);
                }
            } else {
                if n > len - position {
                    return Err(fault(row, RleFault::LiteralBytesOutsideFile));
                }
                if let Some(o) = out.as_deref_mut() {
                    let src = data
                        .get(position..position + n)
                        .ok_or(GafError::Inconsistent)?;
                    o.pixels
                        .get_mut(base + x..base + x + n)
                        .ok_or(GafError::Inconsistent)?
                        .copy_from_slice(src);
                }
                position += n;
            }
            x += n;
        }
        position = row_end;
    }
    Ok(())
}

fn set_transparent(out: &mut RleOut<'_>, start: usize, n: usize) -> Result<(), GafError> {
    out.transparent
        .get_mut(start..start + n)
        .ok_or(GafError::Inconsistent)?
        .fill(true);
    Ok(())
}

/// Decodes every frame of `meta` from `data`. Entries that share a source offset
/// share one decoded entry, and a frame reachable twice is decoded once.
pub(super) fn materialize_gaf(
    data: &[u8],
    meta: &GafMetadata,
    keep_direct_raster: bool,
) -> Result<Gaf, GafError> {
    let mut entries: Vec<Arc<GafEntry>> = Vec::with_capacity(meta.entries.len());
    let mut by_offset: HashMap<u32, Arc<GafEntry>> = HashMap::new();
    let mut frames: HashMap<usize, Arc<GafFrame>> = HashMap::new();
    for source in &meta.entries {
        if let Some(cached) = by_offset.get(&source.source_offset) {
            entries.push(Arc::clone(cached));
            continue;
        }
        let mut refs = Vec::with_capacity(source.frames.len());
        for r in &source.frames {
            let frame = materialize_frame(data, &r.frame, &mut frames, keep_direct_raster, false)?;
            refs.push(GafFrameRef {
                offset: r.offset,
                value: r.value,
                frame,
            });
        }
        let entry = Arc::new(GafEntry {
            name: source.name.clone(),
            frame_count: source.frame_count,
            unknown1: source.unknown1,
            unknown2: source.unknown2,
            frames: refs,
        });
        by_offset.insert(source.source_offset, Arc::clone(&entry));
        entries.push(entry);
    }
    Ok(Gaf {
        version: meta.version,
        entry_count: meta.entry_count,
        unknown: meta.unknown,
        entries,
    })
}

/// Decodes one frame and its children. `cache` is keyed by metadata node address.
/// Recursion depth is bounded by the validated composite depth.
pub(super) fn materialize_frame(
    data: &[u8],
    meta: &Arc<GafMetadataFrame>,
    cache: &mut HashMap<usize, Arc<GafFrame>>,
    keep_direct_raster: bool,
    transient: bool,
) -> Result<Arc<GafFrame>, GafError> {
    let key = Arc::as_ptr(meta) as usize;
    if let Some(frame) = cache.get(&key) {
        return Ok(Arc::clone(frame));
    }
    let pixel_count = usize::from(meta.width) * usize::from(meta.height);
    let mut frame = GafFrame {
        transient,
        width: meta.width,
        height: meta.height,
        x_offset: meta.x_offset,
        y_offset: meta.y_offset,
        color_key: meta.color_key,
        compressed: meta.compressed,
        unknown2: meta.unknown2,
        data_offset: meta.data_offset,
        unknown3: meta.unknown3,
        pixels: vec![0; pixel_count],
        transparent: vec![false; pixel_count],
        subframe_count: meta.subframe_count,
        subframes: Vec::new(),
        alternate_blitter: meta.alternate_blitter,
        composite_depth: meta.composite_depth,
        direct_raster: None,
    };
    if meta.subframe_count != 0 {
        // The authored child tables of a nested composite have no established
        // retail layout, so the host composes them recursively.
        frame.transparent.fill(true);
        frame.subframes.reserve_exact(meta.subframes.len());
        for child_meta in &meta.subframes {
            let child = materialize_frame(data, child_meta, cache, keep_direct_raster, transient)?;
            frame.subframes.push(Arc::clone(&child));
            if child.alternate_blitter != 0 {
                continue;
            }
            composite_plain(&mut frame, &child);
        }
    } else if meta.compressed == 0 {
        let start = to_usize(meta.data_offset);
        let src = start
            .checked_add(pixel_count)
            .and_then(|end| data.get(start..end))
            .ok_or(GafError::RawPixelsTruncated {
                offset: meta.data_offset,
            })?;
        frame.pixels.copy_from_slice(src);
        for (t, p) in frame.transparent.iter_mut().zip(frame.pixels.iter()) {
            if *p == frame.color_key {
                *t = true;
            }
        }
    } else {
        // A direct consumer sees the encoded bytes, even on an RLE frame. Keep at
        // most one raster under the per-frame geometry budget. A short file span is
        // rejected by `direct_raster`, without rejecting ordinary decoding.
        let start = to_usize(meta.data_offset);
        if keep_direct_raster {
            if let Some(src) = start
                .checked_add(pixel_count)
                .and_then(|end| data.get(start..end))
            {
                let mut view = frame.clone();
                view.compressed = 0;
                view.pixels = src.to_vec();
                view.transparent = Vec::new();
                frame.direct_raster = Some(Arc::new(view));
            }
        }
        let mut out = RleOut {
            pixels: &mut frame.pixels,
            transparent: &mut frame.transparent,
        };
        walk_rle(
            data,
            meta.width,
            meta.height,
            meta.data_offset,
            Some(&mut out),
            None,
        )?;
    }
    let frame = Arc::new(frame);
    cache.insert(key, Arc::clone(&frame));
    Ok(frame)
}

/// Draws `child`'s plain raster onto `parent`'s, anchored by the offset difference.
/// Transparent child pixels leave the parent alone.
fn composite_plain(parent: &mut GafFrame, child: &GafFrame) {
    let dx = i64::from(parent.x_offset) - i64::from(child.x_offset);
    let dy = i64::from(parent.y_offset) - i64::from(child.y_offset);
    let (pw, ph) = (usize::from(parent.width), usize::from(parent.height));
    let (cw, ch) = (usize::from(child.width), usize::from(child.height));
    for sy in 0..ch {
        let Ok(dy_pos) = usize::try_from(dy + sy as i64) else {
            continue;
        };
        if dy_pos >= ph {
            continue;
        }
        for sx in 0..cw {
            let Ok(dx_pos) = usize::try_from(dx + sx as i64) else {
                continue;
            };
            if dx_pos >= pw {
                continue;
            }
            let sub_index = sy * cw + sx;
            let (Some(&pixel), Some(&clear)) = (
                child.pixels.get(sub_index),
                child.transparent.get(sub_index),
            ) else {
                continue;
            };
            if clear {
                continue;
            }
            let index = dy_pos * pw + dx_pos;
            if let (Some(p), Some(t)) = (
                parent.pixels.get_mut(index),
                parent.transparent.get_mut(index),
            ) {
                *p = pixel;
                *t = false;
            }
        }
    }
}
