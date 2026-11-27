// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/pal.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! PAL palettes: 256 indexed colours.
//!
//! Retail reads 1024 bytes, 256 entries of red, green, blue and a reserved fourth byte.
//! The 768-byte RGB form is an import extension Nanolathe accepts and so does this
//! parser. The reserved bytes are kept in [`Palette::raw`]; every resolved colour is
//! opaque.
//!
//! [`Rgba`] is also the colour type of [`crate::pcx`]. The GAF module has no palette
//! type of its own (its pixels are indices), so there was nothing to reconcile.
//!
//! Differences from the Go source: none in behaviour. Go's `color.RGBA` is a struct of
//! four bytes; [`Rgba`] is the same.

use std::fmt;

/// An 8-bit-per-channel colour. Same layout as Go's `color.RGBA`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// A 256-entry indexed palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// The resolved colours, all with alpha 255.
    pub colors: [Rgba; 256],
    /// A copy of the source bytes, including the reserved fourth bytes.
    pub raw: Vec<u8>,
}

/// Everything that can go wrong reading a palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PalError {
    /// The input is neither 768 nor 1024 bytes.
    BadLength { len: usize },
}

impl fmt::Display for PalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PalError::BadLength { len } => {
                write!(f, "pal: expected 768 or 1024 bytes, got {len}")
            }
        }
    }
}

impl std::error::Error for PalError {}

impl Palette {
    /// Decode a palette. Both the 768-byte RGB and the 1024-byte RGB-plus-reserved
    /// layouts are accepted; any other length is an error.
    pub fn parse(data: &[u8]) -> Result<Palette, PalError> {
        let stride = match data.len() {
            768 => 3,
            1024 => 4,
            len => return Err(PalError::BadLength { len }),
        };
        let mut colors = [Rgba::default(); 256];
        for (color, entry) in colors.iter_mut().zip(data.chunks_exact(stride)) {
            if let [r, g, b, ..] = *entry {
                *color = Rgba { r, g, b, a: 255 };
            }
        }
        Ok(Palette {
            colors,
            raw: data.to_vec(),
        })
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

    /// Port of `TestPALRetainsReservedBytesAndImportLayout` (formats/pal_store_test.go).
    #[test]
    fn retains_reserved_bytes_and_import_layout() {
        for stride in [3usize, 4] {
            let mut source: Vec<u8> = (0..stride * 256).map(|i| i as u8).collect();
            let pal = Palette::parse(&source).unwrap();
            assert_eq!(pal.raw, source, "lossless palette lost authored bytes");
            source[0] = 99;
            assert_ne!(pal.raw[0], 99, "palette must own its source");
            assert_eq!(pal.colors[0].r, 0);
            assert_eq!(pal.colors[0].a, 255);
            // Entry 1 starts at byte `stride`: the reserved byte is skipped, not read.
            assert_eq!(pal.colors[1].r, stride as u8);
            assert_eq!(pal.colors[255].b, ((255 * stride + 2) % 256) as u8);
        }
    }

    #[test]
    fn rejects_every_other_length() {
        for len in [0usize, 1, 3, 767, 769, 1023, 1025, 4096] {
            assert_eq!(
                Palette::parse(&vec![0; len]),
                Err(PalError::BadLength { len })
            );
        }
        assert!(Palette::parse(&[0; 768]).is_ok());
        assert!(Palette::parse(&[0; 1024]).is_ok());
    }
}
