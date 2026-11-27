// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/gaf.go, formats/gaf_metadata.go, formats/gaf_source.go, formats/gaf_variant.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! GAF animation banks: reading and decoding.
//!
//! A bank holds named entries, each a sequence of frames. A frame is raw indexed
//! pixels, run-length encoded pixels, or a composite of child frames.
//!
//! - [`Gaf::parse`] validates a bank and decodes every frame.
//! - [`GafMetadata::parse`] validates the same bank without decoding any pixels.
//! - [`GafSource`] keeps the encoded bytes and decodes one frame on request.
//! - [`GafFrame::doubled`] makes the nearest-neighbour 2x variant of a decoded frame.
//!
//! Everything takes bytes. Writers are not ported; the test-only encoder in
//! `gaf/write.rs` exists to build fixtures.
//!
//! Differences from the Go original, all deliberate:
//!
//! - Go's `GAFSource` used `unsafe.Sizeof` to cost a frame for cache accounting.
//!   [`gaf_frame_bytes`] uses `std::mem::size_of`, so the numbers are Rust's, not
//!   Go's. Only relative comparisons are meaningful. The SHA-256 digest comes from
//!   a small safe implementation in this module, since the crate has no hash crate.
//! - Go's `GAFFrame` carries `Pixels` and a separately named `PlainPixels` that the
//!   decoder always aliases. Rust has one plane; [`GafFrame::plain_pixels`] and
//!   [`GafFrame::plain_transparent`] return it.
//! - Names are byte strings, because the lookup folds ASCII letters only and leaves
//!   high bytes alone.
//! - Go's frame pointers become `Arc<GafFrame>`, so aliased children and the
//!   shared-entry reference tables stay shared.
//! - Composite depth limits above [`MAX_COMPOSITE_DEPTH_CEILING`] are refused, so
//!   the recursive index pass cannot exhaust the stack.

mod metadata;
mod source;
mod variant;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod write;

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

pub use metadata::{GafMetadata, GafMetadataEntry, GafMetadataFrame, GafMetadataFrameRef};
pub use source::{gaf_frame_bytes, GafSource};

use metadata::materialize_gaf;

/// Largest frame (width times height) the decoder accepts, in pixels.
pub(crate) const MAX_GAF_FRAME_PIXELS: u64 = 16 << 20;

/// Hard ceiling on [`GafLimits::max_composite_depth`]. Go recurses to whatever depth
/// the limit allows; Rust refuses limits that could exhaust the stack.
pub const MAX_COMPOSITE_DEPTH_CEILING: u32 = 1024;

/// Everything that can go wrong reading a GAF. The `Display` text follows the Go
/// messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GafError {
    /// The file is shorter than the 12-byte header.
    FileTooSmall,
    /// A limit is zero, or the composite depth is above [`MAX_COMPOSITE_DEPTH_CEILING`].
    InvalidLimits,
    /// The entry offset table runs past the end of the file.
    EntryTableTruncated,
    /// An entry offset leaves fewer than 40 bytes for the entry header.
    EntryOutsideFile {
        index: u64,
    },
    /// An entry's frame reference table runs past the end of the file.
    FrameTableTruncated {
        entry: String,
    },
    AggregateFrameRefsExceed,
    AggregateDecodedPixelsExceed,
    AggregateExpandedFramesExceed,
    AggregateExpandedPixelsExceed,
    AggregateRleCommandsExceed,
    ExpandedFramesExceed,
    ExpandedPixelsExceed,
    CompositeDepthExceeded,
    FrameCycle {
        offset: u32,
    },
    FramePointerOutsideFile {
        offset: u32,
    },
    FrameDataPointerOutsideFile {
        offset: u32,
    },
    FrameTooLarge {
        offset: u32,
    },
    BadCompression {
        offset: u32,
        value: u8,
    },
    SubframeTableTruncated {
        offset: u32,
    },
    RawPixelsTruncated {
        offset: u32,
    },
    /// A fault inside a frame's RLE stream; `row` is the pixel row.
    Rle {
        row: usize,
        kind: RleFault,
    },
    /// `source` happened while reading entry `entry`.
    InEntry {
        entry: String,
        source: Box<GafError>,
    },
    /// `source` happened while reading frame `frame` of entry `entry`.
    InEntryFrame {
        entry: String,
        frame: usize,
        source: Box<GafError>,
    },
    /// `source` happened while reading the frame header at `offset`.
    InFrame {
        offset: u32,
        source: Box<GafError>,
    },
    /// A frame request to a [`GafSource`] had a zero pixel budget.
    InvalidFrameRequest,
    /// The requested entry or frame index does not exist.
    MissingEntryFrame {
        name: String,
        index: usize,
    },
    /// The selected frame's expanded pixel count is above the caller's budget.
    SelectedExpandedPixelsExceed,
    /// The selected frame's unique pixel count is above the caller's budget.
    SelectedPixelsExceed,
    /// A frame is too large to double (Go would allocate whatever the header says).
    DoubledFrameTooLarge,
    /// Metadata and bytes disagree, which means the metadata did not come from
    /// these bytes. Go would index out of range.
    Inconsistent,
}

/// The fault found in a row of RLE data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RleFault {
    RowHeaderTruncated,
    RowExtentOutsideFile,
    CommandOutsideFile,
    RepeatByteOutsideFile,
    LiteralBytesOutsideFile,
}

impl GafError {
    /// The error beneath any entry/frame context wrappers.
    pub fn innermost(&self) -> &GafError {
        match self {
            GafError::InEntry { source, .. }
            | GafError::InEntryFrame { source, .. }
            | GafError::InFrame { source, .. } => source.innermost(),
            other => other,
        }
    }
}

impl fmt::Display for RleFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RleFault::RowHeaderTruncated => "header is truncated",
            RleFault::RowExtentOutsideFile => "extent is outside file",
            RleFault::CommandOutsideFile => "command is outside file",
            RleFault::RepeatByteOutsideFile => "repeat byte is outside file",
            RleFault::LiteralBytesOutsideFile => "literal bytes are outside file",
        })
    }
}

impl fmt::Display for GafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GafError::FileTooSmall => write!(f, "gaf: file is too small"),
            GafError::InvalidLimits => write!(f, "gaf: invalid decode limits"),
            GafError::EntryTableTruncated => write!(f, "gaf: entry offset table is truncated"),
            GafError::EntryOutsideFile { index } => {
                write!(f, "gaf: entry {index} points outside file")
            }
            GafError::FrameTableTruncated { entry } => {
                write!(f, "gaf: entry {entry:?} frame table is truncated")
            }
            GafError::AggregateFrameRefsExceed => {
                write!(f, "aggregate frame references exceed limit")
            }
            GafError::AggregateDecodedPixelsExceed => {
                write!(f, "aggregate decoded pixels exceed limit")
            }
            GafError::AggregateExpandedFramesExceed => {
                write!(f, "aggregate expanded frames exceed limit")
            }
            GafError::AggregateExpandedPixelsExceed => {
                write!(f, "aggregate expanded pixels exceed limit")
            }
            GafError::AggregateRleCommandsExceed => {
                write!(f, "aggregate RLE commands exceed limit")
            }
            GafError::ExpandedFramesExceed => write!(f, "expanded frames exceed limit"),
            GafError::ExpandedPixelsExceed => write!(f, "expanded pixels exceed limit"),
            GafError::CompositeDepthExceeded => write!(f, "composite depth exceeds limit"),
            GafError::FrameCycle { offset } => write!(f, "frame cycle at 0x{offset:x}"),
            GafError::FramePointerOutsideFile { offset } => {
                write!(f, "frame pointer 0x{offset:x} is outside file")
            }
            GafError::FrameDataPointerOutsideFile { offset } => {
                write!(f, "frame 0x{offset:x} data pointer is outside file")
            }
            GafError::FrameTooLarge { offset } => write!(f, "frame 0x{offset:x} is too large"),
            GafError::BadCompression { offset, value } => {
                write!(f, "frame 0x{offset:x} has compression {value}")
            }
            GafError::SubframeTableTruncated { offset } => {
                write!(f, "frame 0x{offset:x} subframe table is truncated")
            }
            GafError::RawPixelsTruncated { offset } => {
                write!(f, "frame 0x{offset:x} raw pixels are truncated")
            }
            GafError::Rle { row, kind } => write!(f, "RLE row {row} {kind}"),
            GafError::InEntry { entry, source } => write!(f, "gaf: entry {entry:?}: {source}"),
            GafError::InEntryFrame {
                entry,
                frame,
                source,
            } => write!(f, "gaf: entry {entry:?} frame {frame}: {source}"),
            GafError::InFrame { offset, source } => write!(f, "frame 0x{offset:x}: {source}"),
            GafError::InvalidFrameRequest => write!(f, "gaf: invalid frame request"),
            GafError::MissingEntryFrame { name, index } => {
                write!(f, "gaf: missing entry frame {name:?} {index}")
            }
            GafError::SelectedExpandedPixelsExceed => {
                write!(f, "gaf: selected frame expanded pixels exceed limit")
            }
            GafError::SelectedPixelsExceed => {
                write!(f, "gaf: selected frame pixels exceed limit")
            }
            GafError::DoubledFrameTooLarge => write!(f, "gaf: frame is too large to double"),
            GafError::Inconsistent => write!(f, "gaf: metadata does not match the file bytes"),
        }
    }
}

impl std::error::Error for GafError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GafError::InEntry { source, .. }
            | GafError::InEntryFrame { source, .. }
            | GafError::InFrame { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

/// Bounds on aggregate host allocation while decoding a bank. They are
/// implementation safety limits, not retail format behaviour.
///
/// A zero `max_frame_refs`, `max_decoded_pixels` or `max_composite_depth` is
/// invalid. A zero in the other three selects the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GafLimits {
    pub max_frame_refs: u64,
    pub max_decoded_pixels: u64,
    pub max_composite_depth: u32,
    /// Counts every child occurrence below each distinct root frame, including
    /// aliases. It bounds later recursive drawing, which can visit a shared subtree
    /// many times although decoding it once is cheap.
    pub max_expanded_frames: u64,
    pub max_expanded_pixels: u64,
    /// Bounds decoded RLE commands across unique frames, including zero-length
    /// skips and overlapping short-row reads.
    pub max_rle_commands: u64,
}

impl Default for GafLimits {
    /// The host-safety budget for one decoded bank.
    fn default() -> Self {
        GafLimits {
            max_frame_refs: 1 << 20,
            max_decoded_pixels: 128 << 20,
            max_composite_depth: 64,
            max_expanded_frames: 1 << 16,
            max_expanded_pixels: 128 << 20,
            max_rle_commands: 128 << 20,
        }
    }
}

/// A decoded animation bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gaf {
    pub version: u32,
    pub entry_count: u32,
    pub unknown: u32,
    /// In file table order. Two table slots with the same offset share one `Arc`.
    pub entries: Vec<Arc<GafEntry>>,
}

/// One named sequence within a bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GafEntry {
    /// The name bytes up to the first NUL, at most 32.
    pub name: Vec<u8>,
    pub frame_count: u16,
    pub unknown1: u16,
    pub unknown2: u32,
    pub frames: Vec<GafFrameRef>,
}

impl GafEntry {
    /// The name as text; invalid UTF-8 is replaced.
    pub fn name_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.name)
    }
}

/// One frame slot of an entry: the source offset it was read from, the authored
/// word beside it, and the decoded frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GafFrameRef {
    pub offset: u32,
    pub value: u32,
    pub frame: Arc<GafFrame>,
}

/// Decoded indexed pixels. `transparent` is parallel to `pixels`; a false value
/// means the pixel is opaque, even when it is palette index zero. `pixels` always
/// keeps the raw decoded bytes: a colour-key match is recorded in `transparent`
/// and never overwrites `pixels`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GafFrame {
    /// A host cache-lifetime hint for on-demand rasters. Not authored.
    pub transient: bool,
    pub width: u16,
    pub height: u16,
    pub x_offset: i16,
    pub y_offset: i16,
    /// Frame header byte +8. On the raw path (`compressed == 0`) the indexed blitter
    /// skips every source pixel equal to this byte; it is 9 in every retail frame.
    /// The RLE path carries its own skip runs and ignores the key.
    pub color_key: u8,
    pub compressed: u8,
    pub unknown2: u32,
    pub data_offset: u32,
    pub unknown3: u32,
    pub pixels: Vec<u8>,
    pub transparent: Vec<bool>,
    /// The raw low byte beside `alternate_blitter`: the effective child count,
    /// including when it is zero.
    pub subframe_count: u8,
    pub subframes: Vec<Arc<GafFrame>>,
    /// The raw high byte beside the child count. Nonzero selects ALP composition
    /// when this frame is drawn as a child; kept for round-trip fidelity.
    pub alternate_blitter: u8,
    pub(crate) composite_depth: u32,
    pub(crate) direct_raster: Option<Arc<GafFrame>>,
}

impl GafFrame {
    /// The palette index at (x, y) and whether that pixel is opaque. A coordinate
    /// outside the frame reads as transparent.
    pub fn at(&self, x: i32, y: i32) -> (u8, bool) {
        let (Ok(xu), Ok(yu)) = (usize::try_from(x), usize::try_from(y)) else {
            return (0, false);
        };
        let (w, h) = (usize::from(self.width), usize::from(self.height));
        if xu >= w || yu >= h {
            return (0, false);
        }
        let index = yu * w + xu;
        match (self.pixels.get(index), self.transparent.get(index)) {
            (Some(p), Some(t)) => (*p, !*t),
            _ => (0, false),
        }
    }

    /// The ordinary keyed raster of this frame. For a composite it holds only the
    /// children that take the ordinary path; alternate children are absent because
    /// ALP reads the destination at draw time. Same plane as `pixels`.
    pub fn plain_pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Transparency of [`plain_pixels`](Self::plain_pixels). Same plane as `transparent`.
    pub fn plain_transparent(&self) -> &[bool] {
        &self.transparent
    }

    /// The maximum number of subframe links below this frame.
    pub fn composite_depth(&self) -> u32 {
        self.composite_depth
    }

    /// The frame's row-major storage without expanding RLE or composing children.
    /// `None` is a host safety rejection, not a transparent retail image.
    pub fn direct_raster(&self) -> Option<&GafFrame> {
        // Composite direct readers sample runtime-relocated child storage that the
        // authored file cannot determine, so composites have no direct raster.
        if self.subframe_count != 0 || !self.subframes.is_empty() {
            return None;
        }
        if self.compressed != 0 {
            return self.direct_raster.as_deref();
        }
        let need = usize::from(self.width) * usize::from(self.height);
        if self.pixels.len() < need {
            return None;
        }
        Some(self)
    }
}

impl Gaf {
    /// Validates and decodes a bank under the default limits.
    pub fn parse(data: &[u8]) -> Result<Gaf, GafError> {
        Gaf::parse_with_limits(data, GafLimits::default())
    }

    /// Validates and decodes a bank under explicit limits.
    pub fn parse_with_limits(data: &[u8], limits: GafLimits) -> Result<Gaf, GafError> {
        let meta = GafMetadata::parse_with_limits(data, limits)?;
        materialize_gaf(data, &meta, true)
    }

    /// Like [`Gaf::parse`], but keeps no encoded RLE copy for
    /// [`GafFrame::direct_raster`]. It validates exactly as much.
    pub fn parse_decoded(data: &[u8]) -> Result<Gaf, GafError> {
        let meta = GafMetadata::parse(data)?;
        materialize_gaf(data, &meta, false)
    }

    /// The first entry whose name matches, in table order. ASCII letters fold;
    /// high bytes compare unchanged.
    pub fn find(&self, name: impl AsRef<[u8]>) -> Option<&GafEntry> {
        find_by_folded_name(&self.entries, name.as_ref(), |e| &e.name)
    }
}

/// First match in table order under ASCII case folding. The Go helper
/// (`findByFoldedName`) lives outside the files this port may read; this follows the
/// documented rule: "ASCII letter folding and unchanged high bytes".
pub(crate) fn find_by_folded_name<'a, T>(
    entries: &'a [Arc<T>],
    name: &[u8],
    name_of: impl Fn(&T) -> &[u8],
) -> Option<&'a T> {
    entries
        .iter()
        .map(|e| e.as_ref())
        .find(|e| name_of(e).eq_ignore_ascii_case(name))
}
