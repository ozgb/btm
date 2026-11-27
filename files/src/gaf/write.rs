// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/gaf_write.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! Test-only GAF encoder, used to build fixtures. Writing is out of scope for the
//! crate, so this exists only under `cfg(test)`.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::metadata::to_usize;

/// One authored frame of an entry to encode. `transparent` is optional and parallel
/// to `pixels`; `None` means every pixel is opaque.
#[derive(Debug, Clone, Default)]
pub(super) struct GafWriteFrame {
    pub width: u16,
    pub height: u16,
    pub x_offset: i16,
    pub y_offset: i16,
    /// The frame reference's display count in whole simulation ticks.
    pub duration: u32,
    pub pixels: Vec<u8>,
    pub transparent: Option<Vec<bool>>,
    /// Makes this frame a composite; the effective count is one byte.
    pub subframes: Vec<GafWriteFrame>,
    pub alternate_blitter: u8,
}

/// One named sequence. `loop_` is written as the entry's loop byte.
#[derive(Debug, Clone, Default)]
pub(super) struct GafWriteEntry {
    pub name: String,
    pub loop_: bool,
    pub frames: Vec<GafWriteFrame>,
}

/// The raw-path transparent index every retail frame stores.
const GAF_COLOR_KEY: u8 = 9;

struct Plan<'a> {
    frame: &'a GafWriteFrame,
    children: Vec<usize>,
    header: u32,
    data_offset: u32,
    data: Vec<u8>,
    raw: bool,
}

fn plan_frame<'a>(
    plans: &mut Vec<Plan<'a>>,
    entry: &str,
    frame: &'a GafWriteFrame,
    depth: u32,
) -> Result<usize, String> {
    if depth > super::GafLimits::default().max_composite_depth {
        return Err("gaf encode: composite depth exceeds host limit".into());
    }
    if frame.width == 0 || frame.height == 0 {
        return Err(format!(
            "gaf encode: entry {entry:?} frame has empty dimensions"
        ));
    }
    let index = plans.len();
    plans.push(Plan {
        frame,
        children: Vec::new(),
        header: 0,
        data_offset: 0,
        data: Vec::new(),
        raw: false,
    });
    if !frame.subframes.is_empty() {
        if frame.subframes.len() > 0xff {
            return Err(format!(
                "gaf encode: entry {entry:?} composite has {} children",
                frame.subframes.len()
            ));
        }
        let mut children = Vec::new();
        for child in &frame.subframes {
            children.push(plan_frame(plans, entry, child, depth + 1)?);
        }
        plans[index].children = children;
        return Ok(index);
    }
    let n = usize::from(frame.width) * usize::from(frame.height);
    if frame.pixels.len() != n {
        return Err(format!(
            "gaf encode: entry {entry:?} frame: {} pixels for {}x{}",
            frame.pixels.len(),
            frame.width,
            frame.height
        ));
    }
    if let Some(t) = &frame.transparent {
        if t.len() != n {
            return Err(format!(
                "gaf encode: entry {entry:?} frame: transparency mask length {} for {n} pixels",
                t.len()
            ));
        }
    }
    let raw = (0..n).all(|p| {
        !frame.transparent.as_ref().is_some_and(|t| t[p]) && frame.pixels[p] != GAF_COLOR_KEY
    });
    plans[index].raw = raw;
    plans[index].data = if raw {
        frame.pixels.clone()
    } else {
        encode_rle(frame)
    };
    Ok(index)
}

/// Serialises entries into the GAF container: 12-byte header, entry offset table,
/// 40-byte entry headers each followed by its 8-byte frame references, then 24-byte
/// frame headers and pixel data or child tables. Entry names are at most 31 bytes.
pub(super) fn encode_gaf(entries: &[GafWriteEntry]) -> Result<Vec<u8>, String> {
    if entries.is_empty() || entries.len() > 0x7fff {
        return Err(format!(
            "gaf encode: entry count {} outside signed low-word range",
            entries.len()
        ));
    }
    const HEADER: usize = 12;
    const ENTRY: usize = 40;
    const REF: usize = 8;
    const FRAME: usize = 24;
    let mut entry_offsets = Vec::new();
    let mut offset = HEADER + 4 * entries.len();
    for entry in entries {
        if entry.name.is_empty() || entry.name.len() > 31 {
            return Err(format!(
                "gaf encode: entry name {:?} must be 1..31 bytes",
                entry.name
            ));
        }
        if entry.frames.is_empty() || entry.frames.len() > 0xFFFF {
            return Err(format!(
                "gaf encode: entry {:?} has {} frames",
                entry.name,
                entry.frames.len()
            ));
        }
        entry_offsets.push(offset);
        offset += ENTRY + REF * entry.frames.len();
    }
    let mut plans: Vec<Plan> = Vec::new();
    let mut roots: Vec<Vec<usize>> = Vec::new();
    for entry in entries {
        let mut r = Vec::new();
        for frame in &entry.frames {
            r.push(plan_frame(&mut plans, &entry.name, frame, 0)?);
        }
        roots.push(r);
    }
    for plan in &mut plans {
        plan.header = offset as u32;
        offset += FRAME;
    }
    for plan in &mut plans {
        plan.data_offset = offset as u32;
        if !plan.children.is_empty() {
            offset += plan.children.len() * 4;
        } else {
            offset += plan.data.len();
        }
    }
    let mut data = vec![0u8; offset];
    data[0..4].copy_from_slice(&0x0001_0100u32.to_le_bytes());
    data[4..8].copy_from_slice(&(entries.len() as u32).to_le_bytes());
    for (i, entry) in entries.iter().enumerate() {
        data[12 + i * 4..16 + i * 4].copy_from_slice(&(entry_offsets[i] as u32).to_le_bytes());
        let base = entry_offsets[i];
        data[base..base + 2].copy_from_slice(&(entry.frames.len() as u16).to_le_bytes());
        if entry.loop_ {
            data[base + 2..base + 4].copy_from_slice(&1u16.to_le_bytes());
        }
        data[base + 8..base + 8 + entry.name.len()].copy_from_slice(entry.name.as_bytes());
        for (f, frame) in entry.frames.iter().enumerate() {
            let at = base + ENTRY + f * REF;
            data[at..at + 4].copy_from_slice(&plans[roots[i][f]].header.to_le_bytes());
            data[at + 4..at + 8].copy_from_slice(&frame.duration.to_le_bytes());
        }
    }
    let headers: Vec<u32> = plans.iter().map(|p| p.header).collect();
    for plan in &plans {
        let frame = plan.frame;
        let fh = to_usize(plan.header);
        data[fh..fh + 2].copy_from_slice(&frame.width.to_le_bytes());
        data[fh + 2..fh + 4].copy_from_slice(&frame.height.to_le_bytes());
        data[fh + 4..fh + 6].copy_from_slice(&(frame.x_offset as u16).to_le_bytes());
        data[fh + 6..fh + 8].copy_from_slice(&(frame.y_offset as u16).to_le_bytes());
        data[fh + 8] = GAF_COLOR_KEY;
        data[fh + 11] = frame.alternate_blitter;
        if !plan.children.is_empty() {
            data[fh + 10] = plan.children.len() as u8;
            for (i, child) in plan.children.iter().enumerate() {
                let at = to_usize(plan.data_offset) + i * 4;
                data[at..at + 4].copy_from_slice(&headers[*child].to_le_bytes());
            }
        } else {
            if !plan.raw {
                data[fh + 9] = 1;
            }
            let at = to_usize(plan.data_offset);
            data[at..at + plan.data.len()].copy_from_slice(&plan.data);
        }
        data[fh + 16..fh + 20].copy_from_slice(&plan.data_offset.to_le_bytes());
    }
    Ok(data)
}

/// The per-row command stream: a u16 payload length per row, then skip runs (bit 0
/// set, count in the upper seven bits), repeat runs (bit 1 set, count-1 in the upper
/// six bits, one value byte) and literal runs (count-1 in the upper six bits, then
/// the bytes).
fn encode_rle(frame: &GafWriteFrame) -> Vec<u8> {
    let mut out = Vec::new();
    let w = usize::from(frame.width);
    for y in 0..usize::from(frame.height) {
        let row = &frame.pixels[y * w..(y + 1) * w];
        let mask = frame.transparent.as_ref().map(|t| &t[y * w..(y + 1) * w]);
        let transparent = |x: usize| mask.is_some_and(|m| m[x]);
        let mut payload: Vec<u8> = Vec::new();
        let mut x = 0;
        while x < w {
            if transparent(x) {
                let mut n = 0;
                while x + n < w && n < 127 && transparent(x + n) {
                    n += 1;
                }
                payload.push(((n << 1) as u8) | 1);
                x += n;
                continue;
            }
            let mut n = 1;
            while x + n < w && n < 64 && !transparent(x + n) && row[x + n] == row[x] {
                n += 1;
            }
            if n >= 3 {
                payload.push((((n - 1) << 2) as u8) | 2);
                payload.push(row[x]);
                x += n;
                continue;
            }
            // Literal run: stop before a transparent pixel or a run of three.
            n = 0;
            while x + n < w && n < 64 && !transparent(x + n) {
                if n >= 1
                    && x + n + 1 < w
                    && row[x + n] == row[x + n - 1]
                    && !transparent(x + n + 1)
                    && row[x + n + 1] == row[x + n]
                {
                    n -= 1; // leave the repeat for the next command
                    break;
                }
                n += 1;
            }
            if n == 0 {
                n = 1;
            }
            payload.push(((n - 1) << 2) as u8);
            payload.extend_from_slice(&row[x..x + n]);
            x += n;
        }
        out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        out.extend_from_slice(&payload);
    }
    out
}
