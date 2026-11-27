// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/fnt.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! FNT bitmap fonts: one 1-bit glyph per byte code, every glyph as tall as the font.
//!
//! Layout: four header bytes (height, ignored, baseline, first code), then a table of
//! `256 - first_code` little-endian `u16` glyph offsets, then the glyphs. An offset of
//! zero means the font does not author that code. A glyph is a width byte followed by
//! `width * height` bits packed row after row, most significant bit first, with no
//! padding between rows; the last byte is padded with zero bits.
//!
//! Differences from the Go source:
//! - Absent glyphs are `None` (Go used a nil pointer).
//! - Go's `byteCount > MaxInt` test is dropped: a glyph is at most 255 * 255 bits, so it
//!   cannot hold.
//! - Glyph bitmaps are copied, as in Go. Several table entries may name the same offset,
//!   so the copies can total up to 256 times the input; each glyph is at most 8128
//!   bytes, so the total is at most 2 MiB however small the input is.

use std::fmt;

/// The most glyph bits a font may declare in total: 16 Mi bits.
pub const MAX_FNT_GLYPH_BITS: u64 = 16 << 20;

/// Everything that can go wrong reading a font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FntError {
    /// Fewer than four bytes.
    FileTooSmall,
    /// The height byte is zero.
    ZeroGlyphHeight,
    /// The offset table runs past the end of the file.
    OffsetTableTruncated,
    /// A glyph's offset is at or beyond the end of the file.
    GlyphOffsetOutsideFile { code: u8 },
    /// The glyphs declare more than [`MAX_FNT_GLYPH_BITS`] bits in total.
    GlyphDataExceedsLimits,
    /// A glyph's bitmap runs past the end of the file.
    GlyphTruncated { code: u8 },
}

impl fmt::Display for FntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FntError::FileTooSmall => write!(f, "fnt: file is too small"),
            FntError::ZeroGlyphHeight => write!(f, "fnt: zero glyph height"),
            FntError::OffsetTableTruncated => {
                write!(f, "fnt: glyph offset table is truncated")
            }
            FntError::GlyphOffsetOutsideFile { code } => {
                write!(f, "fnt: glyph 0x{code:02x} offset outside file")
            }
            FntError::GlyphDataExceedsLimits => {
                write!(f, "fnt: glyph data exceeds loader limits")
            }
            FntError::GlyphTruncated { code } => {
                write!(f, "fnt: glyph 0x{code:02x} bitmap is truncated")
            }
        }
    }
}

impl std::error::Error for FntError {}

/// One glyph's bitmap: `width` by `height` bits packed row by row, MSB first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FntGlyph {
    pub width: u8,
    pub height: u8,
    pub bits: Vec<u8>,
}

impl FntGlyph {
    /// Whether the bit at (x, y) is set. Coordinates outside the glyph read as clear,
    /// and so does a bit past the end of `bits` (a hand-built glyph can be short; a
    /// parsed one never is).
    pub fn on(&self, x: i32, y: i32) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return false;
        };
        let width = usize::from(self.width);
        if x >= width || y >= usize::from(self.height) {
            return false;
        }
        let bit = y * width + x;
        match self.bits.get(bit / 8) {
            Some(byte) => byte & (0x80 >> (bit % 8)) != 0,
            None => false,
        }
    }
}

/// A decoded bitmap font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fnt {
    pub height: u8,
    /// Header byte 1, kept as authored.
    pub ignored: u8,
    /// Header byte 2 as a signed value.
    pub baseline: i8,
    pub first_code: u8,
    /// One slot per byte code. A code the file does not author is `None`.
    pub glyphs: [Option<FntGlyph>; 256],
}

impl Fnt {
    /// Decode a font from its bytes.
    pub fn parse(data: &[u8]) -> Result<Fnt, FntError> {
        let &[height, ignored, baseline, first_code, ..] = data else {
            return Err(FntError::FileTooSmall);
        };
        if height == 0 {
            return Err(FntError::ZeroGlyphHeight);
        }
        let mut fnt = Fnt {
            height,
            ignored,
            baseline: i8::from_le_bytes([baseline]),
            first_code,
            glyphs: std::array::from_fn(|_| None),
        };
        let table_bytes = 2 * (256 - usize::from(first_code));
        if data.len() < 4 + table_bytes {
            return Err(FntError::OffsetTableTruncated);
        }
        let mut total_bits: u64 = 0;
        for code in first_code..=u8::MAX {
            let table_index = usize::from(code - first_code);
            let at = 4 + table_index * 2;
            let offset = match data.get(at..at + 2) {
                Some(&[lo, hi]) => usize::from(u16::from_le_bytes([lo, hi])),
                _ => return Err(FntError::OffsetTableTruncated),
            };
            if offset == 0 {
                continue;
            }
            // `data[offset]` is the width byte, so the offset must be inside the file.
            let Some(&width) = data.get(offset) else {
                return Err(FntError::GlyphOffsetOutsideFile { code });
            };
            let bit_count = u64::from(width) * u64::from(height);
            if bit_count > MAX_FNT_GLYPH_BITS || total_bits > MAX_FNT_GLYPH_BITS - bit_count {
                return Err(FntError::GlyphDataExceedsLimits);
            }
            let byte_count = usize::try_from(bit_count.div_ceil(8))
                .map_err(|_| FntError::GlyphTruncated { code })?;
            let bits = data
                .get(offset + 1..)
                .and_then(|rest| rest.get(..byte_count))
                .ok_or(FntError::GlyphTruncated { code })?;
            if let Some(slot) = fnt.glyphs.get_mut(usize::from(code)) {
                *slot = Some(FntGlyph {
                    width,
                    height,
                    bits: bits.to_vec(),
                });
            }
            total_bits += bit_count;
        }
        Ok(fnt)
    }

    /// The glyph for a byte code, if the font authors one.
    pub fn glyph(&self, code: u8) -> Option<&FntGlyph> {
        self.glyphs.get(usize::from(code))?.as_ref()
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

    /// Port of `TestFNTContinuousMSBFirstBitsAndAbsentGlyphs`.
    #[test]
    fn continuous_msb_first_bits_and_absent_glyphs() {
        // Height 3, width 3: nine bits packed continuously (101 010 111).
        let mut data = vec![0u8; 516 + 1 + 2];
        data[0] = 3;
        let at = 4 + usize::from(b'A') * 2;
        data[at] = 4;
        data[at + 1] = 2; // offset 516
        data[516] = 3;
        data[517] = 0xab;
        data[518] = 0x80; // 1010101110000000
        let f = Fnt::parse(&data).unwrap();
        let g = f.glyph(b'A').unwrap();
        assert_eq!((g.width, g.height), (3, 3));
        let want = [true, false, true, false, true, false, true, true, true];
        for (i, expected) in want.into_iter().enumerate() {
            assert_eq!(g.on((i % 3) as i32, (i / 3) as i32), expected, "bit {i}");
        }
        assert!(
            f.glyph(b'B').is_none(),
            "zero glyph offset should remain absent"
        );
    }

    /// Port of `TestFNTMalformedGlyphFailsSoft`.
    #[test]
    fn malformed_glyph_fails_soft() {
        let mut data = vec![0u8; 516 + 1];
        data[0] = 8;
        data[4] = 4;
        data[5] = 2; // offset 516
        data[516] = 4; // needs four bitmap bytes, but none is present
        assert_eq!(Fnt::parse(&data), Err(FntError::GlyphTruncated { code: 0 }));
        let g = FntGlyph::default();
        assert!(
            !g.on(0, 0),
            "short manually assembled glyph should be transparent"
        );
    }

    /// Port of `TestFNTReducedTablePreservesNamedHeaderBytes`.
    #[test]
    fn reduced_table_preserves_named_header_bytes() {
        // First code 32 gives a 448-byte table: this 454-byte font is valid even though
        // a full table would require 516 bytes.
        let mut data = vec![0u8; 4 + 2 * (256 - 32) + 2];
        data[0] = 1;
        data[1] = 0x7e;
        data[2] = 0xfe;
        data[3] = 32;
        let at = 4 + 2 * usize::from(b'A' - 32);
        let target = data.len() - 2;
        data[at] = target as u8;
        data[at + 1] = (target >> 8) as u8;
        let n = data.len();
        data[n - 2] = 1;
        data[n - 1] = 0x80;
        let f = Fnt::parse(&data).unwrap();
        assert_eq!(
            (f.height, f.ignored, f.baseline, f.first_code),
            (1, 0x7e, -2, 32)
        );
        assert!(f.glyph(31).is_none());
        assert!(f.glyph(b'A').is_some_and(|g| g.on(0, 0)));
    }

    #[test]
    fn header_errors() {
        assert_eq!(Fnt::parse(&[]), Err(FntError::FileTooSmall));
        assert_eq!(Fnt::parse(&[1, 0, 0]), Err(FntError::FileTooSmall));
        assert_eq!(Fnt::parse(&[0, 0, 0, 0]), Err(FntError::ZeroGlyphHeight));
        // Full table needs 4 + 512 bytes.
        let mut short = vec![0u8; 515];
        short[0] = 1;
        assert_eq!(Fnt::parse(&short), Err(FntError::OffsetTableTruncated));
        // First code 255 needs only a two-byte table.
        let f = Fnt::parse(&[1, 0, 0, 255, 0, 0]).unwrap();
        assert!(f.glyphs.iter().all(Option::is_none));
    }

    #[test]
    fn offset_at_or_past_the_end_is_rejected() {
        let mut data = vec![0u8; 516];
        data[0] = 1;
        data[4] = 4;
        data[5] = 2; // offset 516 == len
        assert_eq!(
            Fnt::parse(&data),
            Err(FntError::GlyphOffsetOutsideFile { code: 0 })
        );
        // One byte more makes the width byte readable; a zero-width glyph has no bits.
        data.push(0);
        let f = Fnt::parse(&data).unwrap();
        assert_eq!(f.glyph(0).unwrap().bits.len(), 0);
    }

    /// Hostile: every table entry names the same maximal glyph in a tiny file. Each copy
    /// is bounded by the bytes the glyph really has in the input.
    #[test]
    fn shared_offsets_stay_bounded() {
        let mut data = vec![0u8; 516 + 1 + 255];
        data[0] = 255;
        for code in 0..256 {
            data[4 + code * 2] = 4;
            data[5 + code * 2] = 2;
        }
        data[516] = 255; // 255 * 255 bits = 8129 bytes, but only 255 are present
        assert_eq!(Fnt::parse(&data), Err(FntError::GlyphTruncated { code: 0 }));
        // With the whole bitmap present all 256 slots fill, at 8129 bytes each.
        data.resize(516 + 1 + 8129, 0xff);
        let f = Fnt::parse(&data).unwrap();
        assert!(f
            .glyphs
            .iter()
            .all(|g| g.as_ref().is_some_and(|g| g.bits.len() == 8129)));
    }

    #[test]
    fn on_reads_clear_outside_and_past_short_bits() {
        let g = FntGlyph {
            width: 3,
            height: 3,
            bits: vec![0xff], // only the first eight bits exist
        };
        assert!(g.on(0, 0));
        assert!(g.on(1, 2)); // bit 7
        assert!(!g.on(2, 2)); // bit 8 is past `bits`
        for (x, y) in [(-1, 0), (0, -1), (3, 0), (0, 3), (i32::MIN, i32::MIN)] {
            assert!(!g.on(x, y));
        }
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let mut data = vec![0u8; 516 + 1 + 2];
        data[0] = 3;
        data[4 + usize::from(b'A') * 2] = 4;
        data[5 + usize::from(b'A') * 2] = 2;
        data[516] = 3;
        for len in 0..data.len() {
            assert!(Fnt::parse(&data[..len]).is_err(), "prefix {len}");
        }
        assert!(Fnt::parse(&data).is_ok());
    }
}
