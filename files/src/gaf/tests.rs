// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/gaf_test.go, formats/gaf_bounds_test.go, formats/gaf_colorkey_test.go, formats/gaf_compat_test.go, formats/gaf_decoded_test.go, formats/gaf_direct_test.go, formats/gaf_expansion_test.go, formats/gaf_metadata_test.go, formats/gaf_source_test.go, formats/gaf_variant_test.go, formats/gaf_write_test.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Tests. Fixtures are built here; no game data is used. Tests are allowed to index
//! and unwrap. Not ported: the retail-corpus tests, the VFS file entry points
//! (`LoadGAFFile` and friends: the parsers take bytes), the VFS read-cap case, the
//! opt-in `NANOLATHE_ESC_GAF_ROOT` probe, and the cyclic-slice writer test (a Rust
//! value cannot contain itself; the writer's depth bound is tested instead).

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::sync::Arc;

use super::write::{encode_gaf, GafWriteEntry, GafWriteFrame};
use super::*;

fn put16(data: &mut [u8], at: usize, v: u16) {
    data[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn put32(data: &mut [u8], at: usize, v: u32) {
    data[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn get32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

fn leaf(w: u16, h: u16, pixels: &[u8]) -> GafWriteFrame {
    GafWriteFrame {
        width: w,
        height: h,
        pixels: pixels.to_vec(),
        ..GafWriteFrame::default()
    }
}

fn entry(name: &str, frames: Vec<GafWriteFrame>) -> GafWriteEntry {
    GafWriteEntry {
        name: name.into(),
        loop_: false,
        frames,
    }
}

fn first_frame(g: &Gaf) -> &GafFrame {
    &g.entries[0].frames[0].frame
}

fn first_frame_offset(data: &[u8]) -> usize {
    let entry_offset = get32(data, 12) as usize;
    get32(data, entry_offset + 40) as usize
}

// ---- gaf_test.go ----

#[test]
fn subframe_count_uses_low_byte_and_preserves_high_byte() {
    const ENTRY: usize = 16;
    const REF: usize = ENTRY + 40;
    const PARENT: usize = REF + 8;
    const TABLE: usize = PARENT + 24;
    const SUB_COUNT: usize = 300; // effective low-byte count 44; high byte selects ALP
    const CHILD: usize = TABLE + SUB_COUNT * 4;
    const PIXEL: usize = CHILD + 24;
    let mut data = vec![0u8; PIXEL + 1];
    put32(&mut data, 4, 1);
    put32(&mut data, 12, ENTRY as u32);
    put16(&mut data, ENTRY, 1);
    data[ENTRY + 8..ENTRY + 17].copy_from_slice(b"composite");
    put32(&mut data, REF, PARENT as u32);
    put16(&mut data, PARENT, 1);
    put16(&mut data, PARENT + 2, 1);
    put16(&mut data, PARENT + 10, SUB_COUNT as u16);
    put32(&mut data, PARENT + 16, TABLE as u32);
    for i in 0..SUB_COUNT {
        put32(&mut data, TABLE + i * 4, CHILD as u32);
    }
    put16(&mut data, CHILD, 1);
    put16(&mut data, CHILD + 2, 1);
    put32(&mut data, CHILD + 16, PIXEL as u32);
    data[PIXEL] = 7;

    let g = Gaf::parse(&data).unwrap();
    let f = first_frame(&g);
    assert_eq!(f.subframe_count, 44);
    assert_eq!(f.subframes.len(), 44);
    assert_eq!(f.alternate_blitter, 1);
    assert_eq!(f.at(0, 0), (7, true));
    assert_eq!(f.plain_pixels().len(), 1);
    assert_eq!(f.plain_transparent().len(), 1);
}

#[test]
fn composite_plain_raster_skips_alternate_child() {
    let mut alt = leaf(1, 1, &[7]);
    alt.alternate_blitter = 1;
    let parent = GafWriteFrame {
        width: 1,
        height: 1,
        subframes: vec![leaf(1, 1, &[5]), alt],
        ..GafWriteFrame::default()
    };
    let data = encode_gaf(&[entry("alternate", vec![parent])]).unwrap();
    let g = Gaf::parse(&data).unwrap();
    assert_eq!(first_frame(&g).at(0, 0), (5, true));
}

#[test]
fn raw_frame_preserves_high_byte_without_children() {
    const ENTRY: usize = 16;
    const REF: usize = ENTRY + 40;
    const FRAME: usize = REF + 8;
    const PIXEL: usize = FRAME + 24;
    let mut data = vec![0u8; PIXEL + 1];
    put32(&mut data, 4, 1);
    put32(&mut data, 12, ENTRY as u32);
    put16(&mut data, ENTRY, 1);
    data[ENTRY + 8..ENTRY + 15].copy_from_slice(b"rawflag");
    put32(&mut data, REF, FRAME as u32);
    put16(&mut data, FRAME, 1);
    put16(&mut data, FRAME + 2, 1);
    data[FRAME + 11] = 0x9a; // low count stays zero: raw frame
    put32(&mut data, FRAME + 16, PIXEL as u32);
    data[PIXEL] = 7;
    let g = Gaf::parse(&data).unwrap();
    let f = first_frame(&g);
    assert_eq!(f.subframe_count, 0);
    assert!(f.subframes.is_empty());
    assert_eq!(f.alternate_blitter, 0x9a);
    assert_eq!(f.at(0, 0), (7, true));
}

#[test]
fn entry_count_is_signed_low_word() {
    let mut data = vec![0u8; 12];
    // A negative signed low word suppresses the table walk even with high bits.
    put32(&mut data, 4, 0x7fff_8000);
    let g = Gaf::parse(&data).unwrap();
    assert!(g.entries.is_empty());
    let m = GafMetadata::parse(&data).unwrap();
    assert!(m.entries.is_empty());
}

#[test]
fn find_uses_first_ascii_folded_match() {
    let e = |name: &[u8]| {
        Arc::new(GafEntry {
            name: name.to_vec(),
            frame_count: 0,
            unknown1: 0,
            unknown2: 0,
            frames: Vec::new(),
        })
    };
    let g = Gaf {
        version: 0,
        entry_count: 3,
        unknown: 0,
        entries: vec![e(b"First"), e(b"fIrSt"), e(b"\xc1")],
    };
    let got = g.find("FIRST").unwrap();
    assert!(std::ptr::eq(got, &*g.entries[0]));
    assert!(g.find(b"\xe1").is_none(), "high-byte lookup folded");
    assert!(g.find(b"\xc1").is_some());
}

#[test]
fn rle_rows_preserve_skip_and_opaque_zero() {
    const ENTRY: usize = 16;
    const REF: usize = ENTRY + 40;
    const FRAME: usize = REF + 8;
    const DATA: usize = FRAME + 24;
    // row 0: one transparent, then literal 4, 0; row 1: repeat 6 x 3.
    let mut data = vec![0u8; DATA + 2 + 4 + 2 + 2];
    put32(&mut data, 4, 1);
    put32(&mut data, 12, ENTRY as u32);
    put16(&mut data, ENTRY, 1);
    data[ENTRY + 8..ENTRY + 11].copy_from_slice(b"rle");
    put32(&mut data, REF, FRAME as u32);
    put16(&mut data, FRAME, 3);
    put16(&mut data, FRAME + 2, 2);
    data[FRAME + 9] = 1;
    put32(&mut data, FRAME + 16, DATA as u32);
    let mut pos = DATA;
    put16(&mut data, pos, 4);
    pos += 2;
    data[pos..pos + 4].copy_from_slice(&[3, 4, 4, 0]);
    pos += 4;
    put16(&mut data, pos, 2);
    pos += 2;
    data[pos..pos + 2].copy_from_slice(&[10, 6]);
    let g = Gaf::parse(&data).unwrap();
    let f = first_frame(&g);
    assert!(!f.at(0, 0).1, "RLE skip pixel became opaque");
    assert_eq!(f.at(1, 0), (4, true));
    assert_eq!(f.at(2, 0), (0, true));
    for x in 0..3 {
        assert_eq!(f.at(x, 1), (6, true));
    }
}

/// Odd widths that are not a multiple of the maximum run: 129 = 64 + 64 + 1.
#[test]
fn rle_odd_width_rows_decode_exactly() {
    const WIDTH: usize = 129;
    const HEIGHT: usize = 4;
    const ENTRY: usize = 16;
    const REF: usize = ENTRY + 40;
    const FRAME: usize = REF + 8;
    const DATA: usize = FRAME + 24;
    let literals: Vec<u8> = (0..WIDTH).map(|i| (11 + i * 7) as u8).collect();
    let mut row0 = vec![0xFC];
    row0.extend_from_slice(&literals[..64]);
    row0.push(0xFC);
    row0.extend_from_slice(&literals[64..128]);
    row0.extend_from_slice(&[0x00, literals[128]]);
    let row1 = vec![0xFE, 0x21, 0xFE, 0x22, 0x02, 0x23];
    let mut row2 = vec![0x7F, 0xFC];
    row2.extend_from_slice(&literals[..64]);
    row2.extend_from_slice(&[0x06, 0x44]);
    let rows: Vec<Vec<u8>> = vec![row0, row1, row2, Vec::new()];
    let size = DATA + rows.iter().map(|r| 2 + r.len()).sum::<usize>();
    let mut data = vec![0u8; size];
    put32(&mut data, 4, 1);
    put32(&mut data, 12, ENTRY as u32);
    put16(&mut data, ENTRY, 1);
    data[ENTRY + 8..ENTRY + 18].copy_from_slice(b"panelwidth");
    put32(&mut data, REF, FRAME as u32);
    put16(&mut data, FRAME, WIDTH as u16);
    put16(&mut data, FRAME + 2, HEIGHT as u16);
    data[FRAME + 8] = 9;
    data[FRAME + 9] = 1;
    put32(&mut data, FRAME + 16, DATA as u32);
    let mut pos = DATA;
    for r in &rows {
        put16(&mut data, pos, r.len() as u16);
        pos += 2;
        data[pos..pos + r.len()].copy_from_slice(r);
        pos += r.len();
    }
    let g = Gaf::parse(&data).unwrap();
    let frame = first_frame(&g);
    assert_eq!((frame.width, frame.height), (WIDTH as u16, HEIGHT as u16));
    for (x, literal) in literals.iter().enumerate() {
        assert_eq!(frame.at(x as i32, 0), (*literal, true), "row 0 x={x}");
    }
    for x in 0..WIDTH {
        let want = if x == 128 {
            0x23
        } else if x >= 64 {
            0x22
        } else {
            0x21
        };
        assert_eq!(frame.at(x as i32, 1), (want, true), "row 1 x={x}");
    }
    for x in 0..63 {
        assert!(!frame.at(x, 2).1, "row 2 x={x} should be a skip pixel");
    }
    for x in 63..127 {
        assert_eq!(frame.at(x, 2), (literals[x as usize - 63], true));
    }
    for x in 127..WIDTH as i32 {
        assert_eq!(frame.at(x, 2), (0x44, true));
    }
    for x in 0..WIDTH as i32 {
        assert!(!frame.at(x, 3).1, "row 3 x={x} should be transparent");
    }
}

// ---- gaf_bounds_test.go ----

#[test]
fn shared_entry_table_uses_one_reference_budget() {
    let mut data = encode_gaf(&[
        entry("shared", vec![leaf(1, 1, &[2])]),
        entry("unused", vec![leaf(1, 1, &[3])]),
    ])
    .unwrap();
    // Both table slots point at the same immutable entry and frame-reference table.
    let first = get32(&data, 12);
    put32(&mut data, 16, first);
    let limits = GafLimits {
        max_frame_refs: 1,
        ..GafLimits::default()
    };
    let g = Gaf::parse_with_limits(&data, limits).unwrap();
    assert_eq!(g.entries.len(), 2);
    assert!(Arc::ptr_eq(&g.entries[0], &g.entries[1]));
}

#[test]
fn aggregate_reference_and_pixel_budgets() {
    let data = encode_gaf(&[
        entry("one", vec![leaf(1, 1, &[2])]),
        entry("two", vec![leaf(1, 1, &[3])]),
    ])
    .unwrap();
    let limits = GafLimits {
        max_frame_refs: 1,
        ..GafLimits::default()
    };
    let err = Gaf::parse_with_limits(&data, limits).unwrap_err();
    assert_eq!(*err.innermost(), GafError::AggregateFrameRefsExceed);
    assert!(err.to_string().contains("aggregate frame references"));
    let limits = GafLimits {
        max_decoded_pixels: 1,
        ..GafLimits::default()
    };
    let err = Gaf::parse_with_limits(&data, limits).unwrap_err();
    assert_eq!(*err.innermost(), GafError::AggregateDecodedPixelsExceed);
    assert!(err.to_string().contains("aggregate decoded pixels"));
}

fn composite_chain(frames: usize) -> Vec<u8> {
    composite_chain_with_references(frames, &[0])
}

fn composite_chain_with_references(frames: usize, references: &[usize]) -> Vec<u8> {
    const ENTRY: usize = 16;
    let frame_offset = ENTRY + 40 + references.len() * 8;
    let subtable = frame_offset + frames * 24;
    let pixel = subtable + (frames - 1) * 4;
    let mut data = vec![0u8; pixel + 1];
    put32(&mut data, 4, 1);
    put32(&mut data, 12, ENTRY as u32);
    put16(&mut data, ENTRY, references.len() as u16);
    data[ENTRY + 8..ENTRY + 13].copy_from_slice(b"chain");
    for (i, r) in references.iter().enumerate() {
        put32(
            &mut data,
            ENTRY + 40 + i * 8,
            (frame_offset + r * 24) as u32,
        );
    }
    for i in 0..frames {
        let h = frame_offset + i * 24;
        put16(&mut data, h, 1);
        put16(&mut data, h + 2, 1);
        if i + 1 < frames {
            put16(&mut data, h + 10, 1);
            put32(&mut data, h + 16, (subtable + i * 4) as u32);
            put32(
                &mut data,
                subtable + i * 4,
                (frame_offset + (i + 1) * 24) as u32,
            );
        } else {
            put32(&mut data, h + 16, pixel as u32);
        }
    }
    data[pixel] = 2;
    data
}

#[test]
fn composite_depth_budget() {
    let data = composite_chain(3);
    let mut limits = GafLimits {
        max_composite_depth: 1,
        ..GafLimits::default()
    };
    let err = Gaf::parse_with_limits(&data, limits).unwrap_err();
    assert_eq!(*err.innermost(), GafError::CompositeDepthExceeded);
    assert!(err.to_string().contains("composite depth"));
    limits.max_composite_depth = 2;
    Gaf::parse_with_limits(&data, limits).unwrap();
}

#[test]
fn composite_depth_budget_includes_cached_subtrees() {
    // Index the leaf and intermediate frames before the root. The cache must keep
    // their depth, or the root could bypass the nesting limit.
    let data = composite_chain_with_references(3, &[2, 1, 0]);
    let limits = GafLimits {
        max_composite_depth: 1,
        ..GafLimits::default()
    };
    let err = Gaf::parse_with_limits(&data, limits).unwrap_err();
    assert_eq!(*err.innermost(), GafError::CompositeDepthExceeded);
}

#[test]
fn invalid_limits_are_refused() {
    let data = composite_chain(2);
    for limits in [
        GafLimits {
            max_frame_refs: 0,
            ..GafLimits::default()
        },
        GafLimits {
            max_decoded_pixels: 0,
            ..GafLimits::default()
        },
        GafLimits {
            max_composite_depth: 0,
            ..GafLimits::default()
        },
        GafLimits {
            max_composite_depth: MAX_COMPOSITE_DEPTH_CEILING + 1,
            ..GafLimits::default()
        },
    ] {
        assert_eq!(
            Gaf::parse_with_limits(&data, limits).unwrap_err(),
            GafError::InvalidLimits
        );
    }
    assert_eq!(Gaf::parse(&[0; 11]).unwrap_err(), GafError::FileTooSmall);
}

// ---- gaf_colorkey_test.go ----

/// Raw frames skip pixels equal to the colour key; the key is recorded in
/// `transparent` and never erased from `pixels`. RLE frames ignore the key.
#[test]
fn raw_frame_color_key() {
    const ENTRY: usize = 16;
    const REF: usize = ENTRY + 40;
    const FRAME: usize = REF + 8;
    const PIXEL: usize = FRAME + 24;
    let mut data = vec![0u8; PIXEL + 4];
    put32(&mut data, 0, 1);
    put32(&mut data, 4, 1);
    put32(&mut data, 12, ENTRY as u32);
    put16(&mut data, ENTRY, 1);
    data[ENTRY + 8..ENTRY + 12].copy_from_slice(b"mask");
    put32(&mut data, REF, FRAME as u32);
    put16(&mut data, FRAME, 2);
    put16(&mut data, FRAME + 2, 2);
    data[FRAME + 8] = 9;
    put32(&mut data, FRAME + 16, PIXEL as u32);
    data[PIXEL..PIXEL + 4].copy_from_slice(&[0, 9, 9, 0]);
    let g = Gaf::parse(&data).unwrap();
    let frame = first_frame(&g);
    assert_eq!(frame.color_key, 9);
    assert_eq!(frame.transparent, vec![false, true, true, false]);
    assert_eq!(frame.pixels[1], 9, "key pixel was overwritten in pixels");
}

// ---- gaf_compat_test.go ----

fn compatibility_fixture(width: u16, height: u16, compressed: u8, stream: &[u8]) -> Vec<u8> {
    const ENTRY: usize = 16;
    const REF: usize = ENTRY + 40;
    const FRAME: usize = REF + 8;
    const PAYLOAD: usize = FRAME + 24;
    let mut data = vec![0u8; PAYLOAD + stream.len()];
    put32(&mut data, 4, 1);
    put32(&mut data, 12, ENTRY as u32);
    put16(&mut data, ENTRY, 1);
    let name = b"authored compatibility";
    data[ENTRY + 8..ENTRY + 8 + name.len()].copy_from_slice(name);
    put32(&mut data, REF, FRAME as u32);
    put16(&mut data, FRAME, width);
    put16(&mut data, FRAME + 2, height);
    data[FRAME + 9] = compressed;
    put32(&mut data, FRAME + 16, PAYLOAD as u32);
    data[PAYLOAD..].copy_from_slice(stream);
    data
}

fn assert_metadata_matches(meta: &GafMetadata, full: &Gaf) {
    assert_eq!(meta.version, full.version);
    assert_eq!(meta.entry_count, full.entry_count);
    assert_eq!(meta.unknown, full.unknown);
    assert_eq!(meta.entries.len(), full.entries.len());
    for (m, f) in meta.entries.iter().zip(&full.entries) {
        assert_eq!(m.name, f.name);
        assert_eq!(m.frame_count, f.frame_count);
        assert_eq!(m.unknown1, f.unknown1);
        assert_eq!(m.unknown2, f.unknown2);
        assert_metadata_matches_refs(&m.frames, &f.frames);
    }
}

fn assert_metadata_matches_refs(meta: &[GafMetadataFrameRef], full: &[GafFrameRef]) {
    assert_eq!(meta.len(), full.len());
    for (m, f) in meta.iter().zip(full) {
        assert_eq!((m.offset, m.value), (f.offset, f.value));
        assert_metadata_matches_frame(&m.frame, &f.frame);
    }
}

fn assert_metadata_matches_frame(meta: &GafMetadataFrame, full: &GafFrame) {
    assert_eq!(
        (
            meta.width,
            meta.height,
            meta.x_offset,
            meta.y_offset,
            meta.color_key,
            meta.compressed,
            meta.unknown2,
            meta.data_offset,
            meta.unknown3,
            meta.subframe_count,
            meta.alternate_blitter
        ),
        (
            full.width,
            full.height,
            full.x_offset,
            full.y_offset,
            full.color_key,
            full.compressed,
            full.unknown2,
            full.data_offset,
            full.unknown3,
            full.subframe_count,
            full.alternate_blitter
        )
    );
    assert_eq!(meta.subframes.len(), full.subframes.len());
    for (m, f) in meta.subframes.iter().zip(&full.subframes) {
        assert_metadata_matches_frame(m, f);
    }
}

fn assert_both_reject(data: &[u8], limits: GafLimits, want: &str) {
    let e = GafMetadata::parse_with_limits(data, limits).unwrap_err();
    assert!(
        e.to_string().contains(want),
        "metadata error {e}, want {want}"
    );
    let e = Gaf::parse_with_limits(data, limits).unwrap_err();
    assert!(e.to_string().contains(want), "pixel error {e}, want {want}");
}

// These authored rows lock the retail command stream, including continuation
// beyond stored row lengths.
#[test]
fn compatible_rle_commands() {
    #[allow(clippy::type_complexity)]
    let cases: Vec<(&str, u16, u16, Vec<u8>, Vec<u8>, Vec<bool>)> = vec![
        (
            "repeat clamps",
            2,
            1,
            vec![2, 0, 254, 7],
            vec![7, 7],
            vec![false, false],
        ),
        (
            "skip clamps",
            2,
            1,
            vec![1, 0, 255],
            vec![0, 0],
            vec![true, true],
        ),
        (
            "literal discards unread suffix",
            2,
            1,
            vec![3, 0, 252, 7, 8],
            vec![7, 8],
            vec![false, false],
        ),
        (
            "zero skip continues",
            2,
            1,
            vec![3, 0, 1, 6, 9],
            vec![9, 9],
            vec![false, false],
        ),
        (
            "unused payload ignored",
            2,
            1,
            vec![3, 0, 6, 9, 255],
            vec![9, 9],
            vec![false, false],
        ),
        (
            "empty row untouched",
            2,
            1,
            vec![0, 0],
            vec![0, 0],
            vec![true, true],
        ),
        // Row 0 reads row 1's length as a repeat-one command and its high byte as
        // opaque palette zero, then borrows the next repeat command. Row 1 still
        // starts at its stored anchor, not after the borrowed bytes.
        (
            "short row preserves next anchor",
            4,
            2,
            vec![2, 0, 0, 7, 2, 0, 14, 8],
            vec![7, 0, 8, 8, 8, 8, 8, 8],
            vec![false; 8],
        ),
    ];
    for (name, w, h, stream, pixels, transparent) in cases {
        let data = compatibility_fixture(w, h, 1, &stream);
        let meta = GafMetadata::parse(&data).unwrap();
        let full = Gaf::parse(&data).unwrap();
        assert_metadata_matches(&meta, &full);
        let frame = first_frame(&full);
        assert_eq!(frame.pixels, pixels, "{name}");
        assert_eq!(frame.transparent, transparent, "{name}");
    }
}

#[test]
fn empty_leaves_preserve_geometry_without_pixel_reads() {
    for (w, h) in [(0u16, 4u16), (4, 0), (0, 0)] {
        for compressed in [0u8, 1] {
            let data = compatibility_fixture(w, h, compressed, &[]);
            let meta = GafMetadata::parse(&data).unwrap();
            let full = Gaf::parse(&data).unwrap();
            assert_metadata_matches(&meta, &full);
            let frame = first_frame(&full);
            assert_eq!((frame.width, frame.height), (w, h));
            assert!(frame.pixels.is_empty());
            assert!(!frame.at(0, 0).1, "empty leaf has an opaque pixel");
            let doubled = frame.doubled().unwrap();
            assert_eq!((doubled.width, doubled.height), (w * 2, h * 2));
            assert!(doubled.pixels.is_empty());
        }
    }
}

#[test]
fn empty_composite_still_retains_children() {
    let mut data = composite_chain(2);
    let frame_offset = first_frame_offset(&data);
    put16(&mut data, frame_offset, 0);
    let full = Gaf::parse(&data).unwrap();
    let frame = first_frame(&full);
    assert!(frame.pixels.is_empty());
    assert!(!frame.subframes.is_empty());
    assert!(!frame.subframes[0].pixels.is_empty());
    let doubled = frame.doubled().unwrap();
    assert_eq!(doubled.subframes.len(), frame.subframes.len());
    assert!(!doubled.subframes[0].pixels.is_empty());
}

#[test]
fn compatibility_still_rejects_out_of_file_access() {
    let cases: [(&str, u16, Vec<u8>, &str); 5] = [
        ("row header", 2, vec![1], "header is truncated"),
        (
            "stored extent",
            2,
            vec![5, 0, 6, 9],
            "extent is outside file",
        ),
        (
            "short row reaches EOF",
            2,
            vec![2, 0, 0, 7],
            "command is outside file",
        ),
        (
            "repeat value",
            2,
            vec![1, 0, 6],
            "repeat byte is outside file",
        ),
        (
            "used literal bytes",
            2,
            vec![2, 0, 4, 7],
            "literal bytes are outside file",
        ),
    ];
    for (_, width, stream, want) in cases {
        let data = compatibility_fixture(width, 1, 1, &stream);
        assert_both_reject(&data, GafLimits::default(), want);
    }
    for compressed in [0u8, 1] {
        let mut data = compatibility_fixture(0, 4, compressed, &[]);
        let offset = first_frame_offset(&data);
        let past = data.len() as u32 + 1;
        put32(&mut data, offset + 16, past);
        assert_both_reject(&data, GafLimits::default(), "data pointer is outside file");
    }
}

#[test]
fn rle_command_budget_counts_every_visit() {
    let cases = [
        (
            "zero progress",
            compatibility_fixture(1, 1, 1, &[5, 0, 1, 1, 1, 2, 9]),
        ),
        (
            "overlapping rows",
            compatibility_fixture(4, 2, 1, &[2, 0, 0, 7, 2, 0, 14, 8]),
        ),
    ];
    for (name, data) in cases {
        let mut limits = GafLimits {
            max_rle_commands: 3,
            ..GafLimits::default()
        };
        assert_both_reject(&data, limits, "aggregate RLE commands exceed limit");
        limits.max_rle_commands = 4;
        GafMetadata::parse_with_limits(&data, limits).unwrap_or_else(|e| panic!("{name}: {e}"));
        Gaf::parse_with_limits(&data, limits).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

// ---- gaf_decoded_test.go ----

/// The same frame with every direct raster removed, recursively.
fn without_direct_rasters(f: &GafFrame) -> GafFrame {
    let mut out = f.clone();
    out.direct_raster = None;
    out.subframes = f
        .subframes
        .iter()
        .map(|c| Arc::new(without_direct_rasters(c)))
        .collect();
    out
}

#[test]
fn decoded_preserves_images_and_aliases_without_direct_copies() {
    let tree = GafWriteFrame {
        width: 4,
        height: 2,
        x_offset: -2,
        duration: 3,
        subframes: vec![
            leaf(2, 1, &[7, 8]),
            GafWriteFrame {
                width: 3,
                height: 1,
                x_offset: 1,
                pixels: vec![9, 5, 6],
                transparent: Some(vec![false, true, false]),
                alternate_blitter: 1,
                ..GafWriteFrame::default()
            },
        ],
        ..GafWriteFrame::default()
    };
    let mut data = encode_gaf(&[
        entry("tree", vec![tree]),
        entry("alias", vec![leaf(1, 1, &[4])]),
    ])
    .unwrap();
    // A repeated entry pointer shares its reference table and decoded frames.
    let first = get32(&data, 12);
    put32(&mut data, 16, first);
    let full = Gaf::parse(&data).unwrap();
    let decoded = Gaf::parse_decoded(&data).unwrap();
    assert!(Arc::ptr_eq(&decoded.entries[0], &decoded.entries[1]));
    let want = &full.entries[0].frames[0].frame;
    let got = &decoded.entries[0].frames[0].frame;
    assert!(
        gaf_frame_bytes(got) < gaf_frame_bytes(want),
        "decoded-only bank kept the RLE storage"
    );

    fn check(want: &GafFrame, got: &GafFrame) {
        if want.compressed != 0 && want.subframes.is_empty() {
            assert!(
                want.direct_raster().is_some(),
                "ordinary loader lost its encoded RLE view"
            );
            assert!(
                got.direct_raster().is_none(),
                "decoded-only loader kept an encoded RLE view"
            );
        } else if got.subframes.is_empty() {
            let raster = got.direct_raster().unwrap();
            assert!(
                std::ptr::eq(raster, got),
                "raw leaf stopped sharing its decoded raster"
            );
        }
        for (w, g) in want.subframes.iter().zip(&got.subframes) {
            check(w, g);
        }
    }
    check(want, got);
    assert_eq!(without_direct_rasters(want), **got);
    assert_eq!(
        without_direct_rasters(want).doubled().unwrap(),
        got.doubled().unwrap()
    );
}

#[test]
fn decoded_keeps_whole_bank_validation() {
    let data = encode_gaf(&[
        entry("rest", vec![leaf(1, 1, &[7])]),
        entry("event", vec![leaf(1, 1, &[8])]),
    ])
    .unwrap();
    let cut = &data[..data.len() - 1];
    let full_err = Gaf::parse(cut).unwrap_err();
    let decoded_err = Gaf::parse_decoded(cut).unwrap_err();
    assert_eq!(full_err, decoded_err);
}

// ---- gaf_direct_test.go ----

#[test]
fn direct_raster_reads_authored_rle_storage() {
    let mask = GafWriteFrame {
        width: 3,
        height: 1,
        pixels: vec![7, 7, 7],
        transparent: Some(vec![true, true, true]),
        ..GafWriteFrame::default()
    };
    let data = encode_gaf(&[entry("mask", vec![mask])]).unwrap();
    let g = Gaf::parse(&data).unwrap();
    let f = first_frame(&g);
    let raster = f
        .direct_raster()
        .expect("encoded storage fits but direct view rejected");
    let start = f.data_offset as usize;
    assert_eq!(raster.pixels, data[start..start + 3]);
    assert_ne!(
        raster.pixels, f.pixels,
        "direct view used decoded RLE pixels"
    );
}

#[test]
fn direct_raster_rejects_composite_and_short_storage() {
    let composite = GafWriteFrame {
        width: 1,
        height: 1,
        subframes: vec![leaf(1, 1, &[7])],
        ..GafWriteFrame::default()
    };
    let short = GafWriteFrame {
        width: 100,
        height: 1,
        pixels: vec![0; 100],
        transparent: Some(vec![true; 100]),
        ..GafWriteFrame::default()
    };
    for authored in [composite, short] {
        let data = encode_gaf(&[entry("unsafe", vec![authored])]).unwrap();
        let g = Gaf::parse(&data).unwrap();
        assert!(
            first_frame(&g).direct_raster().is_none(),
            "unsafe direct raster accepted"
        );
    }
}

// ---- gaf_expansion_test.go ----

/// Each parent references the next frame twice. The stored graph is linear, while a
/// consumer visiting every child occurrence does exponential work.
fn shared_expansion(frames: usize, roots: &[usize], width: u16) -> Vec<u8> {
    const ENTRY: usize = 16;
    let frame_base = ENTRY + 40 + roots.len() * 8;
    let table = frame_base + frames * 24;
    let pixels = table + (frames - 1) * 8;
    let mut data = vec![0u8; pixels + usize::from(width)];
    put32(&mut data, 4, 1);
    put32(&mut data, 12, ENTRY as u32);
    put16(&mut data, ENTRY, roots.len() as u16);
    data[ENTRY + 8..ENTRY + 14].copy_from_slice(b"shared");
    for (i, root) in roots.iter().enumerate() {
        put32(
            &mut data,
            ENTRY + 40 + i * 8,
            (frame_base + root * 24) as u32,
        );
    }
    for i in 0..frames {
        let header = frame_base + i * 24;
        put16(&mut data, header, width);
        put16(&mut data, header + 2, 1);
        if i + 1 < frames {
            data[header + 10] = 2;
            put32(&mut data, header + 16, (table + i * 8) as u32);
            put32(&mut data, table + i * 8, (header + 24) as u32);
            put32(&mut data, table + i * 8 + 4, (header + 24) as u32);
        } else {
            put32(&mut data, header + 16, pixels as u32);
        }
    }
    data
}

fn check_expansion(data: &[u8], limits: GafLimits, want: &str) {
    let meta = GafMetadata::parse_with_limits(data, limits).map(|_| ());
    let full = Gaf::parse_with_limits(data, limits).map(|_| ());
    for r in [meta, full] {
        if want.is_empty() {
            r.unwrap_or_else(|e| panic!("accepted graph: {e}"));
        } else {
            let e = r.unwrap_err();
            assert!(e.to_string().contains(want), "error = {e}, want {want:?}");
        }
    }
}

#[test]
fn expansion_limits_count_cached_child_occurrences() {
    let data = shared_expansion(3, &[0], 4); // seven visits, 28 expanded pixels
    let mut limits = GafLimits {
        max_expanded_frames: 6,
        ..GafLimits::default()
    };
    check_expansion(&data, limits, "expanded frames");
    limits.max_expanded_frames = 7;
    limits.max_expanded_pixels = 27;
    check_expansion(&data, limits, "expanded pixels");
    limits.max_expanded_pixels = 28;
    check_expansion(&data, limits, "");
}

#[test]
fn expansion_limits_aggregate_distinct_roots() {
    // Index small children first, so the larger roots are assembled from cache.
    let data = shared_expansion(4, &[2, 1, 0], 1); // 3 + 7 + 15 visits
    let mut limits = GafLimits {
        max_expanded_frames: 24,
        ..GafLimits::default()
    };
    check_expansion(&data, limits, "aggregate expanded frames");
    limits.max_expanded_frames = 25;
    limits.max_expanded_pixels = 24;
    check_expansion(&data, limits, "aggregate expanded pixels");
    limits.max_expanded_pixels = 25;
    check_expansion(&data, limits, "");
    // Reusing a top-level frame does not create a second cached variant.
    limits.max_expanded_frames = 15;
    limits.max_expanded_pixels = 15;
    check_expansion(&shared_expansion(4, &[0, 0], 1), limits, "");
}

#[test]
fn expansion_limits_reject_tiny_exponential_graph() {
    check_expansion(
        &shared_expansion(32, &[0], 1),
        GafLimits::default(),
        "expanded frames",
    );
}

#[test]
fn expansion_accounting_cannot_wrap() {
    let limits = GafLimits {
        max_expanded_frames: u64::MAX,
        max_expanded_pixels: u64::MAX,
        ..GafLimits::default()
    };
    // The depth remains allowed, but expanding it would overflow u64.
    check_expansion(&shared_expansion(65, &[0], 1), limits, "expanded frames");
    check_expansion(&shared_expansion(64, &[0], 2), limits, "expanded pixels");
}

#[test]
fn expansion_omitted_limits_still_bound_input() {
    let limits = GafLimits {
        max_expanded_frames: 0,
        max_expanded_pixels: 0,
        ..GafLimits::default()
    };
    check_expansion(&shared_expansion(32, &[0], 1), limits, "expanded frames");
}

// ---- gaf_metadata_test.go ----

#[test]
fn metadata_matches_pixel_loader_and_preserves_aliases() {
    let composite = GafWriteFrame {
        width: 2,
        height: 1,
        x_offset: -2,
        y_offset: 3,
        duration: 7,
        subframes: vec![
            leaf(1, 1, &[5]),
            GafWriteFrame {
                x_offset: 1,
                alternate_blitter: 1,
                ..leaf(1, 1, &[7])
            },
        ],
        ..GafWriteFrame::default()
    };
    let mut data = encode_gaf(&[
        entry("duplicate", vec![composite]),
        entry(
            "Duplicate",
            vec![GafWriteFrame {
                duration: 1,
                ..leaf(1, 1, &[4])
            }],
        ),
    ])
    .unwrap();
    // A repeated table pointer is an alias, not a second reference budget or frame
    // table. It also exercises first-match lookup.
    let first = get32(&data, 12);
    put32(&mut data, 16, first);
    let meta = GafMetadata::parse(&data).unwrap();
    let full = Gaf::parse(&data).unwrap();
    assert_metadata_matches(&meta, &full);
    assert!(Arc::ptr_eq(&meta.entries[0], &meta.entries[1]));
    assert!(Arc::ptr_eq(&full.entries[0], &full.entries[1]));
    let found = meta.find("DUPLICATE").unwrap();
    assert!(
        std::ptr::eq(found, &*meta.entries[0]),
        "lookup lost the first match"
    );
    for (m, f) in meta.entries.iter().zip(&full.entries) {
        assert_metadata_matches_refs(&m.frames, &f.frames);
    }
}

#[test]
fn metadata_records_composite_geometry_delay_and_selector() {
    let parent = GafWriteFrame {
        width: 3,
        height: 2,
        x_offset: -4,
        y_offset: 5,
        duration: 13,
        subframes: vec![
            GafWriteFrame {
                x_offset: 6,
                y_offset: -7,
                ..leaf(1, 2, &[2, 3])
            },
            GafWriteFrame {
                x_offset: -8,
                y_offset: 9,
                alternate_blitter: 1,
                ..leaf(2, 1, &[4, 5])
            },
        ],
        ..GafWriteFrame::default()
    };
    let data = encode_gaf(&[entry("composite", vec![parent])]).unwrap();
    let meta = GafMetadata::parse(&data).unwrap();
    let r = &meta.entries[0].frames[0];
    assert_eq!(r.value, 13);
    let f = &r.frame;
    assert_eq!(
        (f.width, f.height, f.x_offset, f.y_offset, f.subframe_count),
        (3, 2, -4, 5, 2)
    );
    assert_eq!(f.subframes.len(), 2);
    let (a, b) = (&f.subframes[0], &f.subframes[1]);
    assert_eq!(
        (
            a.width,
            a.height,
            a.x_offset,
            a.y_offset,
            a.alternate_blitter
        ),
        (1, 2, 6, -7, 0)
    );
    assert_eq!(
        (
            b.width,
            b.height,
            b.x_offset,
            b.y_offset,
            b.alternate_blitter
        ),
        (2, 1, -8, 9, 1)
    );
    assert_eq!(f.composite_depth(), 1);
    assert_eq!(f.expanded_frames(), 3);
}

#[test]
fn metadata_rejects_invalid_pixel_payloads_like_full_loader() {
    // raw: the data pointer leaves no room for the pixels
    let mut data = encode_gaf(&[entry("bad", vec![leaf(1, 1, &[2])])]).unwrap();
    let frame_offset = first_frame_offset(&data);
    let len = data.len() as u32;
    put32(&mut data, frame_offset + 16, len);
    assert_both_reject(&data, GafLimits::default(), "raw pixels are truncated");

    // rle: a zero-length skip run leaves the row unfinished
    let clear = GafWriteFrame {
        transparent: Some(vec![true]),
        ..leaf(1, 1, &[2])
    };
    let mut data = encode_gaf(&[entry("bad", vec![clear])]).unwrap();
    let frame_offset = first_frame_offset(&data);
    let at = get32(&data, frame_offset + 16) as usize + 2;
    data[at] = 1;
    assert_both_reject(&data, GafLimits::default(), "command is outside file");
}

#[test]
fn metadata_preserves_depth_and_reference_budgets() {
    let data = composite_chain(3);
    let limits = GafLimits {
        max_composite_depth: 1,
        ..GafLimits::default()
    };
    let e = GafMetadata::parse_with_limits(&data, limits).unwrap_err();
    assert!(e.to_string().contains("composite depth"), "{e}");
    let limits = GafLimits {
        max_frame_refs: 1,
        ..GafLimits::default()
    };
    let e = GafMetadata::parse_with_limits(&data, limits).unwrap_err();
    assert!(e.to_string().contains("aggregate frame references"), "{e}");
}

/// Metadata with explicit limits accepts and refuses exactly what an on-demand
/// source with the same limits does (the VFS read-cap case is not ported).
#[test]
fn metadata_with_limits_matches_source_policy() {
    let frames = vec![
        GafWriteFrame {
            duration: 3,
            ..leaf(2, 2, &[1, 2, 3, 4])
        },
        leaf(2, 2, &[4, 3, 2, 1]),
    ];
    let data = encode_gaf(&[entry("wide", frames)]).unwrap();
    let tight = GafLimits {
        max_decoded_pixels: 7, // the two unique 2x2 frames need 8
        ..GafLimits::default()
    };
    let roomy = GafLimits {
        max_decoded_pixels: 8,
        ..GafLimits::default()
    };
    let meta = GafMetadata::parse_with_limits(&data, tight);
    let source = GafSource::new(data.clone(), tight);
    assert!(meta.is_err() && source.is_err());
    assert!(meta
        .unwrap_err()
        .to_string()
        .contains("aggregate decoded pixels exceed limit"));
    let meta = GafMetadata::parse_with_limits(&data, roomy).unwrap();
    assert!(GafSource::new(data.clone(), roomy).is_ok());
    assert_eq!(meta.entries.len(), 1);
    assert_eq!(meta.entries[0].frames[0].value, 3);
    GafMetadata::parse(&data).unwrap();
}

// ---- gaf_source_test.go ----

/// `transient` set on every frame, including direct views; returns the frame with
/// the hint cleared so it can be compared with an eager decode.
fn clear_transient(f: &GafFrame) -> GafFrame {
    assert!(f.transient, "source child lost transient hint");
    let mut out = f.clone();
    out.transient = false;
    out.direct_raster = f.direct_raster.as_ref().map(|d| {
        assert!(d.transient);
        let mut d = (**d).clone();
        d.transient = false;
        Arc::new(d)
    });
    out.subframes = f
        .subframes
        .iter()
        .map(|c| Arc::new(clear_transient(c)))
        .collect();
    out
}

#[test]
fn source_matches_eager_composite_and_owns_bytes() {
    let parent = GafWriteFrame {
        width: 3,
        height: 2,
        x_offset: 1,
        duration: 3,
        subframes: vec![
            GafWriteFrame {
                transparent: Some(vec![false, false]),
                ..leaf(2, 1, &[7, 9])
            },
            GafWriteFrame {
                x_offset: -1,
                alternate_blitter: 1,
                ..leaf(1, 2, &[5, 8])
            },
        ],
        ..GafWriteFrame::default()
    };
    let mut data = encode_gaf(&[entry("burst", vec![parent])]).unwrap();
    let eager = Gaf::parse(&data).unwrap();
    let encoded_len = data.len() as u64;
    let source = GafSource::new(data.clone(), GafLimits::default()).unwrap();
    data.fill(0);
    assert!(
        source.frame("BURST", 0, 9).is_err(),
        "root budget ignored children"
    );
    let got = source.frame("BURST", 0, 10).unwrap();
    assert_eq!(clear_transient(&got), *first_frame(&eager));
    let second = source.frame("burst", 0, 10).unwrap();
    assert!(
        !Arc::ptr_eq(&second, &got),
        "source retained a decoded frame"
    );
    assert!(second.transient);
    assert!(second.doubled().unwrap().subframes[0].transient);
    assert_eq!(source.encoded_bytes(), encoded_len);
}

#[test]
fn source_checks_unselected_payload_and_shared_child_cost() {
    let first = leaf(1, 1, &[7]);
    let second = GafWriteFrame {
        width: 1,
        height: 1,
        subframes: vec![leaf(1, 1, &[8]), leaf(1, 1, &[9])],
        ..GafWriteFrame::default()
    };
    let mut data = encode_gaf(&[entry("burst", vec![first, second])]).unwrap();
    let meta = GafMetadata::parse(&data).unwrap();
    let child_table = meta.entries[0].frames[1].frame.data_offset as usize;
    let c0 = get32(&data, child_table);
    put32(&mut data, child_table + 4, c0);
    let source = GafSource::new(data.clone(), GafLimits::default()).unwrap();
    assert!(
        source.frame("burst", 1, 2).is_err(),
        "root work budget ignored repeated child traversal"
    );
    let f = source.frame("burst", 1, 3).unwrap();
    assert!(
        Arc::ptr_eq(&f.subframes[0], &f.subframes[1]),
        "shared child was allocated twice"
    );
    let child = (*f.subframes[0]).clone();
    let mut clone = (*f).clone();
    clone.subframes = vec![Arc::clone(&f.subframes[0]), Arc::new(child.clone())];
    assert_eq!(
        gaf_frame_bytes(&clone) - gaf_frame_bytes(&f),
        gaf_frame_bytes(&child),
        "alias accounting charged the shared node twice"
    );
    // Damage the second frame's payload although a consumer might request only
    // frame zero: validation must still reject the whole bank.
    let damaged = &meta.entries[0].frames[1].frame.subframes[0];
    data.truncate(damaged.data_offset as usize);
    assert!(
        GafSource::new(data, GafLimits::default()).is_err(),
        "accepted malformed unselected frame"
    );
}

#[test]
fn source_rejects_bad_requests() {
    let data = encode_gaf(&[entry("one", vec![leaf(1, 1, &[1])])]).unwrap();
    let source = GafSource::new(data, GafLimits::default()).unwrap();
    assert_eq!(
        source.frame("one", 0, 0).unwrap_err(),
        GafError::InvalidFrameRequest
    );
    assert!(matches!(
        source.frame("one", 1, 8).unwrap_err(),
        GafError::MissingEntryFrame { .. }
    ));
    assert!(matches!(
        source.frame("two", 0, 8).unwrap_err(),
        GafError::MissingEntryFrame { .. }
    ));
    assert!(source.frame("ONE", 0, 8).is_ok());
}

// ---- gaf_variant_test.go ----

/// Doubled is a presentation rule, not retail behaviour.
#[test]
fn doubled_plain_frame() {
    let src = GafFrame {
        width: 2,
        height: 2,
        x_offset: 3,
        y_offset: -4,
        color_key: 9,
        compressed: 1,
        pixels: vec![1, 2, 9, 4],
        transparent: vec![false, false, true, false],
        ..GafFrame::default()
    };
    let got = src.doubled().unwrap();
    assert_eq!((got.width, got.height), (4, 4));
    assert_eq!((got.x_offset, got.y_offset), (6, -8));
    assert_eq!(got.color_key, 9);
    assert_eq!(
        got.compressed, src.compressed,
        "doubled frame must keep the source dispatch"
    );
    let want = [1u8, 1, 2, 2, 1, 1, 2, 2, 9, 9, 4, 4, 9, 9, 4, 4];
    assert_eq!(got.pixels, want);
    for (i, w) in want.iter().enumerate() {
        let clear = *w == 9 && (i / 4 >= 2 && i % 4 < 2);
        assert_eq!(got.transparent[i], clear, "transparency {i}");
    }
    assert_eq!(got.plain_pixels(), got.pixels.as_slice());
    // The 2x2 block of source (0,1) is transparent, the block of (1,1) opaque.
    assert!(!got.at(0, 3).1);
    assert_eq!(got.at(3, 3), (4, true));
    assert_eq!(
        (src.width, src.pixels[0]),
        (2, 1),
        "doubled mutated its source"
    );
}

/// A composite doubles leaf by leaf and keeps its composite fields.
#[test]
fn doubled_composite_keeps_leaves() {
    let leaf_frame = GafFrame {
        width: 1,
        height: 1,
        x_offset: 1,
        y_offset: 1,
        color_key: 9,
        pixels: vec![7],
        transparent: vec![false],
        alternate_blitter: 2,
        ..GafFrame::default()
    };
    let parent = GafFrame {
        width: 2,
        height: 1,
        x_offset: 2,
        color_key: 9,
        pixels: vec![9, 9],
        transparent: vec![true, true],
        subframe_count: 1,
        subframes: vec![Arc::new(leaf_frame)],
        ..GafFrame::default()
    };
    let got = parent.doubled().unwrap();
    assert_eq!((got.subframe_count, got.subframes.len()), (1, 1));
    let child = &got.subframes[0];
    assert_eq!(
        (child.width, child.height, child.x_offset, child.y_offset),
        (2, 2, 2, 2)
    );
    assert_eq!(child.alternate_blitter, 2);
    assert_eq!(got.x_offset, 4);
}

#[test]
fn doubled_short_planes_read_as_the_decoder_left_them() {
    // One pixel present, nothing for the rest: missing pixels are zero and, with no
    // transparency entry, a present pixel is opaque and an absent one transparent.
    let f = GafFrame {
        width: 2,
        height: 1,
        pixels: vec![5],
        ..GafFrame::default()
    };
    let d = f.doubled().unwrap();
    assert_eq!(d.pixels, vec![5, 5, 0, 0, 5, 5, 0, 0]);
    assert_eq!(
        d.transparent,
        vec![false, false, true, true, false, false, true, true]
    );
}

#[test]
fn doubled_refuses_oversized_hand_built_frames() {
    let f = GafFrame {
        width: 60_000,
        height: 60_000,
        ..GafFrame::default()
    };
    assert_eq!(f.doubled().unwrap_err(), GafError::DoubledFrameTooLarge);
}

// ---- gaf_write_test.go ----

#[test]
fn encode_gaf_round_trip() {
    // Entry one: opaque raw art. Entry two: two frames with skip runs, repeat runs,
    // an opaque colour-key index and a literal tail, so every RLE command shape runs.
    let opaque = GafWriteFrame {
        duration: 10,
        ..leaf(4, 2, &[1, 2, 3, 4, 5, 5, 5, 5])
    };
    let mut transparent = vec![false; 16];
    transparent[0] = true;
    transparent[1] = true;
    transparent[15] = true;
    let rle = GafWriteFrame {
        width: 8,
        height: 2,
        x_offset: -3,
        y_offset: 4,
        duration: 2,
        pixels: vec![0, 0, 9, 9, 9, 7, 7, 8, 1, 2, 3, 4, 4, 4, 4, 5],
        transparent: Some(transparent),
        ..GafWriteFrame::default()
    };
    let mut second = rle.clone();
    second.pixels[2] = 6;
    let data = encode_gaf(&[
        GafWriteEntry {
            name: "rm_one".into(),
            loop_: true,
            frames: vec![opaque.clone()],
        },
        entry("rm_two", vec![rle.clone(), second.clone()]),
    ])
    .unwrap();
    let g = Gaf::parse(&data).unwrap();
    assert_eq!(g.entries.len(), 2);
    assert_eq!(g.entries[0].name, b"rm_one");
    assert_eq!(g.entries[1].name, b"rm_two");
    assert_eq!((g.entries[0].unknown1, g.entries[1].unknown1), (1, 0));
    let check = |e: usize, f: usize, want: &GafWriteFrame| {
        let r = &g.entries[e].frames[f];
        let got = &r.frame;
        assert_eq!(
            (r.value, got.width, got.height, got.x_offset, got.y_offset),
            (
                want.duration,
                want.width,
                want.height,
                want.x_offset,
                want.y_offset
            )
        );
        for i in 0..want.pixels.len() {
            let clear = want.transparent.as_ref().is_some_and(|t| t[i]);
            assert_eq!(got.transparent[i], clear, "entry {e} frame {f} pixel {i}");
            if !clear {
                assert_eq!(
                    got.pixels[i], want.pixels[i],
                    "entry {e} frame {f} pixel {i}"
                );
            }
        }
    };
    check(0, 0, &opaque);
    check(1, 0, &rle);
    check(1, 1, &second);
    assert_eq!(g.entries[0].frames[0].frame.compressed, 0);
    assert_eq!(g.entries[1].frames[0].frame.compressed, 1);
}

#[test]
fn encode_gaf_round_trips_composite_alternate_byte() {
    let parent = GafWriteFrame {
        width: 5,
        height: 3,
        x_offset: 4,
        y_offset: -2,
        subframes: vec![
            GafWriteFrame {
                x_offset: 4,
                y_offset: -2,
                ..leaf(1, 1, &[12])
            },
            GafWriteFrame {
                x_offset: 3,
                y_offset: -2,
                alternate_blitter: 0xa5,
                ..leaf(1, 1, &[13])
            },
        ],
        ..GafWriteFrame::default()
    };
    let data = encode_gaf(&[entry("composite", vec![parent])]).unwrap();
    let g = Gaf::parse(&data).unwrap();
    let p = first_frame(&g);
    assert_eq!((p.subframe_count, p.subframes.len()), (2, 2));
    assert_eq!(p.subframes[1].alternate_blitter, 0xa5);
}

#[test]
fn encode_gaf_rejects_unreadable_entry_count_and_deep_children() {
    let blank = || entry("e", vec![leaf(1, 1, &[1])]);
    let many: Vec<GafWriteEntry> = (0..0x8000).map(|_| blank()).collect();
    assert!(
        encode_gaf(&many).is_err(),
        "writer admitted a negative signed low word"
    );
    let mut deep = leaf(1, 1, &[1]);
    for _ in 0..70 {
        deep = GafWriteFrame {
            width: 1,
            height: 1,
            subframes: vec![deep],
            ..GafWriteFrame::default()
        };
    }
    assert!(encode_gaf(&[entry("deep", vec![deep])]).is_err());
}

/// Hostile input never panics or allocates beyond its limits: try every truncation
/// and a byte flip at every position of a small valid composite bank.
#[test]
fn truncations_and_flips_never_panic() {
    let parent = GafWriteFrame {
        width: 3,
        height: 2,
        subframes: vec![
            GafWriteFrame {
                transparent: Some(vec![true, false, false]),
                ..leaf(3, 1, &[1, 2, 2])
            },
            leaf(1, 2, &[5, 6]),
        ],
        ..GafWriteFrame::default()
    };
    let data = encode_gaf(&[entry("a", vec![parent, leaf(2, 1, &[1, 2])])]).unwrap();
    for cut in 0..data.len() {
        let _ = Gaf::parse(&data[..cut]);
    }
    for i in 0..data.len() {
        for flip in [0x01u8, 0x80, 0xff] {
            let mut d = data.clone();
            d[i] ^= flip;
            let _ = Gaf::parse(&d);
            let _ = GafMetadata::parse(&d);
        }
    }
}

/// A frame whose pointer loops back to its own table is refused as a cycle.
#[test]
fn frame_cycle_is_an_error_not_a_hang() {
    let mut data = composite_chain(2);
    // The child table of frame 0 points at frame 0 itself.
    let f0 = first_frame_offset(&data);
    let table = get32(&data, f0 + 16) as usize;
    put32(&mut data, table, f0 as u32);
    let e = Gaf::parse(&data).unwrap_err();
    assert!(matches!(e.innermost(), GafError::FrameCycle { .. }), "{e}");
}
