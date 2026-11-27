// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/gaf_variant.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! The nearest-doubled variant of a decoded frame: a host presentation product, not
//! a file format contract. Nothing in a GAF file describes it.

use std::collections::HashMap;
use std::sync::Arc;

use super::{GafError, GafFrame, MAX_GAF_FRAME_PIXELS};

impl GafFrame {
    /// The same picture at twice the resolution.
    ///
    /// Width, height and the anchor offsets are multiplied by two (wrapping at 16
    /// bits, as Go's fixed-width arithmetic does), so a frame placed by its anchor
    /// lands on exactly the pixels the 1x frame covered, each now a 2x2 block. The
    /// colour key is a palette index and carries over unchanged. `compressed`
    /// keeps the source dispatch. `pixels` and `transparent` double together.
    ///
    /// A composite's children are doubled leaf by leaf and `subframe_count` and
    /// `alternate_blitter` are kept on the parent and on each child. The data
    /// offset, composite depth and direct raster are not carried (Go's literal
    /// omits them). The source is never mutated.
    ///
    /// A frame whose width times height is above the decoder's own frame limit is
    /// refused rather than doubled: Go would allocate whatever the header says. A
    /// decoded frame always passes. A shared child is doubled once and shared in
    /// the result, which is equal in value to Go's per-occurrence copies. Recursion
    /// follows the frame's depth, so a hand-built chain of extreme depth is the
    /// caller's to avoid; decoded frames are bounded by the load limits.
    pub fn doubled(&self) -> Result<GafFrame, GafError> {
        self.doubled_with(&mut HashMap::new())
    }

    fn doubled_with(&self, memo: &mut HashMap<usize, Arc<GafFrame>>) -> Result<GafFrame, GafError> {
        let w = usize::from(self.width);
        let h = usize::from(self.height);
        if (w as u64) * (h as u64) > MAX_GAF_FRAME_PIXELS {
            return Err(GafError::DoubledFrameTooLarge);
        }
        let mut out = GafFrame {
            transient: self.transient,
            width: self.width.wrapping_mul(2),
            height: self.height.wrapping_mul(2),
            x_offset: self.x_offset.wrapping_mul(2),
            y_offset: self.y_offset.wrapping_mul(2),
            color_key: self.color_key,
            compressed: self.compressed,
            unknown2: self.unknown2,
            unknown3: self.unknown3,
            subframe_count: self.subframe_count,
            alternate_blitter: self.alternate_blitter,
            ..GafFrame::default()
        };
        let dw = usize::from(out.width);
        let dh = usize::from(out.height);
        let (pixels, transparent) = double_raster(&self.pixels, &self.transparent, w, h, dw, dh);
        out.pixels = pixels;
        out.transparent = transparent;
        out.subframes.reserve_exact(self.subframes.len());
        for child in &self.subframes {
            let key = Arc::as_ptr(child) as usize;
            if let Some(done) = memo.get(&key) {
                out.subframes.push(Arc::clone(done));
                continue;
            }
            let doubled = Arc::new(child.doubled_with(memo)?);
            memo.insert(key, Arc::clone(&doubled));
            out.subframes.push(doubled);
        }
        Ok(out)
    }
}

/// Maps every output pixel to source floor(j/2) on both axes. A short `pixels` or
/// `transparent` slice reads as the decoder left it: missing pixels are zero and,
/// absent a `transparent` entry, opaque.
fn double_raster(
    pixels: &[u8],
    transparent: &[bool],
    w: usize,
    h: usize,
    dw: usize,
    dh: usize,
) -> (Vec<u8>, Vec<bool>) {
    if w == 0 || h == 0 || dw == 0 || dh == 0 {
        return (Vec::new(), Vec::new());
    }
    let mut out_pixels = Vec::with_capacity(dw * dh);
    let mut out_transparent = Vec::with_capacity(dw * dh);
    for y in 0..dh {
        let sy = (y / 2).min(h - 1);
        let row = sy * w;
        for x in 0..dw {
            let sx = (x / 2).min(w - 1);
            let index = row + sx;
            let pixel = pixels.get(index).copied();
            // Go: clear defaults to true; a transparent entry overrides it; with no
            // entry, an existing pixel is opaque.
            let clear = match transparent.get(index) {
                Some(t) => *t,
                None => pixel.is_none(),
            };
            out_pixels.push(pixel.unwrap_or(0));
            out_transparent.push(clear);
        }
    }
    (out_pixels, out_transparent)
}
