// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/tnt.go, formats/tnt_write.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! TNT terrain files: the map's tiles, height and feature grid, feature names and
//! minimap. No terrain interpretation happens here.
//!
//! Two versions are accepted, and any other version word is an error:
//!
//! - 0x2000, canonical: the attribute map is 4 bytes a cell (height, a `u16` feature
//!   word, one unused byte). Header slot 10 is the minimap offset and bit 0 of slot 11
//!   its present flag. Wind and gravity are not stored.
//! - 0x1020, legacy: the attribute map is 8 bytes a cell. Slots 10, 11 and 13 carry
//!   minimum wind, maximum wind and gravity; the minimap offset moves to slot 14 and its
//!   flag to bit 0 of slot 15. Byte 0 is the height, byte 2 a one-byte feature reference,
//!   byte 6 the per-cell metal seed; bytes 1, 3, 4, 5 and 7 are never read.
//!
//! A feature word below [`FEATURE_SENTINEL_BASE`] (0xFFFB) is an index into the feature
//! table. 0xFFFB and above is a sentinel band (0xFFFF empty, 0xFFFE footprint fringe,
//! 0xFFFD void, with 0xFFFC, the retail on-disk void, and 0xFFFB acting as further void
//! thresholds). A legacy one-byte reference of 0xFC and above is the legacy sentinel
//! band, expanded here to the canonical empty word 0xFFFF; the legacy record does not
//! tell empty from void, and calling it void would block movement on bare terrain.
//!
//! Differences from the Go source:
//! - Go copied the whole input once into `Raw` and made `TileGraphics` and `Minimap`
//!   views into it. Here [`Tnt::raw`] owns the one copy, taken after every check has
//!   passed, and [`Tnt::tile_graphics`] and [`Tnt::minimap`] return slices of it.
//! - Section sizes are checked with `checked_mul`; Go's two dimension-overflow tests
//!   (`width * height`, `tile width * tile height`) are dropped because a product of two
//!   `u32` cannot overflow a `u64`. The minimap pixel count is bounded the same way.
//! - The unsupported-version message no longer carries Nanolathe's spec citations.
//! - `EncodeTNT` (a writer) is a test fixture builder only.
//! - `LoadTNTFile` (VFS read) is not ported.
//!
//! Allocation is bounded by the input. Every section is checked against the file's length
//! before a vector is made for it, so the tile index map, attribute map, feature table
//! and tile graphics each take at most the bytes the file holds for them.

use std::fmt;
use std::ops::Range;

use crate::le::u32_at;

/// Version word of the legacy layout.
pub const VERSION_LEGACY: u32 = 0x1020;
/// Version word of the canonical layout.
pub const VERSION_CANONICAL: u32 = 0x2000;

/// The exclusive upper bound for real feature-record indices in an attribute cell.
/// Consumers test below it before dereferencing, so 0xFFFB..=0xFFFF is a sentinel band.
pub const FEATURE_SENTINEL_BASE: u16 = 0xFFFB;

/// The legacy record's sentinel band: a one-byte reference of this or more is void or
/// none.
const LEGACY_FEATURE_SENTINEL_BASE: u8 = 0xFC;

const HEADER_BYTES: usize = 0x40;
const TILE_BYTES: u64 = 32 * 32;
const FEATURE_RECORD_BYTES: usize = 132;

/// Bounds on what one terrain file may declare. A zero field is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TntLimits {
    pub max_width: u32,
    pub max_height: u32,
    pub max_cells: u64,
    pub max_tiles: u32,
    pub max_feature_records: u32,
    pub max_minimap_pixels: u64,
}

impl Default for TntLimits {
    /// The decode bounds used when a caller states none: large enough for the retail
    /// corpus, but finite.
    fn default() -> Self {
        TntLimits {
            max_width: 8192,
            max_height: 8192,
            max_cells: 16 << 20,
            max_tiles: 1 << 20,
            max_feature_records: 1 << 16,
            max_minimap_pixels: 16 << 20,
        }
    }
}

/// Which part of the file a bounds error names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TntSection {
    TileIndexMap,
    AttributeMap,
    TileGraphics,
    FeatureTable,
    MinimapHeader,
    MinimapPixels,
}

impl TntSection {
    fn name(self) -> &'static str {
        match self {
            TntSection::TileIndexMap => "tile index map",
            TntSection::AttributeMap => "attribute map",
            TntSection::TileGraphics => "tile graphics",
            TntSection::FeatureTable => "feature table",
            TntSection::MinimapHeader => "minimap header",
            TntSection::MinimapPixels => "minimap pixels",
        }
    }
}

/// Everything that can go wrong reading a terrain file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TntError {
    /// Fewer than 0x40 bytes.
    FileTooSmall,
    /// A limit is zero.
    InvalidLimits,
    /// The version word is neither 0x1020 nor 0x2000.
    UnsupportedVersion { version: u32 },
    /// A dimension is zero or above its limit.
    DimensionsExceedLimits { width: u32, height: u32 },
    /// A dimension is odd (a tile covers 2 by 2 cells).
    OddDimensions { width: u32, height: u32 },
    /// The tile count or feature-record count is above its limit.
    CountsExceedLimits,
    /// `width * height` is above the cell limit.
    CellsExceedLimit { cells: u64, limit: u64 },
    /// A section's size does not fit in 64 bits.
    SectionSizeOverflows(TntSection),
    /// A section lies outside the file.
    SectionOutsideFile(TntSection),
    /// A tile index in the index map is at or above the tile count.
    TileIndexOutOfRange { index: u16, cell: usize, tiles: u32 },
    /// An attribute cell names a feature record the table does not have.
    FeatureIndexOutOfRange {
        index: u16,
        cell: usize,
        records: u32,
    },
    /// The minimap is larger than the pixel limit.
    MinimapExceedsLimits,
    /// The minimap offset plus its 8-byte header does not fit in 32 bits.
    MinimapOffsetOverflows,
}

impl fmt::Display for TntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TntError::FileTooSmall => write!(f, "tnt: file is too small"),
            TntError::InvalidLimits => write!(f, "tnt: invalid decode limits"),
            TntError::UnsupportedVersion { version } => write!(
                f,
                "tnt: unsupported version {version:#06x}, want 0x1020 or 0x2000"
            ),
            TntError::DimensionsExceedLimits { width, height } => {
                write!(f, "tnt: dimensions {width}x{height} exceed limits")
            }
            TntError::OddDimensions { width, height } => {
                write!(f, "tnt: dimensions must be even, got {width}x{height}")
            }
            TntError::CountsExceedLimits => write!(f, "tnt: counts exceed limits"),
            TntError::CellsExceedLimit { cells, limit } => {
                write!(f, "tnt: attribute cells {cells} exceed limit {limit}")
            }
            TntError::SectionSizeOverflows(s) => write!(f, "tnt: {} size overflows", s.name()),
            TntError::SectionOutsideFile(s) => write!(f, "tnt: {} is outside file", s.name()),
            TntError::TileIndexOutOfRange { index, cell, tiles } => write!(
                f,
                "tnt: tile index {index} at cell {cell} exceeds tile count {tiles}"
            ),
            TntError::FeatureIndexOutOfRange {
                index,
                cell,
                records,
            } => write!(
                f,
                "tnt: feature index {index} at cell {cell} exceeds feature record count {records}"
            ),
            TntError::MinimapExceedsLimits => write!(f, "tnt: minimap exceeds limits"),
            TntError::MinimapOffsetOverflows => write!(f, "tnt: minimap offset overflows"),
        }
    }
}

impl std::error::Error for TntError {}

/// One 16 by 16 source cell.
///
/// `feature` is a `u16` on purpose: 0xFFFC, 0xFFFE and 0xFFFF are distinct source
/// sentinels, and 0xFFFD is the derived void.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TntAttribute {
    pub height: u8,
    pub feature: u16,
    /// Canonical byte 3: zero across the retail corpus, not interpreted.
    pub unknown: u8,
    /// Legacy byte 6: the per-cell metal seed. Zero on canonical records, where the
    /// metal is seeded from the mission's surface-metal scalar at load.
    pub metal: u8,
}

/// One entry of the feature-name table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TntFeatureRecord {
    pub index: u32,
    /// The name up to its first NUL, as bytes (retail names are Windows-1252).
    pub name: Vec<u8>,
}

/// The decoded source data of a `.tnt` file.
///
/// The header words are kept as authored, including the section pointers; the fields
/// below that are version-dependent are resolved already. Do not index raw header
/// slots: reading slot 10 as an offset on a legacy file seeks to the minimum wind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tnt {
    raw: Vec<u8>,
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub tiles: u32,
    /// The feature-record count (the file calls these tile animations).
    pub tile_anims: u32,
    pub sea_level: u32,
    pub ptr_map_data: u32,
    pub ptr_map_attr: u32,
    pub ptr_tile_gfx: u32,
    pub ptr_tile_anims: u32,
    pub minimap_offset: u32,
    pub minimap_present: bool,
    /// Legacy only; zero on canonical files, where the engine hard-codes wind 100 to
    /// 2000 and gravity 0.
    pub legacy_min_wind: u32,
    pub legacy_max_wind: u32,
    pub legacy_gravity: u32,
    /// Header slot 11 on canonical files: always 1 across the retail corpus. Zero on
    /// legacy files.
    pub unknown1: u32,
    /// `width / 2`.
    pub tile_map_width: u32,
    /// `height / 2`.
    pub tile_map_height: u32,
    pub tile_indices: Vec<u16>,
    /// `width * height` cells, row-major.
    pub attributes: Vec<TntAttribute>,
    pub feature_table: Vec<TntFeatureRecord>,
    pub minimap_width: u32,
    pub minimap_height: u32,
    tile_index_bytes: Range<usize>,
    tile_graphics: Range<usize>,
    minimap: Range<usize>,
}

/// The byte range of a section after checking it lies inside the file.
fn section(
    len: usize,
    name: TntSection,
    offset: u32,
    count: u64,
    stride: u64,
) -> Result<Range<usize>, TntError> {
    let size = count
        .checked_mul(stride)
        .ok_or(TntError::SectionSizeOverflows(name))?;
    let len = len as u64;
    let offset = u64::from(offset);
    if offset > len || size > len - offset {
        return Err(TntError::SectionOutsideFile(name));
    }
    // Both ends are at most `len`, which is a `usize`.
    let start = usize::try_from(offset).map_err(|_| TntError::SectionOutsideFile(name))?;
    let end = usize::try_from(offset + size).map_err(|_| TntError::SectionOutsideFile(name))?;
    Ok(start..end)
}

impl Tnt {
    /// Decode a terrain file under the default limits.
    pub fn parse(data: &[u8]) -> Result<Tnt, TntError> {
        Self::parse_with_limits(data, TntLimits::default())
    }

    /// Decode a terrain file under explicit limits.
    pub fn parse_with_limits(data: &[u8], limits: TntLimits) -> Result<Tnt, TntError> {
        if data.len() < HEADER_BYTES {
            return Err(TntError::FileTooSmall);
        }
        if limits.max_width == 0
            || limits.max_height == 0
            || limits.max_cells == 0
            || limits.max_tiles == 0
            || limits.max_feature_records == 0
            || limits.max_minimap_pixels == 0
        {
            return Err(TntError::InvalidLimits);
        }
        // The header is 0x40 bytes, so every read below is in range.
        let word = |offset: usize| u32_at(data, offset).ok_or(TntError::FileTooSmall);
        let version = word(0)?;
        let width = word(0x04)?;
        let height = word(0x08)?;
        let ptr_map_data = word(0x0c)?;
        let ptr_map_attr = word(0x10)?;
        let ptr_tile_gfx = word(0x14)?;
        let tiles = word(0x18)?;
        let tile_anims = word(0x1c)?;
        let ptr_tile_anims = word(0x20)?;
        let sea_level = word(0x24)?;

        let (minimap_offset, minimap_present, legacy_min_wind, legacy_max_wind, legacy_gravity);
        let mut unknown1 = 0;
        // Header slots 10..15 depend on the version; resolve them before any section
        // offset is used.
        let legacy = match version {
            VERSION_CANONICAL => {
                minimap_offset = word(0x28)?; // slot 10
                unknown1 = word(0x2c)?; // slot 11, always 1 in the retail corpus
                minimap_present = unknown1 & 1 != 0;
                (legacy_min_wind, legacy_max_wind, legacy_gravity) = (0, 0, 0);
                false
            }
            VERSION_LEGACY => {
                legacy_min_wind = word(0x28)?; // slot 10
                legacy_max_wind = word(0x2c)?; // slot 11
                legacy_gravity = word(0x34)?; // slot 13
                minimap_offset = word(0x38)?; // slot 14
                minimap_present = word(0x3c)? & 1 != 0; // slot 15
                true
            }
            _ => return Err(TntError::UnsupportedVersion { version }),
        };
        if width == 0 || height == 0 || width > limits.max_width || height > limits.max_height {
            return Err(TntError::DimensionsExceedLimits { width, height });
        }
        if width % 2 != 0 || height % 2 != 0 {
            return Err(TntError::OddDimensions { width, height });
        }
        if tiles > limits.max_tiles || tile_anims > limits.max_feature_records {
            return Err(TntError::CountsExceedLimits);
        }
        let (tile_map_width, tile_map_height) = (width / 2, height / 2);

        // Each of these products of two `u32` fits a `u64`.
        let attr_count = u64::from(width) * u64::from(height);
        if attr_count > limits.max_cells {
            return Err(TntError::CellsExceedLimit {
                cells: attr_count,
                limit: limits.max_cells,
            });
        }
        let tile_count = u64::from(tile_map_width) * u64::from(tile_map_height);

        let len = data.len();
        let index_range = section(len, TntSection::TileIndexMap, ptr_map_data, tile_count, 2)?;
        let index_bytes = data.get(index_range.clone()).unwrap_or(&[]);
        let mut tile_indices = Vec::with_capacity(index_bytes.len() / 2);
        for (cell, pair) in index_bytes.as_chunks::<2>().0.iter().enumerate() {
            let index = u16::from_le_bytes(*pair);
            if u32::from(index) >= tiles {
                return Err(TntError::TileIndexOutOfRange { index, cell, tiles });
            }
            tile_indices.push(index);
        }

        // The legacy 8-byte record, or the canonical 4-byte one.
        let attr_stride: usize = if legacy { 8 } else { 4 };
        let attr_range = section(
            len,
            TntSection::AttributeMap,
            ptr_map_attr,
            attr_count,
            attr_stride as u64,
        )?;
        let attr_bytes = data.get(attr_range).unwrap_or(&[]);
        let mut attributes = Vec::with_capacity(attr_bytes.len() / attr_stride);
        for (cell, record) in attr_bytes.chunks_exact(attr_stride).enumerate() {
            let at = |i: usize| record.get(i).copied().unwrap_or(0);
            let (feature, metal, unknown);
            if legacy {
                // Byte 0 height, byte 2 a one-byte feature reference with a single
                // 0xFC+ sentinel band, byte 6 the per-cell metal seed. The band is
                // expanded to the canonical empty sentinel.
                feature = if at(2) >= LEGACY_FEATURE_SENTINEL_BASE {
                    0xFFFF
                } else {
                    u16::from(at(2))
                };
                metal = at(6);
                unknown = 0;
            } else {
                // Bytes 1..3 are a little-endian word, unaligned.
                feature = u16::from_le_bytes([at(1), at(2)]);
                metal = 0;
                unknown = at(3);
            }
            // Values at or above 0xFFFB are the sentinel band, not indices: 0xFFFF
            // empty, 0xFFFE footprint fringe, 0xFFFD void hole, with 0xFFFB and 0xFFFC
            // acting as further void thresholds because consumers test below 0xFFFB
            // before dereferencing. The retail corpus writes 0xFFFC for void on disk;
            // the whole band is accepted. Legacy indices were normalized below it above.
            if feature < FEATURE_SENTINEL_BASE && u32::from(feature) >= tile_anims {
                return Err(TntError::FeatureIndexOutOfRange {
                    index: feature,
                    cell,
                    records: tile_anims,
                });
            }
            attributes.push(TntAttribute {
                height: at(0),
                feature,
                unknown,
                metal,
            });
        }

        let tile_graphics = section(
            len,
            TntSection::TileGraphics,
            ptr_tile_gfx,
            u64::from(tiles),
            TILE_BYTES,
        )?;
        let feature_range = section(
            len,
            TntSection::FeatureTable,
            ptr_tile_anims,
            u64::from(tile_anims),
            FEATURE_RECORD_BYTES as u64,
        )?;
        let feature_bytes = data.get(feature_range).unwrap_or(&[]);
        let mut feature_table = Vec::with_capacity(feature_bytes.len() / FEATURE_RECORD_BYTES);
        for record in feature_bytes.as_chunks::<FEATURE_RECORD_BYTES>().0 {
            let index = u32_at(record.as_slice(), 0).unwrap_or(0);
            let name_bytes = record.get(4..).unwrap_or(&[]);
            // Up to the first NUL, or all 128 bytes when there is none.
            let end = name_bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(name_bytes.len());
            feature_table.push(TntFeatureRecord {
                index,
                name: name_bytes.get(..end).unwrap_or(&[]).to_vec(),
            });
        }

        let (mut minimap_width, mut minimap_height) = (0, 0);
        let mut minimap = 0..0;
        if minimap_present {
            let header = section(len, TntSection::MinimapHeader, minimap_offset, 1, 8)?;
            minimap_width = u32_at(data, header.start).ok_or(TntError::FileTooSmall)?;
            minimap_height = u32_at(data, header.start + 4).ok_or(TntError::FileTooSmall)?;
            let pixels = u64::from(minimap_width) * u64::from(minimap_height);
            if pixels > limits.max_minimap_pixels {
                return Err(TntError::MinimapExceedsLimits);
            }
            let pixels_at = minimap_offset
                .checked_add(8)
                .ok_or(TntError::MinimapOffsetOverflows)?;
            minimap = section(len, TntSection::MinimapPixels, pixels_at, pixels, 1)?;
        }

        // Every check has passed: take the one copy of the source.
        Ok(Tnt {
            raw: data.to_vec(),
            version,
            width,
            height,
            tiles,
            tile_anims,
            sea_level,
            ptr_map_data,
            ptr_map_attr,
            ptr_tile_gfx,
            ptr_tile_anims,
            minimap_offset,
            minimap_present,
            legacy_min_wind,
            legacy_max_wind,
            legacy_gravity,
            unknown1,
            tile_map_width,
            tile_map_height,
            tile_indices,
            attributes,
            feature_table,
            minimap_width,
            minimap_height,
            tile_index_bytes: index_range,
            tile_graphics,
            minimap,
        })
    }

    /// The single copy of the source bytes that the views below point into. Immutable.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// The tile index map as stored: `tile_map_width * tile_map_height` little-endian
    /// words.
    pub fn tile_index_bytes(&self) -> &[u8] {
        self.raw.get(self.tile_index_bytes.clone()).unwrap_or(&[])
    }

    /// `tiles * 32 * 32` palette indices, one 32 by 32 tile after another, row-major.
    pub fn tile_graphics(&self) -> &[u8] {
        self.raw.get(self.tile_graphics.clone()).unwrap_or(&[])
    }

    /// `minimap_width * minimap_height` palette indices, or empty when the file has no
    /// minimap.
    pub fn minimap(&self) -> &[u8] {
        self.raw.get(self.minimap.clone()).unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )]

    use super::*;

    fn put32(b: &mut [u8], at: usize, v: u32) {
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// Port of `tntHeader`: a 0x40-byte header with the given version word, 4 by 4
    /// cells, and every section pointer zero.
    fn tnt_header(version: u32) -> Vec<u8> {
        let mut b = vec![0u8; 0x40];
        put32(&mut b, 0, version);
        put32(&mut b, 0x04, 4);
        put32(&mut b, 0x08, 4);
        b
    }

    /// Port of `legacyFixture`: a minimal, structurally valid legacy (0x1020) file, 2 by
    /// 2 cells, one tile, one feature record, no minimap.
    fn legacy_fixture() -> Vec<u8> {
        let mut attr = vec![0u8; 4 * 8];
        // Cell 0: height 5, feature 0xFF (sentinel band -> empty), metal 7.
        attr[0] = 5;
        attr[2] = 0xFF;
        attr[6] = 7;
        // Cell 1: height 6, feature 0 (live index), metal 8.
        attr[8] = 6;
        attr[10] = 0;
        attr[14] = 8;
        let mut b = tnt_header(VERSION_LEGACY);
        put32(&mut b, 0x04, 2); // width
        put32(&mut b, 0x08, 2); // height
        put32(&mut b, 0x0c, 0x40); // tile map
        put32(&mut b, 0x10, 0x40 + 2); // attribute array
        put32(&mut b, 0x14, 0x40 + 2 + 32); // tile graphics
        put32(&mut b, 0x18, 1); // tile count
        put32(&mut b, 0x1c, 1); // feature records
        put32(&mut b, 0x20, 0x40 + 2 + 32 + 1024); // feature table
        put32(&mut b, 0x24, 0); // sea level
        put32(&mut b, 0x28, 500); // slot 10: min wind
        put32(&mut b, 0x2c, 800); // slot 11: max wind
        put32(&mut b, 0x34, 112); // slot 13: gravity
        put32(&mut b, 0x38, 0x40 + 2 + 32 + 1024 + 132); // slot 14: minimap offset
        put32(&mut b, 0x3c, 0); // slot 15: minimap flag
        b.extend_from_slice(&[0, 0]); // one tile index
        b.extend_from_slice(&attr); // 4 x 8-byte attribute records
        b.extend_from_slice(&[0u8; 1024]); // one tile
        let mut feat = vec![0u8; 132];
        feat[4..8].copy_from_slice(b"tree");
        b.extend_from_slice(&feat);
        b.extend_from_slice(&[0u8; 8]); // minimap header (width/height, zeroed)
        b
    }

    /// Port of `TNTWrite`: a canonical (0x2000) terrain file to encode.
    #[derive(Default)]
    struct TntWrite {
        width: usize,
        height: usize,
        sea_level: u8,
        tile_indices: Vec<u16>,
        heights: Vec<u8>,
        features: Vec<u16>,
        tile_graphics: Vec<u8>,
        feature_names: Vec<&'static str>,
        minimap_width: usize,
        minimap_height: usize,
        minimap: Vec<u8>,
    }

    /// Port of `EncodeTNT`, a writer, kept only to build canonical fixtures.
    fn encode_tnt(t: &TntWrite) -> Result<Vec<u8>, String> {
        let cells = t.width * t.height;
        if t.width == 0
            || t.height == 0
            || !t.width.is_multiple_of(2)
            || !t.height.is_multiple_of(2)
        {
            return Err(format!(
                "{}x{} cells, expected positive even dimensions",
                t.width, t.height
            ));
        }
        if t.tile_indices.len() != (t.width / 2) * (t.height / 2) {
            return Err("wrong tile index count".into());
        }
        if t.heights.len() != cells || t.features.len() != cells {
            return Err("wrong heights or feature words count".into());
        }
        if !t.tile_graphics.len().is_multiple_of(1024) || t.tile_graphics.is_empty() {
            return Err("expected whole 32x32 tiles".into());
        }
        if t.minimap.len() != t.minimap_width * t.minimap_height {
            return Err("minimap size mismatch".into());
        }
        let tiles = t.tile_graphics.len() / 1024;
        if t.tile_indices.iter().any(|&v| usize::from(v) >= tiles) {
            return Err("tile index past the tiles".into());
        }
        if t.features
            .iter()
            .any(|&f| f < 0xFFFB && usize::from(f) >= t.feature_names.len())
        {
            return Err("feature index past the records".into());
        }
        if t.feature_names
            .iter()
            .any(|n| n.is_empty() || n.len() >= 128)
        {
            return Err("feature name must be 1..127 bytes".into());
        }
        let map_data = 0x40;
        let map_attr = map_data + 2 * t.tile_indices.len();
        let tile_gfx = map_attr + 4 * t.heights.len();
        let tile_anims = tile_gfx + t.tile_graphics.len();
        let mini_map = tile_anims + 132 * t.feature_names.len();
        let mut size = mini_map;
        if !t.minimap.is_empty() {
            size += 8 + t.minimap.len();
        }
        let mut out = vec![0u8; size];
        put32(&mut out, 0x00, 0x2000);
        put32(&mut out, 0x04, t.width as u32);
        put32(&mut out, 0x08, t.height as u32);
        put32(&mut out, 0x0C, map_data as u32);
        put32(&mut out, 0x10, map_attr as u32);
        put32(&mut out, 0x14, tile_gfx as u32);
        put32(&mut out, 0x18, tiles as u32);
        put32(&mut out, 0x1C, t.feature_names.len() as u32);
        put32(&mut out, 0x20, tile_anims as u32);
        put32(&mut out, 0x24, u32::from(t.sea_level));
        if !t.minimap.is_empty() {
            put32(&mut out, 0x28, mini_map as u32);
            put32(&mut out, 0x2C, 1);
        }
        for (i, v) in t.tile_indices.iter().enumerate() {
            out[map_data + 2 * i..map_data + 2 * i + 2].copy_from_slice(&v.to_le_bytes());
        }
        for (i, h) in t.heights.iter().enumerate() {
            let at = map_attr + 4 * i;
            out[at] = *h;
            out[at + 1..at + 3].copy_from_slice(&t.features[i].to_le_bytes()); // unaligned
        }
        out[tile_gfx..tile_gfx + t.tile_graphics.len()].copy_from_slice(&t.tile_graphics);
        for (i, name) in t.feature_names.iter().enumerate() {
            let at = tile_anims + 132 * i;
            put32(&mut out, at, i as u32);
            out[at + 4..at + 4 + name.len()].copy_from_slice(name.as_bytes());
        }
        if !t.minimap.is_empty() {
            put32(&mut out, mini_map, t.minimap_width as u32);
            put32(&mut out, mini_map + 4, t.minimap_height as u32);
            out[mini_map + 8..].copy_from_slice(&t.minimap);
        }
        Ok(out)
    }

    fn canonical_write() -> TntWrite {
        TntWrite {
            width: 4,
            height: 2,
            sea_level: 9,
            tile_indices: vec![1, 0],
            heights: vec![10, 11, 12, 13, 14, 15, 16, 17],
            features: vec![0xFFFF, 1, 0xFFFE, 0xFFFC, 0, 0xFFFD, 0xFFFB, 0xFFFF],
            tile_graphics: (0..2048).map(|i| (i / 7) as u8).collect(),
            feature_names: vec!["tree", "rock"],
            minimap_width: 3,
            minimap_height: 2,
            minimap: vec![1, 2, 3, 4, 5, 6],
        }
    }

    /// Port of `TestUnknownVersionRejected`.
    #[test]
    fn unknown_version_rejected() {
        let err = Tnt::parse(&tnt_header(0x3000)).unwrap_err();
        assert_eq!(err, TntError::UnsupportedVersion { version: 0x3000 });
        assert!(
            err.to_string().contains("0x3000"),
            "diagnostic {err} does not name the version word"
        );
    }

    /// Port of `TestLegacyTNTLoads`.
    #[test]
    fn legacy_tnt_loads() {
        let tnt = Tnt::parse(&legacy_fixture()).unwrap();
        assert_eq!(tnt.version, VERSION_LEGACY);
        assert_eq!(
            (tnt.legacy_min_wind, tnt.legacy_max_wind, tnt.legacy_gravity),
            (500, 800, 112)
        );
        assert_eq!(tnt.minimap_offset, 0x40 + 2 + 32 + 1024 + 132);
        assert_eq!(tnt.attributes.len(), 4);
        let (a0, a1) = (tnt.attributes[0], tnt.attributes[1]);
        assert_eq!((a0.height, a0.feature, a0.metal), (5, 0xFFFF, 7));
        assert_eq!((a1.height, a1.feature, a1.metal), (6, 0, 8));
        assert_eq!(tnt.feature_table[0].name, b"tree");
        assert_eq!(tnt.feature_table[0].index, 0);
        assert_eq!(tnt.tile_indices, [0]);
        assert_eq!(tnt.tile_graphics().len(), 1024);
        assert_eq!((tnt.tile_map_width, tnt.tile_map_height), (1, 1));
        assert_eq!(tnt.unknown1, 0);
        assert!(!tnt.minimap_present);
        assert!(tnt.minimap().is_empty());
    }

    /// Port of `TestTNTAbsentMinimapDoesNotDereferenceItsPointer`.
    #[test]
    fn absent_minimap_does_not_dereference_its_pointer() {
        let mut data = legacy_fixture();
        // A legacy absent-minimap pointer may name unrelated or unusable bytes.
        put32(&mut data, 0x38, u32::MAX);
        let tnt = Tnt::parse(&data).unwrap();
        assert!(!tnt.minimap_present);
        assert_eq!((tnt.minimap_width, tnt.minimap_height), (0, 0));
        assert!(tnt.minimap().is_empty());
    }

    /// Port of `TestTNTPresentMalformedMinimapStillFails`.
    #[test]
    fn present_malformed_minimap_still_fails() {
        let mut data = legacy_fixture();
        put32(&mut data, 0x3c, 1);
        put32(&mut data, 0x38, u32::MAX);
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::SectionOutsideFile(TntSection::MinimapHeader))
        );
    }

    /// Port of `TestLegacyMinimapSlotIsNotSlotTen`, which locks the header half: on a
    /// legacy file slot 10 is the minimum wind speed and the minimap offset is slot 14.
    #[test]
    fn legacy_minimap_slot_is_not_slot_ten() {
        assert_ne!(
            VERSION_LEGACY, VERSION_CANONICAL,
            "version words must differ"
        );
        let tnt = Tnt::parse(&legacy_fixture()).unwrap();
        assert_ne!(
            tnt.minimap_offset, 500,
            "minimap offset read slot 10 (wind) instead of slot 14"
        );
        // With the flag set the minimap is read from slot 14's offset.
        let mut data = legacy_fixture();
        put32(&mut data, 0x3c, 1);
        let tnt = Tnt::parse(&data).unwrap();
        assert!(tnt.minimap_present);
        assert_eq!((tnt.minimap_width, tnt.minimap_height), (0, 0));
        assert!(tnt.minimap().is_empty());
    }

    /// Port of `TestFeatureSentinelBand`: 0xFFFB and above are sentinels, 0xFFFA is a
    /// real index.
    #[test]
    fn feature_sentinel_band() {
        for sentinel in [0xFFFFu16, 0xFFFE, 0xFFFD, 0xFFFC, 0xFFFB] {
            assert!(
                sentinel >= FEATURE_SENTINEL_BASE,
                "{sentinel:#x} must be inside the sentinel band"
            );
        }
        // In a file, the whole band is accepted with no feature records, and 0xFFFA is
        // a real index that needs one.
        for sentinel in [0xFFFFu16, 0xFFFE, 0xFFFD, 0xFFFC, 0xFFFB] {
            let mut w = canonical_write();
            w.feature_names.clear();
            w.features = vec![sentinel; 8];
            assert!(
                Tnt::parse(&encode_tnt(&w).unwrap()).is_ok(),
                "{sentinel:#x}"
            );
        }
        // The test encoder refuses a missing record, so patch a valid file.
        let mut ok = canonical_write();
        ok.feature_names.clear();
        ok.features = vec![0xFFFF; 8];
        let mut data = encode_tnt(&ok).unwrap();
        let attr = 0x40 + 4;
        data[attr + 1..attr + 3].copy_from_slice(&0xFFFAu16.to_le_bytes());
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::FeatureIndexOutOfRange {
                index: 0xFFFA,
                cell: 0,
                records: 0
            })
        );
    }

    #[test]
    fn canonical_file_round_trips() {
        let w = canonical_write();
        let data = encode_tnt(&w).unwrap();
        let tnt = Tnt::parse(&data).unwrap();
        assert_eq!(tnt.version, VERSION_CANONICAL);
        assert_eq!((tnt.width, tnt.height), (4, 2));
        assert_eq!((tnt.tile_map_width, tnt.tile_map_height), (2, 1));
        assert_eq!(tnt.sea_level, 9);
        assert_eq!(tnt.tiles, 2);
        assert_eq!(tnt.tile_anims, 2);
        assert_eq!(tnt.tile_indices, [1, 0]);
        assert_eq!(tnt.tile_index_bytes(), [1, 0, 0, 0]);
        assert_eq!(tnt.unknown1, 1);
        assert!(tnt.minimap_present);
        assert_eq!(
            (tnt.legacy_min_wind, tnt.legacy_max_wind, tnt.legacy_gravity),
            (0, 0, 0)
        );
        let heights: Vec<u8> = tnt.attributes.iter().map(|a| a.height).collect();
        let features: Vec<u16> = tnt.attributes.iter().map(|a| a.feature).collect();
        assert_eq!(heights, w.heights);
        assert_eq!(features, w.features);
        assert!(tnt
            .attributes
            .iter()
            .all(|a| a.metal == 0 && a.unknown == 0));
        assert_eq!(tnt.tile_graphics(), w.tile_graphics);
        assert_eq!(tnt.feature_table[1].name, b"rock");
        assert_eq!(tnt.feature_table[1].index, 1);
        assert_eq!((tnt.minimap_width, tnt.minimap_height), (3, 2));
        assert_eq!(tnt.minimap(), [1, 2, 3, 4, 5, 6]);
        assert_eq!(tnt.raw(), data);
    }

    #[test]
    fn canonical_without_minimap_flag_bit_zero() {
        let mut data = encode_tnt(&canonical_write()).unwrap();
        // Slot 11 bit 0 is the flag; other bits are only `unknown1`.
        put32(&mut data, 0x2c, 2);
        let tnt = Tnt::parse(&data).unwrap();
        assert!(!tnt.minimap_present);
        assert_eq!(tnt.unknown1, 2);
        assert!(tnt.minimap().is_empty());
    }

    #[test]
    fn canonical_unknown_attribute_byte_is_kept() {
        let mut data = encode_tnt(&canonical_write()).unwrap();
        let attr = 0x40 + 4;
        data[attr + 3] = 0x5a;
        let tnt = Tnt::parse(&data).unwrap();
        assert_eq!(tnt.attributes[0].unknown, 0x5a);
    }

    #[test]
    fn legacy_sentinel_band_and_bytes_never_read() {
        let mut data = legacy_fixture();
        let attr = 0x40 + 2;
        // Bytes 1, 3, 4, 5 and 7 of every record are never read.
        for cell in 0..4 {
            for byte in [1, 3, 4, 5, 7] {
                data[attr + cell * 8 + byte] = 0xAA;
            }
        }
        // 0xFC, the first legacy sentinel, maps to the empty word.
        data[attr + 8 + 2] = 0xFC;
        let tnt = Tnt::parse(&data).unwrap();
        assert_eq!(tnt.attributes[1].feature, 0xFFFF);
        assert_eq!(tnt.attributes[2].feature, 0);
        assert_eq!(tnt.attributes[2].unknown, 0);
        // 0xFB is still an index, and the table has one record.
        data[attr + 8 + 2] = 0xFB;
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::FeatureIndexOutOfRange {
                index: 0xFB,
                cell: 1,
                records: 1
            })
        );
    }

    #[test]
    fn tile_index_must_be_below_the_tile_count() {
        let mut data = legacy_fixture();
        data[0x40] = 1; // tile 1 of 1
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::TileIndexOutOfRange {
                index: 1,
                cell: 0,
                tiles: 1
            })
        );
    }

    #[test]
    fn header_validation() {
        assert_eq!(Tnt::parse(&[0u8; 0x3f]), Err(TntError::FileTooSmall));
        assert_eq!(
            Tnt::parse_with_limits(
                &legacy_fixture(),
                TntLimits {
                    max_cells: 0,
                    ..TntLimits::default()
                }
            ),
            Err(TntError::InvalidLimits)
        );
        let mut data = legacy_fixture();
        put32(&mut data, 0x04, 3);
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::OddDimensions {
                width: 3,
                height: 2
            })
        );
        put32(&mut data, 0x04, 0);
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::DimensionsExceedLimits {
                width: 0,
                height: 2
            })
        );
        put32(&mut data, 0x04, 8194);
        assert!(matches!(
            Tnt::parse(&data),
            Err(TntError::DimensionsExceedLimits { .. })
        ));
        let mut data = legacy_fixture();
        put32(&mut data, 0x18, 1 << 21); // tiles over the limit
        assert_eq!(Tnt::parse(&data), Err(TntError::CountsExceedLimits));
        let mut data = legacy_fixture();
        put32(&mut data, 0x1c, 1 << 17); // feature records over the limit
        assert_eq!(Tnt::parse(&data), Err(TntError::CountsExceedLimits));
    }

    #[test]
    fn custom_limits_apply() {
        let limits = TntLimits {
            max_cells: 3,
            ..TntLimits::default()
        };
        assert_eq!(
            Tnt::parse_with_limits(&legacy_fixture(), limits),
            Err(TntError::CellsExceedLimit { cells: 4, limit: 3 })
        );
        let limits = TntLimits {
            max_minimap_pixels: 5,
            ..TntLimits::default()
        };
        assert_eq!(
            Tnt::parse_with_limits(&encode_tnt(&canonical_write()).unwrap(), limits),
            Err(TntError::MinimapExceedsLimits)
        );
    }

    #[test]
    fn sections_must_lie_inside_the_file() {
        for (slot, section) in [
            (0x0c, TntSection::TileIndexMap),
            (0x10, TntSection::AttributeMap),
            (0x14, TntSection::TileGraphics),
            (0x20, TntSection::FeatureTable),
        ] {
            let mut data = legacy_fixture();
            put32(&mut data, slot, u32::MAX);
            assert_eq!(
                Tnt::parse(&data),
                Err(TntError::SectionOutsideFile(section)),
                "{section:?}"
            );
        }
        let mut data = legacy_fixture();
        data.truncate(data.len() - 9); // cuts into the last feature record
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::SectionOutsideFile(TntSection::FeatureTable))
        );
    }

    #[test]
    fn minimap_bounds() {
        let mut data = encode_tnt(&canonical_write()).unwrap();
        let mini = data.len() - 8 - 6;
        // A minimap larger than the bytes left.
        put32(&mut data, mini, 4);
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::SectionOutsideFile(TntSection::MinimapPixels))
        );
        // Dimensions whose product is huge are refused before any allocation.
        put32(&mut data, mini, u32::MAX);
        put32(&mut data, mini + 4, u32::MAX);
        assert_eq!(Tnt::parse(&data), Err(TntError::MinimapExceedsLimits));
        // An offset with no room for the 8-byte header sum.
        let mut data = encode_tnt(&canonical_write()).unwrap();
        let end = data.len() as u32;
        put32(&mut data, 0x28, end - 4);
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::SectionOutsideFile(TntSection::MinimapHeader))
        );
        // An offset near `u32::MAX` is outside the file before `offset + 8` is formed.
        let mut data = encode_tnt(&canonical_write()).unwrap();
        put32(&mut data, 0x28, u32::MAX - 7);
        assert!(Tnt::parse(&data).is_err());
    }

    /// Hostile: a header that claims the largest legal map in a 0x40-byte file must fail
    /// on the first section without making a vector for any of them.
    #[test]
    fn huge_counts_in_a_tiny_file_fail_before_allocating() {
        let mut data = tnt_header(VERSION_CANONICAL);
        put32(&mut data, 0x04, 4096);
        put32(&mut data, 0x08, 4096);
        put32(&mut data, 0x18, 1 << 20);
        put32(&mut data, 0x1c, 1 << 16);
        assert_eq!(
            Tnt::parse(&data),
            Err(TntError::SectionOutsideFile(TntSection::TileIndexMap))
        );
        // The same with limits raised to the full u32 range: still bounded by the file.
        let limits = TntLimits {
            max_width: u32::MAX,
            max_height: u32::MAX,
            max_cells: u64::MAX,
            max_tiles: u32::MAX,
            max_feature_records: u32::MAX,
            max_minimap_pixels: u64::MAX,
        };
        put32(&mut data, 0x04, u32::MAX - 1);
        put32(&mut data, 0x08, u32::MAX - 1);
        put32(&mut data, 0x18, u32::MAX);
        put32(&mut data, 0x1c, u32::MAX);
        assert_eq!(
            Tnt::parse_with_limits(&data, limits),
            Err(TntError::SectionOutsideFile(TntSection::TileIndexMap))
        );
        // A tile-graphics size of `u32::MAX * 1024` does not overflow a u64 either.
        let mut data = legacy_fixture();
        put32(&mut data, 0x18, u32::MAX);
        let limits = TntLimits {
            max_tiles: u32::MAX,
            ..TntLimits::default()
        };
        // The tile index (0) is fine; the failure is the graphics section.
        assert_eq!(
            Tnt::parse_with_limits(&data, limits),
            Err(TntError::SectionOutsideFile(TntSection::TileGraphics))
        );
    }

    #[test]
    fn feature_name_without_nul_takes_all_128_bytes() {
        let mut data = legacy_fixture();
        let at = 0x40 + 2 + 32 + 1024 + 4;
        data[at..at + 128].fill(b'x');
        let tnt = Tnt::parse(&data).unwrap();
        assert_eq!(tnt.feature_table[0].name, vec![b'x'; 128]);
        // The name stops at the first NUL.
        data[at + 3] = 0;
        let tnt = Tnt::parse(&data).unwrap();
        assert_eq!(tnt.feature_table[0].name, b"xxx");
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        for data in [legacy_fixture(), encode_tnt(&canonical_write()).unwrap()] {
            assert!(Tnt::parse(&data).is_ok());
            for len in 0..data.len() {
                // The last bytes of a legacy file are the unread minimap header, so a
                // short cut there is still valid; only a panic is a bug.
                let _ = Tnt::parse(&data[..len]);
            }
        }
        let data = encode_tnt(&canonical_write()).unwrap();
        for len in 0..data.len() {
            assert!(Tnt::parse(&data[..len]).is_err(), "prefix {len}");
        }
    }
}
