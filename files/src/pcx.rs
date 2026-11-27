// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/pcx.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! PCX images: 8-bit indexed pixels plus the 768-byte palette trailer.
//!
//! Retail validates only the manufacturer byte (0x0a) and the version byte (5); the
//! encoding, bit depth and plane count are not checked, and neither is the 0x0c marker
//! before the palette. The palette is the final 768 bytes of the file. Scanlines are
//! decoded to the visible width, not to `bytes_per_line`, and a run that overshoots the
//! row is clamped to it and the excess dropped.
//!
//! Differences from the Go source:
//! - Go allocated `width * height` bytes (up to 16 MiB) before decoding, so a 900-byte
//!   file could claim a 16 MiB image. Here the pixel buffer grows one row at a time as
//!   rows decode, so the allocation never exceeds what the data has actually produced
//!   (at most 63 pixels per two input bytes). Every error, and the order they are
//!   reported in, is unchanged.
//! - Go's `width * height > MaxInt` test is dropped: it can never hold after the
//!   16 MiB test that follows it, and `usize` is at least 32 bits here.

use std::fmt;

use crate::le::u16_at;
use crate::pal::Rgba;

/// The largest `width * height` this parser accepts: 16 MiB of pixels.
pub const MAX_PCX_PIXELS: u64 = 16 << 20;

const HEADER_BYTES: usize = 128;
const PALETTE_BYTES: usize = 768;

/// Everything that can go wrong reading a PCX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcxError {
    /// Fewer than 128 + 768 bytes.
    FileTooSmall,
    /// The manufacturer byte is not 0x0a or the version byte is not 5.
    UnsupportedHeader { manufacturer: u8, version: u8 },
    /// `xmax < xmin` or `ymax < ymin`.
    InvalidBounds,
    /// The width or height is above 65535 (the bounds span the whole `u16` range).
    DimensionsExceedLimits,
    /// `width * height` is above [`MAX_PCX_PIXELS`].
    ImageTooLarge,
    /// The data ends before a scanline is complete.
    ScanlineTruncated,
    /// An RLE run of length zero, or a run count with no value byte before the palette.
    InvalidRleRun,
}

impl fmt::Display for PcxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PcxError::FileTooSmall => write!(f, "pcx: file is too small"),
            PcxError::UnsupportedHeader {
                manufacturer,
                version,
            } => write!(
                f,
                "pcx: unsupported header (manufacturer {manufacturer:#04x} version {version}, need 0x0a/5)"
            ),
            PcxError::InvalidBounds => write!(f, "pcx: invalid image bounds"),
            PcxError::DimensionsExceedLimits => {
                write!(f, "pcx: image dimensions exceed loader limits")
            }
            PcxError::ImageTooLarge => write!(f, "pcx: image is too large"),
            PcxError::ScanlineTruncated => write!(f, "pcx: scanline data is truncated"),
            PcxError::InvalidRleRun => write!(f, "pcx: invalid RLE run"),
        }
    }
}

impl std::error::Error for PcxError {}

/// A decoded PCX image: indexed pixels and the palette they index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcx {
    pub width: u16,
    pub height: u16,
    pub x_min: u16,
    pub y_min: u16,
    pub x_max: u16,
    pub y_max: u16,
    /// The header's authored stride. Rows are decoded to `width`, not to this.
    pub bytes_per_line: u16,
    /// `width * height` palette indices, row-major.
    pub pixels: Vec<u8>,
    pub palette: [Rgba; 256],
}

impl Pcx {
    /// Decode a PCX image from its bytes.
    pub fn parse(data: &[u8]) -> Result<Pcx, PcxError> {
        if data.len() < HEADER_BYTES + PALETTE_BYTES {
            return Err(PcxError::FileTooSmall);
        }
        let manufacturer = data.first().copied().unwrap_or(0);
        let version = data.get(1).copied().unwrap_or(0);
        if manufacturer != 0x0a || version != 5 {
            return Err(PcxError::UnsupportedHeader {
                manufacturer,
                version,
            });
        }
        let word = |pos| u16_at(data, pos).ok_or(PcxError::FileTooSmall);
        let (x_min, y_min, x_max, y_max) = (word(4)?, word(6)?, word(8)?, word(10)?);
        if x_max < x_min || y_max < y_min {
            return Err(PcxError::InvalidBounds);
        }
        // `x_max - x_min` cannot wrap (checked above), so the width is 1..=65536.
        let width = u64::from(x_max - x_min) + 1;
        let height = u64::from(y_max - y_min) + 1;
        if width > 65535 || height > 65535 {
            return Err(PcxError::DimensionsExceedLimits);
        }
        let bytes_per_line = word(66)?;
        if width * height > MAX_PCX_PIXELS {
            return Err(PcxError::ImageTooLarge);
        }
        // Both fit: each is at most 65535, and the product is at most 16 Mi.
        let width = usize::try_from(width).map_err(|_| PcxError::ImageTooLarge)?;
        let height = usize::try_from(height).map_err(|_| PcxError::ImageTooLarge)?;

        // The palette is the final 768 bytes, with no marker-byte check.
        let trailer = data.len() - PALETTE_BYTES;
        let mut palette = [Rgba::default(); 256];
        let tail = data.get(trailer..).unwrap_or(&[]);
        for (color, rgb) in palette.iter_mut().zip(tail.as_chunks::<3>().0) {
            let [r, g, b] = *rgb;
            *color = Rgba { r, g, b, a: 255 };
        }

        let mut pixels: Vec<u8> = Vec::new();
        let mut position = HEADER_BYTES;
        for row in 0..height {
            let row_start = row * width;
            // Grown a row at a time; see the module comment.
            pixels.resize(row_start + width, 0);
            let mut decoded = 0;
            while decoded < width {
                if position >= trailer {
                    return Err(PcxError::ScanlineTruncated);
                }
                let mut value = data.get(position).copied().unwrap_or(0);
                position += 1;
                let mut run = 1usize;
                if value & 0xc0 == 0xc0 {
                    run = usize::from(value & 0x3f);
                    if run == 0 || position >= trailer {
                        return Err(PcxError::InvalidRleRun);
                    }
                    value = data.get(position).copied().unwrap_or(0);
                    position += 1;
                }
                // Every run is clamped to the remaining visible width and the excess
                // discarded, so a malformed file cannot overflow a scanline. Retail
                // does not fail here.
                run = run.min(width - decoded);
                let start = row_start + decoded;
                if let Some(cells) = pixels.get_mut(start..start + run) {
                    cells.fill(value);
                }
                decoded += run;
            }
        }
        Ok(Pcx {
            width: u16::try_from(width).map_err(|_| PcxError::DimensionsExceedLimits)?,
            height: u16::try_from(height).map_err(|_| PcxError::DimensionsExceedLimits)?,
            x_min,
            y_min,
            x_max,
            y_max,
            bytes_per_line,
            pixels,
            palette,
        })
    }

    /// The palette index and resolved colour at (x, y), or `None` outside the image.
    pub fn at(&self, x: i32, y: i32) -> Option<(u8, Rgba)> {
        let width = usize::from(self.width);
        let x = usize::try_from(x).ok().filter(|&x| x < width)?;
        let y = usize::try_from(y)
            .ok()
            .filter(|&y| y < usize::from(self.height))?;
        let index = *self.pixels.get(y * width + x)?;
        let color = *self.palette.get(usize::from(index))?;
        Some((index, color))
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

    /// Port of `authoredPCX` (formats/pcx_test.go). `width` and `height` of zero would
    /// wrap in Go as well; callers here pass 1 or more.
    fn authored_pcx(width: u16, height: u16, stride: u16, pixels: &[u8], marker: bool) -> Vec<u8> {
        let mut data = vec![0u8; 128];
        data[0] = 0x0a;
        data[1] = 5;
        data[8..10].copy_from_slice(&(width - 1).to_le_bytes());
        data[10..12].copy_from_slice(&(height - 1).to_le_bytes());
        data[66..68].copy_from_slice(&stride.to_le_bytes());
        data.extend_from_slice(pixels);
        if marker {
            data.push(0x0c);
        }
        for i in 0..256u16 {
            data.extend_from_slice(&[i as u8, (i + 1) as u8, (i + 2) as u8]);
        }
        data
    }

    /// Port of `TestPCXRowsConsumeVisibleWidthNotAuthoredStride`.
    #[test]
    fn rows_consume_visible_width_not_authored_stride() {
        let pcx = Pcx::parse(&authored_pcx(2, 2, 4, &[1, 2, 3, 4, 5, 6, 7, 8], false)).unwrap();
        assert_eq!(pcx.pixels, [1, 2, 3, 4]);
        assert_eq!(pcx.bytes_per_line, 4);
        assert_eq!(pcx.palette[0].r, 0);
        assert_eq!(pcx.palette[255].b, 1); // 255 + 2 wraps in a byte
        assert_eq!(pcx.palette[255].a, 255);
    }

    /// Port of `TestPCXRLEClampsToVisibleRowAndFailsSafelyAtEOF`.
    #[test]
    fn rle_clamps_to_visible_row_and_fails_safely_at_eof() {
        let pcx = Pcx::parse(&authored_pcx(2, 1, 1, &[0xc4, 9], false)).unwrap();
        assert_eq!(pcx.pixels, [9, 9]);
        assert_eq!(
            Pcx::parse(&authored_pcx(2, 1, 2, &[0xc2], false)),
            Err(PcxError::InvalidRleRun),
            "truncated RLE value was accepted"
        );
    }

    #[test]
    fn marker_byte_is_not_checked_and_does_not_shift_the_palette() {
        let with = Pcx::parse(&authored_pcx(1, 1, 1, &[7], true)).unwrap();
        let without = Pcx::parse(&authored_pcx(1, 1, 1, &[7], false)).unwrap();
        assert_eq!(with, without);
    }

    #[test]
    fn zero_run_and_short_data_are_errors() {
        assert_eq!(
            Pcx::parse(&authored_pcx(2, 1, 2, &[0xc0, 9], false)),
            Err(PcxError::InvalidRleRun)
        );
        // One pixel of a two-pixel row, then the palette.
        assert_eq!(
            Pcx::parse(&authored_pcx(2, 1, 2, &[5], false)),
            Err(PcxError::ScanlineTruncated)
        );
        // The second row has no data at all.
        assert_eq!(
            Pcx::parse(&authored_pcx(1, 2, 1, &[5], false)),
            Err(PcxError::ScanlineTruncated)
        );
    }

    #[test]
    fn header_checks_are_manufacturer_and_version_only() {
        let mut data = authored_pcx(1, 1, 1, &[7], false);
        data[2] = 0; // encoding unchecked
        data[3] = 4; // bits per pixel unchecked
        data[65] = 3; // plane count unchecked
        assert!(Pcx::parse(&data).is_ok());
        data[0] = 0x0b;
        assert_eq!(
            Pcx::parse(&data),
            Err(PcxError::UnsupportedHeader {
                manufacturer: 0x0b,
                version: 5
            })
        );
        data[0] = 0x0a;
        data[1] = 3;
        assert!(matches!(
            Pcx::parse(&data),
            Err(PcxError::UnsupportedHeader { version: 3, .. })
        ));
    }

    #[test]
    fn bounds_and_dimension_limits() {
        let mut data = authored_pcx(2, 2, 2, &[1, 2, 3, 4], false);
        data[8..10].copy_from_slice(&0u16.to_le_bytes());
        data[4..6].copy_from_slice(&1u16.to_le_bytes()); // xmax 0 < xmin 1
        assert_eq!(Pcx::parse(&data), Err(PcxError::InvalidBounds));

        // The full u16 span is 65536 wide.
        let mut data = authored_pcx(2, 2, 2, &[1, 2, 3, 4], false);
        data[8..10].copy_from_slice(&0xffffu16.to_le_bytes());
        assert_eq!(Pcx::parse(&data), Err(PcxError::DimensionsExceedLimits));

        // 4097 x 4096 is one column over the 16 Mi pixel cap.
        let mut data = authored_pcx(2, 2, 2, &[1, 2, 3, 4], false);
        data[8..10].copy_from_slice(&4096u16.to_le_bytes());
        data[10..12].copy_from_slice(&4095u16.to_le_bytes());
        assert_eq!(Pcx::parse(&data), Err(PcxError::ImageTooLarge));
    }

    /// Hostile: the header claims the largest legal image, the data holds a few bytes.
    /// The parse must fail without first allocating the claimed 16 MiB.
    #[test]
    fn huge_claim_with_tiny_data_fails_without_the_allocation() {
        let mut data = authored_pcx(2, 2, 2, &[1, 2, 3, 4], false);
        data[8..10].copy_from_slice(&4095u16.to_le_bytes());
        data[10..12].copy_from_slice(&4095u16.to_le_bytes());
        assert_eq!(Pcx::parse(&data), Err(PcxError::ScanlineTruncated));
    }

    #[test]
    fn long_run_decodes_a_wide_image_from_small_data() {
        // 63 pixels per two bytes: a 63 x 3 image in six bytes.
        let pcx = Pcx::parse(&authored_pcx(
            63,
            3,
            64,
            &[0xff, 4, 0xff, 5, 0xff, 6],
            false,
        ))
        .unwrap();
        assert_eq!(pcx.pixels.len(), 63 * 3);
        assert_eq!(pcx.at(62, 0), Some((4, pcx.palette[4])));
        assert_eq!(pcx.at(0, 2), Some((6, pcx.palette[6])));
    }

    #[test]
    fn at_is_none_outside_the_image() {
        let pcx = Pcx::parse(&authored_pcx(2, 2, 2, &[1, 2, 3, 4], false)).unwrap();
        assert_eq!(pcx.at(1, 1).map(|p| p.0), Some(4));
        for (x, y) in [(-1, 0), (0, -1), (2, 0), (0, 2), (i32::MIN, i32::MAX)] {
            assert_eq!(pcx.at(x, y), None);
        }
    }

    #[test]
    fn every_truncation_is_handled_without_panic() {
        let full = authored_pcx(3, 2, 3, &[1, 2, 3, 0xc3, 9], true);
        assert!(Pcx::parse(&full).is_ok());
        for len in 0..full.len() {
            // Cutting the palette short makes the trailer overlap pixel data, which is
            // how retail reads it too; only a panic would be a bug here.
            let _ = Pcx::parse(&full[..len]);
        }
        for len in 0..HEADER_BYTES + PALETTE_BYTES {
            assert_eq!(Pcx::parse(&full[..len]), Err(PcxError::FileTooSmall));
        }
    }
}
